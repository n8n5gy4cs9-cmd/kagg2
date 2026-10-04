#!/usr/bin/env bash
# Release build (T31): checks, optimised build and a distributable archive in dist/.
# Usage: tools/release.sh [target-triple]   (default: the host; cross targets need their toolchain)
set -euo pipefail
cd "$(dirname "$0")/.."
target="${1:-}"
ver="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
python3 tools/licence_check.py
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release
if [ -n "$target" ]; then
  cargo build --release --target "$target" --bins
  bin="target/$target/release"; name="$target"
else
  cargo build --release --bins
  bin="target/release"; name="$(uname -s | tr A-Z a-z)-$(uname -m)"
fi
ext=""; case "$name" in *windows*) ext=".exe";; esac
pkg="kagg2-$ver-$name"
rm -rf "dist/$pkg"; mkdir -p "dist/$pkg"
for b in kagg2 server netclient; do cp "$bin/$b$ext" "dist/$pkg/"; done
cp -R assets HTML missions "dist/$pkg/"
cp LICENSE LICENCES.md CREDITS.md README.md "dist/$pkg/"
(cd dist && case "$name" in
  *windows*) zip -qr "$pkg.zip" "$pkg";;
  *) tar czf "$pkg.tar.gz" "$pkg";;
esac)
echo "built dist/$pkg.*"
