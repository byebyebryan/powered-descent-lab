# Shared controller/RL preparation execution

[Plan](controller_rl_preparation_plan.md) · [Review](controller_rl_preparation_review.md) ·
[Documentation home](README.md)

Launched October 12, 2026, from `f108065`, by the owner's instruction to run the
goal loop and commit as work completes. This declaration adopts the reviewed
plan's default allocation. The loop stops at the validated shared checkpoint
and independent branch handoffs.

## Authorized allocation

- Start: 2026-10-12 07:24:37 UTC. Wall ceiling: 2026-10-12 15:24:37 UTC.
- At most 6,016 newly scheduled input proposals, retaining all dispositions.
- Exactly 16 already exposed integration missions, eight per backend, through
  three predeclared composition paths: at most 48 scheduled attempts and one
  fresh independent verification per captured execution.
- Python integration uses an explicitly identified deterministic untrained test
  policy. No optimizer steps, checkpoint selection or training transitions.
- Fresh validation-reference flights are skipped. Final-test flights and
  individual final-test inspection are zero.
- Preserve the nominal 4 m physical polyline, plant, clocks, mission contract,
  reference controller and historical proof/capture/report bytes.
- Commit reviewed slices. Do not push or publish accepted report selectors.
  A disposable QA server and new preparation reports are within the integration
  scope; the user's existing report service remains untouched.

Input requests, stress support selection and integration schedules are committed
before their corresponding generation or measured execution. Proposal and study
budgets are independent of temporary source-fixture software checks. The optional
128 additional resolution proposals may be unused if retained mechanism evidence
suffices. Stress shortfalls remain documented coverage limits.

## Evidence and stage status

The new artifact namespace is
`outputs/controller-rl-preparation/run-20261012T072437Z/`. Artifacts are create-only;
partial or failed stages remain evidence and do not authorize replacement inputs.

| Stage | Status |
| --- | --- |
| P0 physical contract and reference identity | Complete; physical grid retained, warnings/limits recorded |
| P1 exposure inventory and complete requests | Complete; requests committed before generation |
| P2 verified sealed cohorts and stress accounting | Complete; no audited collisions, sparse quotas retained |
| P3 neutral V2 Python/evidence/report interoperability | Complete; maintained, Python and actual fault-fixture checks passed |
| P4 48-slot integration and browser acceptance | Pending |
| P5 common checkpoint and isolated branch handoffs | Pending |

This run creates no controller improvement or RL qualification claim. Completion
requires actual maintained gates, input/evidence checks, fresh smoke verification
and browser acceptance, followed by a source/artifact handoff. Reaching the wall
ceiling with outstanding requirements leaves the goal incomplete.

The [run record](controller_rl_preparation_results.md) binds completed inputs,
physical/reference receipts, exposure limits and outstanding readiness gates.
