# Platform restructure implementation record

[Plan](platform_restructure_plan.md) · [Execution contract](platform_restructure_execution.md) · [Review](platform_restructure_review.md)

Status: active implementation goal, started October 10, 2026. The full restructure
is not complete. Accepted milestones are recorded below; work continues through
the remaining conditions in the execution contract.

## Baseline and ownership

Starting commit: `a449975e3f842094455c228b452e6b9b3e7801f4`.
The source inventory, starting binary patch and copies of initially dirty files
are retained in `outputs/restructure/run-20261010T170648Z/`. `baseline.json`
records 539 tracked/untracked source identities and ten protected dirty paths.
The complete maintained developer gate passed all 11 checks before source edits;
its log is `baseline-gate.log` in that run directory.

Existing report navigation/server work is unrelated and remains owned by its
original writer: root README, reports documentation, navigation fixture and
renderer, scripts README/server launcher, navigation discovery module, Python
server and server/library tests. These starting changes are excluded from this
goal's commits. The architecture plan, review, execution contract and their docs
home/roadmap links are owned by this goal.

The retained Policy 3 comparator
`outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native` was available
at launch. Frozen inputs, historical outcomes, selectors and retained reports
are preserved. New validation artifacts stay under the unique run directory.

## Accepted checkpoints

- A: starting ownership and source inventory captured; baseline gate passed.
  The owner explicitly authorized commits during implementation. No push or
  publication is included.
- B: `pd-run` owns neutral frames, telemetry, update records, performance and
  online/prepared execution. `pd-control` re-exports the unchanged serialized
  record shapes and adapts its existing reset/update interface. The open `Agent`
  interface receives physical inputs without scenario labels, seeds or tags.
  Three maintained controllers were compared with fresh instances using the
  direct core callback oracle: exact commands, frames, events, samples and full
  manifests matched. Four independent runtime tests cover complete replay,
  prepared binding/cadence, odd hard endpoints, zero commands, failed decisions
  and membership-independent actions. Reset failures retain a typed zero-step
  stop. Incremental reset/step and capture allocation checks remain in E.
  `runtime-tests.log` records 176 controller and four independent tests passing.
  The full maintained gate is recorded in `runtime-gate-v2.log`; the first
  attempt caught a constructor naming error before execution and is retained.
  The all-features normal `pd-run` tree contains only core and utility crates.
  All ten unrelated starting dirty files remain byte-for-byte preserved.

- C: `pd-missions` implements versioned recipes, index-local
  seeded value-noise terrain, local pad preparation, finite structural rejection,
  create-only collections, physical/byte identities and append-only usage receipts.
  Seven contract tests and strict crate Clippy passed. Characterization admitted
  10,000 unique missions with one rejection/retry and no exhausted requests:
  10,001 proposals, one more than the plan's nominal 10,000 proposal allowance.
  No solver results informed this distribution. All 10,000 requests were repeated
  in reverse order with exact result identity. The 64-mission integration
  collection was generated and verified; see [workflow/coverage](procedural_missions.md).
  Recipe and mission clocks/resources are explicit. Input readers do not regenerate
  terrain, and no Pylander checkout or retained capture is a dependency.
  The full maintained gate passed all 11 checks (`missions-gate.log`).
  The pre-composition retained comparator passed exact numerical parity on all
  44 cases (`pre-composition-parity.log`); timing exclusions remain the three
  original wall-time fields. Retained inputs and artifacts were read-only.

## Remaining work

- D: neutral evaluation/rendering and explicit application composition.
- E: reset/step, capture modes, independent checkpoint adapter and measurements.
- F: maintained solver ownership and exact preservation with complete configuration.
- G: execution-path consolidation, dependency gates and current workflow docs.

No flight performance improvement or trained policy is claimed by this record.
