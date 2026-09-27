//! `sentinel lsp` — a small Language Server that gives Sentinel's own
//! annotation-lint plugins (void-argument, angle-bracket, type-case) a
//! direct line into the editor via `textDocument/publishDiagnostics`,
//! and keeps `sig/generated` in sync on every save.
//!
//! This replaces the "run `sentinel watch` in a background job from your
//! `on_attach`" wiring documented in the README with a single process the
//! editor already knows how to manage. It runs alongside Steep, not instead
//! of it: Steep still owns real RBS *type checking* against the generated
//! signatures; Sentinel's LSP only reports the same annotation-shape issues
//! `sentinel check`/`init`/`watch` already catch, now anchored to the exact
//! source line instead of printed to a terminal.
//!
//! Diagnostics refresh on `didOpen`/`didSave`, mirroring the watcher's own
//! "on save" model — there is no live-as-you-type checking (no `didChange`
//! handling), since Sentinel always transpiles from what's on disk.

use crate::config::SentinelConfig;
use crate::init::derive_sig_path;
use crate::plugin::Plugin;
use crate::transpiler::SentinelTranspiler;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Start the LSP server on stdio. Called by `sentinel lsp`.
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

struct Backend {
    client: Client,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    config: SentinelConfig,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            state: Mutex::new(State::default()),
        }
    }

    /// Which configured watched folder (if any) contains `path`.
    fn app_root_for(config: &SentinelConfig, path: &Path) -> Option<PathBuf> {
        config
            .folder_paths()
            .into_iter()
            .map(|f| f.canonicalize().unwrap_or(f))
            .find(|root| path.starts_with(root))
    }

    /// Best-effort mapping from a generated method name (e.g. `"self.call"`,
    /// as plugins extract it from the generated `.rbs` text) back to the line
    /// of its `def` in the original `.rb` source, so an issue found in the
    /// *generated* signature lands on the right line in the editor. Prefers
    /// the `#:` annotation line directly above the `def`, since that's what
    /// the person actually edits. Falls back to line 0.
    fn locate_method(source_lines: &[&str], method: &str) -> usize {
        let def_needle = format!("def {}", method);
        source_lines
            .iter()
            .position(|line| {
                let trimmed = line.trim_start();
                trimmed.strip_prefix(&def_needle).is_some_and(|rest| {
                    !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                })
            })
            .map(|def_line| {
                if def_line > 0 && source_lines[def_line - 1].trim_start().starts_with("#:") {
                    def_line - 1
                } else {
                    def_line
                }
            })
            .unwrap_or(0)
    }

    fn line_range(source_lines: &[&str], line: usize) -> Range {
        let end_char = source_lines
            .get(line)
            .map_or(0, |l| l.chars().count() as u32);
        Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: end_char,
            },
        }
    }

    async fn publish_error(&self, uri: &Url, message: String) {
        let diagnostic = Diagnostic {
            range: Range::default(),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("sentinel".into()),
            message,
            ..Diagnostic::default()
        };
        self.client
            .publish_diagnostics(uri.clone(), vec![diagnostic], None)
            .await;
    }

    /// Transpile `uri`, write the refreshed `.rbs` (same as `sentinel watch`),
    /// run the lint plugins against the result, and publish whatever they
    /// find as diagnostics on the source file.
    async fn sync_and_lint(&self, uri: Url) {
        let Ok(path) = uri.to_file_path() else {
            return;
        };
        if path.extension().and_then(|e| e.to_str()) != Some("rb") {
            return;
        }
        // Canonicalize so this lines up with the canonicalized config folder
        // paths in `app_root_for` — e.g. on macOS the editor's URI is under
        // `/tmp/...` while the watched folder resolves to `/private/tmp/...`,
        // and a prefix match against two different roots for the same file
        // would otherwise fail silently.
        let path = path.canonicalize().unwrap_or(path);

        let config = self.state.lock().await.config.clone();
        let Some(app_root) = Self::app_root_for(&config, &path) else {
            // Not inside any folder this project's .sentinel.toml watches.
            return;
        };
        let output = config.output_path();
        let shared = config.shared_type_paths();
        let emit_superclasses = config.emit_superclasses;

        let blocking_path = path.clone();
        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
            let mut transpiler = SentinelTranspiler::new();
            transpiler.set_shared_paths(shared);
            transpiler.set_emit_superclasses(emit_superclasses);
            // `transpile_file` returns `Box<dyn std::error::Error>`, which is
            // not `Send` and so can't cross this `spawn_blocking` boundary via
            // `?`'s blanket `From` impl into `anyhow::Error` — convert via the
            // message instead of the original (non-Send) error value.
            let rbs = transpiler
                .transpile_file(&blocking_path)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if SentinelTranspiler::has_content(&rbs) {
                let target = derive_sig_path(&app_root, &blocking_path, &output);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, &rbs)?;
            }
            Ok(rbs)
        })
        .await;

        let rbs_content = match result {
            Ok(Ok(content)) => content,
            Ok(Err(e)) => {
                self.publish_error(&uri, format!("{:#}", e)).await;
                return;
            }
            Err(e) => {
                self.publish_error(&uri, format!("sentinel panicked: {e}"))
                    .await;
                return;
            }
        };

        let source = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let source_lines: Vec<&str> = source.lines().collect();

        let mut diagnostics = Vec::new();
        for plugin in Plugin::ALL {
            for (method, message) in plugin.check(&rbs_content) {
                let line = Self::locate_method(&source_lines, &method);
                diagnostics.push(Diagnostic {
                    range: Self::line_range(&source_lines, line),
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some(format!("sentinel::{}", plugin.name())),
                    message,
                    ..Diagnostic::default()
                });
            }
        }

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> RpcResult<InitializeResult> {
        // Sentinel resolves `.sentinel.toml` and every watched folder relative
        // to the current directory, exactly like the CLI — so anchor it to
        // the workspace root the editor tells us about.
        if let Some(root) = params
            .root_uri
            .as_ref()
            .and_then(|u| u.to_file_path().ok())
        {
            let _ = std::env::set_current_dir(&root);
        }

        self.state.lock().await.config = SentinelConfig::load().unwrap_or_default();

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "sentinel-rb".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::NONE),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "🛡️  Sentinel LSP ready")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.sync_and_lint(params.text_document.uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.sync_and_lint(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Clear diagnostics so closing a file doesn't leave stale squiggles
        // if it's reopened outside a watched folder later.
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(src: &str) -> Vec<&str> {
        src.lines().collect()
    }

    #[test]
    fn locates_instance_method_annotation_line() {
        let src = "class Widget\n  #: (String) -> void\n  def call(name)\n  end\nend\n";
        let l = lines(src);
        assert_eq!(Backend::locate_method(&l, "call"), 1);
    }

    #[test]
    fn locates_self_method_by_full_name() {
        let src = "class Widget\n  #: (String) -> Widget\n  def self.build(name)\n  end\nend\n";
        let l = lines(src);
        assert_eq!(Backend::locate_method(&l, "self.build"), 1);
    }

    #[test]
    fn falls_back_to_def_line_without_annotation_comment() {
        let src = "class Widget\n  def call(name)\n  end\nend\n";
        let l = lines(src);
        assert_eq!(Backend::locate_method(&l, "call"), 1);
    }

    #[test]
    fn does_not_confuse_prefix_method_names() {
        // "call" must not match "def call_later", only an exact identifier.
        let src = "class Widget\n  #: (String) -> void\n  def call_later(name)\n  end\n  #: () -> void\n  def call\n  end\nend\n";
        let l = lines(src);
        assert_eq!(Backend::locate_method(&l, "call"), 4);
    }

    #[test]
    fn unknown_method_falls_back_to_line_zero() {
        let src = "class Widget\nend\n";
        let l = lines(src);
        assert_eq!(Backend::locate_method(&l, "missing"), 0);
    }

    #[test]
    fn line_range_spans_full_line_width() {
        let src = "class Widget\n  def call(name)\n  end\nend\n";
        let l = lines(src);
        let range = Backend::line_range(&l, 1);
        assert_eq!(range.start, Position::new(1, 0));
        assert_eq!(range.end, Position::new(1, "  def call(name)".chars().count() as u32));
    }
}
