# Harder procedural-terrain coverage pass

Date: 2026-10-07. The owner requested a goal loop after observing that the
[completed 100-case sweep](random_terrain_full_sweep_results.md) is a sanity pack:
all 100 initial nominal paths were clear. Keep that population unchanged.

## Design and bounded execution

Use the existing hash-bound Pylander composition, weighted strength 0.65 and
ridge slice 67.37. Change only global physical scales, not the planner or seeds
after observing their landing outcomes. Four recipes start with vertical scales
4 and 8 times the sanity composition; the 8-times broad and successive variants
also scale horizontal dimensions by 1.5 and 0.6. Amplitudes, noise frequencies,
warp distances and feature-cell widths transform consistently. Do not add extra
octaves, hand-place obstacles, normalize individual profiles, flatten flight
corridors or raise the nominal trajectory in response to terrain.

1. Freeze a 24-case calibration population (six per recipe), excluding every
   sanity/shape-study seed, before generating profiles. Run three original controls
   then all 24 flights: 27 measured attempts, at most four concurrent processes.
2. Assess initial nominal blockage and no-nominal incidence, not just landing
   rate. Any global recipe revision must precede held-out generation and retain
   the entire calibration attempt; never select/discard individual seeds for success.
3. Freeze a separate 100-case held-out population, 25 per recipe, excluding
   sanity and calibration seeds. Run three controls and five declared repeats
   at indices 0, 25, 50, 75 and 1: 108 measured attempts per source-frozen capture.
4. Retain every clear path, blocked path, no-nominal stop, finite failure and
   physical miss. Continue after ordinary flight failures. Repair routine harness
   or implementation faults and retry in a new create-only source-frozen capture.
   Stop the goal finally only for unrecoverable safety/nonfinite/determinism or
   evidence failure, or an impasse requiring new authority.

Both phases use the current vehicle, Earth gravity, 120/60 Hz, 90 s mission,
1200 m pad spacing, original-height 36 m local shelves and 24 m transitions.
The authoritative surface remains the resolved 4 m polyline; this is not a
continuous-noise collision claim. Per-case time is bounded at 300 s and each
capture at 10800 s. Controls/repeats/calibration are not held-out landing trials.

## Integrity and reporting

Reuse the existing collector, projections, strict runtime guards, content-hash
comparison V2 and common rich batch/detail templates. Extend the collector only
for source-bound challenge plans, recipe metadata and explicit repeat indices;
historical absent/V1 comparison semantics and the original survey remain intact.
Require the complete mission/physical/integrity/final-source replay tuple before
counting a landing. Preserve all accepted selector/report bodies and prior sites.

Report overall and per-recipe outcomes, initial clear/blocked/no-nominal counts,
landings conditional on actual blocked paths, completed correction counts and
failure reasons. Show actual handoffs, including multi-handoff flights, in the
existing rich reports. A finite policy rejection is not proof of physical
impossibility; a source proof pass is not a landing.

Publish a separate challenge report in ordinary Waypoint planning navigation,
with calibration clearly distinct from held-out results. No accepted-benchmark
promotion, commit, push or server change is authorized. Run Python study tests,
the maintained development gate, historical saved verification, final-source
freeze checks, protected-file hashes and report payload/link checks.
