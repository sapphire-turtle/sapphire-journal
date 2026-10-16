use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use grain_id::GrainId;
use sapphire_framework::gui::Settings as FrameworkSettings;
use sapphire_framework::gui::{eframe, egui};
use sapphire_journal_core::entry::{Entry, EntryHeader};

use crate::api::{Api, Reply};
use crate::screens;
use crate::screens::settings_panel::SettingsPanelState;
use crate::settings::{RightTab, Settings};

/// How often the journal tab re-reads the entry list on its own, to pick up
/// what the CLI, an agent or sync wrote.
pub const REFRESH_EVERY: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Journal,
    Sync,
}

/// The desktop app: a client of the `sapphire-journal` server and the
/// host's bridge. The Journal tab shows the journal the server serves; the
/// Sync tab (the framework's panel) chooses that journal and manages sync.
pub struct App {
    pub tab: Tab,
    pub settings: Settings,
    pub home: HomeState,
    /// When `Some`, the Settings window is shown on top of the journal tab.
    pub settings_panel: Option<SettingsPanelState>,
    /// The framework's Settings: the workspace, sync, the workgroup, About.
    pub sync: FrameworkSettings,
    pub api: Api,
    replies: mpsc::Receiver<Reply>,
}

/// State for the journal tab (sidebar + editor).
pub struct HomeState {
    pub api: Api,

    /// The journal the server serves, once the server has said.
    pub journal_root: Option<PathBuf>,

    /// Headers for the entry sidebar, as the server last listed them.
    pub entries: Vec<EntryHeader>,
    /// Why the list could not be read (no server, no journal selected, …).
    pub entries_error: Option<String>,
    /// A list request is on its way.
    pub listing: bool,
    pub last_listed: Option<Instant>,

    /// Currently-selected entry, if any.
    pub selected_path: Option<PathBuf>,
    /// The selected entry's form; `None` while it loads.
    pub editor: Option<EditorState>,

    /// Hierarchical (tree) or flat (list) display of entries.
    pub view_mode: ViewMode,
    /// IDs of tree nodes whose children are currently hidden.  Default open
    /// (a node not in this set is expanded).
    pub collapsed: HashSet<GrainId>,
    /// Whether the period / sort / order pickers are visible. Toggled by
    /// the funnel icon in the sidebar toolbar.  Search input stays visible.
    pub show_filters: bool,

    // ── Sidebar filter / sort state ─────────────────────────────────────────
    pub filter_text: String,
    /// Active period filter, serialised as a [`sapphire_journal_core::period`] string.
    /// Empty = no filter (all entries). Day click writes `YYYY-MM-DD`, week / month
    /// clicks write `YYYY-MM-DD/YYYY-MM-DD`, the calendar's "Today" shortcut writes
    /// `today`.
    pub period: String,
    /// The month currently displayed in the home-screen calendar widget.  Mutated
    /// by the calendar's ◀ / ▶ arrows and reset to today by the "Today" shortcut.
    pub calendar_cursor: NaiveDate,
    /// Whether the home-screen calendar is visible (toggled by its caret).
    pub show_calendar: bool,
    /// Sort field. One of "updated_at" | "created_at" | "title" | "id"
    /// | "task_due" | "event_start".
    pub sort_by: String,
    /// "asc" | "desc".
    pub sort_order: String,

    pub confirm_delete_entry: bool,
    pub error_msg: Option<String>,
    pub info_msg: Option<String>,

    /// `true` while a user-triggered refresh is running. Used to
    /// short-circuit duplicate clicks and to render a spinner in the header.
    pub sync_in_progress: bool,
    /// Whether the Obsidian-style right metadata sidebar is currently shown.
    /// Toggled by the panel icon in the top header; persisted in [`Settings`].
    pub show_right_sidebar: bool,
    /// Active tab within the right sidebar (only `Metadata` exists today,
    /// but the field is kept so future tabs can be added without UI churn).
    pub right_sidebar_tab: RightTab,
    /// User-resized width of the right sidebar in panel mode. Saved back to
    /// [`Settings`] when the drag ends.
    pub right_sidebar_width: f32,
}

impl HomeState {
    pub fn new(api: Api, settings: &Settings) -> Self {
        Self {
            api,
            journal_root: None,
            entries: Vec::new(),
            entries_error: None,
            listing: false,
            last_listed: None,
            selected_path: None,
            editor: None,
            view_mode: ViewMode::Tree,
            collapsed: HashSet::new(),
            show_filters: false,
            filter_text: String::new(),
            period: String::new(),
            calendar_cursor: chrono::Local::now().date_naive(),
            show_calendar: true,
            sort_by: "updated_at".to_string(),
            sort_order: "desc".to_string(),
            confirm_delete_entry: false,
            error_msg: None,
            info_msg: None,
            sync_in_progress: false,
            show_right_sidebar: settings.right_sidebar.visible,
            right_sidebar_tab: settings.right_sidebar.active_tab,
            right_sidebar_width: settings.right_sidebar.width,
        }
    }

    /// Look up the cached header for the currently-selected entry.
    pub fn active_header(&self) -> Option<&EntryHeader> {
        let path = self.selected_path.as_ref()?;
        self.entries
            .iter()
            .find(|h| Path::new(&h.path) == path.as_path())
    }

    /// Ask the server for the entry list, unless a request is already out.
    pub fn reload(&mut self) {
        if self.listing {
            return;
        }
        self.listing = true;
        self.last_listed = Some(Instant::now());
        self.api.list();
    }

    /// Select `path` and ask the server for it; the editor shows it when it
    /// arrives.
    pub fn select(&mut self, path: PathBuf) {
        self.selected_path = Some(path.clone());
        self.editor = None;
        self.info_msg = None;
        self.api.get(path);
    }
}

/// How the entry sidebar should display entries.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Hierarchical view following `parent_id` relationships.
    Tree,
    /// Flat list, ignoring hierarchy.
    List,
}

/// Form fields for the entry currently being edited in the main panel.
///
/// The path of the entry being edited lives in `HomeState::selected_path` so
/// that selection-aware UI (e.g. the sidebar's active-row highlight) and the
/// editor stay in sync.
pub struct EditorState {
    /// The entry as the server last returned it: saving writes the form's
    /// fields over it, so what the form does not show (`extra`, timestamps)
    /// survives.
    pub base: Entry,
    pub id: String,
    pub title: String,
    /// Comma-separated tag input.
    pub tags: String,
    pub body: String,
    pub has_task: bool,
    pub task_status: String,
    /// `YYYY-MM-DD` (empty = no due date).
    pub task_due: String,
    pub has_event: bool,
    pub event_start: String,
    pub event_end: String,
}

impl App {
    pub fn new(
        api: Api,
        replies: mpsc::Receiver<Reply>,
        sync: FrameworkSettings,
        data_dir: PathBuf,
    ) -> Self {
        let settings = Settings::load(&data_dir);
        let mut home = HomeState::new(api.clone(), &settings);
        home.reload();
        api.settings();
        Self {
            tab: Tab::Journal,
            settings,
            home,
            settings_panel: None,
            sync,
            api,
            replies,
        }
    }

    fn handle_replies(&mut self) {
        while let Ok(reply) = self.replies.try_recv() {
            let home = &mut self.home;
            match reply {
                Reply::Entries(result) => {
                    home.listing = false;
                    match result {
                        Ok(entries) => {
                            home.entries = entries;
                            home.entries_error = None;
                        }
                        // Keep showing the last list: a server restart should
                        // not blank the window.
                        Err(e) => home.entries_error = Some(e),
                    }
                }
                Reply::Loaded(path, result) => {
                    // A reply for an entry no longer selected is stale.
                    if home.selected_path.as_ref() != Some(&path) {
                        continue;
                    }
                    match result {
                        Ok(entry) => home.editor = Some(screens::journal_home::editor_for(*entry)),
                        Err(e) => home.error_msg = Some(e),
                    }
                }
                Reply::Prepared(result) => match result {
                    Ok(path) => {
                        home.select(path);
                        home.reload();
                    }
                    Err(e) => home.error_msg = Some(e),
                },
                Reply::Saved(old_path, result) => match result {
                    Ok(path) => {
                        if home.selected_path.as_ref() == Some(&old_path) {
                            home.select(path);
                            home.info_msg = Some("Saved.".to_string());
                        }
                        home.reload();
                    }
                    Err(e) => home.error_msg = Some(format!("Save failed: {e}")),
                },
                Reply::Removed(path, result) => match result {
                    Ok(()) => {
                        if home.selected_path.as_ref() == Some(&path) {
                            home.selected_path = None;
                            home.editor = None;
                            home.info_msg = None;
                        }
                        home.reload();
                    }
                    Err(e) => home.error_msg = Some(e),
                },
                Reply::Reparented(old_path, result) => match result {
                    Ok(path) => {
                        // Track selection if it moved with a rename.
                        if home.selected_path.as_ref() == Some(&old_path) {
                            home.selected_path = Some(path);
                        }
                        home.info_msg = Some("Reparented.".to_string());
                        home.reload();
                    }
                    Err(e) => home.error_msg = Some(format!("Reparent failed: {e}")),
                },
                Reply::Refreshed(result) => {
                    home.sync_in_progress = false;
                    match result {
                        Ok(()) => home.reload(),
                        Err(e) => home.error_msg = Some(format!("Refresh failed: {e}")),
                    }
                }
                Reply::Settings(result) => match result {
                    Ok(settings) => {
                        home.journal_root = Some(settings.root.clone());
                        if let Some(panel) = self.settings_panel.as_mut() {
                            panel.loaded(settings);
                        }
                    }
                    Err(e) => {
                        if let Some(panel) = self.settings_panel.as_mut() {
                            panel.error_msg = Some(e);
                        }
                    }
                },
                Reply::SettingsSaved(result) => {
                    if let Some(panel) = self.settings_panel.as_mut() {
                        panel.saved(result);
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_replies();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Re-read the list on its own now and then, and wake up for it even
        // if nothing else happens.
        if self.tab == Tab::Journal
            && self
                .home
                .last_listed
                .is_none_or(|t| t.elapsed() >= REFRESH_EVERY)
        {
            // A switch in the Sync tab may have changed the journal.
            self.api.settings();
            self.home.reload();
        }
        ui.ctx().request_repaint_after(REFRESH_EVERY);

        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Journal, "Journal");
                ui.selectable_value(&mut self.tab, Tab::Sync, "Sync");
            });
        });
        match self.tab {
            Tab::Journal => screens::journal_home::show(self, ui),
            Tab::Sync => {
                egui::CentralPanel::default().show(ui, |ui| self.sync.ui(ui));
            }
        }
    }
}
