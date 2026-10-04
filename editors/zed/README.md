# Sentinel RBS — Zed extension

Runs `sentinel lsp` as a second language server on Ruby files, alongside Zed's own Ruby support (and Steep, if configured). See the [Zed setup page](https://andygauge.github.io/rbs-sentinel/editor-feedback/zed.html) in the docs for full details.

## Install (dev extension)

1. Open Zed's command palette and run **`zed: install dev extension`**.
2. Select this directory (`editors/zed`, the one containing `extension.toml`).

Zed builds it against `wasm32-wasip2` itself — no local Rust toolchain required.

## Develop

```bash
cargo build --target wasm32-wasip2 --release
```

`sentinel` must be resolvable in your Ruby project — either a `bin/sentinel` binstub (`bundle binstubs rbs-sentinel`) or on `$PATH` (`gem install rbs-sentinel`). See [`src/lib.rs`](src/lib.rs) for the exact lookup order.
