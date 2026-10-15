//! The flags an entry's header and a list item carry.

/// A computed flag describing an entry's type or freshness state.
///
/// Serializes to its string representation via [`EntryFlag::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryFlag {
    // Freshness / urgency (slot 1)
    Overdue,
    New,
    Updated,
    // Entry type (slot 2)
    Event,
    /// Past event whose `end` timestamp is before the current time.
    EventClosed,
    Done,
    Cancelled,
    InProgress,
    Archived,
    Open,
    Note,
    // Orthogonal flags (appended after the slot-1/slot-2 flags)
    /// Incomplete task (`closed_at` absent) whose `updated_at` is older than the
    /// configured `stale_after_days` threshold.
    Stale,
    /// Entry explicitly marked `hidden: true` in its frontmatter.
    Hidden,
}

impl EntryFlag {
    /// Every flag, in declaration order.
    pub const ALL: [EntryFlag; 13] = [
        Self::Overdue,
        Self::New,
        Self::Updated,
        Self::Event,
        Self::EventClosed,
        Self::Done,
        Self::Cancelled,
        Self::InProgress,
        Self::Archived,
        Self::Open,
        Self::Note,
        Self::Stale,
        Self::Hidden,
    ];

    const NAMES: [&'static str; 13] = [
        "overdue",
        "new",
        "updated",
        "event",
        "event_closed",
        "done",
        "cancelled",
        "in_progress",
        "archived",
        "open",
        "note",
        "stale",
        "hidden",
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overdue => "overdue",
            Self::New => "new",
            Self::Updated => "updated",
            Self::Event => "event",
            Self::EventClosed => "event_closed",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::InProgress => "in_progress",
            Self::Archived => "archived",
            Self::Open => "open",
            Self::Note => "note",
            Self::Stale => "stale",
            Self::Hidden => "hidden",
        }
    }

    pub fn to_emoji(self) -> &'static str {
        match self {
            Self::Overdue => "⏰",
            Self::New => "🆕",
            Self::Updated => "✏️",
            Self::Event => "📅",
            Self::EventClosed => "🗓️",
            Self::Done => "✅",
            Self::Cancelled => "❌",
            Self::InProgress => "🔄",
            Self::Archived => "📦",
            Self::Open => "⬜",
            Self::Note => "📝",
            Self::Stale => "🕰️",
            Self::Hidden => "🙈",
        }
    }

    pub fn to_nerd(self) -> &'static str {
        match self {
            Self::Overdue => "󱦟",
            Self::New => "󰐕",
            Self::Updated => "󰏫",
            Self::Event => "󰃭",
            Self::EventClosed => "󰄻",
            Self::Done => "󰄲",
            Self::Cancelled => "󰜺",
            Self::InProgress => "󰔛",
            Self::Archived => "󰀼",
            Self::Open => "󰄱",
            Self::Note => "󰈙",
            Self::Stale => "󰔚",
            Self::Hidden => "󰈈",
        }
    }

    pub fn to_initial(self) -> char {
        match self {
            Self::Overdue => '!',
            Self::New => '+',
            Self::Updated => '~',
            Self::Event => 'E',
            Self::EventClosed => 'e',
            Self::Done => 'D',
            Self::Cancelled => 'C',
            Self::InProgress => 'I',
            Self::Archived => 'A',
            Self::Open => 'O',
            Self::Note => 'N',
            Self::Stale => 'S',
            Self::Hidden => 'H',
        }
    }
}

impl serde::Serialize for EntryFlag {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for EntryFlag {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::ALL
            .into_iter()
            .find(|f| f.as_str() == s)
            .ok_or_else(|| serde::de::Error::unknown_variant(&s, &Self::NAMES))
    }
}

/// Identifies which timestamp field caused an entry to match a filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchFlag {
    /// Task is incomplete (`closed_at` absent) and `due ≤ period_end` (or `due < now`).
    TaskOverdue,
    /// Task is incomplete (`closed_at` absent) and `started_at ≤ period_end` (or `started_at` set).
    TaskInProgress,
    /// Task exists but `started_at` and `closed_at` are both absent.
    TaskUnstarted,
    /// The filter period overlaps the event's [start, end] span.
    EventSpan,
    CreatedAt,
    UpdatedAt,
    /// Entry did not match the filter itself, but one or more of its
    /// descendants did (included to preserve tree context).
    ParentOfMatch,
}

impl MatchFlag {
    pub fn as_str(self) -> &'static str {
        match self {
            MatchFlag::TaskOverdue => "task_overdue",
            MatchFlag::TaskInProgress => "task_in_progress",
            MatchFlag::TaskUnstarted => "task_unstarted",
            MatchFlag::EventSpan => "event_span",
            MatchFlag::CreatedAt => "created",
            MatchFlag::UpdatedAt => "updated",
            MatchFlag::ParentOfMatch => "parent_of_match",
        }
    }
}

impl serde::Serialize for MatchFlag {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for MatchFlag {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        const ALL: [MatchFlag; 7] = [
            MatchFlag::TaskOverdue,
            MatchFlag::TaskInProgress,
            MatchFlag::TaskUnstarted,
            MatchFlag::EventSpan,
            MatchFlag::CreatedAt,
            MatchFlag::UpdatedAt,
            MatchFlag::ParentOfMatch,
        ];
        let s = String::deserialize(d)?;
        ALL.into_iter()
            .find(|f| f.as_str() == s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown match flag `{s}`")))
    }
}
