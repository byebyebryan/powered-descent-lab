# Canonical initial direct transfer protocol

This evaluator-only pass tests the revised direct-first rule from source-pad
rest. Generate a canonical launch, ballistic coast, and powered terminal
transfer from endpoint and vehicle feasibility alone, then audit the fixed
proposal against actual unmodified terrain. Interior terrain must not alter
the nominal candidate family, rejection ledger, commands, or ranking. This
protocol and its input-selection manifest precede implementation and flights.

The pass stops before local obstacle clearing, repeated waypoints, production
planner integration, and real-time claims. Existing source-rest generators,
the accepted airborne canary, default behavior, and contact physics retain
their semantics. The separate terminal completion reserve remains design-only.

## Supported physical inputs

Use forward upright source-pad rest, zero angular rate, the existing vehicle,
flat in-domain source and target shelves, route-free target landing, unchanged
120 Hz physics and held 60 Hz commands, and the original absolute mission
budget. Invalid or unsupported inputs execute no flight. Endpoint shelf
validation is permitted; interior terrain is not a nominal selection input.

The selection manifest is
`fixtures/research/canonical_initial_direct_canary_inputs_v1.json`. Development
uses the four exposed 685 and 845 m flat and 845 m uphill and downhill 75 m
controls from the operational manifest. Stop on the first decisive failure
of nominal generation, source departure, actual terrain audit, or replay.
Record downstream cases and gates as not evaluated rather than substituting
another policy or tuning a case.

Only after development passes, use the four already-sealed physical endpoint
inputs at 735 and 915 m flat and 915 m uphill and downhill 75 m from the
completion-input manifest. Reuse physical scenarios only. Their earlier
design exposure is explicit: these are additional endpoint coverage, not
newly hidden missions, and no reserve or completion-hold semantics apply.

## Fixed nominal family

Policy `canonical_initial_direct_lowest_peak_v1` uses four virtual ballistic
duration seeds with multipliers 0.75, 1, 1.25, and 1.5 of
`sqrt(2 * horizontal_span / gravity)`, rounded at the original physics clock.
Release and target references use endpoint pad heights and existing body and
endpoint clearances, never the interior terrain profile. Source handoffs lie
on the ascending arc; terminal handoffs lie on its descending part. The
existing 0.25 s handoff grid, 0.5 s primitive bridge-duration grid, original
budget, derating, and fixed numerical tolerances remain bounded.

For each seed, choose a nominal source and terminal bridge pair using only
primitive thrust, throttle, endpoint, attitude, slew, fuel, and time checks.
Use deterministic feasibility, normalized nominal margin, fuel, elapsed time,
and handoff/bridge tuple ordering. No terrain clearance metric participates.
These are nominal seeds, not terrain certificates or production route proofs.

For each basis, enumerate source-duration offsets -240, -180, -120, -60, and
0 ticks and terminal-duration offsets 0, 60, 120, 180, 240, 300, and 360 ticks.
Retain all 140 outer rows, including unavailable bases and rejected durations.
The fixed 60-tick upright and 12-tick tilt launch is integrated from the real
source state. Re-solve and fit the source bridge from its actual launch state,
retaining the existing paired-mean seed, finite four-parameter correction,
six-iteration and eight-line-search budgets, 0.25 acceleration correction cap,
and 1e-6 m and m/s source-handoff tolerances. Nominal stepping may observe core
terrain contact but never uses interior contact or terrain-derived extrema
to accept, reject, or rank a proposal. Endpoint plane checks are local to the
source and target pads, not a synthetic floor spanning the route.

Use the unchanged body-aware quadratic terminal reference and held-pair
throttle inversion. Require actual nominal launch/source ascent, a positive
ballistic coast containing the apex, descending terminal entry from above,
at most one flight apex, positive remaining fuel, safe intended-pad first
contact, original time budget, and exact update coverage. No saved suffix,
historical command seed, restored serialized state, impulse, floor cutaway,
missing-command padding, or diagnostic final-command hold is permitted.

Rank complete nominal proposals by lowest complete-flight peak center of mass
height, then planned contact tick, then stable proposal identity. This is a
minimum within the declared finite family, not global optimality. Audit only
the selected proposal: terrain rejection never selects a taller alternative.

## Actual terrain and evidence

Run ordinary and neutral replay under the same complete selected commands
from original source rest. Retain exact incoming contact before touchdown
normalization and the complete final state. Require parity, safe first target
contact, original global time and fuel continuity, exact action coverage, and
the existing rotated-body clearance policy with source and descending target
pad transition exceptions. Audit the actual terrain without modifying it.
Safety is discrete 120 Hz nominal body/contact evidence, not swept collision
or perturbation robustness.

For each accepted development case, insert a blocking triangular obstruction
ahead of source and before the target shelf, higher than the complete nominal
peak, and a low nonblocking interior change using a deterministic declared
construction. Keep source and target shelves and all other inputs fixed. The
complete nominal search and selected commands must be identical on both
twins. The blocking twin must report an actual clearance/contact conflict;
the nonblocking twin must retain safe landing. An unrelated binding or replay
error is not a terrain-blocked result. Retain first-conflict tick, phase,
position, and clearance evidence for later local-clearing design.

Separately evaluate the exposed 900 m late/broad obstacle and its flat twin.
Require the new nominal choice to remain identical and be terrain-blocked,
and independently regenerate the old terrain-aware control to demonstrate
that a higher direct arc remains possible. Old-policy diagnostics cannot seed
or replace the canonical choice. If this discriminator does not hold, report
the incompatibility without changing obstacle geometry or policy after results.

From the new accepted development initial flights, capture real ascending,
near-apex, and descending coast states using the airborne canary's aligned
capture rules. Stop before consuming the old command at each capture, retain
the live state, regenerate the suffix with the unchanged airborne family,
and compare complete whole-source stitched replay against the full final
state and full incoming contact plus standard actions, events, and samples.
Capture or finite regeneration failure is a compatibility failure, not terrain
blockage. Planning pauses the offline simulator; record wall time separately.

## Closure and stopping

Outputs are create-only. Retain the complete ledger and honest partial evidence
on failure. Bind source, protocol, physical manifests, and selection manifest
with hashes and verify that closure does not change during measurements.
Observational wall times and output paths are excluded from deterministic
identity; physical evidence, failure causes, commands, and source/input hashes
remain bound. Repeat the final retained gate on unchanged source and compare
all deterministic evidence.

Preserve the 24-control old source-rest regression and the existing twelve-state
airborne canary without changing their policies. Preservation does not require
the new canonical policy to accept old obstacle-aware selections. Run focused
tests, complete workspace tests, strict all-target Clippy, formatting, and
whitespace checks on the integrated tree.

The successful exit establishes canonical initial Direct versus demonstrated
TerrainBlocked decisions and compatibility with actual-state regeneration.
The bounded failure exit establishes the first precise compatibility failure,
unchanged accepted controls, and an evidence-backed next implementation slice.
Neither exit implements a local clearing maneuver, promotes defaults, changes
contact thresholds, claims arbitrary powered handoff coverage or real-time
latency safety, or authorizes commit, push, or deployment.
