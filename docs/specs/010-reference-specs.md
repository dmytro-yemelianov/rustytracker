# Reference Specs

RustyTracker treats MilkyTracker's checked-out source tree and bundled reference
documents as the compatibility baseline. The goal is FastTracker II XM behavior
first, with ProTracker MOD compatibility as a later track.

## Primary Targets

- FastTracker II XM file format and replay behavior
- MilkyTracker-compatible XM import/export quirks
- ProTracker MOD behavior for later MOD playback/export work

## Local References

MilkyTracker repository root:

```text
/home/dmytro/github/MilkyTracker
```

Authoritative files for the current Rust rewrite:

- `README.md`: states the product target: XM tracker, `.MOD` and `.XM` files,
  FastTracker II replay/user experience, and ProTracker 2/3 compatibility modes.
- `resources/reference/xm-form.txt`: bundled "Complete" XM module format
  specification v0.81; use for binary layout and field ordering.
- `resources/reference/xmeffects.html`: bundled XM effects reference; use for
  effect names and FT2 effect-column/volume-column behavior.
- `src/milkyplay/LoaderXM.cpp`: load-time XM compatibility behavior; use for
  parser semantics and MilkyTracker quirks.
- `src/milkyplay/ExporterXM.cpp`: save-time XM compatibility behavior; use for
  writer semantics and inverse effect mapping.
- `src/milkyplay/XModule.cpp` and `src/milkyplay/XModule.h`: shared internal
  module model, effect constants, valid XM effects, and conversion helpers.
- `src/milkyplay/PlayerSTD.cpp` and `src/milkyplay/PlayerIT.cpp`: playback
  effect semantics and compatibility flags; use when starting the playback
  milestone.

## Rules For Using References

- Tests must name which compatibility behavior they pin when the behavior comes
  from MilkyTracker or the bundled XM references.
- Constants copied from file-layout or effect behavior must be named in code.
- Prefer `xm-form.txt` for byte layout and `LoaderXM.cpp` / `ExporterXM.cpp`
  for MilkyTracker compatibility deviations.
- If a bundled reference and MilkyTracker source disagree, RustyTracker follows
  MilkyTracker source for compatibility and records the discrepancy in the spec
  or test name.

## Current Implementation Links

- XM parser and writer spec: `docs/specs/005-xm-parser.md`
- XM writer milestone: `docs/specs/009-xm-writer.md`
- Playback skeleton spec: `docs/specs/011-playback-skeleton.md`
- Roadmap and active tasks: `docs/specs/007-roadmap-and-tasks.md`
