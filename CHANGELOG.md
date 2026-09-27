# Changelog

## [Unreleased]

### Added
- **`sentinel lsp`**: Sentinel now runs as its own Language Server (stdio, via `tower-lsp`). On `didOpen`/`didSave` it transpiles the file, writes the refreshed `.rbs` into `sig/generated` (same effect as `sentinel watch`), and publishes the annotation lint plugins' findings (void-argument, angle-bracket, type-case) as real `textDocument/publishDiagnostics`, anchored to the `#:` annotation line in the original `.rb` source — previously these only printed to whichever terminal was running `init`/`check`/`watch`. Runs alongside Steep (which still owns actual RBS type checking); see the README's Editor Setup section for Neovim/VS Code/Zed wiring.

### Fixed
- **`Type Case` plugin missed lowercase primitives in named-positional-arg style**: `TypeCasePlugin` only matched `(string)` (bare single arg), `: string`, `[string]`, and `-> string` — it had no pattern for Sentinel's own `(Type paramName, ...)` positional-arg convention (e.g. `(string username, String password)`), so a lowercase type there produced invalid RBS with no warning from `init`/`check`/`watch`/`lsp`. Rewrote the check to use type-position context (after `(`, `,`, `[`, `:`, or `->`, skipping a keyword-arg name like `array:`) instead of four literal patterns.

## [0.5.0] - 2026-07-31

### Added
- **`emit_superclasses` config option** (default `false`): emit `class Foo < Bar` instead of a bare `class Foo`.

  Without a parent in the generated RBS, every class looks to Steep like it inherits from `Object`. Inherited methods, class-level DSL macros and inherited type aliases are therefore invisible — and because `Steep::Diagnostic::Ruby::NoMethod` is commonly disabled in `Steepfile`, a call to an inherited macro type-checks silently no matter what it is passed. Enabling this lets Steep resolve the parent and actually check those call sites.

  ```toml
  # .sentinel.toml
  emit_superclasses = true
  ```

  ```rbs
  # false (default)     # true
  class User            class User < ApplicationRecord
  ```

  Off by default deliberately: turning it on makes Steep check inherited signatures for the first time, which surfaces pre-existing type errors. That is the intent, but it belongs in a deliberate migration rather than a version bump. On one ~5,500-file Rails app this changed 165 of 195 generated files and surfaced 12 previously-invisible diagnostics.

  Parents that are not a plain constant path (`Struct.new(:a)`, `Class.new`, `Data.define(...)`) are skipped, since RBS cannot name an anonymous class. Modules never receive a parent. The path is emitted exactly as written, so a namespace-relative parent stays relative — RBS resolves it the same way Ruby does.

## [0.4.2] - 2026-05-11

### Added
- **Recursive import resolution**: `# @rbs import` now recursively resolves nested type references. When an imported type references other types defined in `sig/shared/`, those dependencies are automatically resolved, stripped of MCP annotations, and inlined in the generated `.rbs` output — emitted before the types that reference them. Cycle detection prevents infinite loops. Previously, only the directly imported type was expanded; nested references were left as bare names, causing Steep `RBS::UnknownTypeName` errors. (#21)

  ```ruby
  # sig/shared/funnel.rbs
  type stage_ref = { id: String @desc(Stage ID), title: String }

  # sig/shared/funnel_result.rbs
  type funnel_result = { title: String, ?stages: Array[stage_ref] }
  ```

  ```ruby
  # app/services/tool/get_funnel.rb
  # @rbs import funnel_result   # ← only imports the top-level type
  ```

  Before (0.4.1): `stage_ref` left as bare name → Steep error
  ```rbs
  type funnel_result = { title: String, ?stages: Array[stage_ref] }
  ```

  After (0.4.2): `stage_ref` automatically resolved and emitted first
  ```rbs
  type stage_ref = {id: String, title: String}
  type funnel_result = { title: String, ?stages: Array[stage_ref] }
  ```

- **Cross-file type lookup**: `resolve_single_import` now falls back to scanning all `.rbs` files in `shared_paths` when the type name doesn't match a filename. This supports types co-located in a single file (e.g. `stage_ref` defined in `funnel.rbs`).

## [0.4.1] - 2026-05-08

### Fixed
- **Sub-module RBS generation for concerns**: When a Ruby module contains both annotated methods and a nested module (e.g. `module ClassMethods` inside `McpMigrated`), sentinel now emits both the parent module's methods and the nested module in the generated `.rbs` output. Previously, the nested module's capture would shadow the parent, silently dropping the parent's methods (e.g. `initialize`). Discovered via `Tool::Concerns::McpMigrated` in the Hire monolith, which uses a plain-Ruby `def self.included(base)` pattern with `module ClassMethods`. (#19)

## [0.4.0] - 2026-05-07

### Added
- **`# @rbs import <name>` directive**: Resolves `sig/shared/<name>.rbs`, parses bare RBS type definitions inside, strips MCP-style annotations (`@desc`, `@example`, etc.), and inlines them into the generated `.rbs` output. Aligns sentinel with `mcp_authorization`, which already supports the same `# @rbs import` convention for schema compilation. Before this change, sentinel silently ignored the directive, causing Steep to fail with `Unknown alias name` errors when a handler used `# @rbs import error` instead of duplicating the type inline. (#17)

  ```ruby
  # sig/shared/error.rbs
  type error = { code: String, message: String }

  # app/handlers/foo.rb
  class Foo
    # @rbs import error

    #: (error) -> bool
    def call(err); end
  end
  ```

  Generated:
  ```rbs
  class Foo
    type error = { code: String, message: String }
    def call: (error) -> bool
  end
  ```

- **`shared_paths` config key**: Customize where `# @rbs import` resolves shared types from. Defaults to `["sig/shared"]`. Existing `.sentinel.toml` files without this key continue to work unchanged.

### Notes
- `# @rbs import` directives must live inside the `class`/`module` body — same constraint as `# @rbs type`. Top-of-file placement (above the class) is silently dropped.

## [0.3.7] - 2026-05-06

### Fixed
- **Strip MCP-style annotations from `# @rbs type` aliases too**: 0.3.6 stripped `@desc(...)`, `@example(...)`, `@min(...)`, `@format(...)`, etc. from `#:` method signatures, but `# @rbs type` aliases passed through verbatim. Steep rejected those tags inside record-type braces with the same `RBS::SyntaxError: cannot start a declaration, token=@desc` error, so consumers still needed per-file `ignore_signature` workarounds for any handler that put `@desc` on a record field. The strip now runs on type-alias bodies as well as method signatures. (#15)

  Before:
  ```rbs
  type applicant_local = { external_id: String @desc(Stable applicant external_id) @example(app_123), email: String @format(email) @desc(Primary email) }
  ```
  After:
  ```rbs
  type applicant_local = { external_id: String, email: String }
  ```

## [0.3.6] - 2026-05-06

### Fixed
- **Strip MCP-style annotations from `.rbs` output**: Inline `#:` annotations such as `@desc(...)`, `@example(...)`, `@min(...)`, `@format(...)`, and `@requires(...)` are now removed from generated `.rbs` signatures. Steep previously rejected the output with `RBS::SyntaxError: cannot start a declaration, token=@desc`, forcing per-handler `ignore_signature` entries and hand-written stubs as a workaround. (#12)

  Before:
  ```rbs
  def call: (applicant_id: String @desc(External_id of the applicant to move) @example(app_abc123)) -> output
  ```
  After:
  ```rbs
  def call: (applicant_id: String) -> output
  ```

  The annotations remain available to downstream consumers (e.g. `mcp_authorization`) which read them directly from the Ruby source via `Method#source_location`.

## [0.3.5] - 2026-05-01

### Changed
- **Multi-line formatting for wide records and signatures**: Record types with more than 3 keys and method signatures with more than 3 parameters are now emitted across multiple lines in `.rbs` output. Previously these collapsed onto a single line, producing 1500+ character lines that were unreadable in PR diffs. Records and signatures with 3 or fewer entries are unaffected. (#10)

  Before:
  ```rbs
  type applicant_local = { external_id: String, name: String, email: String, status: String }
  ```
  After:
  ```rbs
  type applicant_local = {
    external_id: String,
    name: String,
    email: String,
    status: String,
  }
  ```

### Migration notes
- Regenerating signatures will produce a one-time diff in committed `sig/` files for any record or method over the 3-entry threshold. The output is semantically identical — re-run `sentinel init` to land the formatting change in one commit.

## [0.3.4] - 2026-03-30

### Added
- **Module RBS generation**: Modules with `#:` annotations (e.g. ActiveSupport::Concern modules) now generate `.rbs` signatures. Previously only classes were processed, causing `RBS::UnknownTypeName` errors for included concerns. (#7)

## [0.3.3] - 2026-03-30

### Fixed
- **Multiline `# @rbs type` union aliases**: Continuation lines using `|` (both leading-pipe and trailing-pipe styles) were silently dropped. Only the first line was included in generated `.rbs` output. (#5)
- **Trailing-pipe type alias followed by `#:` annotation**: A `#:` method signature immediately after a trailing-pipe union type alias is no longer swallowed as part of the type alias.

## [0.3.2] - 2026-03-29

### Fixed
- **Multiline `#:` annotations**: Continuation lines are now accumulated instead of overwriting earlier lines. Previously only the closing line was captured, dropping all parameters. Uses the existing `is_balanced()` delimiter check to detect incomplete signatures. (#3)

## [0.3.1] - 2026-03-29

### Added
- **Class method support (`def self.method_name`)**: Singleton methods are now detected and emitted as `def self.name: sig` in RBS output.
- **`class << self` block support**: Methods defined inside singleton class blocks are emitted as class methods.
- **`# @rbs type` alias support**: Single-line and multi-line type alias declarations are parsed and emitted as `type name = definition` in RBS output.
- **`attr_reader`/`attr_writer`/`attr_accessor` support**: Typed attribute annotations using `#:` are detected and emitted (e.g., `attr_reader name: String`). Supports multiple symbols per call.

### Changed
- Refactored class body traversal into `flatten_children()` + `scan_body()` to support all new node types without duplication.
- Files containing only type aliases or attributes are no longer skipped during `init` and `check`.
- `is_balanced()` uses stack-based delimiter matching instead of a simple depth counter.

## [0.3.0] - 2026-03-26

### Added
- **`sentinel check` command**: Read-only verification that generated RBS signatures are up to date. Exits with code 1 if any files are missing or stale. Designed for CI pipelines and Git pre-commit hooks.

## [0.2.2] - 2026-03-18

### Added
- **`.sentinel.toml` config file**: Sentinel now reads watched folders and output path from a `.sentinel.toml` file at the project root. Created automatically on first `init` or `watch` with `app` as the default folder.
- **`sentinel add <folder>`**: Add a folder to the watch list from the CLI.
- **`sentinel remove <folder>`**: Remove a folder from the watch list.
- **`sentinel list`**: Display current watched folders and output path.
- **Multi-folder support**: `init` and `watch` now process all configured folders, not just `./app`.

### Changed
- `serde` and `toml` added as dependencies for config serialization.
- Output path (`sig/generated`) is now configurable via the `output` key in `.sentinel.toml`.

## [0.2.1] - 2026-03-17

### Added
- **`sentinel init` command**: Parallel batch RBS generation using rayon. Scans all `.rb` files in `./app` and generates signatures across all CPU cores.
- **Init-on-watch**: `sentinel watch` (the default) now runs a full init before starting the file watcher, ensuring `sig/generated` is always complete on startup.
- **Skip unannotated files**: Files with no `#:` annotations are skipped entirely, avoiding unnecessary I/O. On a ~5k file codebase, this reduced init time from ~1.4s to ~0.4s.

### Changed
- **Gem renamed to `rbs-sentinel`**: The gem is now `gem 'rbs-sentinel'` on rubygems.org. The `bundle exec sentinel` command is unchanged.
- Added `rayon` and `walkdir` dependencies for parallel file processing.

## [0.2.0] - 2026-03-16

### Fixed
- **Fully-qualified class names**: Sentinel now emits proper nested `module`/`class` declarations instead of bare class names. Classes like `Top::Middle::Set` are wrapped in `module Top; module Middle; class Set` rather than emitting a flat `class Set` that collides across namespaces.
- **Compact namespace support**: Classes using `class Proxy::Set` syntax are now recognized (previously emitted `class UnknownClass`).
- **Multiple method signatures**: All `#:` annotated methods in a class are now emitted. Previously only the first method was captured; subsequent annotations inside `body_statement` were silently dropped.
- **Editor temp file filtering**: Sentinel no longer attempts to transpile `sed` temp files (`.!PID!filename.rb`) or editor swap files, which caused spurious "No such file or directory" errors.
- **Watcher debounce**: Replaced destructive event drain with a collect-then-process debounce. The old logic discarded pending events indiscriminately, causing ~1 in 3 saves to be missed.

### Added
- **Angle bracket lint plugin**: Warns when RBS output contains `Array<Hash>` style generics instead of the correct `Array[Hash]` square bracket syntax.
- **Linux platform binaries**: Gem now ships `aarch64-linux` and `x86_64-linux` binaries in addition to the existing macOS builds.
- **Release script**: `scripts/release.sh` cross-compiles all 4 platform binaries and packages the gem.

### Changed
- Removed unused dependencies (`reqwest`, `serde`, `serde_json`, `lsp-types`), reducing binary size from ~5MB to ~3MB.
- Replaced tree-sitter query-based method extraction with direct AST walking for correctness.

## [0.1.0] - 2026-03-16

### Added
- Initial release.
- Tree-sitter based Ruby parser that extracts `#:` (rbs_inline) type annotations and generates `.rbs` signature files.
- File watcher daemon that monitors `./app` and auto-generates signatures into `./sig/generated`.
- **VoidArgument lint plugin**: Warns on `void` used as a method parameter type.
- **TypeCase lint plugin**: Warns on lowercase primitive types (`string`, `integer`, etc.).
- Ruby gem wrapper (`sentinel`) with platform-specific Rust binary dispatch.
- macOS binaries (aarch64, x86_64).
