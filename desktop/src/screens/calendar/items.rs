//! What the calendar draws: an entry's event and its task's due date, and an
//! index to find the ones that fall on a day.

use std::collections::BTreeMap;
use std::path::PathBuf;

use jiff::civil::{Date, DateTime, Time};
use sapphire_journal_api::DateOrTime;
use sapphire_journal_core::entry::EntryHeader;

use crate::screens::journal_home::matches_search;

#[derive(Debug, Clone)]
pub enum ItemKind {
    Event { start: DateOrTime, end: DateOrTime },
    Due(DateOrTime),
}

#[derive(Debug, Clone)]
pub struct CalendarItem {
    pub path: PathBuf,
    pub title: String,
    pub kind: ItemKind,
    /// Both ends of an event are dates, or the due is a date.
    pub all_day: bool,
    /// The task's status is `done` or `cancelled`.
    pub done: bool,
}

impl CalendarItem {
    /// The bounds of the item, as wall-clock times here. An end before the
    /// start is clamped to it, so a hand-edited event lasts from its start to
    /// its start.
    pub fn span(&self) -> (DateTime, DateTime) {
        let here = |t| sapphire_journal_api::time::local(t).datetime();
        match &self.kind {
            ItemKind::Event { start, end } => {
                let s = here(start.start_bound());
                (s, here(end.end_bound()).max(s))
            }
            ItemKind::Due(d) if d.is_date() => (here(d.start_bound()), here(d.end_bound())),
            ItemKind::Due(d) => (here(d.start_bound()), here(d.start_bound())),
        }
    }

    pub fn first_day(&self) -> Date {
        self.span().0.date()
    }

    /// The last day the item shows on. A timed end at exactly 00:00 on a
    /// later day belongs to the day before, so it leaves no empty sliver.
    pub fn last_day(&self) -> Date {
        let first = self.first_day();
        let end = self.span().1;
        if end.date() > first && end.time() == Time::midnight() {
            end.date().yesterday().ok().unwrap_or(first).max(first)
        } else {
            end.date()
        }
    }
}

/// The items an entry puts on the calendar: its event and its task's due date.
pub fn items_of(header: &EntryHeader) -> Vec<CalendarItem> {
    let fm = &header.frontmatter;
    let mut items = Vec::new();
    if let Some(e) = &fm.event {
        items.push(CalendarItem {
            path: PathBuf::from(&header.path),
            title: fm.title.clone(),
            kind: ItemKind::Event {
                start: e.start,
                end: e.end,
            },
            all_day: e.start.is_date() && e.end.is_date(),
            done: false,
        });
    }
    if let Some(t) = &fm.task
        && let Some(due) = t.due
    {
        items.push(CalendarItem {
            path: PathBuf::from(&header.path),
            title: fm.title.clone(),
            kind: ItemKind::Due(due),
            all_day: due.is_date(),
            done: matches!(t.status.as_str(), "done" | "cancelled"),
        });
    }
    items
}

/// The items that match a search, found by day. One-day items are kept per
/// day; longer ones in a list checked for each requested day, so an event
/// that lasts decades costs one entry, not one per day.
#[derive(Debug, Default)]
pub struct CalendarIndex {
    items: Vec<CalendarItem>,
    by_day: BTreeMap<Date, Vec<usize>>,
    spans: Vec<usize>,
}

impl CalendarIndex {
    pub fn build(headers: &[EntryHeader], needle: &str) -> Self {
        let mut index = Self::default();
        for h in headers.iter().filter(|h| matches_search(h, needle)) {
            for item in items_of(h) {
                let at = index.items.len();
                let (first, last) = (item.first_day(), item.last_day());
                if first == last {
                    index.by_day.entry(first).or_default().push(at);
                } else {
                    index.spans.push(at);
                }
                index.items.push(item);
            }
        }
        index
    }

    /// The items on `day`: all-day first, then by start, then by title.
    pub fn on(&self, day: Date) -> Vec<&CalendarItem> {
        let one_day = self.by_day.get(&day).into_iter().flatten();
        let spanning = self
            .spans
            .iter()
            .filter(|&&i| (self.items[i].first_day()..=self.items[i].last_day()).contains(&day));
        let mut found: Vec<&CalendarItem> =
            one_day.chain(spanning).map(|&i| &self.items[i]).collect();
        found.sort_by(|a, b| {
            b.all_day
                .cmp(&a.all_day)
                .then_with(|| a.span().0.cmp(&b.span().0))
                .then_with(|| a.title.cmp(&b.title))
        });
        found
    }
}

/// The index for the current search, kept until the entries change or the
/// search does.
#[derive(Debug, Default)]
pub struct CalendarCache {
    needle: String,
    index: Option<CalendarIndex>,
    builds: usize,
}

impl CalendarCache {
    pub fn invalidate(&mut self) {
        self.index = None;
    }

    pub fn get(&mut self, headers: &[EntryHeader], needle: &str) -> &CalendarIndex {
        if self.index.is_none() || self.needle != needle {
            self.index = Some(CalendarIndex::build(headers, needle));
            self.needle = needle.to_string();
            self.builds += 1;
        }
        self.index.as_ref().expect("built above")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapphire_journal_core::entry::{EventMetaView, FrontmatterView, TaskMetaView};

    fn day(d: i8) -> Date {
        jiff::civil::date(2026, 11, d)
    }

    fn at(d: i8, h: i8, m: i8) -> jiff::Timestamp {
        sapphire_journal_api::time::from_local(day(d).at(h, m, 0, 0)).unwrap()
    }

    fn header(
        title: &str,
        event: Option<(DateOrTime, DateOrTime)>,
        task: Option<(Option<DateOrTime>, &str)>,
    ) -> EntryHeader {
        let t = at(1, 0, 0);
        EntryHeader {
            path: format!("/{title}.md"),
            frontmatter: FrontmatterView {
                id: grain_id::GrainId::now_unix(),
                parent_id: None,
                title: title.into(),
                slug: title.into(),
                created_at: t,
                updated_at: t,
                tags: Vec::new(),
                task: task.map(|(due, status)| TaskMetaView {
                    due,
                    status: status.into(),
                    started_at: None,
                    closed_at: None,
                }),
                event: event.map(|(start, end)| EventMetaView { start, end }),
                hidden: None,
            },
            flags: Vec::new(),
        }
    }

    fn event(title: &str, start: DateOrTime, end: DateOrTime) -> EntryHeader {
        header(title, Some((start, end)), None)
    }

    fn task(title: &str, due: Option<DateOrTime>, status: &str) -> EntryHeader {
        header(title, None, Some((due, status)))
    }

    fn dt(d: i8, h: i8, m: i8) -> DateOrTime {
        DateOrTime::Time(at(d, h, m))
    }

    fn date(d: i8) -> DateOrTime {
        DateOrTime::Date(day(d))
    }

    #[test]
    fn note_has_no_items() {
        assert!(items_of(&header("n", None, None)).is_empty());
    }

    #[test]
    fn event_and_due_give_two_items() {
        let h = header(
            "both",
            Some((dt(8, 9, 0), dt(8, 10, 0))),
            Some((Some(date(9)), "open")),
        );
        let items = items_of(&h);
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0].kind, ItemKind::Event { .. }));
        assert!(matches!(items[1].kind, ItemKind::Due(_)));
    }

    #[test]
    fn all_day_detection() {
        let a = &items_of(&event("a", date(8), date(9)))[0];
        assert!(a.all_day);
        let b = &items_of(&event("b", date(8), dt(9, 10, 0)))[0];
        assert!(!b.all_day);
        let c = &items_of(&task("c", Some(date(8)), "open"))[0];
        assert!(c.all_day);
    }

    #[test]
    fn midnight_due_is_timed() {
        let c = &items_of(&task("c", Some(dt(8, 0, 0)), "open"))[0];
        assert!(!c.all_day);
    }

    #[test]
    fn done_and_cancelled_are_done() {
        let done = |s: &str| items_of(&task("t", Some(date(8)), s))[0].done;
        assert!(done("done"));
        assert!(done("cancelled"));
        assert!(!done("in_progress"));
    }

    #[test]
    fn multi_day_event_on_each_day() {
        let idx = CalendarIndex::build(&[event("e", date(7), date(9))], "");
        for d in [7, 8, 9] {
            assert_eq!(idx.on(day(d)).len(), 1, "day {d}");
        }
        assert!(idx.on(day(10)).is_empty());
        assert!(idx.on(day(6)).is_empty());
    }

    #[test]
    fn midnight_end_stops_the_day_before() {
        let i = &items_of(&event("e", dt(7, 22, 0), dt(8, 0, 0)))[0];
        assert_eq!(i.last_day(), day(7));
    }

    #[test]
    fn inverted_event_is_one_day() {
        let i = &items_of(&event("e", dt(9, 10, 0), dt(7, 10, 0)))[0];
        assert_eq!(i.first_day(), day(9));
        assert_eq!(i.last_day(), day(9));
    }

    #[test]
    fn long_span_is_not_expanded() {
        let end = DateOrTime::Date(jiff::civil::date(2099, 12, 31));
        let start = DateOrTime::Date(jiff::civil::date(2026, 1, 1));
        let idx = CalendarIndex::build(&[event("long", start, end)], "");
        assert!(idx.by_day.is_empty());
        assert_eq!(idx.spans.len(), 1);
        let far = jiff::civil::date(2050, 6, 1);
        assert_eq!(idx.on(far).len(), 1);
    }

    #[test]
    fn search_filters() {
        let hs = [event("Standup", dt(8, 9, 0), dt(8, 10, 0))];
        assert!(CalendarIndex::build(&hs, "lunch").on(day(8)).is_empty());
        assert_eq!(CalendarIndex::build(&hs, "stand").on(day(8)).len(), 1);
    }

    #[test]
    fn order_within_day() {
        let hs = [
            event("c", dt(8, 9, 0), dt(8, 10, 0)),
            event("b", dt(8, 8, 0), dt(8, 9, 0)),
            event("a", date(8), date(8)),
        ];
        let idx = CalendarIndex::build(&hs, "");
        let titles: Vec<&str> = idx.on(day(8)).iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, ["a", "b", "c"]);
    }

    #[test]
    fn cache_rebuilds_only_when_needed() {
        let hs = [event("e", date(8), date(8))];
        let mut cache = CalendarCache::default();
        cache.get(&hs, "x");
        cache.get(&hs, "x");
        assert_eq!(cache.builds, 1);
        cache.invalidate();
        cache.get(&hs, "x");
        assert_eq!(cache.builds, 2);
        cache.get(&hs, "y");
        assert_eq!(cache.builds, 3);
    }
}
