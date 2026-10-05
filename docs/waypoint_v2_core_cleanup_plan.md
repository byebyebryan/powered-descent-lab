# Planner V2 core loop cleanup plan

Refactor the accepted synchronous V2 flight loop into named planning and
execution phases without changing its flight policy. This pass follows the
records, replay-safety and test separation committed at `894238e` and the
current-state reconciliation at `58013c8`. The goal is a core loop that is easy
to inspect and maintain before broader terrain evaluation.

## Scope and ownership

The primary owns phase boundaries, integration, acceptance and checkpoint
commits. One Luna Max worker owns implementation in
`pd-eval/src/waypoint_v2.rs`, its internal child modules and focused tests.
The primary maintains this plan and the final results.

The large pack/report split is a separate follow-up. No new solver, standalone
crate extraction, tuning, terrain corpus, controller-default change, report
redesign, publication, push or server restart is included.

## Core phases

1. Start a cycle at initial rest or the retained actual handoff, respecting the
   original deadline, correction count and command clock.
2. Establish a nominal program through the existing initial, policy-3 airborne
   or historical policy-1/2 path. Share repeated status accounting while keeping
   each generator's ordering and rejection semantics.
3. Audit that fixed program, validate the live-origin comparison and prove its
   consumed prefix. Distinguish a non-terrain rejection from a real obstruction.
4. Execute an accepted direct piece, or derive the conflict and search the
   unchanged finite local clearing family.
5. Execute the selected nominal approach to intervention E and correction to
   actual handoff H, prove H and the private continuation certificate, then
   return handoff progress for the next cycle.

`advance_piece` should read as this orchestration rather than contain all search,
bookkeeping and execution details. Extract cohesive helpers with explicit
inputs and outputs; do not just relocate the long method intact or introduce a
generic framework. Remove the unreachable `NoProgress` branch in the hardcoded
`NoClearing` path, but retain persisted enum variants and historical policy APIs.

## Invariants

- Preserve nominal families, policy parameters, entry/template iteration order,
  selection ranking, commands and terrain-blind nominal choice.
- Preserve full actual state, fuel, clock, attitude/rate, original deadline,
  correction counting and initial-versus-airborne phase ownership. Queries and
  continuation certificates remain private clones, never restart inputs.
- Preserve cycle, segment and command ownership. Piece origin is not E; the
  next cycle starts at actual H rather than the certificate endpoint.
- Preserve planning/execution/replay timing attribution. Numerical comparisons
  exclude wall-time values, not the work assigned to each timing bucket.
- Preserve pending-cycle and local-search checkpoint order, completed rejection
  counts, failed-row retention, typed terminal stops and partial execution on
  error. Do not convert integrity errors into finite coverage misses.
- Keep phase-aware guards, admission checks, independent replay proofs and
  finalization intact. Final proof may override provisional landing progress.
- Preserve public exports, serde names and field order, existing CLI contracts,
  create-only artifacts and report paths. Do not modify frozen inputs or saved
  captures to match the refactor.

## Checkpoints and validation

1. Commit this settled plan from the clean starting checkpoint. Record the
   current selection and retained evidence hashes before implementation.
2. Delegate the cohesive core refactor and focused regression coverage to Luna.
   The worker runs focused tests and hands off changed files and findings;
   it does not commit or run acceptance/publication matrices.
3. Review the actual diff, including error paths and timing/checkpoint ordering.
   Run focused V2 and CLI tests and the exact retained-capture numerical
   regression. Correct ordinary defects before committing the core checkpoint.
4. Run the final feature-enabled workspace gate once, default-off CLI checks,
   formatting, strict Clippy with the established `single_element_loop`
   exception, 62 maintained JavaScript tests and the saved common-report check
   using a rebuilt evaluator. Repeat full gates only for relevant later changes.
5. Verify saved evidence, current selection and published reports are unchanged,
   reconcile module ownership and record results in a separate results note.
   Review and commit the final checkpoint; finish with a clean worktree.

The numerical regression uses the accepted 44-case capture
`outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native/`:

```sh
rtk proxy env PD_V2_PARITY_CAPTURE=outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native \
  cargo test --release -p pd-eval --lib retained_capture_numerical_parity -- --ignored --nocapture
```

All complete flight records must match after excluding only the three flight
wall-time fields, including unsupported inputs and finite diagnostics. This is
local regression execution, not a new measured acceptance/publication matrix.
Run it after integration and repeat only if subsequent code changes require it.
Retain the first difference and diagnose it; do not widen comparison exclusions,
change inputs or tune selection to obtain parity.

## Exit gate

Finish only when named phases replace the mixed-responsibility core method,
dead/repeated scaffolding is simplified, all relevant tests and exact 44-case
parity pass, failure evidence and policy-1/2 compatibility are preserved, saved
reports/evidence are unchanged and all intended checkpoints are reviewed and
committed. Remaining work should be the separately scoped pack organization or
broader terrain evaluation, not an unresolved core refactor defect.
