# AI Review Policy

RustyTracker uses AI review tools as advisory reviewers, not as sources of
truth. Their findings must be verified against the repository's specs, tests,
and compatibility model before code changes are made.

## CodeRabbit Role

CodeRabbit should be used as a post-validation review gate for pull requests:

- Run the relevant local validation first (`cargo fmt --check`, focused tests,
  workspace tests when appropriate, clippy, and diff whitespace checks).
- Let CodeRabbit review the pushed PR.
- Treat each comment as a hypothesis. Verify the claim against current code,
  docs/specs, fixture behavior, and parser/writer normalization before acting.
- Fix correctness, compatibility, API-stability, missing-test, and
  behavior-preservation findings that remain valid.
- Skip or answer findings that conflict with documented RustyTracker behavior,
  especially internal normalized representations that are intentionally
  converted at format boundaries.

## Mechanical Split PRs

For monolith-splitting PRs, CodeRabbit feedback is useful when it checks:

- moved code still preserves public crate-root API;
- moved constants retain names and domain-specific meaning;
- trait implementations and error conversions did not get lost;
- tests still cover the facade or boundary being moved.

It should not drive new architecture in these PRs. Architecture changes need a
separate design update or a follow-up issue, not an opportunistic rewrite during
a mechanical extraction.

## Merge Policy

A PR can merge when local validation passes, CodeRabbit's status is complete,
and there are no unresolved valid actionable review threads. While CodeRabbit is
queued or reviewing, independent branches may continue, but the pending PR
should not merge on stale review state.
