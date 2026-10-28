//! The open entry: its form, under a header with its info button and its
//! menu.
//!
//! Every view opens entries in this one pane, so editing looks the same
//! from the list as from the kanban. It is drawn in the shell's side pane,
//! which decides where it sits: a column at the right, a card over the view,
//! or the view's place on a phone.

use sapphire_framework::gui::egui;

use sapphire_journal_core::{entry::Entry, entry::EntryHeader, labels::EntryFlag};

use crate::app::{EditorState, HomeState};
use crate::icons;
use crate::widgets::buttons::{icon_btn, icon_text_btn};
use crate::widgets::date_picker::date_picker_button;

/// Draw the pane for the open entry into `ui`.
pub fn show(home: &mut HomeState, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    draw_header(home, ui);
    ui.separator();
    if home.editor.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.spinner();
        });
        return;
    }
    draw_form(home, ui);
}

/// The open entry's title, for the unsaved-changes question.
pub fn title(home: &HomeState) -> String {
    match &home.editor {
        Some(editor) if !editor.title.trim().is_empty() => editor.title.clone(),
        Some(_) => "(untitled)".to_string(),
        None => "Loading…".to_string(),
    }
}

// ── Header ───────────────────────────────────────────────────────────────────

fn draw_header(home: &mut HomeState, ui: &mut egui::Ui) {
    let dirty = home.is_dirty();
    let save_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
    let mut save = home.editor.is_some() && ui.input_mut(|i| i.consume_shortcut(&save_shortcut));
    let mut close = false;
    let mut delete = false;
    let mut revert = false;
    let mut add_child = false;

    ui.horizontal(|ui| {
        if let Some(editor) = &home.editor {
            if dirty {
                ui.label(egui::RichText::new("●").color(ui.visuals().warn_fg_color))
                    .on_hover_text("Unsaved changes");
            }
            ui.weak(format!("@{}", editor.id));
        }
        if let Some(msg) = &home.info_msg {
            ui.colored_label(egui::Color32::LIGHT_GREEN, msg);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_btn(ui, icons::close(), "Close").clicked() {
                close = true;
            }
            if home.editor.is_none() {
                return;
            }
            draw_menu(
                ui,
                dirty,
                MenuPicks {
                    save: &mut save,
                    revert: &mut revert,
                    add_child: &mut add_child,
                    delete: &mut delete,
                    close: &mut close,
                },
            );
            draw_info_button(home, ui);
            let save_btn = ui
                .add_enabled(dirty, egui::Button::new("Save"))
                .on_hover_text(save_shortcut_text(ui.ctx(), &save_shortcut));
            if save_btn.clicked() {
                save = true;
            }
        });
    });

    if save {
        save_current_entry(home);
    }
    if revert && let Some(editor) = &home.editor {
        home.editor = Some(editor_for(editor.base.clone()));
    }
    if add_child && let Some(editor) = &home.editor {
        let parent = editor.base.frontmatter.id;
        home.new_entry(Some(parent));
    }
    if delete {
        home.confirm_delete = home.selected_path.clone();
    }
    if close {
        home.close_entry();
    }
}

fn save_shortcut_text(ctx: &egui::Context, shortcut: &egui::KeyboardShortcut) -> String {
    format!("Save ({})", ctx.format_shortcut(shortcut))
}

/// What the menu's items were clicked for.
struct MenuPicks<'a> {
    save: &'a mut bool,
    revert: &'a mut bool,
    add_child: &'a mut bool,
    delete: &'a mut bool,
    close: &'a mut bool,
}

/// The ⋯ menu: what can be done to the open entry.
fn draw_menu(ui: &mut egui::Ui, dirty: bool, picks: MenuPicks<'_>) {
    let tint = ui.visuals().text_color();
    let img = egui::Image::new(icons::ellipsis())
        .fit_to_exact_size(egui::vec2(16.0, 16.0))
        .tint(tint);
    let button = ui.add(egui::Button::image(img)).on_hover_text("Entry menu");
    egui::Popup::menu(&button).show(|ui| {
        if ui.add_enabled(dirty, egui::Button::new("Save")).clicked() {
            *picks.save = true;
        }
        if ui
            .add_enabled(dirty, egui::Button::new("Revert changes"))
            .clicked()
        {
            *picks.revert = true;
        }
        ui.separator();
        if ui.button("Add child entry").clicked() {
            *picks.add_child = true;
        }
        ui.separator();
        if icon_text_btn(ui, icons::trash(), "Delete entry…").clicked() {
            *picks.delete = true;
        }
        if ui.button("Close").clicked() {
            *picks.close = true;
        }
    });
}

/// The ⓘ button: the entry's metadata on hover, and pinned open on click.
fn draw_info_button(home: &HomeState, ui: &mut egui::Ui) {
    let header = home.active_header();
    let button = icon_btn_plain(ui, icons::info());
    let popup_id = button.id.with("entry_info");
    let pinned = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let button = if pinned {
        button
    } else {
        button.on_hover_ui(|ui| info_ui(header, ui))
    };
    egui::Popup::from_toggle_button_response(&button)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| info_ui(header, ui));
}

fn icon_btn_plain(ui: &mut egui::Ui, src: egui::ImageSource<'static>) -> egui::Response {
    let tint = ui.visuals().text_color();
    let img = egui::Image::new(src)
        .fit_to_exact_size(egui::vec2(16.0, 16.0))
        .tint(tint);
    ui.add(egui::Button::image(img))
}

/// The entry's metadata, as the server last listed it.
fn info_ui(header: Option<&EntryHeader>, ui: &mut egui::Ui) {
    ui.set_min_width(220.0);
    let Some(header) = header else {
        ui.weak("Not listed yet");
        return;
    };
    ui.label(
        egui::RichText::new(&header.frontmatter.title)
            .strong()
            .size(15.0),
    );
    ui.add_space(4.0);
    egui::Grid::new("entry_info_grid")
        .num_columns(2)
        .spacing([8.0, 2.0])
        .show(ui, |ui| {
            let field = |ui: &mut egui::Ui, label: &str, value: egui::WidgetText| {
                ui.weak(label);
                ui.label(value);
                ui.end_row();
            };
            field(
                ui,
                "ID",
                egui::RichText::new(header.frontmatter.id.to_string())
                    .monospace()
                    .into(),
            );
            field(
                ui,
                "Slug",
                egui::RichText::new(&header.frontmatter.slug)
                    .monospace()
                    .into(),
            );
            field(
                ui,
                "Created",
                format_dt(header.frontmatter.created_at).into(),
            );
            field(
                ui,
                "Updated",
                format_dt(header.frontmatter.updated_at).into(),
            );
            let tags = if header.frontmatter.tags.is_empty() {
                "(none)".to_string()
            } else {
                header
                    .frontmatter
                    .tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            field(ui, "Tags", tags.into());
        });
    ui.add_space(4.0);
    if header.flags.is_empty() {
        return;
    }
    let tint = ui.visuals().text_color();
    ui.horizontal_wrapped(|ui| {
        for flag in &header.flags {
            ui.add(
                egui::Image::new(icons::flag_icon(*flag))
                    .fit_to_exact_size(egui::vec2(14.0, 14.0))
                    .tint(tint),
            );
            ui.label(flag_label(*flag));
            ui.add_space(4.0);
        }
    });
}

fn flag_label(flag: EntryFlag) -> &'static str {
    match flag {
        EntryFlag::Overdue => "Overdue",
        EntryFlag::New => "New",
        EntryFlag::Updated => "Updated",
        EntryFlag::Event => "Event",
        EntryFlag::EventClosed => "Event closed",
        EntryFlag::Done => "Done",
        EntryFlag::Cancelled => "Cancelled",
        EntryFlag::InProgress => "In progress",
        EntryFlag::Archived => "Archived",
        EntryFlag::Open => "Open",
        EntryFlag::Note => "Note",
        EntryFlag::Stale => "Stale",
        EntryFlag::Hidden => "Hidden",
    }
}

fn format_dt(dt: chrono::NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M").to_string()
}

// ── Form ─────────────────────────────────────────────────────────────────────

fn draw_form(home: &mut HomeState, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let editor = home.editor.as_mut().unwrap();

            ui.label("Title");
            ui.add(
                egui::TextEdit::singleline(&mut editor.title)
                    .hint_text("Untitled")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);

            ui.label("Tags (comma-separated)");
            ui.add(
                egui::TextEdit::singleline(&mut editor.tags)
                    .hint_text("work, journal")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);

            // Task fieldset
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.checkbox(&mut editor.has_task, "Task");
                if editor.has_task {
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Status");
                            egui::ComboBox::from_id_salt("editor_task_status")
                                .selected_text(&editor.task_status)
                                .show_ui(ui, |ui| {
                                    for s in TASK_STATUS_OPTIONS {
                                        ui.selectable_value(
                                            &mut editor.task_status,
                                            (*s).to_string(),
                                            *s,
                                        );
                                    }
                                });
                        });
                        ui.vertical(|ui| {
                            ui.label("Due (YYYY-MM-DD)");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut editor.task_due)
                                        .hint_text("2026-05-16")
                                        .desired_width(120.0),
                                );
                                date_picker_button(ui, "task_due", &mut editor.task_due);
                            });
                        });
                    });
                }
            });
            ui.add_space(6.0);

            // Event fieldset
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.checkbox(&mut editor.has_event, "Event");
                if editor.has_event {
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Start (YYYY-MM-DD)");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut editor.event_start)
                                        .desired_width(120.0),
                                );
                                date_picker_button(ui, "event_start", &mut editor.event_start);
                            });
                        });
                        ui.vertical(|ui| {
                            ui.label("End (YYYY-MM-DD)");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut editor.event_end)
                                        .desired_width(120.0),
                                );
                                date_picker_button(ui, "event_end", &mut editor.event_end);
                            });
                        });
                    });
                }
            });
            ui.add_space(8.0);

            ui.label("Body (Markdown)");
            ui.add(
                egui::TextEdit::multiline(&mut editor.body)
                    .desired_rows(20)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
        });
}

/// The statuses a task can be in, in the order the kanban shows them.
pub const TASK_STATUS_OPTIONS: &[&str] = &["open", "in_progress", "done", "cancelled", "archived"];

// ── Editor ↔ entry ───────────────────────────────────────────────────────────

/// The editor's form for an entry the server returned.
pub fn editor_for(entry: Entry) -> EditorState {
    let fm = &entry.frontmatter;
    EditorState {
        id: fm.id.to_string(),
        title: fm.title.clone(),
        tags: fm.tags.join(", "),
        body: entry.body.clone(),
        has_task: fm.task.is_some(),
        task_status: fm
            .task
            .as_ref()
            .map(|t| t.status.clone())
            .unwrap_or_else(|| "open".to_string()),
        task_due: fm
            .task
            .as_ref()
            .and_then(|t| t.due)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        has_event: fm.event.is_some(),
        event_start: fm
            .event
            .as_ref()
            .map(|e| e.start.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        event_end: fm
            .event
            .as_ref()
            .map(|e| e.end.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        base: entry,
    }
}

impl EditorState {
    /// Whether the form differs from the entry it was loaded from.
    pub fn is_dirty(&self) -> bool {
        let loaded = editor_for(self.base.clone());
        self.title != loaded.title
            || self.tags != loaded.tags
            || self.body != loaded.body
            || self.has_task != loaded.has_task
            || (self.has_task
                && (self.task_status != loaded.task_status || self.task_due != loaded.task_due))
            || self.has_event != loaded.has_event
            || (self.has_event
                && (self.event_start != loaded.event_start || self.event_end != loaded.event_end))
    }
}

/// Write the form over the entry it was loaded from and send it to the
/// server, which saves it, normalizes it and answers with its path.
/// `false` (with the reason in `error_msg`) when the form does not make an
/// entry.
pub fn save_current_entry(home: &mut HomeState) -> bool {
    let Some(editor) = home.editor.as_ref() else {
        return false;
    };
    home.error_msg = None;
    home.info_msg = None;
    match entry_from_editor(editor) {
        Ok(entry) => {
            home.api.save(entry);
            true
        }
        Err(msg) => {
            home.error_msg = Some(msg);
            false
        }
    }
}

fn entry_from_editor(editor: &EditorState) -> Result<Entry, String> {
    let mut entry = editor.base.clone();
    entry.frontmatter.title = editor.title.clone();
    entry.frontmatter.tags = editor
        .tags
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    entry.body = editor.body.clone();

    if editor.has_task {
        let due = if editor.task_due.trim().is_empty() {
            None
        } else {
            Some(
                sapphire_journal_core::period::parse_datetime_end(editor.task_due.trim())
                    .map_err(|e| format!("task.due: {e}"))?,
            )
        };
        let status = if editor.task_status.is_empty() {
            "open".to_string()
        } else {
            editor.task_status.clone()
        };
        let prev = entry.frontmatter.task.take();
        entry.frontmatter.task = Some(sapphire_journal_core::entry::TaskMeta {
            due,
            status,
            started_at: prev.as_ref().and_then(|t| t.started_at),
            closed_at: prev.as_ref().and_then(|t| t.closed_at),
            extra: prev.map(|t| t.extra).unwrap_or_default(),
        });
    } else {
        entry.frontmatter.task = None;
    }

    if editor.has_event {
        if editor.event_start.trim().is_empty() && editor.event_end.trim().is_empty() {
            return Err("event requires a start or end date".to_string());
        }
        let start_str = if editor.event_start.trim().is_empty() {
            editor.event_end.trim()
        } else {
            editor.event_start.trim()
        };
        let end_str = if editor.event_end.trim().is_empty() {
            editor.event_start.trim()
        } else {
            editor.event_end.trim()
        };
        let start = sapphire_journal_core::period::parse_datetime(start_str)
            .map_err(|e| format!("event.start: {e}"))?;
        let end = sapphire_journal_core::period::parse_datetime_end(end_str)
            .map_err(|e| format!("event.end: {e}"))?;
        let prev = entry.frontmatter.event.take();
        entry.frontmatter.event = Some(sapphire_journal_core::entry::EventMeta {
            start,
            end,
            extra: prev.map(|e| e.extra).unwrap_or_default(),
        });
    } else {
        entry.frontmatter.event = None;
    }
    Ok(entry)
}
