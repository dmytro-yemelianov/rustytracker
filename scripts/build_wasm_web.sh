#!/usr/bin/env sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
WASM_TARGET="wasm32-unknown-unknown"
WASM_BINDGEN_VERSION="0.2.123"
OUT_DIR="$ROOT_DIR/web/pkg"
WASM_FILE="$ROOT_DIR/target/$WASM_TARGET/release/rustytracker_wasm.wasm"

if ! rustup target list --installed | grep -q "^$WASM_TARGET$"; then
  echo "Installing Rust target $WASM_TARGET"
  rustup target add "$WASM_TARGET"
fi

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "wasm-bindgen CLI is required."
  echo "Install it with:"
  echo "  cargo install --locked wasm-bindgen-cli --version $WASM_BINDGEN_VERSION"
  exit 1
fi

cargo build -p rustytracker-wasm --release --target "$WASM_TARGET"

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

wasm-bindgen "$WASM_FILE" \
  --target web \
  --out-dir "$OUT_DIR" \
  --out-name rustytracker_wasm

echo "Built web WASM package in $OUT_DIR"
