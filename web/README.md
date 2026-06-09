# RustyTracker Web Harness

This directory contains a minimal browser player for `rustytracker-wasm`.

## Build

```sh
cargo install --locked wasm-bindgen-cli --version 0.2.123
./scripts/build_wasm_web.sh
```

## Run

Serve the repository root or the `web` directory with any static HTTP server.
For example, from the repository root:

```sh
python3 -m http.server 8080
```

Then open:

```text
http://localhost:8080/web/
```

The generated `web/pkg/` directory is ignored by git.
