# Getting Started

## Install

Sentinel ships as a Ruby gem (`rbs-sentinel`) that wraps a precompiled Rust binary — no Rust toolchain needed to use it.

```ruby
# Gemfile
group :development do
  gem "rbs-sentinel"
end
```

```bash
bundle install
```

## Initialize

```bash
bundle exec sentinel init
```

This creates a `.sentinel.toml` config file (if one doesn't exist yet) watching `app` by default, then generates `.rbs` signatures for every annotated method it finds in that folder.

## Configure watched folders

```bash
bundle exec sentinel add lib
bundle exec sentinel add config/initializers
bundle exec sentinel remove app
bundle exec sentinel list        # show current config
```

These edit `.sentinel.toml` at your project root — you can also hand-edit it directly:

```toml
folders = ["app", "lib"]
output = "sig/generated"
shared_paths = ["sig/shared"]
```

- **`folders`** — directories Sentinel scans/watches for `#:`-annotated `.rb` files.
- **`output`** — where generated `.rbs` files land, mirroring the source tree.
- **`shared_paths`** — where `# @rbs import` looks for shared type definitions.

## Run it

Three ways to keep signatures current, in increasing order of "does something for you automatically":

```bash
bundle exec sentinel check   # CI-safe: verify signatures are current, exit 1 if stale — no writes
bundle exec sentinel init    # one-shot: (re)generate everything, once
bundle exec sentinel watch   # background: regenerate on every save, forever
```

For live feedback in your editor instead of a terminal, skip straight to [Editor Feedback](./editor-feedback.md) — `sentinel lsp` does what `watch` does, plus pushes lint diagnostics into the buffer.

## Next

- [Adding Steep](./adding-steep.md) so the generated `.rbs` actually type-checks something.
- [CI and Pre-commit Checks](./ci-and-pre-commit.md) so `sentinel check` gates merges.
