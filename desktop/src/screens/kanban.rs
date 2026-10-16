//! The kanban view: tasks in one column per status.
//!
//! A card opens its entry in the pane; dragging it to another column sets
//! the task's status on the server. Entries without a task are not shown.

use std::path::PathBuf;

use sapphire_framework::gui::egui;

use sapphire_journal_core::entry::EntryHeader;

use crate::app::HomeState;
use crate::icons;
use crate::screens::entry_pane::TASK_STATUS_OPTIONS;

/// The narrowest a column gets; below that the board scrolls sideways.
const MIN_COLUMN_WIDTH: f32 = 200.0;

/// What a card carries while it is dragged.
struct CardPayload {
    path: PathBuf,
    status: String,
}

/// Draw `entries` (filtered and sorted already) as a board.
pub fn show(home: &mut HomeState, entries: &[EntryHeader], ui: &mut egui::Ui) {
    let tasks: Vec<(&EntryHeader, &str)> = entries
        .iter()
        .filter_map(|h| Some((h, h.frontmatter.task.as_ref()?.status.as_str())))
        .collect();

    // The known statuses in order, then any other a task uses.
    let mut columns: Vec<&str> = TASK_STATUS_OPTIONS.to_vec();
    for (_, status) in &tasks {
        if !columns.contains(status) {
            columns.push(status);
        }
    }

    let spacing = ui.spacing().item_spacing.x;
    let n = columns.len() as f32;
    let width = ((ui.available_width() - spacing * (n - 1.0)) / n).max(MIN_COLUMN_WIDTH);
    let height = ui.available_height();

    let mut open: Option<PathBuf> = None;
    let mut moved: Option<(PathBuf, String)> = None;
    egui::ScrollArea::horizontal()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for status in &columns {
                    let cards: Vec<&EntryHeader> = tasks
                        .iter()
                        .filter(|(_, s)| s == status)
                        .map(|(h, _)| *h)
                        .collect();
                    let layout = egui::Layout::top_down(egui::Align::Min);
                    ui.allocate_ui_with_layout(egui::vec2(width, height), layout, |ui| {
                        ui.set_width(width);
                        if let Some(payload) =
                            draw_column(home, ui, status, &cards, height, &mut open)
                            && payload.status != *status
                        {
                            moved = Some((payload.path.clone(), status.to_string()));
                        }
                    });
                }
            });
        });

    if let Some(path) = open {
        home.open(path);
    }
    if let Some((path, status)) = moved {
        home.api.set_task_status(path, status);
    }
}

/// One status's column; the card dropped on it, if one was.
fn draw_column(
    home: &HomeState,
    ui: &mut egui::Ui,
    status: &str,
    cards: &[&EntryHeader],
    height: f32,
    open: &mut Option<PathBuf>,
) -> Option<std::sync::Arc<CardPayload>> {
    let frame = egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::same(6));
    let (_, dropped) = ui.dnd_drop_zone::<CardPayload, _>(frame, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(height - 16.0);
        ui.horizontal(|ui| {
            ui.strong(status_label(status));
            ui.weak(cards.len().to_string());
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt(("kanban_column", status))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for header in cards {
                    if draw_card(home, ui, header, status).clicked() {
                        *open = Some(PathBuf::from(&header.path));
                    }
                    ui.add_space(4.0);
                }
            });
    });
    dropped
}

/// A card: draggable, and clickable to open the entry.
fn draw_card(
    home: &HomeState,
    ui: &mut egui::Ui,
    header: &EntryHeader,
    status: &str,
) -> egui::Response {
    let path = PathBuf::from(&header.path);
    let is_active = home.selected_path.as_ref() == Some(&path);
    let dnd_id = egui::Id::new(("kanban_card", header.id()));

    // As the tree's rows do: a press-and-release is a click, a press-and-
    // drag a drag (the stock `dnd_drag_source` would swallow the click).
    if ui.ctx().is_being_dragged(dnd_id) {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            CardPayload {
                path,
                status: status.to_string(),
            },
        );
        let layer_id = egui::LayerId::new(egui::Order::Tooltip, dnd_id);
        let inner = ui.scope_builder(egui::UiBuilder::new().layer_id(layer_id), |ui| {
            card_ui(ui, header, is_active)
        });
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            let delta = pos - inner.response.rect.center();
            ui.ctx().transform_layer_shapes(
                layer_id,
                egui::emath::TSTransform::from_translation(delta),
            );
        }
        return inner.response;
    }
    let inner = ui.scope(|ui| card_ui(ui, header, is_active));
    ui.interact(inner.response.rect, dnd_id, egui::Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::Grab)
}

fn card_ui(ui: &mut egui::Ui, header: &EntryHeader, is_active: bool) {
    let visuals = ui.visuals();
    let fill = if is_active {
        visuals.selection.bg_fill
    } else {
        visuals.window_fill
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let title = if header.title().is_empty() {
                "(untitled)"
            } else {
                header.title()
            };
            ui.add(egui::Label::new(egui::RichText::new(title).strong()).wrap());
            let due = header.frontmatter.task.as_ref().and_then(|t| t.due);
            let tags = &header.frontmatter.tags;
            if due.is_some() || !tags.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    if let Some(due) = due {
                        let tint = ui.visuals().weak_text_color();
                        ui.add(
                            egui::Image::new(icons::calendar())
                                .fit_to_exact_size(egui::vec2(12.0, 12.0))
                                .tint(tint),
                        );
                        ui.weak(due.format("%Y-%m-%d").to_string());
                    }
                    for tag in tags {
                        ui.weak(format!("#{tag}"));
                    }
                });
            }
        });
}

fn status_label(status: &str) -> &str {
    match status {
        "open" => "Open",
        "in_progress" => "In progress",
        "done" => "Done",
        "cancelled" => "Cancelled",
        "archived" => "Archived",
        other => other,
    }
}
