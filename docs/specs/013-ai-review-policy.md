# AI Review Policy

RustyTracker uses AI review tools as advisory reviewers, not as sources of
truth. Their findings must be verified against the repository's specs, tests,
and compatibility model before code changes are made.

## CodeRabbit Role

CodeRabbit should be used as a post-validation review gate for pull requests:

- Run the relevant local validation first (`cargo fmt --check`, focused tests,
  workspace tests when appropriate, clippy, and diff whitespace checks).
- Let CodeRabbit review the pushed PR.
- Request re-review deliberately after meaningful fixes instead of relying on
  review runs for every small follow-up commit.
- Treat each comment as a hypothesis. Verify the claim against current code,
  docs/specs, fixture behavior, and parser/writer normalization before acting.
- Fix correctness, compatibility, API-stability, missing-test, and
  behavior-preservation findings that remain valid.
- Skip or answer findings that conflict with documented RustyTracker behavior,
  especially internal normalized representations that are intentionally
  converted at format boundaries.

## Review Budget

CodeRabbit review capacity is finite. To avoid burning review slots while a
stack of small PRs is in flight:

- keep automatic initial PR review enabled, but disable automatic incremental
  review;
- batch small fixups before requesting another review;
- prefer local validation and human judgment for docs-only or configuration-only
  PRs when CodeRabbit is rate-limited;
- continue using CodeRabbit for behavior-sensitive Rust changes, compatibility
  fixes, public API moves, and parser/writer boundary work.

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

A behavior-changing code PR can merge when local validation passes, CodeRabbit's
status is complete, and there are no unresolved valid actionable review threads.
While CodeRabbit is queued or reviewing, independent branches may continue, but
the pending code PR should not merge on stale review state.

Docs-only, configuration-only, and already-reviewed mechanical cleanup PRs may
merge after local validation when CodeRabbit is unavailable or rate-limited, as
long as there are no unresolved valid actionable comments already present.
Generic warnings such as docstring coverage are advisory for mechanical split
PRs unless they identify a concrete public API regression.
