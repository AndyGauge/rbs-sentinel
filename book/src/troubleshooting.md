# Troubleshooting

## Steep fails to start with "Exit Code 2"

Ensure you're running within the correct Ruby environment — Steep depends on the bundle context to resolve paths correctly. If you use asdf or rbenv, make sure your shims are current:

```bash
asdf reshim ruby
```

## `sentinel lsp` reports "not found" or does nothing

- Make sure `rbs-sentinel` is in your `Gemfile` and `bundle install` has run.
- If your editor integration expects a `bin/sentinel` binstub (the [Zed extension](./editor-feedback/zed.md) checks for one first), generate it: `bundle binstubs rbs-sentinel`. Otherwise it falls back to whatever `sentinel` resolves to on `$PATH`.
- Confirm the editor actually launched it as `bundle exec sentinel lsp` (or a binstub/`$PATH` equivalent) — running plain `sentinel lsp` outside the project's bundle context can resolve a different Ruby/gem set than your editor expects.

## No diagnostics show up, but `sentinel check` finds issues on the command line

`sentinel lsp` only processes files inside a folder listed in `.sentinel.toml`'s `folders`. A file opened from outside those folders (or before `.sentinel.toml` exists) is silently skipped — it isn't a file the project is configured to watch. Run `bundle exec sentinel init` once to create the default config, or add the folder with `bundle exec sentinel add <folder>`.

## Generated `.rbs` looks stale in Steep, even after saving

Diagnostics and regeneration are triggered on **save**, not on every keystroke — `sentinel lsp`, like `sentinel watch` before it, only ever transpiles what's actually on disk. If Steep still looks stale after a save, check that its own client is configured to re-check on file changes (`check_on_save = true` in the Neovim config in [Editor Feedback](./editor-feedback.md)), not just on document open.

## `sentinel check` fails in CI but passes locally

Usually a generated-file drift issue: someone committed a `.rbs` file by hand, or an editor's LSP wrote one with a config (`shared_paths`, `output`) that differs from what CI uses. Run `bundle exec sentinel init` locally with the exact `.sentinel.toml` CI uses and diff the result against what's committed.
