use std::path::Path;

use crate::{entry::Entry, error::Result};

/// Parse a Markdown file into an [`Entry`].
///
/// The file must start with a `---` line; everything until the closing `---`
/// is parsed as YAML. The rest, without its leading blank lines, is the body.
pub fn parse_entry(path: &Path, source: &str) -> Result<Entry> {
    let (frontmatter, body) = sapphire_frontmatter::parse(source)?;
    Ok(Entry {
        path: path.to_path_buf(),
        frontmatter,
        body: body.trim_start_matches(['\n', '\r']).to_owned(),
    })
}

/// Read a file from disk and parse it.
pub fn read_entry(path: &Path) -> Result<Entry> {
    let source = std::fs::read_to_string(path)?;
    parse_entry(path, &source)
}

/// Serialize an [`Entry`] back to Markdown source.
pub fn render_entry(entry: &Entry) -> String {
    sapphire_frontmatter::render(&entry.frontmatter, &entry.body)
        .expect("frontmatter serialization failed")
}

/// Write an [`Entry`] back to its source file, updating `updated_at` first.
pub fn write_entry(entry: &mut Entry) -> Result<()> {
    entry.frontmatter.updated_at = chrono::Local::now().naive_local();
    std::fs::write(&entry.path, render_entry(entry))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use std::path::PathBuf;

    /// A valid sapphire-journal-managed path with CarettaId "0000000" (NIL).
    fn managed_path() -> PathBuf {
        PathBuf::from("0000000_test.md")
    }

    #[test]
    fn parses_entry_with_frontmatter() {
        let src = "---\nid: '0000000'\ntitle: Hello\ntags: [rust, cli]\n---\nsome body\n";
        let entry = parse_entry(&managed_path(), src).unwrap();
        assert_eq!(entry.frontmatter.title, "Hello");
        assert_eq!(entry.frontmatter.tags, vec!["rust", "cli"]);
        assert_eq!(entry.body, "some body\n");
    }

    #[test]
    fn parses_entry_with_crlf_frontmatter() {
        let src =
            "---\r\nid: '0000000'\r\ntitle: Hello\r\ntags: [rust, cli]\r\n---\r\nsome body\r\n";
        let entry = parse_entry(&managed_path(), src).unwrap();
        assert_eq!(entry.frontmatter.title, "Hello");
        assert_eq!(entry.frontmatter.tags, vec!["rust", "cli"]);
        assert_eq!(entry.body, "some body\r\n");
    }

    #[test]
    fn renders_entry_with_task() {
        use crate::entry::TaskMeta;
        let src = "---\nid: '0000000'\n---\nbody\n";
        let mut entry = parse_entry(&managed_path(), src).unwrap();
        entry.frontmatter.title = "My Task".into();
        entry.frontmatter.task = Some(TaskMeta {
            status: "open".into(),
            due: Some("2026-03-10T00:00:00".parse().unwrap()),
            started_at: None,
            closed_at: None,
            extra: IndexMap::new(),
        });
        let rendered = render_entry(&entry);
        assert!(rendered.contains("title: My Task"));
        assert!(rendered.contains("status: open"));
        assert!(rendered.contains("due: 2026-03-10"));
    }
}
