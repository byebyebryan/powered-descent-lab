# Correction-cap sensitivity diagnostic

Date: 2026-10-07. User-approved follow-up to the
[ten-world diagnostics](terrain_diagnostics_results.md).
Completed by the [cap diagnostic results](terrain_cap_probe_results.md): six
cap-12 attempts; cap 24 was not admitted. This is not an open retry allowance.
`030` and `280` are physically still flying with mission in progress when the
planner stops at six corrections. They are incomplete under that budget, not
crashes, demonstrated physical impossibilities or proven faulty short-hop logic.
Check the cap before redesigning correction progress.

## Question and fixed inputs

Can the unchanged planner complete these two exact missions with more permitted
corrections, or does it reach another finite stop/deadline? Re-run from the
original pad source and prove the executed six-correction prefix identical;
do not restore a synthetic airborne snapshot or change the remaining budget.

- Subjects: frozen `random-030`, `random-280` scenarios.
- Controls: clear-direct `random-000` and six-handoff landing `random-253`.
- Baseline: authenticated `capture-diagnostics-20261007-v1`, receipt
  `03b8ac8f74b7fd7290bf0e65cd1e306fbe47da27689631e234f881d1eb6ea09c`.
- No changes to terrain, pads, vehicle, gravity, 120/60 Hz, initial state, 80 s
  absolute deadline, initial fuel, nominal logic, local templates, entries,
  ranking, clearance/continuation guards, controller or replay logic.

## Isolated execution design

Production policy 3 admits its exact six-correction definition. Do not loosen
that validator or add another production policy/CLI flag for this diagnostic.
Generate create-only build-source copies under the new capture. Each differs
from the frozen workspace in exactly one file, `pd-plan/src/waypoint_v2.rs`:
the revision-3 correction cap and its explicit diagnostic policy identity.

The probe identities are
`piecewise_local_clearing_v2_policy_3_cap_probe_12` and `_24`. The raw flight
policy and native input identity must retain that difference; never relabel a
probe as ordinary accepted policy 3. Build each copy with locked/offline Cargo
in a separate task-specific target directory, then retain its binary/hash and
complete source inventory. The ordinary `target/release/pd-eval`, tracked Rust
source, default pack, selectors and published reports remain untouched.

This is a saved-input study, not a new supported planner revision. Probe flights
use the same native single-flight writer, physical simulation, original-source
replay and common rich detail report. The collector has its own explicit
diagnostic policy admission; production acceptance is not relaxed.

## Finite allowance and conditional second stage

1. Cap **12**: controls `000`, `253`; subjects `030`, `280`; repeats of both
   subjects. Six measured attempts.
2. Cap **24**, only if a cap-12 subject still stops specifically at
   `CorrectionLimit`: the same four primaries and two subject repeats. Six more.
   A `NoNominal`, `NoClearing`, deadline or landing cannot justify this stage by
   itself: a larger correction cap would not change an earlier stop.

Maximum **12 measured attempts**, no tuning, retry, seed replacement or rescue
flight. Per-flight limit 120 s; collection allowance 900 s across both stages;
each offline build has its own 900 s bound. Input-only preflight, synthetic tests
and ordinary development checks are outside the measured-flight allowance.

Stop on collector/source/integrity/replay error, changed six-correction prefix,
control regression, unexpected physical stop or accepted evidence drift. Retain
partial evidence and do not expand the allowance. Repeat every recorded subject
within its stage; compare its complete same-source flight except the three wall
timings. Honest finite planning stops are recorded, not called landings.

## Comparison and review gates

- Authenticate and snapshot the original four scenario/flight inputs before
  collection. Bind contract, tool, workspace, probe source, executable and
  baseline digests. Review the exact source transformation before building.
- Both controls must preserve every non-timing flight field except the two
  declared root policy-bound identities (`policy`, `input_identity`). Validate
  the new policy explicitly and native full/compact/CLI input/result coherence
  before applying that comparison. No numeric tolerance or proof exclusion.
- Subject comparisons require exact first-six planning/query cycles, all
  executed segments through H6, all source-prefix actions/events/samples, the
  actual state at H6, and the original deadline. The next cycle's nominal build
  and audit must also match; only its cap decision/local search may differ.
  This proves the experiment extends the prior flight rather than changing its
  earlier maneuver selection. Input policy identity legitimately differs.
- Preserve final physical/mission outcomes and integrity/original-source replay
  together. Record total corrections, H positions, clock, fuel, remaining route
  and whether the larger cap is actually binding.
- Recheck source/default evaluator and accepted report/selector hashes after
  each stage. Retain rich native reports without publishing/navigation changes.
- Run pure collector/transform/comparison tests and the maintained development
  gate. Saved verification runs no flights, fresh replay, build or report writes.

## Decision boundary

If either subject lands, classify its old result as cap-truncated success in this
paired diagnostic; do not claim a general percentage improvement. If it stops
for another reason, identify that actual later stop before proposing progress
logic changes. If it still hits 24 with remaining budget, the diagnostic is
bounded but unresolved—not permission to remove all limits.

This pass does not promote a larger default cap or reopen the complete sweep.
A broader cap decision needs development-population evidence and an untouched
test population, separately authorized. No commit, push, report publication or
server operation is part of this pass.
