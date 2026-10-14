use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse frontmatter: {0}")]
    FrontmatterParse(#[from] sapphire_frontmatter::Error),

    #[error("Invalid entry file: {0}")]
    InvalidEntry(String),

    #[error("not a sapphire-journal: no .sapphire-journal directory there")]
    JournalNotFound,

    #[error("no entry found with id prefix `{0}`")]
    EntryNotFound(String),

    #[error("id prefix `{0}` is ambiguous ({1} matches)")]
    AmbiguousId(String, usize),

    #[error("no entry found with title `{0}`")]
    EntryNotFoundByTitle(String),

    #[error("title `{0}` is ambiguous ({1} matches)")]
    AmbiguousTitle(String, usize),

    #[error("duplicate title `{0}` — set duplicate_title = \"allow\" in config to permit this")]
    DuplicateTitle(String),

    #[error("duplicate id `{0}` found in files: {1} and {2}")]
    DuplicateId(String, String, String),

    #[error("{0} already exists")]
    EntryAlreadyExists(String),

    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("cache error: {0}")]
    Cache(#[from] rusqlite::Error),

    /// The cache DB was created by a newer version of sapphire-journal.
    /// The user must either update sapphire-journal or recreate the cache.
    #[error(
        "cache schema v{db_version} is newer than this app supports (v{app_version}); \
         update sapphire-journal, or recreate the cache with `sapphire-journal cache rebuild`"
    )]
    CacheSchemaTooNew { db_version: i32, app_version: i32 },
}

pub type Result<T> = std::result::Result<T, Error>;
