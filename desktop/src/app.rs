use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use grain_id::GrainId;
use sapphire_framework::gui::client::FrameworkClient;
use sapphire_framework::gui::{Destination, Settings as FrameworkSettings, Shell, eframe, egui};
use sapphire_journal_core::entry::{Entry, EntryHeader};

use crate::api::{Api, Reply};
use crate::icons;
use crate::screens;
use crate::screens::settings_panel::{JournalSettingsPage, SettingsPanelState};
use crate::settings::{RightTab, Settings};

/// How often the journal tab re-reads the entry list on its own, to pick up
/// what the CLI, an agent or sync wrote.
pub const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// The app's own screens, on the framework's rail.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Journal,
}

/// The window: the framework's [`Shell`] around the journal. The rail opens
/// the Journal screen, and Settings at its foot, which chooses the journal,
/// manages sync and holds the journal's own settings page.
pub struct Window {
    shell: Shell<Screen>,
    app: App,
}

impl Window {
    pub fn new(
        api: Api,
        replies: mpsc::Receiver<Reply>,
        framework: FrameworkClient,
        data_dir: PathBuf,
    ) -> Self {
        let journal_settings = Rc::new(RefCell::new(SettingsPanelState::open()));
        let settings = FrameworkSettings::new(framework).with_app_sections(
            "Journal",
            vec![Box::new(JournalSettingsPage::new(
                Rc::clone(&journal_settings),
                api.clone(),
            ))],
        );
        let shell = Shell::new(
            vec![Destination::new(
                Screen::Journal,
                icons::book_open(),
                "Journal",
            )],
            settings,
        );
        Self {
            shell,
            app: App::new(api, replies, journal_settings, data_dir),
        }
    }
}

impl eframe::App for Window {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.handle_replies();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Re-read the list on its own now and then, and wake up for it even
        // if nothing else happens.
        if self.shell.current() == Some(Screen::Journal)
            && self
                .app
                .home
                .last_listed
                .is_none_or(|t| t.elapsed() >= REFRESH_EVERY)
        {
            // A switch in Settings may have changed the journal.
            self.app.api.settings();
            self.app.home.reload();
        }
        ui.ctx().request_repaint_after(REFRESH_EVERY);

        let app = &mut self.app;
        self.shell.ui(ui, |ui, screen| match screen {
            Screen::Journal => screens::journal_home::show(app, ui),
        });
        if std::mem::take(&mut self.app.open_settings) {
            self.shell.open_settings();
        }
    }
}

/// The journal: a client of the `sapphire-journal` server, so it shows the
/// journal the server serves.
pub struct App {
    pub settings: Settings,
    pub home: HomeState,
    /// The journal's page in Settings, filled from the server's answers.
    pub journal_settings: Rc<RefCell<SettingsPanelState>>,
    /// Set to have the window open Settings (from the journal menu).
    pub open_settings: bool,
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
        journal_settings: Rc<RefCell<SettingsPanelState>>,
        data_dir: PathBuf,
    ) -> Self {
        let settings = Settings::load(&data_dir);
        let mut home = HomeState::new(api.clone(), &settings);
        home.reload();
        api.settings();
        Self {
            settings,
            home,
            journal_settings,
            open_settings: false,
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
                        self.journal_settings.borrow_mut().refreshed(settings);
                    }
                    Err(e) => self.journal_settings.borrow_mut().error_msg = Some(e),
                },
                Reply::SettingsSaved(result) => {
                    self.journal_settings.borrow_mut().saved(result);
                }
            }
        }
    }
}
