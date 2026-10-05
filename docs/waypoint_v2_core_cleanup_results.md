# Planner V2 core loop cleanup results

The core refactor at `408a368` preserves the accepted policy-3 flight behavior
while making its planning and execution phases explicit. All 44 retained inputs
match the accepted flight records exactly after excluding only the three
wall-time values. This is a structural cleanup, not a new solver or wider
terrain-coverage claim.

## Scope and review

The [approved plan](waypoint_v2_core_cleanup_plan.md) was committed at `95d97d2`
from the clean `58013c8` checkpoint. One Luna Max worker implemented the bounded
Rust slice; the primary retained phase-boundary decisions, diff review,
integration, final validation and local commits. The settled behavior and exact
regression gate made this a bounded implementation assignment rather than a
new architecture investigation.

`FlightLoop::advance_piece` is now 94 lines rather than 501. It delegates to
named helpers for nominal generation and audit, fixed consumed-prefix proof,
direct execution, local clearing search, and actual E-to-H execution with a
private continuation certificate. The helpers remain internal to
`pd-eval/src/waypoint_v2.rs`; this pass does not introduce a planner framework
or move an intact mixed-responsibility method to another file.

Initial, policy-3 airborne and historical policy-1/2 nominal generation remain
distinct. They share status/rejection accounting and successful proposal
bookkeeping. The hardcoded `NoClearing` path no longer contains an unreachable
`NoProgress` comparison; both persisted enum variants remain available.

Review checked generator dispatch, entry/template order and ranking, full
state/fuel/clock continuity, timing-bucket attribution and diagnostic checkpoint
order against `95d97d2`. Queries and the private certificate are still clones;
the next cycle begins at actual H. Both independent correction source proofs
must pass before proof flags, completed-cycle evidence and correction count are
committed. Existing guard and original-source replay implementations are
unchanged. CLI contracts, public exports, record schemas and report templates
are unchanged.

## Validation

The worker's focused V2 tests passed: 12 passed and the explicit retained-capture
test remained ignored in the ordinary run. Three new regressions cover shared
attempt/rejection counts, an audit error retaining the pre-audit pending cycle,
and a corrupted private certificate leaving both source-proof flags false.
Existing tests still cover legacy multi-handoff execution, odd-tick contact,
partial finalization evidence and segment/command tampering.

The primary's checks passed:

- Exact retained-capture numerical regression: all 44 full flight records match;
  only `planning_s`, `execution_s` and `replay_s` are excluded.
- Feature-enabled CLI: four unit tests and five integration tests.
- Default-off CLI tests and help: only ordinary commands are exposed, and the
  normal dependency tree contains no `pd-eval`.
- Formatting, diff whitespace checks and strict workspace/all-target Clippy,
  retaining only the established command-line `single_element_loop` exception.
- All 62 maintained JavaScript tests.
- Rebuilt evaluator validation of the saved accepted capture.
- Read-only saved common-report workflow: 44 cases, 42 rich payloads, 36 actual
  handoffs and 46 receipt-bound pages.
- Final feature-enabled workspace gate: 1,019 passed, zero failed and ten
  intentionally ignored tests. The retained-capture test was run separately
  and passed as described above.

The full workspace gate ran once after the reviewed core commit. No code
changed afterward; the remaining reconciliation edits are documentation only.

## Evidence preservation and limits

The current report remains bound to
`outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native/`
and its original measured source `561b6e5`. It has not been relabeled as a
capture produced by this refactor. Local regression execution does not publish
new acceptance evidence or benchmark runtime improvement.

Before implementation, the primary recorded path/content hashes for 26 fixture,
research, evaluation, validation and published-report scopes: 3,270 file or
symlink entries across those scopes. The final before/after comparison matched
every scope's entry count and path/content digest, including symlink targets.
The current selection, published batch and session-repair preservation receipt
also retained their individual hashes. The five artifact digests bound by that
receipt were independently checked unchanged.

The flight verdict is unchanged: 36 mandatory landings, two diagnostic landings,
four zero-step diagnostic `NoClearing` stops and two unsupported inputs. This
does not establish arbitrary airborne starts, random-terrain reliability, other
vehicles/gravities or a per-tick game controller.

## Remaining work

The pack/report module split remains a separate optional organization pass,
not a flight defect or prerequisite. No tuning, new terrain experiment,
standalone crate extraction, controller-default promotion, publication, push
or server restart was performed. The next substantive choice remains bounded
procedural-terrain evaluation or a concrete host-consumer requirement; neither
is started by this cleanup.
