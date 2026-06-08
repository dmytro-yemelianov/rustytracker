# TDD Plan

## Rule

Every compatibility behavior starts as a failing test.

## Test Pyramid

1. Unit tests in each crate for local invariants.
2. Property tests for bounds, packing, and roundtrip edge cases.
3. Fixture tests against known XM/MOD files.
4. Golden JSON tests generated from MilkyTracker.
5. PCM render tests against MilkyTracker WAV output.

## Initial Red-Green-Refactor Loop

1. Write a contract test in `crates/*/tests`.
2. Implement the smallest code that satisfies the test.
3. Refactor only after the behavior is green.
4. Add the MilkyTracker reference file/line in the test name or comment when a
   behavior is compatibility-sensitive.

## Golden Data

Use the existing MilkyTracker files first:

- `resources/music/milky.xm`
- `resources/music/slumberjack.xm`
- `resources/music/sv_ttt.xm`
- `resources/music/theday.xm`
- `resources/music/universalnetwork2_real.xm`

Planned fixture outputs:

```text
tests/fixtures/milkytracker/
  music/*.xm
  golden-json/*.json
  golden-pcm/*.wav
```

## Acceptance Gates

The rewrite cannot proceed to UI work until:

- XM structural load/save roundtrip passes for all fixtures.
- Core editing commands are covered by direct unit tests.
- Playback has fixture render comparisons for tempo, envelopes, loops, volume,
  arpeggio, portamento, vibrato, pattern break, and order jump behavior.

