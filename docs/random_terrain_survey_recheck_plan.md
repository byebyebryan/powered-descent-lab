# Random terrain recheck — numerical fix and bounded sweep plan

[Documentation home](README.md) · [Stopped survey](random_terrain_survey_results.md) · [Original frozen contract](random_terrain_survey_plan.md)

Execution update, 2026-10-06: the user authorized this bounded pass. It stopped
at the first preservation control on the explicitly anticipated numerical
mismatch; no random flights were launched. See the separate
[recheck results and next decision](random_terrain_survey_recheck_results.md).
The approved protocol below remains recorded; it does not authorize a retry.

Subsequent authorization, 2026-10-06: the user requested the comparator fix,
a readiness review, then another goal loop for the unchanged sweep. The
[comparison-aware follow-up](#comparison-aware-follow-up-authorized-2026-10-06)
below is the new protocol, not a resumption or relabeling of either stopped root.

## Status and scope

The requested source geometry/initial-guard fix is implemented and developer-validated.
The follow-up sweep below was proposed before the execution update above;
authorization came from the user, not this document.
Keep the stopped capture, its receipt and its published report intact. The fix
has unit-level evidence; the affected procedural flight has not yet been rerun
to establish a new full physical/mission/integrity/replay result.

There is no terrain tuning, raised source pose, cutaway, solver redesign or
change to nominal/correction selection policy. Current vehicle, Earth gravity,
120/60 Hz clocks, mission horizon, command ordering and policy 3 stay unchanged.

## Implemented numerical fix

[`body_aabb_from_pose`](../pd-eval/src/planner_flight/geometry.rs) now rotates the
local feet/hull first and computes conservative symmetric extents from those
local coordinates. Translation is applied only for world-space pad containment.
The vehicle's size therefore does not change with its position. At the exact
stopped source height, the vertical extent is 5 m and raw clearance is zero,
rather than an invented -3.55e-15 m penetration.

Local extents alone do not eliminate the independent `(surface + offset) -
offset` roundoff at every fractional height. Four of the 100 saved source states
still have a microscopic negative result in that arithmetic, with minimum
-3.55e-15 m. This was an arithmetic-only inspection, not 100 new flight checks.

The existing 1e-9 m source-rest admission/flat-pad tolerance is now a shared
[`PAD_REST_TOLERANCE_M`](../pd-eval/src/planner_flight/input.rs).
[`FlightGuard`](../pd-eval/src/waypoint_v2/execution.rs) uses it only when:

- the global physics step is zero;
- the existing flat source-pad corridor makes required clearance zero; and
- position, velocity, attitude and angular rate exactly match the original,
  admitted source state.

The allowance also covers the before-first-command check at that same state.
It never applies to poststep states, later handoffs/pad transitions or positive
airborne reserves. The terrain query itself is not clamped, non-finite/domain
failures remain errors, and penetration beyond the admitted tolerance is rejected.
No input acceptance threshold was increased.

## Fix validation

Two regression tests reproduced the original failure before the change and
passed afterward. Eight focused geometry/guard tests cover:

- the exact stopped source height;
- translation-invariant rotated extents, including feet beyond the hull;
- zero, positive, negative, fractional and floating-point boundary pad heights;
- unchanged world-space foot bounds;
- genuine penetration, tilt contact, domain overrun and non-finite state;
- admission/guard agreement at the existing tolerance boundary; and
- no allowance in poststep checks or positive required-clearance comparisons.

The final maintained eleven-step developer gate passed: workspace/all-feature
tests (211 evaluator tests passed, four deliberately ignored), optional/default-off
CLI boundaries, help/dependency checks, formatting, strict Clippy, maintained
JavaScript tests and local documentation checks. All 36 terrain study/survey
tests passed. The old stopped capture still verifies read-only, including its
48 protected accepted benchmark files. These checks do not create or publish a
new procedural flight capture. A full failed-case replay and fresh random-terrain
coverage verdict remain follow-up work.

## Proposed execution plan

### 1. Finish readiness before measuring

Review the geometry/guard diff independently from the existing survey/report
changes. Keep the original machine-readable `survey_plan.json`, study/refinement
plans, recipe, seed list and prepared scenarios unchanged.

The render adapter currently accepts only the already-existing create-only
survey report root. Before the new source freeze, add a narrowly validated
capture-specific child destination below that development collection, with
path/symlink/create-only tests. This is a publication-path change, not a new
report template. Do not overwrite the stopped site's index/detail bodies or
the accepted planner site/selector. Report attempted, verified and unattempted
counts explicitly; a planned-cohort count is not a landing percentage.

Build the final release executable and run the maintained developer gate.
Optionally run the separately approved retained-capture numerical regression
before freezing. It is not silently included in this sweep's allowance.

### 2. Freeze a new source-bound root with identical inputs

Proposed create-only root:
`outputs/eval/planner_v2_random_terrain/capture-random-20261006-v2-pad-rest`.
Reprepare with the fixed executable using the same frozen master seed
2026100601 and original recipe. Authenticate the stopped capture first, then
require **byte-identical** 100 raw terrain files and all 103 prepared scenarios
against `capture-random-20261006-v1-reviewed` before launching any measured flight.

Repeat the 100 native input-only preflights and reversed-order generation check;
save new source/executable snapshots and receipts. Reject any seed, geometry,
pad, vehicle, clock, horizon or input-byte difference. Never overwrite or resume
the stopped root's create-only `run-start.json`.

### 3. Preservation controls, then the small checkpoint

Run the same three controls serially: `v2_clear_685`, `v2_ridge_early` and
`v2_plateau_reference_900`. Keep the existing complete non-timing comparator.
The geometry change may legitimately alter a recorded clearance by a few ulps;
any mismatch still stops for explicit review before random flights. Do not
broadly ignore geometry fields or substitute new control answers. Commands,
states, contacts, selections, handoffs, outcomes and proof flags remain essential.

Run the first ten original random identities with at most four workers. The
first wave already contains `random-001` (seed 1634476889), so it is the failed
case recheck **within the 100-case allowance**, not an extra canary run. Verify
each wave before launching another. At the checkpoint, require valid collection
and complete applicable proof evidence, not ten landings or a waypoint quota.

For `random-001`, a new verified landing requires the complete tuple and consumed
command replay. A different ordinary finite outcome remains data; do not label
the original numerical issue closed merely because initial rejection disappeared.
Any new implementation/proof failure stops collection for diagnosis.

### 4. Finish the unchanged sample, then repeat

If the controls/checkpoint pass, run the remaining 90 random cases, then the
five predeclared repeats of indices 0–4. Retain all ordinary physical misses,
finite planning stops and simulation timeouts. Do not reroll seeds, raise terrain
amplitude, add waypoint quotas or tune flight policy after inspecting outcomes.

The complete proposed allowance remains **108 measured attempts**: three
sentinels + 100 random cases + five repeats, with no extra failed-case attempt.
Use the existing four-worker maximum, 300 s process wall limit and 10800 s
campaign ceiling. Stop new launches for source/input drift, implementation errors
or applicable integrity/replay failures; drain in-flight workers and account for
every unattempted row. No automatic retry or second campaign.

### 5. Review and report separately

Validate the complete saved capture and exact repeat comparisons. Review counts
for nominal clear/blocked/not-established, zero/one/multiple corrections, planning
stops, physical/mission outcomes, integrity/replay and actual attempted coverage.
Compare the four previously attempted random inputs without rewriting their old
outcomes. Do not infer coverage of other terrain distributions or vehicle regimes.

Use the common batch/detail templates with actual handoff annotations at a new
create-only capture-specific report destination, for example
`/reports/eval/planner_v2_random_terrain/recheck-20261006-v2/`.
Connect it through normal navigation while retaining the stopped result as history.
Verify all report links/annotations and protected accepted files. Server changes,
commits and pushes need separate authorization.

## Decision boundary

The next requested action should be the bounded readiness-and-sweep pass above,
not another terrain research phase. A stopped integrity/proof pass remains a
stop, not permission to extend its budget. Only after a trustworthy sample is
collected should ordinary miss clusters determine whether any flight change or
broader terrain distribution is worth another pass.

## Comparison-aware follow-up (authorized 2026-10-06)

Execution result: this pass stopped at the corrected control after accepting
the direct control. See [complete comparison-aware results](random_terrain_survey_comparison_results.md).
All 100 random cases remain unattempted in its new capture; the source-bound
rule below was not widened after the stop.

Finish the comparator fix and readiness review before creating the sweep goal.
No planner/controller, terrain recipe, source-rest guard or runtime safety
threshold change is part of this follow-up.

The new tracked `sentinel_comparison.json` defines
`derived_clearance_preservation_v1`. It permits an absolute difference at most
**1e-12 m** only at these cross-source sentinel diagnostic paths:

```text
/cycles/*/audit/clearance_scan/minimum_airborne/clearance_m
/cycles/*/audit/clearance_scan/first_violation/clearance_m
```

Both come from the changed body-envelope geometry arithmetic. The bound is a
small numerical comparison allowance, not a physical collision reserve. Require
finite values, matching field inventory, cycle/measurement metadata, required
clearance and safety results. Reject crossings of zero (including touching
zero) or the required reserve. Keep every other non-timing field exact.
Record every exception's path, baseline, actual and signed delta; recompute and
authenticate the list in both Python saved verification and the Rust report
verifier. Use one tracked contract and shared conformance corpus for both.

Embed the rule in the new manifest, copy it into the input tools and bind its
hash to the frozen source. A missing rule means the historical exact contract;
do not retroactively reinterpret stopped captures. Same-source repeats use the
original exact rule, never the cross-source diagnostic allowance. Reject
unknown/modified contracts and malformed timing exclusions.

After focused tests, saved-capture checks and the full maintained developer gate,
build/freeze the release executable and prepare the new create-only root:
`outputs/eval/planner_v2_random_terrain/capture-random-20261006-v3-comparison`.
Require the same 100 raw files, all 103 scenario bytes, seed list, recipe and
frozen plan as the original reviewed capture. Preserve both earlier stopped
captures and their report bodies, and the accepted benchmark/selector.

The sweep retains the **108-attempt ceiling**: three serial controls, the first
ten random cases as a checkpoint, the remaining 90, then five predeclared
repeats. At most four workers, 300 s/process and 10800 s/campaign. `random-001`
is in the first random wave, not an extra canary. Stop new waves on unpermitted
comparison differences, source/input drift, implementation/proof/replay errors
or runner limits; drain launched workers and retain all unattempted rows.
Ordinary finite misses are data. No retries, tuning or extra campaigns.

Verify the saved capture and publish its separate common-template report at
`/reports/eval/planner_v2_random_terrain/recheck-20261006-v3/`, with actual counts,
real handoff annotations and audited comparator exceptions. Connect normal
navigation without replacing old bodies or promoting the accepted benchmark.
No server change, commit or push is included.
