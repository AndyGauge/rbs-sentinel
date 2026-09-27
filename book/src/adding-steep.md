# Adding Steep

[Steep](https://github.com/soutaro/steep) is the type checker that actually reads your `.rbs` signatures and reports where your Ruby disagrees with them. Sentinel generates the signatures; Steep is what makes them worth generating.

## Install

```ruby
# Gemfile
group :development do
  gem "steep"
  gem "rbs-sentinel"
end
```

```bash
bundle install
```

## Initialize a Steepfile

```bash
bundle exec steep init
```

This creates a `Steepfile` at your project root. Point it at your source and at **both** of Sentinel's output directories — the generated signatures and any shared types:

```ruby
# Steepfile
target :app do
  check "app"                    # directories to type-check
  signature "sig/generated"      # Sentinel's generated .rbs output
  signature "sig/shared"         # shared type definitions (# @rbs import)

  configure_code_diagnostics(D::Ruby.strict)
end
```

Add every folder you configured in `.sentinel.toml` under `folders` as a `check` line, and every folder under `output`/`shared_paths` as a `signature` line — they should match.

## Run it

```bash
bundle exec sentinel init      # generate .rbs first — Steep needs them to exist
bundle exec steep check
```

`steep check` walks every file under a `check` path and reports type errors against the signatures under `signature` paths. Run `sentinel init` (or make sure `watch`/`lsp` has run) before the first `steep check` in a fresh checkout — an empty `sig/generated` just means Steep infers `untyped` everywhere and catches nothing.

## Keeping it current

From here, the workflow is:

1. Write a `#:` annotation above a method.
2. Sentinel regenerates the matching `.rbs` (via [`watch`](./editor-feedback/background-watcher.md) or [`sentinel lsp`](./editor-feedback/neovim.md)).
3. Steep re-checks against the fresh signature and flags anything that doesn't line up.

Wire steps 2–3 into your editor next: [Editor Feedback](./editor-feedback.md). Or wire `sentinel check` + `steep check` into CI: [CI and Pre-commit Checks](./ci-and-pre-commit.md).
