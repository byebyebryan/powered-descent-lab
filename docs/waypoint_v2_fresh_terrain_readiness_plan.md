# Waypoint V2 fresh terrain readiness plan

Status: Executed after user approval on 2026-10-03. The declared pass completed
28/28 allowed primary attempts; both fresh runs passed. See the
[readiness results](waypoint_v2_fresh_terrain_readiness_results.md). The contract
below preserves the predeclared recipes and gates; it does not promote a default.

The next pass should test the accepted policy 3 on a small, predeclared set of
new ordinary terrains. Keep the planner frozen and use its existing lab CLI.
The decision is whether the current implementation is useful beyond its exposed
suite, not whether it can recover from every possible airborne state.

## What the current evidence supports

The [accepted integration](waypoint_v2_airborne_integration_acceptance.md) lands
8/8 uncut clear controls and 16/16 ordinary terrain missions. The reference
plateau lands after three corrections. Those are historical, frozen-suite
results, not a measurement on the inputs proposed here.

The supported outer loop already follows the intended division of work:
construct an executable terrain-blind nominal transfer, audit that fixed
program against terrain, clear a blocking feature locally when necessary,
execute to the actual handoff, then plan again from that state. A clearing
waypoint does not require a complete landing suffix. Policy 3 replaces airborne
nominal generation; the accepted canonical ground launch remains unchanged.
This is not a guarantee of a globally lowest mathematical arc.

Four difficult diagnostics still stop `NoClearing`; two different setups reject
as unsupported. These remain disclosed limits. A finite planning stop while
physically flying is a stopped offline attempt, not a crash and not proof that
the vehicle could safely continue without a controller.

## Why this pass is smaller than an adoption refactor

`pd-eval waypoint-v2-flight` already accepts ordinary `ScenarioSpec` JSON plus
source and target pad IDs. Explicit `--policy-version 3` runs the complete loop
and writes scenario, full flight, summary and the original rich detailed report.
It plans from the input state and subsequent actual handoffs; the runtime does
not select a stored answer from the research corpus.

The immediate lab gap is therefore fresh coverage, not a missing runnable
interface. Moving the loop into `pd-control`, adding a `pd-cli` controller mode
or defining an emergency controller would introduce separate dependency and
lifecycle decisions. Defer those until there is a concrete adoption use case.
The V2 capture is also not the standard `pd-cli` action-log bundle; do not imply
that standard replay commands consume it unchanged.

Use existing typed outcomes. In particular, CLI exit code zero means the
attempt completed without an input or integrity error; it does not imply a
landing. `NoClearing` and other finite coverage misses can return zero. A
successful mission must have `planning_stop = landed`,
`physical_outcome = landed_on_target`, `mission_outcome = success`, and passing
integrity and final-source replay in the saved evidence.

## Scope and frozen behavior

Keep the currently tested vehicle, Earth gravity, 120 Hz physics and 60 Hz
commands. Initial missions start upright and resting on the source pad, use
forward route-free pad landing, and contain continuous uncut heightfields.
Later replanning uses the existing supported airborne handoff family.

Freeze policy 3, nominal and terminal parameters, three-trial airborne cap,
local candidate generation and ranking, six-correction cap, 5 m reserve,
continuation certificate and original deadline. Keep default policy 1 and
policies 1 and 2 unchanged. Do not add terrain-driven higher nominal retries,
reset state at handoffs or tune the policy after seeing this set.

Exclude launch unification, reverse or arbitrary powered starts, new vehicles
or gravities, perturbation sweeps, swept-geometry work, extreme diagnostic
rescue, policy 4 and default promotion. Report navigation and future waypoint
annotations are separate work; do not reopen their design in this pass.

## Proposed fresh inputs

Use twelve fixed cases: three clear controls and nine terrain cases in three
groups. The following recipes are proposed now, before flight outcomes. Exact
expanded scenarios must be reviewed and sealed before execution.

Fractions below are measured from source to target. Feature heights are metres
above the interpolated uncut base. Three vertices form a ridge; four form a
plateau. For height H, their elevation offsets are respectively 0, H, 0 and
0, H, H, 0, returning to the base at each feature edge. Source and target
shelves and pad widths stay intact.

| Case | Span and target elevation change | Added feature fractions and height |
| --- | --- | --- |
| Clear flat | 805 m, 0 m | None |
| Clear uphill | 805 m, +60 m | None |
| Clear downhill | 805 m, -60 m | None |
| Ridge early | 900 m, 0 m | 0.28, 0.38, 0.48 at 270 m |
| Ridge middle | 900 m, 0 m | 0.42, 0.54, 0.66 at 300 m |
| Ridge late | 900 m, 0 m | 0.58, 0.69, 0.82 at 295 m |
| Plateau early | 900 m, 0 m | 0.28, 0.33, 0.49, 0.54 at 270 m |
| Plateau broad | 900 m, 0 m | 0.37, 0.43, 0.69, 0.75 at 300 m |
| Plateau late | 900 m, 0 m | 0.61, 0.66, 0.78, 0.83 at 295 m |
| Compound successive | 900 m, 0 m | 0.24, 0.34, 0.44 at 265 m; 0.57, 0.69, 0.81 at 305 m |
| Compound uphill ridge | 805 m, +60 m | 0.39, 0.51, 0.63 at 300 m |
| Compound downhill plateau | 805 m, -60 m | 0.43, 0.49, 0.67, 0.73 at 300 m |

The compound group deliberately samples three combinations rather than claiming
three separately validated families. Heights, positions and spans interpolate
the earlier ordinary range; this is a local generalization check, not broad
procedural-terrain certification. Every case changes physical geometry, not
just its ID.

For the new clear bases, copy the tested vehicle, clocks, mission settings and
pad widths from the tracked operational input manifest. Set the source centre
to -805 m and target centre to 0 m; retain the 36 m flat pad shelves and 160 m
outer domain margins. Connect the shelves continuously, with target surface
0, +60 or -60 m. Keep the source initial state centred on its shelf with the
existing 5 m gear offset. Do not shorten the source shelf by scaling the
vehicle or pad width.

For 900 m flat bases, use the tracked discriminator flat scenario. Insert only
the stated feature vertices above the unchanged base. Preserve vehicle, fuel,
seed, clocks, horizon, pad geometry and mission settings. The relevant tracked
sources are [operational inputs](../fixtures/research/nominal_direct_operational_fresh_inputs_v1.json)
and [discriminator inputs](../fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json).
Do not depend on an untracked capture to invent new inputs.

Freeze expanded inputs, recipe order and hashes before any mission simulation.
Read-only preflight must accept all twelve. Geometry review must confirm unique,
ordered in-domain vertices, unchanged shelves, no authored route and no floor
cutaway. Fix invalid assembly before the seal; once flights begin, do not
replace inputs, relabel ordinary misses as diagnostics or adjust features to
obtain a desired correction count.

## Preservation checks

Run four existing policy 3 cases as regression sentinels, outside the fresh
denominator:

| Existing case | Historical behavior to preserve |
| --- | --- |
| `v2_clear_845` | Landing, zero corrections |
| `v2_ridge_late` | Landing, one correction |
| `v2_successive_rising` | Landing, two corrections |
| `v2_plateau_reference_900` | Landing, three corrections |

Compare scenario content and summary/full-flight payloads to the accepted
capture, excluding only the established timing and output-path fields.
Commands, state, fuel, clocks, handoff boundaries and outcomes are not exclusions.
Record current and historical source provenance separately; report work changed
the checkout, so identical historical source hashes are not expected.

Fresh terrain may need zero, one or several corrections. Do not demand a
particular count merely because a recipe contains two features. The preserved
reference checks already establish repeated replanning mechanics.

## Proposed acceptance gates

These are recommended game-oriented readiness gates, not previously measured
results or a statistical claim about all terrains.

- All three fresh clear controls land directly with zero corrections. Their
  fixed audit matches the nominal commands, and consumed commands equal the
  selected direct program.
- At least seven of nine fresh terrain cases land, with at least two of three
  in each ridge, plateau and compound group. Every ordinary miss stays in the
  denominator and receives its typed reason.
- At least six of nine terrain cases actually block the initial selected
  nominal program. If fewer do, report insufficient obstruction coverage;
  do not reshape the set after viewing results.
- Every supported complete or partial attempt passes integrity, final-source
  replay and the existing phase-aware safety checks. Every selected local
  handoff and continuation certificate retains its source replay proof.
  Actual state, command coverage and original-clock continuity must agree.
- No attempt crashes, exhausts fuel, records an implementation error or violates
  an executed safety check. Those are hard failures, not part of the two-miss
  allowance. Finite planning stops with intact partial-flight evidence can be
  acceptable misses; they are not an emergency-flight guarantee.
- All four sentinels preserve their accepted physical programs and outcomes.
- Total planning time per fresh mission has median at most 2 s and nearest-rank
  p95 at most 5 s, including finite misses. With twelve inputs, nearest-rank p95
  is the maximum observation. This is an offline lab budget, not a 60 Hz planner
  requirement or comparative speedup claim.
- If repeated, both fresh runs pass and all twelve summary/full-flight pairs
  match under the existing exclusions, with identical bound runtime source,
  runner, binary and input hashes.

## Execution stages and allowance

1. Review this plan before implementation. Confirm that the recipes, scope and
   readiness floor answer the usefulness question without expanding the planner.
2. Implement the small input fixture and CLI harness. The existing 32-case runner
   has fixed counts and family gates; leave its contract and fixture unchanged.
   Use a thin new readiness runner, not a general matrix framework. Reuse pure
   utilities only where that does not fork numerical behavior or change the old
   runner's outputs.
3. Validate without primary mission runs: geometry and preflight, fake-result
   tests, stop/denominator handling, strict evidence parsing, create-only output,
   repeat comparison and syntax. In particular, a zero-exit `NoClearing` must not
   count as a landing; contradictory summary/flight evidence must fail. Build and
   run the relevant existing native tests, and record their actual scope.
4. Freeze the tested source, binary, runner and input digests. Run the four
   sentinels once. Stop on a preservation failure before the fresh batch.
5. Run the twelve fresh missions once. Finite misses continue through the fixed
   batch; stop early for an integrity, physical-safety or implementation failure
   and mark the batch incomplete. Keep completed evidence without restarting it.
6. Only if the first fresh batch passes every gate, repeat the same twelve once
   on the same frozen build. No new inputs, tuning or diagnostic flights.
7. Review all finite misses, the full denominator, actual handoffs and selected
   rich reports. Record one compact result and the resulting adoption decision.

The proposed allowance is at most **28 primary CLI mission attempts**: four
sentinels, twelve fresh cases and one conditional twelve-case repeat. Do not
silently add a development matrix, rerun the old 32-case suite, call a research
flight gate outside that allowance or restart a failed batch. Unit/fake tests
are separate validation and must not conceal extra primary mission runs.
Any need for additional mission attempts or runtime changes returns for review.

## Outputs and ownership

Expected implementation changes are one fresh-input manifest under
`fixtures/research/`, one bounded runner/check under `scripts/`, and a compact
result document plus progress link. No planner, controller or core Rust changes
are expected. If the CLI or runtime cannot satisfy this contract, stop and explain
the concrete gap rather than broadening the implementation.

Each attempt uses a new output directory. Preserve the accepted captures,
published report selection and historical report bodies. New supported attempts
keep the existing rich report writer, without claiming dynamic annotations that
the future-capture writer does not yet produce.

The result document should group cases by the four fresh groups, show the
recorded outcome, correction count and reason, and link the generated rich
reports by friendly case name. Keep sentinel results separate. Native result
JSON remains authoritative; an extra dashboard, collection renderer or automatic
publication is not required. Do not switch the LAN site's selected capture,
refresh the site or restart its server as an implicit validation step.

## Decision after the pass

If both runs pass, call policy 3 ready for bounded use through the existing
offline lab CLI on this tested setup and ordinary terrain range. Document the
invocation and finite-stop behavior, then choose any controller or interactive
adoption work separately. Do not claim all-terrain robustness or default status.

If coverage fails but integrity is sound, classify the actual ordinary misses
and decide whether one common limitation justifies a targeted follow-up. Do not
automatically reopen the four old hard diagnostics or every deferred feature.
If integrity or safety fails, that defect takes precedence over coverage.

Stop after the declared decision. This pass is meant to answer whether further
planner work is needed, not generate another open-ended research loop.

## Evidence used for this plan

The analysis reads the current
[flight command](../pd-eval/src/main.rs),
[V2 loop](../pd-eval/src/waypoint_v2.rs),
[capture writer](../pd-eval/src/waypoint_v2_output.rs),
[input preflight](../pd-eval/src/nominal_direct_flight.rs),
[existing suite runner](../scripts/run_waypoint_v2_practical_suite.mjs),
[suite recipes](../fixtures/research/waypoint_v2_practical_suite_plan_v1.json)
and [accepted results](waypoint_v2_airborne_integration_results.md).
This is repository-based analysis and a proposed execution contract, not a new
simulation, benchmark, browser validation or independent acceptance review.
