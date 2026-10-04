# Neovim

Register `sentinel lsp` as a second language server for `ruby` files, alongside `steep`:

```lua
vim.lsp.config('sentinel', {
  cmd = { 'bundle', 'exec', 'sentinel', 'lsp' },
  filetypes = { 'ruby' },
  root_markers = { '.sentinel.toml', 'Gemfile' },
})
vim.lsp.enable('sentinel')

require('lspconfig').steep.setup({
  cmd = { "bundle", "exec", "steep", "langserver" },
  capabilities = {
    workspace = {
      didChangeWatchedFiles = { dynamicRegistration = true },
    },
  },
  settings = {
    steep = {
      check_on_save = true,
      enable_diagnostics = true,
    }
  }
})
```

(`vim.lsp.config`/`vim.lsp.enable` is the built-in client, available without `nvim-lspconfig`; swap in the equivalent `require('lspconfig').sentinel.setup{...}`-style config if you're on an older Neovim or prefer lspconfig for both.)

That's it — no background job to manage. Save a `.rb` file and:

- Sentinel's diagnostics (tagged `sentinel::<plugin>`) show up from its own LSP client.
- Steep's diagnostics show up from its own LSP client, once it picks up the refreshed `.rbs` in `sig/generated`.

## Previous approach

Earlier versions of this project's setup ran `sentinel watch` as a detached background job kicked off from Steep's `on_attach`, since there was no `sentinel lsp` yet. That still works — see [Steep-only, no in-editor lint](./background-watcher.md) — but `sentinel lsp` is simpler and is now the recommended path.
