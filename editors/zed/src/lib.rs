//! Zed extension: runs `sentinel lsp` as a second language server on Ruby
//! files, alongside Zed's built-in Ruby (and Steep, if configured) support.
//! See the "Editor Feedback" chapter of the RBS Sentinel book for setup.

use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct SentinelExtension;

impl SentinelExtension {
    /// Prefer a project-local binstub at `bin/sentinel` (from
    /// `bundle binstubs rbs-sentinel`), then fall back to whatever `sentinel`
    /// resolves to on `$PATH` (e.g. `gem install rbs-sentinel`).
    ///
    /// Existence is checked via `Worktree::read_text_file`, not `std::fs` —
    /// extensions run sandboxed inside a WASM component, and `std::fs` does
    /// not reliably see the worktree; `read_text_file` (and `which`) are the
    /// host-provided calls that do. `read_text_file` takes a path *relative*
    /// to the worktree root (see zed-extensions/ruby's own `Steepfile` check).
    fn find_sentinel(&self, worktree: &Worktree) -> Result<String> {
        if worktree.read_text_file("bin/sentinel").is_ok() {
            return Ok(format!("{}/bin/sentinel", worktree.root_path()));
        }

        worktree.which("sentinel").ok_or_else(|| {
            "`sentinel` not found. Run `bundle add rbs-sentinel && bundle binstubs rbs-sentinel` \
             (or `gem install rbs-sentinel` for a global install), then restart Zed."
                .to_string()
        })
    }
}

impl zed::Extension for SentinelExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let command = self.find_sentinel(worktree)?;
        Ok(Command {
            command,
            args: vec!["lsp".to_string()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(SentinelExtension);
