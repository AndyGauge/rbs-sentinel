# Comparison with rbs-inline

Sentinel covers the full [rbs-inline](https://github.com/soutaro/rbs-inline) annotation surface and is ahead on two items from its [roadmap](https://github.com/soutaro/rbs-inline/wiki/Roadmap):

| Feature | rbs-inline | Sentinel |
|---|---|---|
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

`# @rbs import` is unique to Sentinel: it resolves shared type definitions from `sig/shared/`, walks transitive dependencies in topological order, and handles circular references — rbs-inline has no equivalent.

## Performance

Benchmarked on a production Rails app (5,000+ files):

| Tool | Files generated | Time | Peak memory | Header required |
|---|---|---|---|---|
| inline-rbs | 174 | 2.540s | 76.8 MB | yes |
| Sentinel | 174 | 306ms | 9.7 MB | no |

~8x faster, ~8x less memory, identical output.

### Watching for changes

rbs-inline has no built-in watcher — the documented approach composes three external tools:

```bash
fswatch -0 lib | xargs -0 -n1 bundle exec rbs-inline --output
```

That needs `fswatch` installed separately, spawns a new Ruby process on every save, and is macOS-specific.

Sentinel builds and watches in one command, no extra dependencies, cross-platform, no per-save process spawn:

```bash
bundle exec sentinel watch
```

Or, for live editor feedback instead of a background process: `bundle exec sentinel lsp` — see [Editor Feedback](./editor-feedback.md).
