# Planner V2 evaluation activation results

## Verdict and scope

Accepted for the default **planner evaluation/report** workflow on the current
supported vehicle, Earth gravity, and 120/60 Hz setup. This implements the
[activation contract](waypoint_v2_eval_activation_plan.md); it does not change
ordinary `pd-cli run`, controller defaults, planner numerical policy, terrain,
plant/contact rules, or the frozen research protocols.

The normal command now selects the named policy-3 pack when no pack is supplied:

```bash
cargo run --release -p pd-eval -- run-pack --workers 4
```

Explicit terminal/transfer/V1 packs retain their existing evaluation workflow.
The V2 single-flight command also defaults to policy 3; explicit policy 1/2
selection remains available for historical investigation.

## One new 44-case batch

The run used the unchanged 32 practical inputs and 12 additional terrain inputs.
Typed expansion matched all 44 retained scenarios before execution. There was
one new native pack run, not a sequence of tuned flight retries.

| Group | Cases | Result |
| --- | ---: | --- |
| Clear, uncut flat/uphill/downhill controls | 11 | 11 direct target landings, zero corrections |
| Ordinary terrain | 16 | 16 target landings with corrections |
| Additional terrain | 9 | 9 target landings with corrections |
| Diagnostics | 8 | 2 target landings, 4 `NoClearing` stops, 2 unsupported rejections |

The ordinary coverage denominator is **36/36 target landings**, including
11 direct and 25 corrected missions. Diagnostics remain separate: total target
landings are 38/44 processed inputs, not 44/44. There were no crashes, integrity
failures, or unverified simulator results. All 44 inputs passed integrity checks;
all 42 supported results passed final-source replay. Unsupported cases have no
simulation and no replay claim.

The four finite diagnostics stopped before departure: zero commands, zero
physics/controller steps, and one initial sample. Their `flying` / `in_progress`
tuple describes an unexecuted stop, not a crash, touchdown, or proof of physical
infeasibility. The 38 landing flights contain 36 executed waypoint handoffs in
total. Planning-only timing across the 42 supported records was approximately
0.364 s median and 0.820 s maximum on this host with four workers; this is not a
real-time game-loop performance guarantee.

## Evidence and provenance

- Capture: `outputs/eval/planner_v2_lab_suite/capture-1791074995928276719`.
- Captured at clean source `689d05eb97d5750bfe0729308ab17c087fa3532c`.
- Capture executable SHA-256:
  `3feb7830ba79e2a3689f0ced881c3db54ea3d71657ee548f85ecc4045ab8dff5`.
- Source/input identity was unchanged before/after capture. Source coverage
  includes evaluator, planner, core, controller, report, and Cargo manifests/lock.
- Independent acceptance receipt and 14 screenshots:
  `outputs/research/planner_v2_eval_activation_20261003/acceptance_f/`.
- Preservation ledger:
  `outputs/research/planner_v2_eval_activation_20261003/preservation.json`.

Every complete `flight.json`, including both typed unsupported results, matches
its accepted archive exactly after excluding only
`/timings/planning_s`, `/timings/execution_s`, and `/timings/replay_s`. No numeric
tolerance, state/command/segment removal, or path exclusion was used in this run.
All 42 rich report payloads match their original raw reports after removing only
the additive `flightAnnotations` field. All handoff/entry snapshots and correction
counts match the raw local-correction segments exactly.

The 177 raw capture files remain byte-identical through report-only regeneration.
All 504 protected files also remain byte-identical: frozen fixtures and source
manifests, original suite runners, both accepted practical and fresh repeats,
and the original rich navigation edition. Derived detail/batch pages were refined
without rewriting original rich reports or rerunning flights. Final display-only
source is `afdeae4`; its rebuilt release executable has SHA-256
`e34ff9bd7760db90cbfc2d0d9566d03b6a9cc2c2728513b44bb7ac6b66d03353`.

## Reports and navigation

Start at `/` or `/reports/`, then **Waypoint planning -> Current V2 batch ->
Mission detail**. The stable batch URL is
`/reports/eval/planner_v2_lab_suite/index.html`; `current.json` selects the actual
create-only capture behind it. Historical previews and V1 scorecards remain
available with explicit history/legacy labels.

Every supported detail retains the original trajectory views, metrics, inspection,
events, and statistics. Executed H markers appear on both spatial and time plots;
the correction panel explains why a correction was needed and what replanning
after that handoff found. Zero-correction reports retain source links without
inventing waypoint controls. The two unsupported details have diagnostic pages,
not fabricated flight plots. All 44 details have batch/home/topic/neighbor links.

Suggested first cases: `v2_clear_845` (direct), `v2_ridge_early` (one handoff), and
`v2_plateau_reference_900` (three handoffs). Then inspect `v2_diag_high_900` for
the finite pre-departure limit and `v2_diag_lunar_gravity` for unsupported setup.

## Review and validation

Luna implemented the bounded native pack and independent checker slices. The
primary owned CLI/publication/navigation integration, architectural decisions,
review corrections, execution, browser acceptance, and incremental commits.

- Final workspace tests, formatting, and Clippy passed with the existing
  `-A clippy::single_element_loop` allowance only.
- Focused JavaScript checks passed: 25 top-level tests, including seven new
  checker tests and the unchanged frozen-harness/corpus tests.
- Explicit archive expansion comparison passed; all three retained rich-report
  presentation gates passed.
- HTTP traversal passed for all 44 details and 135 unique source links.
- Browser checks passed for seven pages at both 1440 px desktop and 390 px mobile:
  home, topic, batch, direct, one-H, three-H, and unsupported. Charts were ready;
  H toggling/selection worked; no document overflow or report JavaScript/chart-asset
  errors were observed. Optional favicon diagnostics are not report failures.
  The case table alone deliberately scrolls horizontally.
- Existing capture roots, zero workers, and unsupported legacy comparison flags
  fail before new flight execution. Final release report-only regeneration and
  home-only refresh preserve the selected capture and all saved capture bytes.

Earlier local receipts `acceptance_a` through `acceptance_e` are retained, not
accepted: they exposed checker assumptions about empty commands/JSON source
links, the missing source panel on zero-correction details, and two mobile layout
issues. Corrections affected auditing/presentation only; no mission inputs or
trajectory selection changed. `acceptance_f` is the final passed receipt.

## What remains

Use this pack as the current planner regression/viewing baseline. Ordinary
controller/game-loop integration is a separate adapter phase; do not infer it
from evaluator execution. Additional vehicles/gravities, hard-diagnostic success,
and arbitrary mission coverage remain outside this acceptance claim. No push or
report-server restart was performed.
