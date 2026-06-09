# RustyTracker

RustyTracker is a Rust rewrite of MilkyTracker's core tracker engine.

The rewrite is intentionally test-first. The C++ MilkyTracker tree is treated as
the behavioral reference, while RustyTracker gets a smaller, typed core:

- module model
- XM/MOD load and save
- pattern and instrument editing
- playback and offline rendering
- UI only after the headless core is proven

The first milestone is not a GUI. It is a Rust CLI/library that can load a
reference XM, dump normalized structure, save it back, and render PCM close to
MilkyTracker's output.

## Repository Layout

```text
crates/
  rustytracker-core/   Typed module, pattern, note, instrument, and sample model
  rustytracker-cli/    Structural dump CLI and golden fixture tests
  rustytracker-xm/     Read-only XM header, pattern metadata, and packed cell decoder
  rustytracker-wasm/   Browser-loadable WASM playback engine bindings
docs/specs/            Rewrite specs and TDD plan
web/                   Minimal browser player harness for rustytracker-wasm
```

Planned crates:

```text
rustytracker-mod       MOD parser/writer
rustytracker-play      Playback, effects, mixer, render-to-buffer
rustytracker-edit      Editing commands, undo, transformations
rustytracker-cli       Golden-test and inspection CLI
rustytracker-ui        Eventual desktop UI
```

## Browser Harness

The WASM engine can be packaged for a local browser player:

```sh
cargo install --locked wasm-bindgen-cli --version 0.2.123
./scripts/build_wasm_web.sh
python3 -m http.server 8080
```

Open `http://localhost:8080/web/` and load an XM or MOD file.

## Test Policy

No compatibility-sensitive behavior is implemented without a test first.

The test ladder is:

1. Unit tests for typed domain invariants.
2. Parser/writer roundtrip fixtures.
3. Golden JSON dumps generated from MilkyTracker.
4. Offline PCM render comparison against MilkyTracker.
5. UI behavior tests only after the core is stable.

Current coverage:

- `rustytracker-core`: empty module defaults, pattern bounds, fixed text,
  orders, notes, instruments, samples, envelopes, vibrato, and sample loop
  kinds.
- `rustytracker-xm`: MilkyTracker-bundled XM headers, pattern headers, packed
  pattern cell expansion, instrument/sample-header metadata, delta-coded sample
  payload decoding, ModPlug stereo sample mixing, loop-kind normalization,
  ADPCM unsupported errors, sparse order references, XM header/order and simple
  pattern writing, XM instrument metadata and delta-coded sample payload
  writing, end-to-end load into `rustytracker-core::Module`, and malformed
  input checks.
- `rustytracker-cli`: `rustytracker dump <module.xm> --format json`, schema
  validation, and golden structural dumps for bundled fixtures.
