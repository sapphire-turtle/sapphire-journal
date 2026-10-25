use std::collections::HashSet;
use std::path::PathBuf;

use chrono::NaiveDate;
use grain_id::GrainId;
use sapphire_framework::gui::egui;

use sapphire_journal_core::{
    entry::EntryHeader,
    ops::{
        EntryFilter, EntryTreeNode, FieldSelector, SortField as CoreSortField,
        SortOrder as CoreSortOrder, build_entry_tree,
    },
    period::{Period, parse_period},
};

use crate::app::{App, HomeState, Pending};
use crate::icons;
use crate::screens::entry_pane::{self, PaneAction};
use crate::screens::kanban;
use crate::settings::{ListLayout, MainView, PanePlacement};
use crate::widgets::buttons::{icon_btn, icon_text_btn, icon_toggle_btn};
use crate::widgets::period_picker::period_picker;

/// The journal screen: a view of the entries, and the pane the open one is
/// edited in. Everything it shows comes from the running server's
/// `sapphire-journal.*` methods, so it is the journal the server serves.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let open = app.home.selected_path.is_some();
    let mut pane_action = None;

    if open && app.home.pane_placement == PanePlacement::Side {
        let panel = egui::Panel::right("entry_pane")
            .resizable(true)
            .default_size(app.home.pane_width)
            .size_range(300.0..=720.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                entry_pane::show(&mut app.home, ui)
            });
        pane_action = panel.inner;
        // Remember a resize once the drag is over.
        let width = panel.response.rect.width();
        if (width - app.home.pane_width).abs() > 1.0 && !ui.ctx().input(|i| i.pointer.any_down()) {
            app.home.pane_width = width;
            app.settings.pane.width = width;
            save_settings(app);
        }
    }

    egui::CentralPanel::default().show(ui, |ui| {
        draw_toolbar(app, ui);
        let home = &mut app.home;
        if let Some(msg) = home.error_msg.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::LIGHT_RED, msg);
                if ui.small_button("×").clicked() {
                    home.error_msg = None;
                }
            });
        }
        if let Some(msg) = &home.entries_error {
            ui.colored_label(egui::Color32::LIGHT_RED, msg);
            ui.weak("Pick or create a journal in Settings → Workspace, and make sure the sapphire-journal server is running.");
        }
        ui.separator();

        let filtered = filter_and_sort(home);
        match home.view {
            MainView::List => draw_list(home, filtered, ui),
            MainView::Kanban => kanban::show(home, &filtered, ui),
        }
    });

    if open && app.home.pane_placement == PanePlacement::Floating {
        let ctx = ui.ctx().clone();
        let title = entry_pane::title(&app.home);
        egui::Window::new(title)
            .id(egui::Id::unique("entry_pane_window"))
            .title_bar(false)
            .resizable(true)
            .default_size([420.0, 560.0])
            // Where the docked pane would be: at the right, under the toolbar.
            .pivot(egui::Align2::RIGHT_TOP)
            .default_pos(ctx.content_rect().right_top() + egui::vec2(-16.0, 48.0))
            .min_width(300.0)
            .constrain_to(ctx.content_rect())
            .show(&ctx, |ui| {
                pane_action = entry_pane::show(&mut app.home, ui);
            });
    }

    match pane_action {
        Some(PaneAction::Place(placement)) => {
            app.home.pane_placement = placement;
            app.settings.pane.placement = placement;
            save_settings(app);
        }
        None => {}
    }

    let ctx = ui.ctx().clone();
    if app.home.confirm_delete.is_some() {
        draw_confirm_delete_entry(&mut app.home, &ctx);
    }
    if app.home.unsaved.is_some() {
        draw_confirm_unsaved(&mut app.home, &ctx);
    }
}

fn save_settings(app: &mut App) {
    if let Err(e) = app.settings.save() {
        app.home.error_msg = Some(format!("Failed to save settings: {e}"));
    }
}

// ── Toolbar ──────────────────────────────────────────────────────────────────

/// The view's toolbar: the journal menu, the period and search. Which view
/// is shown is picked on the rail.
fn draw_toolbar(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        draw_journal_menu(app, ui);
        let home = &mut app.home;
        let entries = &home.entries;
        period_picker(ui, &mut home.period, &mut home.calendar_cursor, &|d| {
            count_on(entries, d)
        });
        ui.add(
            egui::TextEdit::singleline(&mut home.filter_text)
                .hint_text("Search title / tag / id")
                .desired_width(200.0),
        );
        if home.sync_in_progress {
            ui.add(egui::Spinner::new().size(14.0));
            // While a refresh is running, keep painting so the spinner
            // animates even without user input.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_text_btn(ui, icons::plus(), "New Entry").clicked() {
                home.new_entry(None);
            }
            if icon_btn(ui, icons::refresh(), "Refresh entry list").clicked() {
                home.reload();
            }
            if home.view == MainView::List {
                // Show the icon for the *other* layout (what clicking would switch to).
                let (icon, tooltip) = match home.list_layout {
                    ListLayout::Tree => (icons::list_view(), "Show as a flat list"),
                    ListLayout::Flat => (icons::tree_view(), "Show as a tree"),
                };
                if icon_btn(ui, icon, tooltip).clicked() {
                    home.list_layout = match home.list_layout {
                        ListLayout::Tree => ListLayout::Flat,
                        ListLayout::Flat => ListLayout::Tree,
                    };
                }
            }
            let filters_tooltip = if home.show_filters {
                "Hide sort options"
            } else {
                "Show sort options"
            };
            if icon_toggle_btn(ui, icons::funnel(), home.show_filters, filters_tooltip).clicked() {
                home.show_filters = !home.show_filters;
            }
        });
    });

    let home = &mut app.home;
    if home.show_filters {
        ui.horizontal(|ui| {
            ui.label("Sort by");
            egui::ComboBox::from_id_salt("home_sort_by")
                .selected_text(sort_label(&home.sort_by))
                .show_ui(ui, |ui| {
                    for (value, label) in SORT_OPTIONS {
                        ui.selectable_value(&mut home.sort_by, (*value).to_string(), *label);
                    }
                });
            egui::ComboBox::from_id_salt("home_sort_order")
                .selected_text(if home.sort_order == "asc" {
                    "Ascending"
                } else {
                    "Descending"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut home.sort_order, "desc".to_string(), "Descending");
                    ui.selectable_value(&mut home.sort_order, "asc".to_string(), "Ascending");
                });
        });
    }
    ui.add_space(4.0);
}

/// How many entries fall on `date`, for the period picker's dots. Mirrors
/// the filter applied in `filter_and_sort`, so the dots match what the view
/// shows after a click.
fn count_on(entries: &[EntryHeader], date: NaiveDate) -> usize {
    let filter = period_filter(Period::Range(
        date.and_hms_opt(0, 0, 0).unwrap(),
        date.and_hms_opt(23, 59, 59).unwrap(),
    ));
    entries.iter().filter(|e| filter.matches(e).0).count()
}

fn period_filter(period: Period) -> EntryFilter {
    EntryFilter {
        period: Some(period),
        fields: FieldSelector::active(),
        task_status: Vec::new(),
        tags: Vec::new(),
        sort_by: CoreSortField::Unsorted,
        sort_order: CoreSortOrder::Asc,
        // GUI は stale/hidden をまだ扱わない（別タスク）。従来どおり全件表示。
        include_stale: true,
        include_hidden: true,
        stale_after_days: 30,
    }
}

// ── List view ────────────────────────────────────────────────────────────────

fn draw_list(home: &mut HomeState, filtered: Vec<EntryHeader>, ui: &mut egui::Ui) {
    if filtered.is_empty() {
        ui.add_space(8.0);
        ui.weak("No entries match.");
        return;
    }

    let mut reparent_action: Option<(PathBuf, Option<GrainId>)> = None;
    let mut entry_action: Option<EntryAction> = None;
    let mut want_select: Option<PathBuf> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| match home.list_layout {
            ListLayout::Flat => {
                for header in &filtered {
                    let path = PathBuf::from(header.path.clone());
                    let is_active = home.selected_path.as_ref() == Some(&path);
                    if draw_entry_row(ui, header, is_active, &mut entry_action).clicked() {
                        want_select = Some(path);
                    }
                }
            }
            ListLayout::Tree => {
                // Root drop zone: dropping above the first row clears `parent_id`.
                draw_root_drop_zone(ui, &home.entries, &mut reparent_action);
                let pairs = filtered.into_iter().map(|h| (h, Vec::new())).collect();
                let tree = build_entry_tree(pairs);
                draw_tree(
                    ui,
                    &tree,
                    0,
                    &mut home.collapsed,
                    &home.selected_path,
                    &mut want_select,
                    &home.entries,
                    &mut reparent_action,
                    &mut entry_action,
                );
            }
        });

    if let Some(path) = want_select {
        home.open(path);
    }
    match entry_action {
        Some(EntryAction::AddChild(parent_id)) => home.new_entry(Some(parent_id)),
        Some(EntryAction::RequestDelete(path)) => home.confirm_delete = Some(path),
        None => {}
    }
    if let Some((source_path, new_parent)) = reparent_action {
        home.api.reparent(source_path, new_parent);
    }
}

#[derive(Clone)]
enum EntryAction {
    AddChild(GrainId),
    RequestDelete(PathBuf),
}

fn entry_row_context_menu(
    response: &egui::Response,
    header: &EntryHeader,
    pending: &mut Option<EntryAction>,
) {
    response.context_menu(|ui| {
        if ui.button("Add child entry").clicked() {
            *pending = Some(EntryAction::AddChild(header.id()));
            ui.close();
        }
        if ui.button("Delete entry…").clicked() {
            *pending = Some(EntryAction::RequestDelete(PathBuf::from(
                header.path.clone(),
            )));
            ui.close();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_tree(
    ui: &mut egui::Ui,
    nodes: &[EntryTreeNode],
    depth: usize,
    collapsed: &mut HashSet<GrainId>,
    selected: &Option<PathBuf>,
    want_select: &mut Option<PathBuf>,
    entries: &[EntryHeader],
    reparent: &mut Option<(PathBuf, Option<GrainId>)>,
    pending: &mut Option<EntryAction>,
) {
    for node in nodes {
        let id = node.entry.frontmatter.id;
        let has_children = !node.children.is_empty();
        let is_collapsed = collapsed.contains(&id);
        let path = PathBuf::from(node.entry.path.clone());
        let is_active = selected.as_ref() == Some(&path);

        let row = ui
            .horizontal(|ui| {
                // Indent + collapse toggle (or spacer for leaves). The spacer
                // is allocated as a widget (not raw `add_space`) so the same
                // `item_spacing.x` is inserted on each side as for the chevron
                // button — keeping leaf rows aligned with parent rows.
                ui.add_space(depth as f32 * 12.0);
                if has_children {
                    let icon = if is_collapsed {
                        icons::chevron_right()
                    } else {
                        icons::chevron_down()
                    };
                    if tree_toggle(ui, icon).clicked() {
                        if is_collapsed {
                            collapsed.remove(&id);
                        } else {
                            collapsed.insert(id);
                        }
                    }
                } else {
                    ui.allocate_exact_size(
                        egui::vec2(TREE_TOGGLE_SIZE, TREE_TOGGLE_SIZE),
                        egui::Sense::hover(),
                    );
                }
                let dnd_id = egui::Id::unique(("entry_dnd", id));
                // Inlined `dnd_drag_source` using `Sense::click_and_drag()` so
                // a press-and-release without movement registers as a click
                // (for selection) while a press-and-drag still starts a drag.
                // The stock `dnd_drag_source` overlays a `Sense::drag()` widget
                // that shadows the row's click sense, so clicks were lost.
                if ui.ctx().is_being_dragged(dnd_id) {
                    egui::DragAndDrop::set_payload(ui.ctx(), id);
                    let layer_id = egui::LayerId::new(egui::Order::Tooltip, dnd_id);
                    let inner = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
                        draw_entry_row(ui, &node.entry, is_active, pending)
                    });
                    if let Some(pos) = ui.ctx().pointer_interact_pos() {
                        let delta = pos - inner.response.rect.center();
                        ui.ctx().transform_layer_shapes(
                            layer_id,
                            egui::emath::TSTransform::from_translation(delta),
                        );
                    }
                    inner.response
                } else {
                    let inner = ui.scope(|ui| draw_entry_row(ui, &node.entry, is_active, pending));
                    let outer = ui
                        .interact(inner.response.rect, dnd_id, egui::Sense::click_and_drag())
                        .on_hover_cursor(egui::CursorIcon::Grab);
                    entry_row_context_menu(&outer, &node.entry, pending);
                    outer
                }
            })
            .inner;

        if row.clicked() {
            *want_select = Some(path.clone());
        }

        // ── drag-and-drop drop target ────────────────────────────────────
        // Highlight the row when something is being dragged over it.
        if let Some(payload) = row.dnd_hover_payload::<GrainId>() {
            let src = *payload;
            let would_be_cycle = is_descendant_or_self(entries, src, id);
            let stroke = if would_be_cycle {
                egui::Stroke::new(1.5, egui::Color32::DARK_RED)
            } else {
                egui::Stroke::new(1.5, ui.visuals().selection.bg_fill)
            };
            ui.painter()
                .rect_stroke(row.rect, 4.0, stroke, egui::StrokeKind::Inside);
        }
        if let Some(payload) = row.dnd_release_payload::<GrainId>() {
            let src = *payload;
            if src != id
                && !is_descendant_or_self(entries, src, id)
                && let Some(header) = entries.iter().find(|e| e.frontmatter.id == src)
            {
                // Only act when the parent actually changes.
                if header.frontmatter.parent_id != Some(id) {
                    *reparent = Some((PathBuf::from(header.path.clone()), Some(id)));
                }
            }
        }

        if has_children && !is_collapsed {
            draw_tree(
                ui,
                &node.children,
                depth + 1,
                collapsed,
                selected,
                want_select,
                entries,
                reparent,
                pending,
            );
        }
    }
}

/// Returns `true` if `candidate` is `ancestor` itself, or any descendant of
/// `ancestor`, by walking the `parent_id` chain on `candidate`.  Used to
/// prevent the user from dropping an entry into its own subtree.
fn is_descendant_or_self(entries: &[EntryHeader], ancestor: GrainId, candidate: GrainId) -> bool {
    if candidate == ancestor {
        return true;
    }
    let mut current = candidate;
    let mut seen: HashSet<GrainId> = HashSet::new();
    loop {
        if !seen.insert(current) {
            return false;
        }
        let entry = match entries.iter().find(|e| e.frontmatter.id == current) {
            Some(e) => e,
            None => return false,
        };
        match entry.frontmatter.parent_id {
            Some(pid) if pid == ancestor => return true,
            Some(pid) => current = pid,
            None => return false,
        }
    }
}

/// Thin strip at the top of the tree that accepts drops to clear `parent_id`
/// (make the entry top-level).  Only renders visibly when a drag is in
/// progress, so it doesn't add chrome the rest of the time.
fn draw_root_drop_zone(
    ui: &mut egui::Ui,
    entries: &[EntryHeader],
    reparent: &mut Option<(PathBuf, Option<GrainId>)>,
) {
    let is_dragging = egui::DragAndDrop::has_payload_of_type::<GrainId>(ui.ctx());
    if !is_dragging {
        return;
    }
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
    let visuals = ui.visuals();
    let is_hover = resp.contains_pointer();
    let bg = if is_hover {
        visuals.selection.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    ui.painter().rect_filled(rect, 4.0, bg);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Drop here for top level",
        egui::FontId::proportional(11.0),
        visuals.text_color(),
    );
    if let Some(payload) = resp.dnd_release_payload::<GrainId>() {
        let src = *payload;
        if let Some(header) = entries.iter().find(|e| e.frontmatter.id == src)
            && header.frontmatter.parent_id.is_some()
        {
            *reparent = Some((PathBuf::from(header.path.clone()), None));
        }
    }
}

fn draw_entry_row(
    ui: &mut egui::Ui,
    header: &EntryHeader,
    is_active: bool,
    pending: &mut Option<EntryAction>,
) -> egui::Response {
    let title = if header.title().is_empty() {
        "(untitled)".to_string()
    } else {
        header.title().to_string()
    };
    let id_str = header.id().to_string();
    let tags_str = if header.frontmatter.tags.is_empty() {
        String::new()
    } else {
        header
            .frontmatter
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ")
    };

    let frame = if is_active {
        egui::Frame::new()
            .fill(ui.visuals().selection.bg_fill)
            .inner_margin(egui::Margin::symmetric(6, 4))
            .corner_radius(4.0)
    } else {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(6, 4))
    };

    let resp = frame
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let tint = ui.visuals().strong_text_color();
                    for &flag in &header.flags {
                        ui.add(
                            egui::Image::new(icons::flag_icon(flag))
                                .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                .tint(tint),
                        )
                        .on_hover_text(flag.as_str());
                    }
                    ui.add(egui::Label::new(egui::RichText::new(&title).strong()).truncate());
                });
                ui.horizontal(|ui| {
                    ui.weak(format!("@{id_str}"));
                    if !tags_str.is_empty() {
                        ui.weak(tags_str);
                    }
                });
            });
        })
        .response;

    // Make the row clickable.
    let resp = ui.interact(resp.rect, resp.id.with("row_btn"), egui::Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    entry_row_context_menu(&resp, header, pending);
    resp
}

// ── Dialogs ──────────────────────────────────────────────────────────────────

fn draw_confirm_delete_entry(home: &mut HomeState, ctx: &egui::Context) {
    let mut cancel = false;
    let mut confirm = false;
    let mut open = true;

    egui::Window::new("Delete entry?")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.label("This will permanently delete the entry file.");
            ui.label("This action cannot be undone.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Delete").clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        });

    if !open || cancel {
        home.confirm_delete = None;
    }
    if confirm && let Some(path) = home.confirm_delete.take() {
        home.api.remove(path);
    }
}

/// Asked before something would throw away the open entry's changes:
/// save them first, drop them, or stay.
fn draw_confirm_unsaved(home: &mut HomeState, ctx: &egui::Context) {
    let title = entry_pane::title(home);
    let quitting = home.unsaved == Some(Pending::Quit);
    let mut save = false;
    let mut discard = false;
    let mut cancel = false;

    let modal = egui::Modal::new(egui::Id::unique("unsaved_changes")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.heading("Unsaved changes");
        ui.add_space(4.0);
        ui.label(format!("“{title}” has changes that are not saved."));
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Save").clicked() {
                save = true;
            }
            let discard_label = if quitting {
                "Quit without saving"
            } else {
                "Discard"
            };
            if ui.button(discard_label).clicked() {
                discard = true;
            }
            if ui.button("Cancel").clicked() {
                cancel = true;
            }
        });
    });

    if save {
        let pending = home.unsaved.take();
        // A form that does not make an entry stays open, with the reason.
        if entry_pane::save_current_entry(home) {
            home.after_save = pending;
        }
    } else if discard {
        if let Some(pending) = home.unsaved.take() {
            home.perform(pending);
        }
    } else if cancel || modal.should_close() {
        home.unsaved = None;
    }
}

// ── Filtering ────────────────────────────────────────────────────────

fn filter_and_sort(home: &HomeState) -> Vec<EntryHeader> {
    let mut headers = home.entries.clone();

    // Period filter via core EntryFilter.
    if !home.period.trim().is_empty()
        && let Ok(period) = parse_period(home.period.trim())
    {
        let filter = period_filter(period);
        headers.retain(|h| filter.matches(h).0);
    }

    // Substring filter on title / tags / id.
    let needle = home.filter_text.trim().to_lowercase();
    if !needle.is_empty() {
        headers.retain(|h| {
            h.title().to_lowercase().contains(&needle)
                || h.id().to_string().to_lowercase().contains(&needle)
                || h.frontmatter
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&needle))
        });
    }

    // Sort.
    match home.sort_by.as_str() {
        "title" => headers.sort_by(|a, b| a.title().cmp(b.title())),
        "id" => headers.sort_by_key(|a| a.id()),
        "created_at" => headers.sort_by_key(|a| a.frontmatter.created_at),
        "updated_at" => headers.sort_by_key(|a| a.frontmatter.updated_at),
        "task_due" => headers.sort_by(|a, b| {
            let av = a.frontmatter.task.as_ref().and_then(|t| t.due);
            let bv = b.frontmatter.task.as_ref().and_then(|t| t.due);
            cmp_opt(av, bv)
        }),
        "event_start" => headers.sort_by(|a, b| {
            let av = a.frontmatter.event.as_ref().map(|e| e.start);
            let bv = b.frontmatter.event.as_ref().map(|e| e.start);
            cmp_opt(av, bv)
        }),
        _ => {}
    }
    if home.sort_order == "desc" {
        headers.reverse();
    }
    headers
}

fn cmp_opt<T: Ord>(a: Option<T>, b: Option<T>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

// ── Journal menu ────────────────────────────────────────────────────────────

/// Top-left journal menu: refresh and settings. Which journal is served is
/// chosen in Settings → Workspace.
fn draw_journal_menu(app: &mut App, ui: &mut egui::Ui) {
    let name = app
        .home
        .journal_root
        .as_ref()
        .and_then(|root| root.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Journal".to_string());

    let mut refresh_now = false;
    let mut open_settings = false;
    let mut choose_journal = false;

    let caret = egui::Image::new(icons::chevron_down())
        .fit_to_exact_size(egui::vec2(14.0, 14.0))
        .tint(ui.visuals().text_color());
    let root = app
        .home
        .journal_root
        .as_ref()
        .map(|root| root.display().to_string());
    let menu = ui.menu_button((name, caret), |ui| {
        if ui.button("Refresh").clicked() {
            refresh_now = true;
            ui.close();
        }
        if ui.button("Settings…").clicked() {
            open_settings = true;
            ui.close();
        }
        if ui.button("Choose Journal…").clicked() {
            choose_journal = true;
            ui.close();
        }
    });
    if let Some(root) = root {
        menu.response.on_hover_text(root);
    }

    if refresh_now {
        let home = &mut app.home;
        // A previous refresh may still be out; don't stack another.
        if home.sync_in_progress {
            home.error_msg = Some("Refresh already in progress".to_string());
        } else {
            home.sync_in_progress = true;
            home.error_msg = None;
            home.api.refresh();
        }
    } else if open_settings || choose_journal {
        // Settings holds both the journal's page and the workspace picker.
        app.open_settings = true;
    }
}

// ── Display helpers ─────────────────────────────────────────────────────────

/// Exact width reserved for the tree expand/collapse toggle (and the
/// matching leaf spacer).  Kept narrow so titles align with leaf rows.
const TREE_TOGGLE_SIZE: f32 = 14.0;

/// Frameless expand/collapse triangle for tree rows.  Matches `TREE_TOGGLE_SIZE`
/// exactly so leaf rows (which insert a spacer of the same width) stay aligned.
fn tree_toggle(ui: &mut egui::Ui, src: egui::ImageSource<'static>) -> egui::Response {
    let tint = ui.visuals().weak_text_color();
    let img = egui::Image::new(src)
        .fit_to_exact_size(egui::vec2(TREE_TOGGLE_SIZE, TREE_TOGGLE_SIZE))
        .tint(tint);
    ui.add_sized(
        [TREE_TOGGLE_SIZE, TREE_TOGGLE_SIZE],
        egui::Button::image(img).frame(false),
    )
}

const SORT_OPTIONS: &[(&str, &str)] = &[
    ("updated_at", "Updated"),
    ("created_at", "Created"),
    ("title", "Title"),
    ("id", "ID"),
    ("task_due", "Task due"),
    ("event_start", "Event start"),
];

fn sort_label(value: &str) -> &'static str {
    SORT_OPTIONS
        .iter()
        .find(|(v, _)| *v == value)
        .map(|(_, l)| *l)
        .unwrap_or("Updated")
}
