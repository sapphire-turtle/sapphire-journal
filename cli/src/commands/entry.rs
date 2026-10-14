use anyhow::{bail, Context, Result};
use sapphire_journal_core::{
    entry::Entry,
    entry_ref::EntryRef,
    labels::EntryFlag,
    ops::{EntryListItem, EntryTreeNode, UpdateOption},
    period::{parse_datetime, parse_datetime_end},
    text_input::filter as core_filter,
};
use sapphire_journal_mcp::methods::{
    EntryCheckParams, EntryFixParams, EntryGetParams, EntryListParams, EntryModifyParams,
    EntryNewParams, EntryPrepareParams, EntryRemoveParams, EntryResolveParams, names,
};
use sapphire_framework::backend::{IpcBackend, SearchMode, WorkspaceBackend as _};
use sapphire_framework::ipc::Endpoint;

use chrono::NaiveDateTime;
use clap::{Args, Subcommand};
use std::{path::Path, process::Command};

use crate::client::Server;

/// Filter and sort arguments shared by `entry list` and `entry tree`.
#[derive(Args)]
pub struct EntryFilterArgs {
    /// Time range to filter against. Without field selectors the period is applied to all
    /// timestamp fields simultaneously (OR). Add selectors to restrict matching.
    ///
    /// PERIOD: today | yesterday | tomorrow |
    ///   this_week | last_week | next_week |
    ///   this_month | last_month | next_month | none |
    ///   YYYY | YYYY-MM | YYYY-Www |
    ///   YYYY-MM-DD | YYYY-MM-DD/YYYY-MM-DD | YYYY-MM-DDTHH:MM/YYYY-MM-DDTHH:MM
    #[arg(value_name = "PERIOD")]
    pub period: Option<String>,

    /// Show all entries regardless of time range (required when PERIOD is omitted)
    #[arg(long)]
    pub all_periods: bool,

    /// Enable all selectors at once: task_overdue, task_in_progress, event_span,
    /// created_at, updated_at. With a PERIOD this produces a Bullet Journal-style
    /// daily/weekly/monthly log view. Individual flags can still be added on top.
    #[arg(long)]
    pub active: bool,

    /// Include incomplete tasks whose due date falls within (or before) the period.
    /// Without --period: include tasks whose due date is in the past and are not yet closed.
    #[arg(long)]
    pub task_overdue: bool,

    /// Include incomplete tasks that were started within (or before) the period.
    /// Without --period: include all tasks that have started_at set and are not yet closed.
    #[arg(long)]
    pub task_in_progress: bool,

    /// Include tasks that have not been started yet (started_at and closed_at both absent).
    /// Period is not applied to this filter.
    #[arg(long)]
    pub task_unstarted: bool,

    /// Restrict --period to event span (overlap semantics).
    /// Without --period: include entries that have an event set.
    #[arg(long)]
    pub event_span: bool,

    /// Restrict --period to created_at timestamp.
    #[arg(long)]
    pub created_at: bool,

    /// Restrict --period to updated_at timestamp.
    #[arg(long)]
    pub updated_at: bool,

    /// Filter by task status (AND with timestamp filters).
    /// Comma-separated for multiple values, e.g. open,in_progress
    #[arg(long, value_name = "STATUS[,...]", value_delimiter = ',', num_args = 1..)]
    pub task_status: Option<Vec<String>>,

    /// AND filter: include only entries that have ALL specified tags.
    /// Comma-separated, e.g. work,urgent
    #[arg(long, value_name = "TAG[,...]", value_delimiter = ',', num_args = 1..)]
    pub tags: Option<Vec<String>>,

    /// Sort results by a field.
    /// Values: id | title | task_status | created_at | updated_at | task_due | event_start | event_end
    #[arg(long, value_name = "FIELD")]
    pub sort_by: Option<String>,

    /// Sort direction: asc (default) or desc
    #[arg(long, value_name = "ORDER", default_value = "asc")]
    pub sort_order: String,

    /// Include stale (long-neglected) tasks that are otherwise excluded by default.
    #[arg(long)]
    pub include_stale: bool,

    /// Include explicitly hidden entries that are otherwise excluded by default.
    #[arg(long)]
    pub include_hidden: bool,
}

/// The server's list parameters for `args`, checked here first so that a
/// mistyped period fails before anything is sent.
fn list_params(args: &EntryFilterArgs) -> Result<EntryListParams> {
    if args.period.is_none() && !args.all_periods {
        bail!(
            "PERIOD is required (e.g. `today`, `this_week`, `2026-03-15`), \
             or pass `--all-periods` to show all entries"
        );
    }
    core_filter::build_filter(core_filter::FilterInputs::from(args))?;
    let flag = |b: bool| b.then_some(true);
    Ok(EntryListParams {
        period: args.period.clone(),
        active: flag(args.active),
        task_overdue: flag(args.task_overdue),
        task_in_progress: flag(args.task_in_progress),
        task_unstarted: flag(args.task_unstarted),
        event_span: flag(args.event_span),
        created_at: flag(args.created_at),
        updated_at: flag(args.updated_at),
        task_status: args.task_status.clone(),
        tags: args.tags.clone(),
        sort_by: args.sort_by.clone(),
        sort_order: Some(args.sort_order.clone()),
        include_stale: flag(args.include_stale),
        include_hidden: flag(args.include_hidden),
    })
}

impl<'a> From<&'a EntryFilterArgs> for core_filter::FilterInputs<'a> {
    fn from(a: &'a EntryFilterArgs) -> Self {
        Self {
            period: a.period.as_deref(),
            active: a.active,
            task_overdue: a.task_overdue,
            task_in_progress: a.task_in_progress,
            task_unstarted: a.task_unstarted,
            event_span: a.event_span,
            created_at: a.created_at,
            updated_at: a.updated_at,
            task_status: a.task_status.as_deref().unwrap_or(&[]),
            tags: a.tags.as_deref().unwrap_or(&[]),
            sort_by: a.sort_by.as_deref(),
            sort_order: Some(a.sort_order.as_str()),
            include_stale: a.include_stale,
            include_hidden: a.include_hidden,
        }
    }
}

#[derive(Subcommand)]
pub enum EntryCommand {
    /// List entries; optionally filter by period, field selectors, task status, and tags
    List {
        #[command(flatten)]
        filter: EntryFilterArgs,

        /// Output all matching entries as JSON (metadata + body) for AI/machine consumption
        #[arg(long)]
        json: bool,

        /// Display emoji glyphs for entry type and freshness
        #[arg(long, conflicts_with = "nerd")]
        emoji: bool,

        /// Display Nerd Font glyphs for entry type and freshness
        #[arg(long, conflicts_with = "emoji")]
        nerd: bool,
    },
    /// Display entries as a parent-child tree; supports the same filters as `entry list`
    Tree {
        #[command(flatten)]
        filter: EntryFilterArgs,

        /// Output the tree as JSON (nested `children` arrays) for machine consumption
        #[arg(long)]
        json: bool,

        /// Display emoji glyphs for entry type and freshness
        #[arg(long, conflicts_with = "nerd")]
        emoji: bool,

        /// Display Nerd Font glyphs for entry type and freshness
        #[arg(long, conflicts_with = "emoji")]
        nerd: bool,
    },
    /// Show the contents of an entry
    Show {
        /// Path to the entry file, or an ID / ID prefix
        entry: String,
    },
    /// Create a new entry with an optional body.
    New {
        #[command(flatten)]
        fields: EntryFields,
    },
    /// Open an entry in $EDITOR. Without an argument, or with --new, creates a new entry.
    Edit {
        /// Path to the entry file, or an ID / ID prefix
        entry: Option<String>,

        /// Create a new entry and open it in $EDITOR with a pre-filled frontmatter template
        #[arg(long)]
        new: bool,
    },
    /// Update frontmatter fields without opening an editor
    Modify {
        /// Path to the entry file, or an ID / ID prefix
        entry: String,

        #[command(flatten)]
        fields: EntryFields,

        /// Remove the parent relationship (make this a root entry)
        #[arg(long, conflicts_with = "parent")]
        no_parent: bool,
    },
    /// Check whether an entry's frontmatter and filename are valid
    Check {
        /// Path to the entry file, or an ID / ID prefix
        entry: String,
    },
    /// Normalize an entry: sync closed_at, update updated_at, and rename to match frontmatter
    Fix {
        /// Path to the entry file, or an ID / ID prefix
        entry: String,
    },
    /// Print the absolute path of an entry, or create a new template and print its path.
    /// Intended for editor integrations that need a file path without opening an editor.
    Path {
        /// File path to the entry, or an ID / ID prefix.
        /// Required unless --new is given.
        #[arg(required_unless_present = "new")]
        entry: Option<String>,

        /// Create a new entry template file and print its path instead of resolving an existing entry
        #[arg(long)]
        new: bool,

        /// Parent entry for the new file — accepts `@ID`, a file path, or a title.
        /// Only valid together with --new.
        #[arg(long, value_name = "ENTRY", requires = "new")]
        parent: Option<String>,
    },
    /// Delete an entry file
    Remove {
        /// Path to the entry file, or an ID / ID prefix
        entry: String,
    },
    /// Search the journal by full-text or semantic similarity
    ///
    /// Asks the running server, which indexes the journal it serves: without
    /// --semantic, the trigram full-text index (substring / CJK friendly); with
    /// --semantic, the vectors the host's sapphire-bridge embeds. Without an
    /// embedding model, semantic search falls back to full text.
    Search {
        /// Search query text
        query: String,

        /// Use vector similarity search instead of full-text search
        #[arg(long)]
        semantic: bool,

        /// Maximum number of results to return
        #[arg(long, short, default_value = "10")]
        limit: usize,
    },
}

/// Frontmatter fields shared between `entry new` and `entry modify` (clap-aware).
///
/// After parsing this is converted into the server's `entry_new` /
/// `entry_modify` parameters.
#[derive(Args)]
pub struct EntryFields {
    /// Title of the entry — written into the frontmatter and used to generate the filename slug
    #[arg(long, short)]
    pub title: Option<String>,

    /// Body content (Markdown). For `entry modify`, replaces the existing body.
    #[arg(long, short)]
    pub body: Option<String>,

    /// Parent entry — accepts `@ID`, a file path, or a title.
    #[arg(long, value_name = "ENTRY")]
    pub parent: Option<String>,

    /// Slug override in the frontmatter
    #[arg(long)]
    pub slug: Option<String>,

    /// Tags (comma-separated); pass with no value to clear all tags
    #[arg(long, short = 'T', num_args = 0.., value_delimiter = ',')]
    pub tags: Option<Vec<String>>,

    /// Task due date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM; date-only = 23:59)
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime_end)]
    pub task_due: Option<NaiveDateTime>,

    /// Task status (open | in_progress | done | cancelled | archived)
    #[arg(long)]
    pub task_status: Option<String>,

    /// Task start date/time; set automatically when status → in_progress
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime)]
    pub task_started_at: Option<NaiveDateTime>,

    /// Task close date/time; set automatically when status → done/cancelled/archived
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime)]
    pub task_closed_at: Option<NaiveDateTime>,

    /// Event start date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM)
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime)]
    pub event_start: Option<NaiveDateTime>,

    /// Event end date/time (YYYY-MM-DD or YYYY-MM-DDTHH:MM; date-only = 23:59)
    #[arg(long, value_name = "DATETIME", value_parser = parse_datetime_end)]
    pub event_end: Option<NaiveDateTime>,

    /// Hide the entry from default list/tree results (true) or reveal it (false).
    #[arg(long)]
    pub hidden: Option<bool>,
}

/// The server's spelling of a timestamp the CLI parsed.
fn datetime(dt: Option<NaiveDateTime>) -> Option<String> {
    dt.map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
}

impl EntryFields {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.body.is_none()
            && self.parent.is_none()
            && self.slug.is_none()
            && self.tags.is_none()
            && self.task_due.is_none()
            && self.task_status.is_none()
            && self.task_started_at.is_none()
            && self.task_closed_at.is_none()
            && self.event_start.is_none()
            && self.event_end.is_none()
            && self.hidden.is_none()
    }

    fn into_new_params(self) -> EntryNewParams {
        EntryNewParams {
            title: self.title,
            body: self.body,
            parent: self.parent.as_deref().map(EntryRef::parse),
            slug: self.slug,
            // `--tags` with no value means "no tags", which is the empty list.
            tags: self.tags.map(|t| t.join(",")),
            task_due: datetime(self.task_due),
            task_status: self.task_status,
            task_started_at: datetime(self.task_started_at),
            task_closed_at: datetime(self.task_closed_at),
            event_start: datetime(self.event_start),
            event_end: datetime(self.event_end),
            hidden: self.hidden,
        }
    }

    fn into_modify_params(self, entry: EntryRef, no_parent: bool) -> EntryModifyParams {
        EntryModifyParams {
            entry,
            title: self.title,
            body: self.body,
            parent: match (no_parent, self.parent.as_deref()) {
                (true, _) => UpdateOption::Clear,
                (false, Some(parent)) => UpdateOption::Set(EntryRef::parse(parent)),
                (false, None) => UpdateOption::Unchanged,
            },
            slug: self.slug,
            tags: self.tags.map(|t| t.join(",")),
            task_due: datetime(self.task_due),
            task_status: self.task_status,
            task_started_at: datetime(self.task_started_at),
            task_closed_at: datetime(self.task_closed_at),
            event_start: datetime(self.event_start),
            event_end: datetime(self.event_end),
            hidden: self.hidden,
        }
    }
}

/// Run an entry command. Everything goes to the running server, which owns the
/// journal's files and its cache.
pub fn run(server: &Server, cmd: EntryCommand) -> Result<()> {
    match cmd {
        EntryCommand::List { filter, json, emoji, nerd } => {
            let params = list_params(&filter)?;
            if json {
                println!("{}", server.call(names::ENTRY_LIST, params)?);
                return Ok(());
            }
            let items: Vec<EntryListItem> = server.read(names::ENTRY_LIST, params)?;
            print_entries(&items, DisplayMode::from_flags(emoji, nerd));
            Ok(())
        }
        EntryCommand::Tree { filter, json, emoji, nerd } => {
            let params = list_params(&filter)?;
            if json {
                println!("{}", server.call(names::ENTRY_TREE, params)?);
                return Ok(());
            }
            let roots: Vec<EntryTreeNode> = server.read(names::ENTRY_TREE, params)?;
            print_tree(&roots, DisplayMode::from_flags(emoji, nerd));
            Ok(())
        }
        EntryCommand::Show { entry } => {
            let entry: Entry =
                server.read(names::ENTRY_GET, EntryGetParams { entry: EntryRef::parse(&entry) })?;
            show(&entry);
            Ok(())
        }
        EntryCommand::New { fields } => {
            println!("{}", server.call(names::ENTRY_NEW, fields.into_new_params())?);
            Ok(())
        }
        EntryCommand::Edit { entry, new } => {
            if new {
                edit_new(server)
            } else if let Some(e) = entry {
                edit(server, &e)
            } else {
                bail!("specify an entry or use --new to create one")
            }
        }
        EntryCommand::Modify { entry, fields, no_parent } => {
            if fields.is_empty() && !no_parent {
                bail!("nothing to update — specify at least one field");
            }
            let params = fields.into_modify_params(EntryRef::parse(&entry), no_parent);
            println!("{}", server.call(names::ENTRY_MODIFY, params)?);
            Ok(())
        }
        EntryCommand::Check { entry } => {
            let params = EntryCheckParams { entry: EntryRef::parse(&entry) };
            println!("{}", server.call(names::ENTRY_CHECK, params)?);
            Ok(())
        }
        EntryCommand::Fix { entry } => {
            let params = EntryFixParams { entry: EntryRef::parse(&entry) };
            println!("{}", server.call(names::ENTRY_FIX, params)?);
            Ok(())
        }
        EntryCommand::Path { entry, new, parent } => {
            let path = if new {
                let parent = parent.as_deref().map(EntryRef::parse);
                server.call(names::ENTRY_PREPARE, EntryPrepareParams { parent })?
            } else {
                let entry = EntryRef::parse(entry.as_deref().expect("clap requires it without --new"));
                server.call(names::ENTRY_RESOLVE, EntryResolveParams { entry })?
            };
            println!("{path}");
            Ok(())
        }
        EntryCommand::Remove { entry } => {
            let params = EntryRemoveParams { entry: EntryRef::parse(&entry) };
            println!("{}", server.call(names::ENTRY_REMOVE, params)?);
            Ok(())
        }
        EntryCommand::Search { query, semantic, limit } => search(&query, semantic, limit),
    }
}

// ── display mode ──────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
enum DisplayMode { Initials, Emoji, Nerd }

impl DisplayMode {
    fn from_flags(emoji: bool, nerd: bool) -> Self {
        if emoji { Self::Emoji } else if nerd { Self::Nerd } else { Self::Initials }
    }
}

fn render_slot(flags: &[EntryFlag], mode: DisplayMode) -> String {
    let absent_fresh = match mode {
        DisplayMode::Initials => "·",
        DisplayMode::Emoji    => "・",
        DisplayMode::Nerd     => " ",
    };
    let (fresh, type_flag) = match flags.len() {
        0 => (absent_fresh.to_owned(), EntryFlag::Note),
        1 => (absent_fresh.to_owned(), flags[0]),
        _ => {
            let f = match mode {
                DisplayMode::Initials => flags[0].to_initial().to_string(),
                DisplayMode::Emoji    => flags[0].to_emoji().to_owned(),
                DisplayMode::Nerd     => flags[0].to_nerd().to_owned(),
            };
            (f, flags[1])
        }
    };
    let typ = match mode {
        DisplayMode::Initials => type_flag.to_initial().to_string(),
        DisplayMode::Emoji    => type_flag.to_emoji().to_owned(),
        DisplayMode::Nerd     => type_flag.to_nerd().to_owned(),
    };
    format!("{fresh}{typ}")
}

// ── list output ───────────────────────────────────────────────────────────────

fn print_entries(items: &[EntryListItem], mode: DisplayMode) {
    let rows: Vec<(String, String, String)> = items
        .iter()
        .map(|EntryListItem { entry, .. }| {
            let id = entry.id().to_string();
            let slot = render_slot(&entry.flags, mode);
            (id, slot, entry.title().to_owned())
        })
        .collect();

    let id_w = rows.iter().map(|(id, _, _)| id.len()).max().unwrap_or(7);
    for (id, slot, title) in &rows {
        println!("{slot}  {:<id_w$}  {title}", id);
    }
}

// ── tree output ───────────────────────────────────────────────────────────────

fn print_tree(roots: &[EntryTreeNode], mode: DisplayMode) {
    fn render_node(node: &EntryTreeNode, mode: DisplayMode, prefix: &str, is_last: bool) {
        let connector = if is_last { "└─" } else { "├─" };
        let id = node.entry.id().to_string();
        let slot = render_slot(&node.entry.flags, mode);
        let title = node.entry.title();
        if prefix.is_empty() {
            println!("{slot}  {id}  {title}");
        } else {
            println!("{prefix}{connector} {slot}  {id}  {title}");
        }
        let child_prefix = if prefix.is_empty() {
            if is_last { "   ".to_owned() } else { "│  ".to_owned() }
        } else {
            format!("{}{}", prefix, if is_last { "   " } else { "│  " })
        };
        let n = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            render_node(child, mode, &child_prefix, i + 1 == n);
        }
    }

    let n = roots.len();
    for (i, root) in roots.iter().enumerate() {
        render_node(root, mode, "", i + 1 == n);
    }
}

// ── show ──────────────────────────────────────────────────────────────────────

fn show(entry: &Entry) {
    use sapphire_journal_core::labels::{entry_flags, STALE_AFTER_DAYS_DEFAULT};

    let fm_view = sapphire_journal_core::entry::FrontmatterView::from(entry.frontmatter.clone());
    let fm = &fm_view;

    let flags = entry_flags(
        fm.task.as_ref(),
        fm.event.as_ref(),
        fm.hidden == Some(true),
        fm.created_at,
        fm.updated_at,
        chrono::Duration::days(STALE_AFTER_DAYS_DEFAULT as i64),
    );
    let flags_str: Vec<&str> = flags.iter().map(|f| f.as_str()).collect();

    println!("# {}", entry.title());
    println!("flags:    {}", flags_str.join(", "));
    println!("created:  {}", fm.created_at.format("%Y-%m-%dT%H:%M"));
    println!("updated:  {}", fm.updated_at.format("%Y-%m-%dT%H:%M"));
    if !fm.tags.is_empty() {
        println!("tags:     {}", fm.tags.join(", "));
    }
    if let Some(task) = &fm.task {
        let status = task.status.as_str();
        match task.due {
            Some(d) => println!("task:     {status} (due {})", d.format("%Y-%m-%d")),
            None => println!("task:     {status}"),
        }
        if let Some(sa) = task.started_at {
            println!("started:  {}", sa.format("%Y-%m-%dT%H:%M"));
        }
        if let Some(ca) = task.closed_at {
            println!("closed:   {}", ca.format("%Y-%m-%dT%H:%M"));
        }
    }
    if let Some(event) = &fm.event {
        println!("event:    {} – {}", event.start.format("%Y-%m-%d"), event.end.format("%Y-%m-%d"));
    }
    println!();
    print!("{}", entry.body);
}

// ── edit ──────────────────────────────────────────────────────────────────────
//
// The user's editor writes the file itself, like any editor the user points at
// the journal. The server prepares and resolves the path, and normalizes the
// entry afterwards (`entry_fix`), which also brings its cache row up to date.

fn edit(server: &Server, entry: &str) -> Result<()> {
    let path = server.call(names::ENTRY_RESOLVE, EntryResolveParams { entry: EntryRef::parse(entry) })?;
    run_editor(Path::new(&path))?;
    let fixed = server.call(names::ENTRY_FIX, EntryFixParams { entry: EntryRef::Path(path.clone().into()) })?;
    println!("updated: {path}");
    if fixed.starts_with("renamed:") {
        println!("{fixed}");
    }
    Ok(())
}

fn edit_new(server: &Server) -> Result<()> {
    let path = server.call(names::ENTRY_PREPARE, EntryPrepareParams::default())?;
    let entry = EntryRef::Path(path.clone().into());
    if let Err(err) = run_editor(Path::new(&path)) {
        let _ = server.call(names::ENTRY_REMOVE, EntryRemoveParams { entry });
        return Err(err);
    }
    let fixed = server.call(names::ENTRY_FIX, EntryFixParams { entry })?;
    println!("created: {path}");
    if fixed.starts_with("renamed:") {
        println!("{fixed}");
    }
    Ok(())
}

fn run_editor(path: &Path) -> Result<()> {
    let editor = resolve_editor();
    let status = Command::new(&editor)
        .arg(path)
        .status()
        .with_context(|| format!("failed to launch editor `{editor}`"))?;
    if !status.success() {
        bail!("editor exited with non-zero status");
    }
    Ok(())
}

// ── search ────────────────────────────────────────────────────────────────────

fn search(query: &str, semantic: bool, limit: usize) -> Result<()> {
    let mode = if semantic { SearchMode::Semantic } else { SearchMode::Fts };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let results = runtime.block_on(async {
        let app = sapphire_journal_core::JOURNAL_CTX.app_name;
        let endpoint = Endpoint::for_app(app)?;
        let backend =
            IpcBackend::connect(&endpoint, app, "cli", env!("CARGO_PKG_VERSION"), None).await?;
        // The CLI writes entries without the server; catch the index up with
        // them first, incrementally, as the cache is before a listing.
        backend.sync().await?;
        anyhow::Ok(backend.search(query, limit, mode).await?)
    })?;

    if results.is_empty() {
        println!("no results");
        return Ok(());
    }
    for r in &results {
        if semantic {
            println!("{:.4}  {}", r.score, r.path);
        } else {
            println!("{}", r.path);
        }
        if !r.snippet.is_empty() {
            println!("    {}", r.snippet);
        }
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn resolve_editor() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into())
}
