# Editor Feedback

Two separate concerns want to run against your Ruby files as you edit:

- **Sentinel** — reads the `#:` annotation, checks its shape (no lowercase `string`, no `Array<...>` angle brackets, no `void` as an argument type), and regenerates the `.rbs` signature.
- **Steep** — reads the `.rbs` signature and checks it against your actual implementation.

They're independent tools with independent jobs, so the setup below always attaches **two** language servers to your Ruby files, not one. Pick your editor:

- [Neovim](./editor-feedback/neovim.md)
- [VS Code](./editor-feedback/vscode.md)
- [Zed](./editor-feedback/zed.md)

Each of those wires up `sentinel lsp` (Sentinel's own LSP — diagnostics on save, no terminal required) alongside Steep. If you'd rather not run a second language server at all, see [Steep-only, no in-editor lint](./editor-feedback/background-watcher.md), which keeps `.rbs` generation running as a plain background process instead.

## What `sentinel lsp` actually does

On `didOpen`/`didSave` of a `.rb` file inside a watched folder:

1. Transpiles the file and writes the refreshed `.rbs` into `sig/generated` — identical output to `sentinel watch`.
2. Runs the same lint plugins `sentinel check`/`init`/`watch` run, and publishes anything they find as `textDocument/publishDiagnostics`, anchored to the `#:` annotation line.
3. Does **not** type-check. That stays Steep's job — Sentinel only catches annotation-shape mistakes (wrong case, wrong bracket style) before Steep ever sees the generated signature.

There's no live-as-you-type checking — diagnostics refresh on save, matching how `sentinel watch` has always worked. See [How It Works](./how-it-works.md) for the full picture.
