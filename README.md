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

### LLM-friendly API (`api` subcommand)

`rustytracker api` accepts a JSON request and returns a JSON response.

```json
{
  "id": "optional",
  "method": "module.render_wav",
  "params": {
    "module_path": "path/to/module.xm",
    "sample_rate": 44100,
    "duration_ms": 500,
    "mixer": "hifi",
    "include_wav": false,
    "output_path": "/tmp/preview.wav"
  }
}
```

Supported methods:

- `api.methods` – discover available methods and supported values.
- `module.load` / `module.dump` – validate and dump module metadata.
- `module.play_state` – step through playback state (rows).
- `module.render_wav` – render headless WAV. Use `duration_ms` or `max_frames` to bound output.
  Set `include_wav=false` and `output_path` for large file rendering.
- `module.launch_ui` – write module file and launch `rustytracker-ui` with that path.
- `module.new` – create an empty module and optionally apply creation/structure patches
  in one request.
- `module.apply_patch` – mutate notes/effects with deterministic operations.
- `module.write`, `module.write_xm`, `module.write_mod` – serialize module bytes.

`module.apply_patch` supports creation and structural operations for
`create_sample`, `create_instrument`, `create_pattern`, and track operations in
addition to note/effect edits:

- `insert_track` / `delete_track`
- `create_pattern` / `delete_pattern` (pattern operations)
- `create_instrument` / `delete_instrument` / `rename_instrument`
- `create_sample` / `delete_sample` / `rename_sample`

Discovery + deterministic examples for LLM tool use:

```json
{ "method": "api.methods" }
```

```json
{
  "id": "create-song",
  "method": "module.new",
  "params": {
    "module_title": "LLM Demo",
    "module_channel_count": 6,
    "patch": [
      { "op": "create_pattern", "rows": 64 },
      { "op": "create_sample", "index": 0, "sample": { "name": "Kick", "data": { "kind": "empty" } } },
      { "op": "create_instrument", "index": 0, "name": "Drums", "default_sample_index": 0 },
      { "op": "set_note", "pattern": 0, "channel": 0, "row": 0, "note": 49 }
    ]
  }
}
```

```json
{
  "method": "module.render_wav",
  "params": {
    "module_bytes_b64": "<bytes from module.new.result.module_bytes_b64>",
    "duration_ms": 1500,
    "sample_rate": 22050
  }
}
```

```json
{
  "method": "module.launch_ui",
  "params": {
    "module_bytes_b64": "<bytes from module.new.result.module_bytes_b64>",
    "output_format": "xm"
  }
}
```

This API is intentionally request/response oriented for tool callers: every
response includes `schema_version`, `ok`, `id`, `method`, and either `result` or
`error`.

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

This API path is optimized for LLM/tool-driven composition and playback workflows (headless or headed).
