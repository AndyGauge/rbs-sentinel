#!/usr/bin/env bash
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo "Building sentinel-rb v${VERSION} for all platforms..."

# Build all targets
cargo zigbuild --release --target aarch64-apple-darwin 2>&1 | tail -1
cargo zigbuild --release --target x86_64-apple-darwin 2>&1 | tail -1
cargo zigbuild --release --target x86_64-unknown-linux-gnu 2>&1 | tail -1
cargo zigbuild --release --target aarch64-unknown-linux-gnu 2>&1 | tail -1
# GNU ABI only (zig bundles mingw-w64) - `-pc-windows-msvc` is not buildable
# this way, since that needs Microsoft's own linker/SDK.
cargo zigbuild --release --target x86_64-pc-windows-gnu 2>&1 | tail -1

# Copy binaries into gem
cp target/aarch64-apple-darwin/release/sentinel-rb sentinel-gem/exe/sentinel-aarch64-darwin
cp target/x86_64-apple-darwin/release/sentinel-rb sentinel-gem/exe/sentinel-x86_64-darwin
cp target/x86_64-unknown-linux-gnu/release/sentinel-rb sentinel-gem/exe/sentinel-x86_64-linux
cp target/aarch64-unknown-linux-gnu/release/sentinel-rb sentinel-gem/exe/sentinel-aarch64-linux
cp target/x86_64-pc-windows-gnu/release/sentinel-rb.exe sentinel-gem/exe/sentinel-x86_64-windows.exe

echo "Binaries:"
ls -lh sentinel-gem/exe/

# Build the gem
echo ""
echo "Building gem..."
rm -f sentinel-gem/rbs-sentinel-*.gem
cd sentinel-gem
gem build rbs-sentinel.gemspec
echo ""
echo "Done! rbs-sentinel-${VERSION}.gem is ready in sentinel-gem/"
