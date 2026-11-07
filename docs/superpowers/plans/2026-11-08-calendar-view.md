# Calendar View, Startup Period and All-Day Dates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Store all-day dates as a date-only `DateOrTime`. Give the desktop alumina's period and date pickers, ◀ ▶ period steps, a startup-period setting and a rail "new entry" button. Add a read-only calendar view (day and week timeline, month grid, one-row-per-day agenda).

**Architecture:** Three stacked PRs on `feat/calendar-view`.
- PR 1 changes the frontmatter type in `sapphire-journal-api`, and core, the CLI, MCP and the desktop follow it.
- PR 2 replaces the desktop's own pickers with `corundum_framework_gui::datetime` and makes the shared `HomeState.period` an `Option<datetime::Period>`.
- PR 3 adds `screens/calendar/`, which reads a cached per-day index built from the entry list.

**Tech Stack:** Rust 2024, chrono, serde, schemars, clap, egui 0.37, sapphire-framework 0.28, corundum-framework-gui 0.11 (`datetime` = alumina-datetime 0.2).

**Spec:** `docs/superpowers/specs/2026-11-08-calendar-view-design.md`

## Global Constraints

- `DateOrTime` writes `YYYY-MM-DD` (date only) or `YYYY-MM-DDTHH:MM`, and never drops `T00:00`.
- `end_bound()` of a `Date` is that day at 23:59:59. `start_bound()` is 00:00:00.
- Due dates compare by `end_bound()`. Event spans compare by `start.start_bound()` and `end.end_bound()`.
- All-day means: an event with both ends `Date`, or a task whose `due` is a `Date`. Mixed ends count as timed.
- `started_at`, `closed_at`, `created_at` and `updated_at` stay `NaiveDateTime`.
- `sapphire-journal-api` becomes `5.0.0`, and every path dependency on it says `version = "5.0.0"`. The cache `SCHEMA_VERSION` becomes `7`.
- Desktop dependency: `corundum-framework-gui = { version = "0.11.0", default-features = false, features = ["datetime"] }`. Every picker gets `.config(datetime::config(ctx))`.
- Weeks start on Monday (`chrono::Weekday::Mon`).
- `settings.toml` uses `startup_period = "last" | "today" | "this_week" | "this_month"` (default `last`) and `last_period` (`Period` `Display`, or `""` for all). A missing or unparsable `last_period` means this week.
- Calendar constants: timeline `HOUR_HEIGHT = 48.0`, `MIN_BLOCK = 20.0`, initial scroll to 08:00. The month grid shows 42 days from the Monday on or before the 1st. "All" in the agenda spans today ± 36500 days.
- A task counts as done for display when its status is `done` or `cancelled`.
- No calendar editing (drag or create) and no migration command.
- CI gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## Review Focus

1. **A hand-edited event whose end is before its start.** The calendar must neither panic nor loop over a huge range. It treats such an event as lasting from its start to its start (Task 11, `inverted_event_is_one_day`).
2. **A timed event ending exactly at 00:00 the next day.** It must not show an empty sliver on the next day (Task 11 `last_day`, and Task 13 `segment_on`).
3. **An event spanning decades, such as `end: 2099-12-31`.** Building the index must not insert one row per day. Multi-day items live in a span list that is checked per visible day (Task 11, `long_span_is_not_expanded`).
4. **A task due at `T00:00` (timed) versus a task due on `2026-11-08` (date only).** Only the date-only one is all-day, and the timed one sits at the top of the timeline (Task 11, `midnight_due_is_timed`).
5. **A date-only due date that is today.** It must not be flagged overdue before the day ends, in both labels and the `EntryFilter` overdue selector (Task 2, `date_due_today_is_not_overdue`).

---

## Phase 1 — PR 1: `DateOrTime`

### Task 1: The `DateOrTime` type

**Files:**
- Create: `crates/sapphire-journal-api/src/date_or_time.rs`
- Modify: `crates/sapphire-journal-api/src/lib.rs` (add `mod date_or_time; pub use date_or_time::DateOrTime;`)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum DateOrTime { Date(NaiveDate), DateTime(NaiveDateTime) }
  impl DateOrTime {
      pub fn start_bound(&self) -> NaiveDateTime;
      pub fn end_bound(&self) -> NaiveDateTime;
      pub fn is_date(&self) -> bool;
      pub fn date(&self) -> NaiveDate;
  }
  impl FromStr for DateOrTime { type Err = String; }
  impl Display;  // and Serialize, Deserialize (as a string), schemars::JsonSchema
  ```

- [ ] **Step 1: Write failing tests** in `date_or_time.rs` `mod tests`:
  - `date_round_trips`: parsing `"2026-11-08"` gives `Date`, and `to_string()` gives `"2026-11-08"`.
  - `midnight_keeps_its_time`: parsing `"2026-11-08T00:00"` gives `DateTime`, and `to_string()` gives `"2026-11-08T00:00"`.
  - `legacy_formats_read`: `"2026-11-08T09:30:15"` and `"2026-11-08T09:30:15.250"` parse as `DateTime`, and both display as `"2026-11-08T09:30"`.
  - `bounds_of_a_date`: `start_bound()` gives `2026-11-08T00:00:00` and `end_bound()` gives `2026-11-08T23:59:59`.
  - `bounds_of_a_datetime_are_itself`.
  - `serde_yaml_round_trip`: through `serde_json` with the value `"2026-11-08"` and the value `"2026-11-08T10:00"`.
  - `rejects_garbage`: `"11/08"` is an `Err` whose message names `YYYY-MM-DD`.
- [ ] **Step 2:** `cargo test -p sapphire-journal-api date_or_time` fails (the type is not defined yet).
- [ ] **Step 3: Implement.**
  - Parse the three datetime formats first, as `naive_datetime_serde::parse_with_fallback` does, then `%Y-%m-%d`.
  - Write `JsonSchema` by hand: `type: string` with the description `"YYYY-MM-DD (all day) or YYYY-MM-DDTHH:MM"`.
- [ ] **Step 4:** The tests pass.
- [ ] **Step 5:** Commit `feat(api): DateOrTime, a date (all day) or a date and time`.

### Task 2: Frontmatter fields and core

**Files:**
- Modify: `crates/sapphire-journal-api/src/entry.rs` (`TaskMeta.due`, `TaskMetaView.due` → `Option<DateOrTime>`; `EventMeta`/`EventMetaView` `start`, `end` → `DateOrTime`; delete `naive_datetime_serde::eod`)
- Modify: `crates/sapphire-journal-core/src/ops.rs` (`EntryFields.task_due`, `event_start`, `event_end` → `Option<DateOrTime>`; filter at ~228–300; sort at ~600; create/update at ~700–870)
- Modify: `crates/sapphire-journal-core/src/labels.rs:71`, `crates/sapphire-journal-core/src/cache.rs` (write `to_string()` and read `parse().ok()` for the three columns; `SCHEMA_VERSION = 7`), `crates/sapphire-journal-core/src/text_input/fields.rs`, `crates/sapphire-journal-core/src/parser.rs` (wherever it builds these fields)
- Modify: `crates/sapphire-journal-api/Cargo.toml` (`version = "5.0.0"`) and the five dependents' `sapphire-journal-api` lines

**Interfaces:**
- Consumes: Task 1's `DateOrTime`.
- Produces:
  - `sapphire_journal_core::text_input::fields::parse_optional_date_or_time(s: Option<&str>) -> Result<Option<DateOrTime>>`.
  - `parse_optional_datetime` stays, for `started_at` and `closed_at`. `parse_optional_datetime_end` is deleted.
  - `EntryFields { task_due: Option<DateOrTime>, event_start: Option<DateOrTime>, event_end: Option<DateOrTime>, .. }`.

- [ ] **Step 1: Write failing tests** in `ops.rs` `mod tests`:
  - `date_due_matches_today`: a task with `due = Date(2026-11-08)` matches a filter over `Period::Range(2026-11-08T00:00:00, 2026-11-08T23:59:59)` with `fields.task_due`.
  - `date_due_today_is_not_overdue`: in `labels.rs` with now `2026-11-08T18:00`, the `Date(2026-11-08)` task is not overdue. With now `2026-11-09T00:00:01`, it is overdue. With the `EntryFilter` `task_overdue` selector over `today` = 2026-11-08, it does not get `TaskOverdue` while today is the due day.
  - `timed_event_ending_yesterday_is_out`: an event `2026-11-07T22:00`–`2026-11-07T23:59` does not overlap 2026-11-08.
  - `all_day_event_over_two_days_overlaps_both`: `Date(11-07)`–`Date(11-08)` overlaps both days.
  - `create_keeps_date_only_due`: `create_entry` with `task_due: Some(Date(..))` writes a file containing `due: 2026-11-08`.
  - In `cache.rs` tests (or a new one): `cache_round_trips_date_or_time`. Store and load a header with a `Date` due and a `DateTime` event, and expect equal values.
- [ ] **Step 2:** `cargo test -p sapphire-journal-core` fails to compile or fails.
- [ ] **Step 3: Change the field types** and fix every comparison to use bounds:
  - `p.matches(due.map(|d| d.end_bound()))`
  - `overlaps_event(Some(e.start.start_bound()), Some(e.end.end_bound()))`
  - overdue: `due.end_bound() <= *end` and `due.end_bound() < now`
  - sort keys: `end_bound()` for due, `start_bound()` for event start
  - The "end ≥ start" check compares bounds.
- [ ] **Step 4:** `cargo test -p sapphire-journal-api -p sapphire-journal-core` passes.
- [ ] **Step 5:** Commit `feat(core)!: due and event dates keep a date-only value as all day`.

### Task 3: CLI and MCP inputs

**Files:**
- Modify: `crates/sapphire-journal-cli/src/commands/entry.rs:13,299-320,356-383,625` (clap `value_parser = DateOrTime::from_str`, `value_name = "DATE[THH:MM]"`, fields `Option<DateOrTime>`, and params via `.map(|d| d.to_string())`)
- Modify: `crates/sapphire-journal-mcp/src/methods.rs:288-308,398` (`parse_optional_date_or_time` for `task_due`, `event_start` and `event_end`)
- Test: `crates/sapphire-journal-cli/tests/cli_parse.rs`

**Interfaces:** Consumes Task 2's `parse_optional_date_or_time` and `EntryFields`.

- [ ] **Step 1: Write failing tests** in `cli_parse.rs`:
  - `due_date_only_parses_as_date`: `entry modify X --task-due 2026-11-08` parses to `Some(DateOrTime::Date(..))`.
  - `event_start_with_time_parses_as_datetime`: `--event-start 2026-11-08T09:00`.
- [ ] **Step 2:** `cargo test -p sapphire-journal-cli --test cli_parse` fails.
- [ ] **Step 3:** Change the types and parsers. Update the MCP tool parameter descriptions to read `"YYYY-MM-DD (all day) or YYYY-MM-DDTHH:MM"`.
- [ ] **Step 4:** `cargo test -p sapphire-journal-cli -p sapphire-journal-mcp` passes.
- [ ] **Step 5:** Commit `feat(cli,mcp)!: a date without a time is stored as all day`.

### Task 4: Desktop follows the type; PR 1 gate

**Files:**
- Modify: `desktop/src/screens/entry_pane.rs:398-410,428-431,469-510` (load via `to_string()`, save via `trim().parse::<DateOrTime>()`)
- Modify: `desktop/src/screens/journal_home.rs:1046-1055`, `desktop/src/screens/kanban.rs` (sorts and labels use bounds)

- [ ] **Step 1:** `cargo build --workspace` shows the remaining type errors. Fix them. No behaviour change beyond parsing through `DateOrTime`.
- [ ] **Step 2:** `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` all pass.
- [ ] **Step 3:** Commit `fix(desktop): follow DateOrTime`. Open PR 1 against `main`.

## Phase 2 — PR 2: Desktop base

### Task 5: alumina pickers and `Option<Period>` state

**Files:**
- Modify: `desktop/Cargo.toml` (the corundum-framework-gui dependency from Global Constraints)
- Create: `desktop/src/period_nav.rs` (registered in `main.rs`)
- Modify: `desktop/src/app.rs` (`HomeState.period: Option<datetime::Period>`; delete `calendar_cursor`), `desktop/src/screens/journal_home.rs` (toolbar uses `PeriodPickerButton`; `filter_and_sort` uses `to_filter`)
- Delete: `desktop/src/widgets/period_picker.rs`, `desktop/src/widgets/month_grid.rs` (and their `mod` lines)

**Interfaces:**
- Produces in `period_nav.rs`:
  ```rust
  pub fn to_filter(period: &Period) -> sapphire_journal_core::period::Period; // first_day 00:00:00 ..= last_day 23:59:59
  pub fn period_string(period: Option<&Period>) -> String;                   // Display, or "" for all
  pub fn parse_period_string(s: &str) -> Option<Option<Period>>;            // "" → Some(None); bad → None
  ```

- [ ] **Step 1: Write failing tests** in `period_nav.rs`:
  - `filter_of_a_week`: `Week{start: 2026-11-02}` gives `Range(2026-11-02T00:00:00, 2026-11-08T23:59:59)`.
  - `period_string_round_trip`: for `2026-W45`, `2026-11`, `2026`, `2026-11-01..2026-11-10` and `""`.
  - `bad_period_string_is_none`.
- [ ] **Step 2:** `cargo test -p sapphire-journal-desktop period_nav` fails.
- [ ] **Step 3: Implement**, then swap the toolbar to the following and delete the two widgets:
  ```rust
  ui.add(PeriodPickerButton::new("home_period", &mut home.period)
      .clearable(true).day_badge(&|d| count_on(entries, d)).config(datetime::config(ui.ctx())))
  ```
- [ ] **Step 4:** The tests pass, and `cargo clippy -p sapphire-journal-desktop --all-targets -- -D warnings` is clean.
- [ ] **Step 5:** Commit `feat(desktop): pick the period with corundum's period picker`.

### Task 6: ◀ ▶ period steps

**Files:** Modify `desktop/src/period_nav.rs` and `desktop/src/screens/journal_home.rs` (the buttons on each side of the picker, using `icons::chevron_left()` and `icons::chevron_right()`, or the corundum icons).

**Interfaces:** Produces `pub fn shift(period: &Period, by: i32) -> Option<Period>`.

- [ ] **Step 1: Write failing tests:**
  - `shift_day`: `Day(2026-12-31)` + 1 gives `Day(2027-01-01)`.
  - `shift_iso_week_across_year`: `2026-W53` + 1 gives `2027-W01`, and − 1 from `2027-W01` gives `2026-W53`.
  - `shift_month`: `2026-12` + 1 gives `2027-01`.
  - `shift_year`: `2026` − 1 gives `2025`.
  - `shift_range_is_none`.
- [ ] **Step 2:** The tests fail.
- [ ] **Step 3: Implement** with `datetime::calc::add_months` for months, and `Period::Week { start: start ± 7 days }` for weeks. Disable both buttons when `home.period` is `None` or `shift` returns `None`.
- [ ] **Step 4:** The tests pass.
- [ ] **Step 5:** Commit `feat(desktop): step the period back and forward`.

### Task 7: Startup period setting

**Files:** Modify `desktop/src/settings.rs` and `desktop/src/app.rs` (the initial `HomeState.period`, and saving `last_period` on change next to the `view` and `list_layout` save).

**Interfaces:**
- Produces:
  ```rust
  #[serde(rename_all = "snake_case")] #[derive(Default, ..)]
  pub enum StartupPeriod { #[default] Last, Today, ThisWeek, ThisMonth }
  // Settings gains: pub startup_period: StartupPeriod, pub last_period: Option<String>
  impl Settings { pub fn initial_period(&self, today: NaiveDate) -> Option<Period>; }
  ```

- [ ] **Step 1: Write failing tests** in `settings.rs`:
  - `initial_today`: gives `Day(today)`.
  - `initial_this_week`: on Sunday 2026-11-08 gives `Week{start: 2026-11-02}`.
  - `initial_this_month`.
  - `last_restores`: `Some("2026-11")` gives `Month`.
  - `last_all`: `Some("")` gives `None`.
  - `last_missing_is_this_week`.
  - `last_garbage_is_this_week`.
  - `old_file_loads`: a TOML with only `view` and `list_layout` loads with `Last` and `None`.
- [ ] **Step 2:** The tests fail.
- [ ] **Step 3:** Implement. In `Window::ui`, save when `settings.last_period.as_deref() != Some(&period_string(home.period.as_ref()))`.
- [ ] **Step 4:** The tests pass.
- [ ] **Step 5:** Commit `feat(desktop): choose the period the window opens on`.

### Task 8: Settings "Desktop" page

**Files:**
- Create: `desktop/src/screens/desktop_settings.rs` (registered in `screens/mod.rs`)
- Modify: `desktop/src/app.rs` (`App.settings: Rc<RefCell<Settings>>`; add the page to `with_app_sections` after the Journal page)

**Interfaces:**
- Consumes Task 7's `StartupPeriod`.
- Produces `pub struct DesktopSettingsPage` with `pub fn new(settings: Rc<RefCell<Settings>>) -> Self` and `impl SettingsSection` (label `"Desktop"`).

- [ ] **Step 1:** Implement the page. It has the heading `"Startup period"` and a `ComboBox` with the options `"Last period shown"`, `"Today"`, `"This week"` and `"This month"`. A change calls `settings.save()` and shows the error inline if saving fails. There is no Save button.
- [ ] **Step 2:** `cargo test -p sapphire-journal-desktop` and clippy pass. Run the app (`cargo run -p sapphire-journal-desktop`). Pick "Today" and restart: the window opens on today. Pick "Last period shown", set `2026-11`, and restart: it opens on `2026-11`.
- [ ] **Step 3:** Commit `feat(desktop): a Desktop settings page with the startup period`.

### Task 9: Rail "new entry" action

**Files:** Modify `desktop/src/app.rs` (`Shell::new(..).with_actions(vec![Action::new("new_entry", icons::plus(), "New entry")])`; `impl ShellApp::action`) and `desktop/src/screens/journal_home.rs` (remove the toolbar "New Entry" button).

- [ ] **Step 1:** Implement `fn action(&mut self, id: &'static str, side: &mut Side)`. On `"new_entry"` it calls `self.home.new_entry(None)` and then `self.home.apply_pane_request(side)`.
- [ ] **Step 2:** Run the app and check three cases:
  - Clicking ＋ on the rail opens a new entry from List, from Kanban and from Settings.
  - With unsaved edits it asks first.
  - The context menu still has "Add child entry".
- [ ] **Step 3:** Commit `feat(desktop): new entry from the rail`.

### Task 10: Entry pane date inputs with "All day"

**Files:**
- Modify: `desktop/src/app.rs` (`EditorState.task_due: Option<DateOrTime>`, `event_start: DateOrTime`, `event_end: DateOrTime`; dirty check compares them)
- Modify: `desktop/src/screens/entry_pane.rs:320-365,390-515`
- Create: `desktop/src/widgets/date_or_time.rs`
- Delete: `desktop/src/widgets/date_picker.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn date_or_time_input(ui: &mut egui::Ui, id_salt: &str, value: &mut DateOrTime) -> egui::Response; // DatePickerButton if Date, DateTimePickerButton if DateTime
  pub fn set_event_all_day(start: &mut DateOrTime, end: &mut DateOrTime, all_day: bool);
  pub fn set_due_all_day(due: &mut DateOrTime, all_day: bool);
  ```

- [ ] **Step 1: Write failing tests:**
  - `event_all_day_on`: `09:00`/`10:30` on 11-08 become `Date(11-08)` for both.
  - `event_all_day_off`: `Date(11-08)`/`Date(11-09)` become `2026-11-08T09:00`/`2026-11-09T10:00`.
  - `due_all_day_off`: `Date(11-08)` becomes `2026-11-08T09:00`.
  - `due_all_day_on` keeps the date.
- [ ] **Step 2:** The tests fail.
- [ ] **Step 3: Implement**, then rebuild the pane rows:
  - Due row: a "Due" checkbox (`Option` some or none), the input, and an "All day" checkbox.
  - Event: one shared "All day" checkbox, then the start input and the end input.
  - A new due date defaults to `Date(today)`. A new event defaults to `Date(today)`–`Date(today)`.
- [ ] **Step 4:** The tests pass. Run the app: save a date-only due date and confirm the file says `due: 2026-11-08`. Toggle "All day" off and confirm it says `due: 2026-11-08T09:00`.
- [ ] **Step 5:** Run the PR 2 gate (fmt, clippy, workspace tests). Commit `feat(desktop): date pickers with an all-day switch`. Open PR 2 stacked on PR 1.

## Phase 3 — PR 3: Calendar view

### Task 11: Calendar items and index

**Files:** Create `desktop/src/screens/calendar/mod.rs` (only `pub mod` lines for now) and `desktop/src/screens/calendar/items.rs`. Modify `desktop/src/screens/journal_home.rs`: extract `pub fn matches_search(h: &EntryHeader, needle: &str) -> bool` from `filter_and_sort`.

**Interfaces:**
- Produces:
  ```rust
  pub enum ItemKind { Event { start: DateOrTime, end: DateOrTime }, Due(DateOrTime) }
  pub struct CalendarItem { pub path: PathBuf, pub title: String, pub kind: ItemKind, pub all_day: bool, pub done: bool }
  impl CalendarItem {
      pub fn span(&self) -> (NaiveDateTime, NaiveDateTime); // bounds; inverted end clamped to start; Due(d) → (d.start_bound(), d.start_bound()) if timed, day bounds if Date
      pub fn first_day(&self) -> NaiveDate;
      pub fn last_day(&self) -> NaiveDate;                  // a timed end at exactly 00:00 after the first day → the day before
  }
  pub fn items_of(header: &EntryHeader) -> Vec<CalendarItem>;
  pub struct CalendarIndex { /* items, by_day: BTreeMap<NaiveDate, Vec<usize>> for one-day items, spans: Vec<usize> for multi-day */ }
  impl CalendarIndex {
      pub fn build(headers: &[EntryHeader], needle: &str) -> Self;
      pub fn on(&self, day: NaiveDate) -> Vec<&CalendarItem>; // all-day first, then span start, then title
  }
  pub struct CalendarCache { /* needle, index: Option<CalendarIndex>, builds: usize */ }
  impl CalendarCache {
      pub fn invalidate(&mut self);
      pub fn get(&mut self, headers: &[EntryHeader], needle: &str) -> &CalendarIndex; // rebuilds only when invalidated or the needle changed
  }
  ```
  The spec says multi-day events are registered on each day. The span list keeps that observable behaviour (`on(day)` returns them for every day they cover) without one insert per day (Review Focus 3).

- [ ] **Step 1: Write failing tests** in `items.rs`:
  - `note_has_no_items`.
  - `event_and_due_give_two_items`.
  - `all_day_detection`: `Date`/`Date` gives true, `Date`/`DateTime` gives false, `Due(Date)` gives true.
  - `midnight_due_is_timed`: `Due(2026-11-08T00:00)` has `all_day == false`.
  - `done_and_cancelled_are_done`: `done` and `cancelled` are done, `in_progress` is not.
  - `multi_day_event_on_each_day`: an event 11-07 → 11-09 appears in `on()` for 11-07, 11-08 and 11-09, and not for 11-10.
  - `midnight_end_stops_the_day_before`: `2026-11-07T22:00`–`2026-11-08T00:00` has `last_day == 11-07`.
  - `inverted_event_is_one_day`: end before start gives `first_day == last_day == start date`.
  - `long_span_is_not_expanded`: an event 2026-01-01 → 2099-12-31 has `by_day` empty and `spans.len() == 1`, and `on(2050-06-01)` contains it.
  - `search_filters`: a needle not matching the title gives an empty `on()`.
  - `order_within_day`: all-day, then 08:00, then 09:00.
  - `cache_rebuilds_only_when_needed`: two `get` calls with the same needle give `builds == 1`. After `invalidate` it is 2, and with another needle it is 3.
- [ ] **Step 2:** `cargo test -p sapphire-journal-desktop calendar::items` fails.
- [ ] **Step 3:** Implement. Task status `done` or `cancelled` sets `done`.
- [ ] **Step 4:** The tests pass.
- [ ] **Step 5:** Commit `feat(desktop): calendar items and their per-day index`.

### Task 12: Calendar view wiring and chips

**Files:**
- Modify: `desktop/src/settings.rs` (`MainView::Calendar`), `desktop/src/app.rs` (a rail `Destination::new(MainView::Calendar, icons::calendar(), "Calendar")` after Kanban; `HomeState.calendar: CalendarCache`; `invalidate()` in the `Reply::Entries` `Ok` arm), `desktop/src/screens/journal_home.rs` (dispatch `MainView::Calendar => calendar::show(home, ui)`)
- Modify: `desktop/src/screens/calendar/mod.rs` (`pub fn show(home: &mut HomeState, ui: &mut egui::Ui)`, dispatching on `home.period.map(|p| p.kind())`: `Day` or `Week` → `timeline::show`, `Month` → `month::show`, otherwise → `agenda::show`)
- Create: `desktop/src/screens/calendar/chip.rs`

**Interfaces:**
- Produces:
  - `pub fn chip(ui: &mut egui::Ui, item: &CalendarItem, label: &str, selected: bool) -> egui::Response`. Event: filled with the selection colour. Task: outlined, with `icons::check()` or the closest existing icon. Done: weak text with strikethrough. The hover text is the title plus the date (date only when all-day) or `YYYY-MM-DD HH:MM–HH:MM`.
  - `pub fn open_day(home: &mut HomeState, day: NaiveDate)`, which sets `home.period = Some(Period::Day(day))`.
- Each view module exposes `pub fn show(home: &mut HomeState, index_days: ..., ui)`. The implementer picks the exact parameters, but they take `&mut HomeState` and the `Period`.

- [ ] **Step 1:** Implement the wiring with stub `timeline`, `month` and `agenda` `show` functions that each print their kind.
- [ ] **Step 2:** Run the app: the Calendar rail button shows, switching periods changes the stub, and the chosen view persists across restarts. `old_file_loads` still passes.
- [ ] **Step 3:** Commit `feat(desktop): the calendar view on the rail`.

### Task 13: Day and week timeline

**Files:** Create `desktop/src/screens/calendar/timeline.rs`.

**Interfaces:**
- Produces:
  ```rust
  pub const HOUR_HEIGHT: f32 = 48.0;
  pub const MIN_BLOCK: f32 = 20.0;
  pub fn y_of(minutes: u32) -> f32;                                      // minutes * HOUR_HEIGHT / 60
  pub fn segment_on(item: &CalendarItem, day: NaiveDate) -> Option<(u32, u32)>; // minutes 0..=1440 clipped to day; None if not on day or all-day
  pub fn assign_columns(segments: &[(u32, u32)]) -> Vec<(usize, usize)>; // (column, columns in its overlap group), input order kept
  ```
  `assign_columns` treats each segment as lasting at least `MIN_BLOCK` worth of minutes (25) when testing overlap, so a due marker and a block at the same time do not overlap on screen.

- [ ] **Step 1: Write failing tests:**
  - `y_of_noon`: `y_of(720) == 576.0`.
  - `segment_clipped_to_day`: an event 11-07 22:00 → 11-08 02:00 gives `(1320, 1440)` on 11-07 and `(0, 120)` on 11-08.
  - `segment_midnight_end_none_next_day`: an event ending 11-08 00:00 gives `None` on 11-08.
  - `due_segment_is_a_point`: due 15:00 gives `(900, 900)`.
  - `all_day_has_no_segment`.
  - `columns_no_overlap`: `[(0,60),(60,120)]` gives `[(0,1),(0,1)]`.
  - `columns_two_overlap`: `[(0,120),(60,180)]` gives `[(0,2),(1,2)]`.
  - `columns_chain`: `[(0,120),(60,180),(150,240)]` gives one group of 2 columns, `[(0,2),(1,2),(0,2)]`.
- [ ] **Step 2:** The tests fail.
- [ ] **Step 3: Implement the math, then `show`:**
  - Layout:
    - A header row: weekday + date per column. Today uses the strong colour, and a click calls `open_day`.
    - An all-day strip: `chip` per all-day item per column.
    - A vertical `ScrollArea` of `24 * HOUR_HEIGHT`, with an hour gutter of 40px.
  - Blocks: `ui.put` at `y_of(start)`, with height `max(y_of(end) - y_of(start), MIN_BLOCK)`. Width = column width / columns, offset by column. Timed blocks are labelled `HH:MM title`.
  - Now line: drawn on today's column.
  - Initial scroll: `vertical_scroll_offset(y_of(480))`, once per period change. Store the period last scrolled for in `HomeState.calendar_scrolled: Option<Period>`.
  - Week view uses 7 columns from `Week.start`; day view uses 1.
- [ ] **Step 4:** The tests pass. Run the app with a fixture journal holding overlapping, midnight-crossing and all-day events: the week and day views look right, and clicking a block opens the entry in the side pane.
- [ ] **Step 5:** Commit `feat(desktop): day and week timeline`.

### Task 14: Month grid

**Files:** Create `desktop/src/screens/calendar/month.rs`.

**Interfaces:** Produces `pub fn grid_start(year: i32, month: u32) -> NaiveDate`, the Monday on or before the 1st. The grid is 42 days from it.

- [ ] **Step 1: Write failing tests:**
  - `grid_starts_on_the_first_when_monday`: 2026-06 (the 1st is a Monday) gives 2026-06-01.
  - `grid_starts_before_when_sunday`: 2026-11 (the 1st is a Sunday) gives 2026-10-26.
  - `grid_covers_month_end`: `grid_start(2026, 11) + 41 days >= 2026-11-30`.
- [ ] **Step 2:** The tests fail.
- [ ] **Step 3: Implement the math, then `show`:**
  - A 7×6 `egui::Grid` with equal cell sizes from the available rect. A header row gives the weekday names from `datetime::labels(ctx).weekdays_short`.
  - Each cell: the day number (weak outside the month, strong when today; clickable via `open_day`), then chips while they fit (all-day ones as the title, timed ones as `HH:MM title`), then `+n` (clickable via `open_day`).
- [ ] **Step 4:** The tests pass, and the app shows the month correctly.
- [ ] **Step 5:** Commit `feat(desktop): month grid`.

### Task 15: Agenda; PR 3 gate

**Files:** Create `desktop/src/screens/calendar/agenda.rs`.

**Interfaces:**
- Produces:
  ```rust
  pub const ROW_HEIGHT: f32 = 24.0;
  pub struct AgendaRange { pub first: NaiveDate, pub len: usize }
  pub fn agenda_range(period: Option<&Period>, today: NaiveDate) -> AgendaRange; // Some → first_day..=last_day; None → today-36500 ..= today+36500
  impl AgendaRange { pub fn date_at(&self, row: usize) -> NaiveDate; pub fn row_of(&self, day: NaiveDate) -> Option<usize>; }
  ```

- [ ] **Step 1: Write failing tests:**
  - `year_has_365_rows`: 2026 gives 365 rows, 2028 gives 366.
  - `range_ends`: `2026-11-01..2026-11-10` gives `len == 10`, `date_at(9) == 11-10`, `row_of(11-11) == None`.
  - `all_is_centred_on_today`: `len == 73001` and `row_of(today) == Some(36500)`.
- [ ] **Step 2:** The tests fail.
- [ ] **Step 3: Implement the math, then `show`:**
  - Use `ScrollArea::vertical().show_rows(ui, ROW_HEIGHT, range.len, ..)`. On the first show for a period, set `vertical_scroll_offset(row_of(today or first) * ROW_HEIGHT)`, using `HomeState.calendar_scrolled` as in Task 13.
  - Row label: `MM-DD (曜)` from `datetime::labels`, with `YYYY-` prefixed on January 1st and on the first visible row. The label is clickable via `open_day`.
  - Background: weak on weekends, highlighted on today.
  - Chips are laid out horizontally and clipped to the row. When they do not all fit, the overflow becomes `+n`.
- [ ] **Step 4:** The tests pass. Run the app:
  - `2026` scrolls through 365 rows.
  - All (cleared) opens on today and scrolls smoothly far up and down.
  - A journal with many entries stays responsive.
- [ ] **Step 5:** Run the PR 3 gate (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`). Commit `feat(desktop): one-row-per-day agenda`. Open PR 3 stacked on PR 2.
