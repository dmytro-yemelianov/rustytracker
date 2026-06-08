# Structural Dump CLI Spec

## Scope

`rustytracker-cli` provides the first user-facing tool for the headless rewrite.
It does not play or edit modules. Its job is to load supported module files and
emit stable JSON that makes parser regressions easy to review.

Command:

```text
rustytracker dump path/to/module.xm --format json
```

## Dump Contract

The JSON schema lives at:

```text
crates/rustytracker-cli/schema/module-dump.schema.json
```

The current dump schema version is `1`. Version `1` contains:

- module header fields
- active order table
- per-pattern row/channel/effect-slot shape
- per-pattern non-empty cell count
- per-pattern expanded-cell checksum
- instrument names, sample slots, note/sample-map checksum
- instrument volume and panning envelopes
- instrument vibrato metadata and volume fadeout
- sample metadata, loop kind, payload kind, frame count, payload checksum, and
  short payload prefixes

The dump intentionally does not include full pattern cell streams or full sample
payloads. Those are represented by stable checksums and short prefixes so golden
diffs stay readable while still catching structural regressions.

## Golden Fixtures

Golden outputs live in:

```text
crates/rustytracker-cli/tests/golden/
```

The fixture set matches the current XM parser fixture base:

- `milky.xm`
- `slumberjack.xm`
- `sv_ttt.xm`
- `theday.xm`
- `universalnetwork2_real.xm`

`cargo test --workspace` verifies that each bundled fixture still dumps to the
committed JSON exactly.
