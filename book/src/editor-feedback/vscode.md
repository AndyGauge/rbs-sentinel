# VS Code

1. Install the [Steep VS Code extension](https://marketplace.visualstudio.com/items?itemName=soutaro.steep-vscode) as usual, and point it at your bundled Steep:

   ```json
   {
     "steep.command": "bundle exec steep langserver",
     "steep.enableDiagnostics": true
   }
   ```

2. Attach `sentinel lsp` as a second language server for `.rb` files. VS Code doesn't ship a generic "run this LSP command" setting the way Neovim does, so the practical options are:

   - Use a generic LSP client extension (e.g. one that lets you register an arbitrary stdio command per language) and point it at `bundle exec sentinel lsp` for the `ruby` language ID.
   - Or wrap it in a minimal extension using [`vscode-languageclient`](https://www.npmjs.com/package/vscode-languageclient), spawning `bundle exec sentinel lsp` the same way the Steep extension spawns `steep langserver`.

Either way, Sentinel's diagnostics are tagged `sentinel::<plugin name>` in the Problems panel, so they read alongside — not on top of — Steep's type errors.

## Simpler alternative

If wiring a second language client feels like more ceremony than it's worth, keep `sentinel watch` running as a background terminal (or a VS Code task) instead — see [Steep-only, no in-editor lint](./background-watcher.md). You'll still get `.rbs` regeneration on save and therefore live Steep diagnostics; you'll just read Sentinel's own lint output from its terminal instead of the Problems panel.
