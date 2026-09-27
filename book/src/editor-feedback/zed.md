# Zed

Sentinel ships a small Zed extension at [`editors/zed/`](https://github.com/andygauge/rbs-sentinel/tree/master/editors/zed) in this repo. It registers `sentinel lsp` as an additional language server for Ruby files — Zed already has its own Ruby language server (`ruby-lsp` or `solargraph`, whichever you have configured); this adds Sentinel's diagnostics on top, the same way the [Neovim](./neovim.md) and [VS Code](./vscode.md) setups add it alongside Steep.

## What it does

The extension is a tiny Rust crate compiled to WebAssembly (that's how all Zed extensions work — no Ruby, no separate runtime). Its entire job is:

```rust
fn language_server_command(&mut self, _id: &LanguageServerId, worktree: &Worktree) -> Result<Command> {
    let command = self.find_sentinel(worktree)?; // bin/sentinel binstub, else $PATH
    Ok(Command { command, args: vec!["lsp".into()], env: worktree.shell_env() })
}
```

It looks for a project-local binstub at `bin/sentinel` first (from `bundle binstubs rbs-sentinel`), then falls back to `sentinel` on `$PATH` (from a global `gem install rbs-sentinel`). Whichever it finds, it launches `sentinel lsp` and lets Zed manage the process — same diagnostics, same `sig/generated` sync on save described in [Editor Feedback](../editor-feedback.md).

## Installing it

This extension isn't (yet) published to Zed's official extension registry, so install it as a **dev extension** from a local clone:

1. Clone the repo (or just keep your existing clone of the project you're adding Sentinel to — the extension source doesn't need to live inside *your* Rails app, it just needs to be on disk):
   ```bash
   git clone https://github.com/andygauge/rbs-sentinel
   ```
2. In Zed, open the command palette (<kbd>Cmd+Shift+P</kbd> / <kbd>Ctrl+Shift+P</kbd>) and run **`zed: install dev extension`**.
3. Select the `rbs-sentinel/editors/zed` directory (the one containing `extension.toml`).
4. Zed builds the extension (it manages its own Rust + `wasm32-wasip2` toolchain, so you don't need one installed) and loads it immediately — no restart required.

Make sure `rbs-sentinel` is in your Ruby project's `Gemfile` (see [Getting Started](../getting-started.md)) before opening a `.rb` file — otherwise the extension reports "`sentinel` not found" instead of starting.

## Updating it

Dev extensions don't auto-update. After pulling changes to `editors/zed/`, re-run **`zed: install dev extension`** on the same directory to rebuild and reload it.

## Publishing (future)

Submitting this to the [official Zed extension registry](https://github.com/zed-industries/extensions) — so it installs from Zed's Extensions panel instead of as a dev extension — is a separate step (a PR to that repo adding this extension as a submodule) and isn't done yet. The dev-extension install above is the supported path today.
