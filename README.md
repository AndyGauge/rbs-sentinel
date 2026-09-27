# 🛡️ RBS Sentinel

RBS type signature generator for Rails — Rust-powered, CI-ready.

**RBS Sentinel** keeps your Ruby code and RBS type signatures in perfect sync. It bridges the gap between dynamic Ruby models and static RBS type definitions using a Rust transpiler for speed at scale.

📖 **[Read the full docs](https://andygauge.github.io/rbs-sentinel/)** — what RBS and inline RBS are, adding Steep, and editor setup for Neovim, VS Code, and Zed.

## 🚀 Getting Started

### 1. Install the Gem
Add RBS Sentinel to your Gemfile:
```
    group :development do
      gem 'rbs-sentinel'
    end
```
Then run:
```
    bundle install
```
### 2. Initialize the Project
Run the following command to set up the necessary directories and generate RBS files:
```
    bundle exec sentinel init
```
This creates a `.sentinel.toml` config file (if one doesn't exist) with `app` as the default watched folder, then generates RBS signatures for all Ruby files in it.

### 3. Configure Watched Folders

Sentinel watches the `app` folder by default. Use the CLI to add or remove folders:

```bash
# Add a folder
bundle exec sentinel add lib

# Add another
bundle exec sentinel add config/initializers

# Remove a folder
bundle exec sentinel remove app

# List current configuration
bundle exec sentinel list
```

The configuration is stored in `.sentinel.toml` at your project root:
```toml
folders = ["app", "lib"]
output = "sig/generated"
```

You can also edit this file directly. Sentinel reads it on every `init` and `watch` command.

### 4. Emitting Superclasses (opt-in)

By default a class is emitted without its parent — `class User < ApplicationRecord`
becomes `class User`. Set `emit_superclasses` to keep the parent:

```toml
folders = ["app"]
output = "sig/generated"
emit_superclasses = true
```

```rbs
# emit_superclasses = false (default)      # emit_superclasses = true
class User                                 class User < ApplicationRecord
```

Turn this on when you want Steep to resolve **inherited** methods, class-level DSL
macros and type aliases. Without a parent, every generated class looks like it
inherits from `Object`, so a call to an inherited macro is not merely untyped — Steep
cannot see the method at all, and a `NoMethod` diagnostic is often disabled in
`Steepfile`, so it passes silently.

**It is off by default because it is not a no-op.** Once the parent is visible, Steep
starts checking inherited signatures and will surface pre-existing type errors that
were previously unreachable. That is the point, but it should be a deliberate
migration rather than something a version bump does to you. Expect to fix or
`# steep:ignore` some findings on first enable.

Parents that are not a plain constant path are skipped, since RBS has no way to name
an anonymous class:

```ruby
class Meta < Struct.new(:a, :b)   # emitted as `class Meta` either way
```

A superclass is written exactly as it appears in source. RBS resolves relative to the
enclosing namespace the same way Ruby does, so a relative parent stays relative.

---

## 🔍 Checking Signatures in CI / Pre-commit

Use `sentinel check` to verify that generated RBS files are up to date **without modifying anything**. It exits with code 1 if any signatures are missing or stale.

```bash
bundle exec sentinel check
```

### GitHub Actions

```yaml
- name: Check RBS signatures are up to date
  run: bundle exec sentinel check
```

### Git Pre-commit Hook

Add the following to `.git/hooks/pre-commit` (or use a framework like [Lefthook](https://github.com/evilmartians/lefthook) or [Husky](https://github.com/typicode/husky)):

```bash
#!/usr/bin/env bash
set -e

bundle exec sentinel check
```

Then make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

If the check fails, run `bundle exec sentinel init` to regenerate, review the changes, and commit again.

---

## 🛠️ Editor Setup

Two things want to run against your Ruby files: **Sentinel** (annotation lint + `.rbs` generation) and **Steep** (real RBS type checking). They're separate concerns — Sentinel doesn't type-check, and Steep doesn't know how to read `#:` annotations — so both stay attached to your `.rb` files as two separate language servers.

### Option A — `sentinel lsp` (recommended)

Sentinel can run as its own Language Server: on `didOpen`/`didSave` it transpiles the file, writes the refreshed `.rbs` into `sig/generated` (same as `sentinel watch`), and publishes its lint plugin findings (void-argument, angle-bracket, type-case) straight into your editor via `textDocument/publishDiagnostics` — no more reading them off a terminal. It runs *alongside* Steep, not instead of it.

#### Neovim
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

#### VS Code
1. Install the Steep VS Code extension as usual.
2. Point a generic LSP client extension (or a thin wrapper extension) at `bundle exec sentinel lsp` for `ruby` files, alongside it.
3. Sentinel's diagnostics are tagged `sentinel::<plugin name>` in the Problems panel, so they never get confused with Steep's type errors.

#### Zed
A ready-made extension lives in [`editors/zed/`](editors/zed) — it registers `sentinel lsp` as a second language server for Ruby files, alongside Zed's own Ruby support (and Steep, if you have a Steep extension configured). Install it as a dev extension:

1. Command palette → **`zed: install dev extension`**.
2. Select this repo's `editors/zed` directory (the one with `extension.toml`).

Zed builds and loads it immediately — no separate toolchain required. Full details, including how it locates the `sentinel` binary, are in the [Zed docs page](https://andygauge.github.io/rbs-sentinel/editor-feedback/zed.html).

### Option B — background job + Steep only

If you'd rather not register a second language server, run `sentinel watch` as a detached background process kicked off from Steep's `on_attach` instead. This is the original wiring and still works fine — you just lose in-editor diagnostics for the lint plugins (they still print to the terminal running `watch`/`check`).

```lua
-- Add this to your lsp.lua, outside the return table or inside on_attach
local function start_sentinel()
  if _G.sentinel_job_id then return end -- Don't start it twice

  _G.sentinel_job_id = vim.fn.jobstart({ "bundle", "exec", "sentinel", "watch" }, {
    detach = true, -- Process keeps running in background
    on_stderr = function(_, data)
      -- Optional: print errors to Neovim's :messages if something breaks
      if data and data[1] ~= "" then
        vim.schedule(function()
          vim.notify("Sentinel Error: " .. table.concat(data, "\n"), vim.log.levels.ERROR)
        end)
      end
    end,
    on_exit = function()
      _G.sentinel_job_id = nil
    end,
  })
end

-- Update your steep setup to trigger this
require('lspconfig').steep.setup({
  on_attach = function(client, bufnr)
    start_sentinel()
    -- ... rest of your on_attach
  end,
  -- ... rest of your config
})
```

---

## 🔄 How it Works

Sentinel is designed to be a "set-and-forget" background service, in one of two shapes:

1. **As an LSP** (`sentinel lsp`, recommended): your editor starts it like any other language server. On save, it transpiles the file, writes `sig/generated/*.rbs`, and publishes its own lint diagnostics directly into the buffer — Steep then picks up the refreshed `.rbs` and updates its type diagnostics the same way it always has.
2. **As a background watcher** (`sentinel watch`): stays active, monitoring your configured folders (default: `app`) and regenerating `.rbs` files the moment you save, with lint issues printed to its own log instead of your editor.

Either way, Steep is what turns the generated `.rbs` into live type-checking diagnostics — Sentinel's job is keeping those signatures current and (via `sentinel lsp`) catching annotation-shape mistakes before Steep ever sees them.

---

## Feature Comparison

Sentinel covers the full rbs-inline annotation surface and is ahead on two items from the [rbs-inline roadmap](https://github.com/soutaro/rbs-inline/wiki/Roadmap):

| Feature | rbs-inline | sentinel |
|---------|-----------|---------|
| Instance methods (`#:`) | ✓ | ✓ |
| Self/class methods | ✓ | ✓ |
| Modules | ✓ | ✓ |
| `attr_reader` / `attr_writer` / `attr_accessor` | ✓ | ✓ |
| Type alias declarations (`# @rbs type`) | not yet | ✓ |
| Overload annotations | not yet | partial (last annotation wins) |
| Shared type imports (`# @rbs import`) | — | ✓ transitive, cycle-safe |
| Header required per file | yes | no |
| Class variables | not yet | not yet |
| Module functions | not yet | not yet |
| Interface declarations | not yet | not yet |
| Class/module aliases | not yet | not yet |

The `# @rbs import <name>` feature is unique to Sentinel: it resolves shared type definitions from `sig/shared/`, walks transitive dependencies in topological order, and handles circular references. rbs-inline has no equivalent.

---

## Performance

Benchmarked on a production Rails app (5,000+ files):

| Tool | Files generated | Time | Peak memory | Header required |
|------|----------------|------|-------------|----------------|
| inline-rbs | 174 | 2.540s | 76.8 MB | yes |
| sentinel | 174 | 306ms | 9.7 MB | no |

~8x faster, ~8x less memory, identical output. Sentinel also requires no opt-in header in each source file, which matters when rolling out type coverage across a large existing codebase or a Rails engine where you don't own every file.

### Watching for changes

**inline-rbs** has no built-in watcher. The recommended approach requires composing three external tools:

```bash
fswatch -0 lib | xargs -0 -n1 bundle exec rbs-inline --output
```

This requires `fswatch` to be installed separately, spawns a new Ruby process on every file save, and is macOS-specific.

**Sentinel** builds and watches in one command:

```bash
bundle exec sentinel
```

The watcher is built into the Rust binary — no extra dependencies, cross-platform, and no per-save process spawn overhead.

---

## ⚠️ Troubleshooting

If Steep fails to start with "Exit Code 2", ensure you are running within the correct environment. Sentinel depends on the bundle context to resolve paths correctly. If using asdf or rbenv, ensure your shims are updated:

    asdf reshim ruby

---

## License

MIT — see [LICENSE](LICENSE).
