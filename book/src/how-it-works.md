# How It Works

Sentinel is a single native Rust binary (distributed as the `rbs-sentinel` gem, which just execs a precompiled binary for your platform — see `sentinel-gem/bin/sentinel`). It runs in one of two shapes, both built on the same core:

```text
                ┌───────────────────────────┐
  .rb source ─▶ │  tree-sitter Ruby parser   │
                │  + #: annotation walker    │──▶  .rbs (sig/generated/)
                └───────────────────────────┘
                              │
                              ▼
                     lint plugins (Type Case,
                     Angle Bracket, Void Argument)
                              │
              ┌───────────────┴───────────────┐
              ▼                                ▼
     printed to stderr                textDocument/publishDiagnostics
     (`init`/`check`/`watch`)              (`sentinel lsp` only)
```

## The pieces

- **Parser** (`transpiler.rs`) — a [tree-sitter](https://tree-sitter.github.io/tree-sitter/) grammar for Ruby, walked to find `#:`/`# @rbs`-annotated methods, `attr_*` calls, type aliases, and imports, and to render them as `.rbs`.
- **Watcher** (`watcher.rs`) — a native filesystem watcher ([`notify`](https://docs.rs/notify)) with debouncing, driving the same transpile-and-write path on every save. This is what `sentinel watch` runs.
- **Lint plugins** (`plugin.rs`) — small, independent checks over the *generated* `.rbs` text: catching lowercase primitives (`string` instead of `String`), RBS's square-bracket generics written as Ruby-style angle brackets (`Array<String>` instead of `Array[String]`), and `void` used as an argument type instead of a return type. These run identically whether triggered from `init`, `check`, `watch`, or `lsp`.
- **LSP server** (`lsp.rs`, added for `sentinel lsp`) — a [`tower-lsp`](https://docs.rs/tower-lsp)-based Language Server. On `didOpen`/`didSave`, it runs the same transpile step as the watcher, then maps each lint plugin issue (found in the *generated* RBS text, by method name) back to the line of that method's `#:` annotation in the *original* source, and publishes it as a real diagnostic instead of a printed line.

## Why not just watch and re-run Steep?

Steep already has its own file-watching mode. Sentinel isn't a wrapper around Steep or a replacement for it — the two check different things against different inputs:

- Sentinel checks: does this `#:` annotation *parse* into valid RBS shape? (lint, structural)
- Steep checks: does the implementation *match* the type the signature claims? (type checking, semantic)

Sentinel has to run first, because Steep is only as good as the `.rbs` file it's given — a signature Sentinel never regenerated is a signature quietly going stale under Steep's nose.

## Why a Rust binary instead of a Ruby gem doing the parsing

A pure-Ruby watcher (`fswatch` + spawn a Ruby process per changed file, roughly what composing `rbs-inline` externally looks like) pays a process-spawn and Ruby-boot cost on every single save. Benchmarked on a 5,000+ file production Rails app:

| | Time | Peak memory |
|---|---|---|
| inline-rbs (external fswatch) | 2.540s | 76.8 MB |
| Sentinel (built-in watcher) | 306ms | 9.7 MB |

Same 174 files generated, same output. See [Comparison with rbs-inline](./comparison.md) for the full picture, including where the annotation *feature* coverage differs, not just performance.
