# Waypoint V2 fresh terrain readiness results

Policy 3 passed the approved fresh-terrain check on 2026-10-03. Both runs land
all twelve missions: three uncut clear routes directly, and nine obstructed
routes after local corrections. The four historical regression missions also
retain their exact physical programs and outcomes.

Verdict: ready for bounded use through the existing offline lab CLI on the
tested vehicle, Earth gravity, 120/60 Hz setup and ordinary terrain range.
This is fresh coverage evidence, not an all-terrain guarantee or promotion to
the default planner. No planner/controller/core Rust or numerical policy changed.

## Gates and execution allowance

The [approved plan](waypoint_v2_fresh_terrain_readiness_plan.md) fixed inputs and
gates before simulation. Execution used exactly **28 primary mission attempts**:
four sentinels, twelve fresh missions, then the conditional identical twelve-case
repeat. All completed; no mission was replaced, tuned or restarted.

| Gate | First fresh run | Identical repeat | Required |
| --- | --- | --- | --- |
| Direct clear landings | 3/3, zero corrections | 3/3, zero corrections | 3/3 |
| Terrain landings | 9/9 | 9/9 | At least 7/9 |
| Ridge / plateau / compound landings | 3/3 each | 3/3 each | At least 2/3 each |
| Initial nominal actually blocked by terrain | 9/9 | 9/9 | At least 6/9 |
| Integrity and final-source replay | 12/12 | 12/12 | Every attempt |
| Recorded crash, fuel exhaustion or executed safety failure | None | None | None |
| Total planning median per mission | 0.373514 s | 0.376731 s | At most 2 s |
| Total planning nearest-rank p95 per mission | 0.796573 s | 0.805806 s | At most 5 s |

Each timing population contains all twelve attempts. With N=12, nearest-rank
p95 is the maximum. These are offline measurements on this host, not a 60 Hz
planning guarantee or a comparative speedup claim. All saved successful outcomes
are `landed` / `landed_on_target` / `success`; CLI exit zero alone is not used
as landing evidence.

The [repeat comparison](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/repeat-comparison.json)
passes for all twelve saved scenario/summary/full-flight sets: 36 compared
artifacts. Only the established timing and output-path fields are excluded.
Commands, state, fuel, clock, handoff boundaries and outcomes match exactly.

## Fresh missions and reports

These are physically new inputs, not renamed copies of the old 32-case suite.
They retain continuous uncut bases, unchanged source/target shelves, the tested
vehicle and route-free missions. Feature recipes and expanded-scenario hashes
are pinned in the [input fixture](../fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json).

All outcomes and correction counts below are identical in both runs. Links open
the first run's original rich report. Each landing has no failure reason.

| Group | Mission report | Outcome | Corrections | Failure reason |
| --- | --- | --- | --- | --- |
| Clear | [Flat, 805 m](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_clear_flat_805/report.html) | Landed | 0 | None |
| Clear | [Uphill, +60 m](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_clear_uphill_805/report.html) | Landed | 0 | None |
| Clear | [Downhill, -60 m](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_clear_downhill_805/report.html) | Landed | 0 | None |
| Ridge | [Early ridge](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_ridge_early_900/report.html) | Landed | 1 | None |
| Ridge | [Middle ridge](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_ridge_middle_900/report.html) | Landed | 1 | None |
| Ridge | [Late ridge](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_ridge_late_900/report.html) | Landed | 1 | None |
| Plateau | [Early plateau](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_plateau_early_900/report.html) | Landed | 2 | None |
| Plateau | [Broad plateau](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_plateau_broad_900/report.html) | Landed | 1 | None |
| Plateau | [Late plateau](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_plateau_late_900/report.html) | Landed | 1 | None |
| Compound | [Successive ridges](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_compound_successive_900/report.html) | Landed | 2 | None |
| Compound | [Uphill plus ridge](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_compound_uphill_ridge_805/report.html) | Landed | 1 | None |
| Compound | [Downhill plus plateau](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs/fresh_compound_downhill_plateau_805/report.html) | Landed | 1 | None |

For a short walkthrough, start with Flat or Uphill, then Early plateau and
Successive ridges. The first pair shows direct uncut flight; the latter pair
each requires two actual handoffs and subsequent replanning.

The new reports preserve the rich layout, two charts, five spatial modes,
telemetry, events and statistics. They do **not** add the selected preview's
dynamic waypoint annotations or new navigation. Exact handoff states are in
each `flight.json` and the [primary artifact review](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/primary-artifact-review.json).
For example, Early plateau hands off at ticks 2220 and 2952; Successive ridges
at 2206 and 3140. These are actual executed states, not authored route points.

## Preserved regression missions

These four missions are outside the fresh denominator. Their saved scenario,
summary and full-flight payloads match the accepted historical capture under
the existing exclusions, despite a different current binary/source build.

| Sentinel report | Outcome | Corrections | Exact program/outcome preserved |
| --- | --- | --- | --- |
| [Clear, 845 m](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/sentinels/runs/v2_clear_845/report.html) | Landed | 0 | Yes |
| [Late ridge](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/sentinels/runs/v2_ridge_late/report.html) | Landed | 1 | Yes |
| [Successive rising terrain](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/sentinels/runs/v2_successive_rising/report.html) | Landed | 2 | Yes |
| [Reference plateau, 900 m](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/sentinels/runs/v2_plateau_reference_900/report.html) | Landed | 3 | Yes |

## What this confirms about the intended logic

The three clear controls use one direct cycle and consume exactly the selected
nominal program. No cutaway or clearing waypoint is needed on flat, uphill or
downhill terrain in these inputs.

The [nominal invariance review](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/nominal-invariance-review.json)
also finds exact initial-command equality across all seven 900 m flat-base
obstruction cases, and between the matched clear/obstructed uphill and downhill
pairs. Adding an obstruction did not cause selection of a higher initial arc in
these matched groups. The fixed route is audited first; terrain correction is
separate. This observed equality is not a proof over arbitrary inputs.

The nine blocked terrain routes use one or two local corrections, then replan
from the actual handoff. Each selected correction has a replayed handoff and
240-tick neutral continuation certificate, not a precomputed landing suffix.
All 28 executed attempts preserve state, command and original-clock continuity
and the tick-9600 deadline. Across them, the 28 selected local handoffs retain
their 5 m reserve; the smallest recorded local-trajectory clearance is
6.163038 m. Source/contact exceptions remain phase-aware; this is not a claim
that whole-flight minimum clearance exceeds 5 m, nor a swept-geometry proof.

## Validation, provenance and preservation

Before primary missions, real CLI preflight accepts all twelve fixed inputs
with `simulation_created = false`. The new bounded harness has 15 passing fake
tests covering input seals, finite-stop denominators, hard-stop handling,
contradictory evidence, exact policy identity, odd terminal contact, create-only
outputs and repeat/provenance tampering. Its strict parser also accepts all 30
supported historical captures in read-only checks; those are not extra flights.

Native validation on the unchanged Rust source passes: release build, workspace
tests **957 passed / 7 ignored**, format check, and Clippy with the existing
`single_element_loop` allowance. All three explicit retained-report gates pass.
The original 32-case structural and fake checks pass without running its matrix.

Primary read-only review binds all 28 new rich reports to their native evidence:
11,016 sample records retain their clocks, positions, velocities, fuel and
clearance fields; all 28 visible key-event records agree. Both charts, all five
modes and detailed data remain present. No browser interaction or visual
acceptance check was performed in this pass.

The [execution seal](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/execution-seal.json)
records reviewed input order, build identity, the tested harness and the allowance
before the first primary mission. Runtime before/after hashes match it in all
three simulated batches:

| Identity | SHA-256 |
| --- | --- |
| CLI binary | `1e3e62ce7af7c3770648f24ed37d38fa7449455bf9e9ad6024d9775406d1dd10` |
| Runtime source tree | `15cffa3ef3a369d40d50e1eeb70a00661f58f93bd48881949d5d1369e7837992` |
| Executed readiness runner | `6bf70ff99fe5c332adbbc2793a9ecc315bed221172f28ed3ae4f344d539c3215` |
| Fresh fixture | `554b406f62387e9598d89e53a26ed705b2fce5380b4e7a216f92f3003950cd9c` |
| Expanded scenario set | `311752c8da362c2261ce069e633041407b6b59ff88c0c32381d21bd095128c52` |

During mission capture, the checkout was `main` at `12cc3f5` with the readiness
changes uncommitted.
Historical sentinel provenance is recorded separately; it is not represented as
the current build. Generated evidence is local and ignored by Git. The
[preservation receipt](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/preservation-after.json)
matches the pre-pass fingerprint for all eight monitored roots: 554 regular
files and two symlinks covering accepted A/B captures, published reports/homes,
navigation selection and the original fixture/runner. No site refresh, selection
change, server restart, commit or push occurred.

Native batch records: [sentinels](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/sentinels/suite-summary.json),
[first fresh run](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/suite-summary.json),
[identical repeat](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_b/suite-summary.json).

Validation commands include:

```sh
rtk proxy cargo build -p pd-eval --release
rtk proxy cargo test --workspace
rtk proxy cargo fmt --all --check
rtk proxy cargo clippy --workspace --all-targets -- -D warnings -A clippy::single_element_loop
rtk proxy node --test scripts/test_run_waypoint_v2_fresh_terrain_readiness.mjs
rtk proxy node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --check-only
rtk proxy node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --compare outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_b
```

## Review before commit

The requested review finds one runner-status defect, not a planner or flight
failure: a provenance mismatch after `--preflight-only` marked acceptance false
but left status `preflight_passed`, causing CLI exit zero. A fake-CLI regression
first reproduces that exit and then passes after the guard sets
`preflight_failed`. Drift now returns exit one without simulating a mission.

All **16 local tests pass**, including the archive-backed sentinel comparison.
A clean tracked-source snapshot without ignored captures passes **15 tests with
one explicit archive-dependent skip**; a present but invalid baseline still
fails. Syntax, input seals, whitespace and the retained 36-artifact repeat
comparison pass. Runtime source, binary, inputs, all eight preserved roots and
the 28-attempt count remain unchanged. No additional real preflight or mission
is run during this review. The earlier 957-test native validation remains bound
to the unchanged Rust source; it is not represented as a new review-time rerun.

The [commit review receipt](../outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/commit-review.json)
records the reviewed runner hash
`661d1254106d9f962cf6c14b22a2e007e620c6a23a08b61614bd72902a196e76`
separately from the original executed runner above. Captures and their execution
seal are not rewritten or relabeled as flights run by the revised harness.
The review does not change numerical behavior, trajectory selection or the
bounded-use verdict. Implementation/tests are commit `d325fde`; results
documentation is a separate commit. No push, site refresh, selection change or
server restart is included.

## Decision and next boundary

This pass found no ordinary miss to repair. Stop the open-ended planner research
loop and use explicit policy 3 for supported offline lab missions. The runtime
calculates from the supplied scenario state and actual later handoffs; this
fixture is a test corpus, not a lookup table of flight answers.

The existing invocation is:

```sh
target/release/pd-eval waypoint-v2-flight --policy-version 3 \
  --scenario SCENARIO.json --source-pad-id SOURCE --target-pad-id TARGET \
  --output-dir NEW_CAPTURE_DIRECTORY
```

Choose real pad IDs and a new output directory. Inspect saved typed outcomes,
not just exit status. A future finite `NoClearing` while physically airborne
means a stopped offline attempt, not safe autonomous continuation without a
controller. The four old hard diagnostics, two unsupported setups, arbitrary
powered/reverse starts, other vehicles/gravities and interactive/emergency
execution remain outside this acceptance.

After review and commit, choose a concrete use case for adoption. Controller
extraction, default promotion or interactive replanning should be separately
scoped; none is needed to claim this bounded offline result.
