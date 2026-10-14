//! The journal's MCP tools: an `rmcp` handler whose tools call the app
//! server's `sapphire-journal.*` methods ([`crate::methods`]), beside the
//! framework's own tools.
//!
//! The handler holds the framework's [`ToolContext`], so the same tools serve
//! the `mcp` subcommand (which dials the running server) and `/mcp` on the
//! server's HTTP listener (an in-process connection). Hand it to the server
//! with [`service`].

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::ServerConfig,
    tool, tool_handler, tool_router,
};
use sapphire_framework::ipc;
use sapphire_framework::mcp::{self, McpService, ToolContext};
use serde::Serialize;

use crate::methods::{
    EmptyParams, EntryCheckParams, EntryFixParams, EntryListParams, EntryModifyParams,
    EntryNewParams, EntryRemoveParams, EntryShowParams, EntryTreeParams, names,
};

/// How the journal answers MCP: [`JournalTools`], one per session.
pub fn service() -> McpService {
    McpService::new(JournalTools::new)
}

#[derive(Clone, Debug)]
pub struct JournalTools {
    ctx: ToolContext,
    tool_router: ToolRouter<Self>,
}

impl AsRef<ToolContext> for JournalTools {
    fn as_ref(&self) -> &ToolContext {
        &self.ctx
    }
}

impl JournalTools {
    /// The journal's tools and the framework's read-only ones, whose
    /// `workspace_search` is the journal's search. The framework's raw file
    /// writes are left out: an entry written without the journal's
    /// operations misses its frontmatter, id and filename conventions.
    pub fn new(ctx: ToolContext) -> Self {
        Self {
            ctx,
            tool_router: Self::tool_router() + mcp::framework_tools(),
        }
    }

    /// Call one of the journal's methods and hand its text back as the tool's.
    ///
    /// An error the method answered with is the journal's message to the
    /// agent, so it goes through without the IPC layer's framing.
    async fn call(&self, method: &str, params: impl Serialize) -> Result<String, String> {
        self.ctx.call(method, params).await.map_err(|e| match e {
            mcp::Error::Ipc(ipc::Error::Rpc(rpc)) => rpc.message,
            other => other.to_string(),
        })
    }
}

#[tool_router]
impl JournalTools {
    #[tool(
        description = "List journal entries as JSON. \
        Use `period` to specify a time range. \
        Use field selectors (task_overdue, task_in_progress, event_span, created_at, updated_at) \
        to restrict which conditions apply; omitting all selectors applies the period to all \
        timestamp fields (OR). Without a period, field selectors filter entries where that \
        condition is met. event_span uses interval-overlap semantics so in-progress events are \
        included. task_status and tags are independent AND filters. \
        Stale tasks and explicitly hidden entries (`hidden: true`) are excluded from \
        results by default; set `include_stale` / `include_hidden` to restore them.",
        annotations(read_only_hint = true)
    )]
    pub async fn entry_list(
        &self,
        Parameters(p): Parameters<EntryListParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_LIST, p).await
    }

    #[tool(
        description = "List journal entries as a JSON tree, preserving parent–child relationships. \
        Accepts the same filter parameters as entry_list. \
        Each node contains an `id`, `title`, `task`, `tags`, and a `children` array of nested nodes.",
        annotations(read_only_hint = true)
    )]
    pub async fn entry_tree(
        &self,
        Parameters(p): Parameters<EntryTreeParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_TREE, p).await
    }

    #[tool(
        description = "Show the contents of a journal entry by ID or file path",
        annotations(read_only_hint = true)
    )]
    pub async fn entry_show(
        &self,
        Parameters(p): Parameters<EntryShowParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_SHOW, p).await
    }

    #[tool(description = "Create a new journal entry")]
    pub async fn entry_new(
        &self,
        Parameters(p): Parameters<EntryNewParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_NEW, p).await
    }

    #[tool(description = "Update frontmatter fields of an existing journal entry")]
    pub async fn entry_modify(
        &self,
        Parameters(p): Parameters<EntryModifyParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_MODIFY, p).await
    }

    #[tool(
        description = "Check whether an entry's frontmatter and filename are valid. \
        Returns 'ok' or a list of issues (e.g. filename mismatch).",
        annotations(read_only_hint = true)
    )]
    pub async fn entry_check(
        &self,
        Parameters(p): Parameters<EntryCheckParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_CHECK, p).await
    }

    #[tool(description = "Normalize an entry: sync closed_at, update updated_at, and rename the file \
        to match its frontmatter ID and title/slug. \
        Reports the rename or confirms the filename is already correct.")]
    pub async fn entry_fix(
        &self,
        Parameters(p): Parameters<EntryFixParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_FIX, p).await
    }

    #[tool(description = "Delete an entry file from the journal")]
    pub async fn entry_remove(
        &self,
        Parameters(p): Parameters<EntryRemoveParams>,
    ) -> Result<String, String> {
        self.call(names::ENTRY_REMOVE, p).await
    }

    #[tool(
        description = "Show the entry cache's location, schema version, and entry/tag counts.",
        annotations(read_only_hint = true)
    )]
    pub async fn cache_info(
        &self,
        Parameters(p): Parameters<EmptyParams>,
    ) -> Result<String, String> {
        self.call(names::CACHE_INFO, p).await
    }

    #[tool(description = "Incrementally sync the entry cache with the current journal state. \
        Re-indexes files whose mtime has changed and removes entries for deleted files. \
        Returns the number of entries in the cache after sync.")]
    pub async fn cache_sync(
        &self,
        Parameters(p): Parameters<EmptyParams>,
    ) -> Result<String, String> {
        self.call(names::CACHE_SYNC, p).await
    }

    #[tool(description = "Rebuild the entry cache from scratch. \
        Use this after updating sapphire-journal when the schema has changed, \
        or when the cache has become inconsistent. \
        Returns the number of entries indexed.")]
    pub async fn cache_rebuild(
        &self,
        Parameters(p): Parameters<EmptyParams>,
    ) -> Result<String, String> {
        self.call(names::CACHE_REBUILD, p).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for JournalTools {
    fn get_info(&self) -> ServerConfig {
        let info = mcp::server_info(&self.ctx);
        let framework = info.instructions.clone().unwrap_or_default();
        info.with_instructions(format!(
            "Sapphire Journal is a Markdown-based journal/task manager. Use entry_list to \
             browse entries, entry_show to read one, entry_new to create, and entry_modify \
             to update metadata. To find entries by their content, use workspace_search \
             (full-text, and semantic when the host's bridge embeds).\n\n{framework}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Vec<rmcp::model::Tool> {
        let endpoint = ipc::Endpoint::in_dir("sapphire-journal", std::env::temp_dir());
        let ctx = ToolContext::dial("sapphire-journal", "0.0.0", None, endpoint);
        JournalTools::new(ctx).tool_router.list_all()
    }

    /// Every tool must advertise a top-level `type: object` input schema, or
    /// Anthropic rejects the tool with
    /// `tools.<N>.custom.input_schema.type: Field required`. Zero-argument
    /// tools are the ones at risk (sapphire-agent#155).
    #[test]
    fn every_tool_input_schema_declares_object_type() {
        for tool in tools() {
            let schema = serde_json::to_value(&tool.input_schema).expect("schema to json");
            assert_eq!(
                schema.get("type").and_then(|t| t.as_str()),
                Some("object"),
                "tool {} has a non-object input schema: {schema}",
                tool.name
            );
        }
    }

    /// The journal's eleven, the framework's read-only seven, and none of the
    /// framework's raw file writes.
    #[test]
    fn the_journal_serves_its_tools_and_the_frameworks_read_only_ones() {
        let names: Vec<String> = tools().iter().map(|t| t.name.to_string()).collect();
        for expected in [
            "entry_list",
            "entry_new",
            "cache_rebuild",
            "status",
            "workspace_search",
            "workspace_read_file",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected}: {names:?}"
            );
        }
        assert!(
            !names.iter().any(|n| n.starts_with("workspace_write")
                || n == "workspace_append_file"
                || n == "workspace_delete_file"),
            "raw writes would bypass the journal's operations: {names:?}"
        );
        assert_eq!(names.len(), 11 + 7, "{names:?}");
    }
}
