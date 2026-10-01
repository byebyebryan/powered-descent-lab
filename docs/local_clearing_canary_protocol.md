# Local clearing canary protocol

This sealed experiment tests one trajectory-derived clearing waypoint on the
previously exposed 900 m late broad obstruction, followed by fresh nominal
replanning from its actual state. Local acceptance never depends on a later
landing. A precise finite-search or composition failure is a valid research
result. This is opt-in evaluator work, not a production planner change.

## Inputs and policy

The selection manifest pins the existing obstacle-discrimination input file and
selects exactly `fresh_flat_control_span_900` and `fresh_late_broad_span_900`.
Use their complete requests, unchanged vehicle, 120 Hz physics, 60 Hz updates,
route-free LandingOnPad goal, 5 m body reserve and original 80 s absolute
planning deadline. The obstacle remains its existing 315 m plateau. Both inputs
were already exposed; no held-out or general obstacle coverage is claimed.

Generate the canonical terrain-blind direct search independently on both inputs.
Require complete search equality, safe flat contact with ordinary replay, and
an actual geometric terrain conflict on the obstruction with audit parity and
supplied-command agreement. Record its first conflict, never execute from it.

For canonical source handoff S, query actual obstructed prefixes at S+2 and
72+2*floor(f*(S-72)/2), for f=0.75, 0.5 and 0.25. Each independent ordinary
prefix stops before its pending command. Retain real in-memory SimulationState;
snapshots are evidence only. Unsupported or unsafe entries remain exclusions.
The local entry requires full 5 m clearance and may have powered held throttle;
the existing airborne direct admission predicate remains unchanged.

At each of four entries enumerate target attitudes -30, 0, +30 degrees, thrust
acceleration factors 0.75 and 1, and powered durations 60, 120, 240, 360, 480,
720, 960 physics ticks: exactly 42 rows per entry, at most 168 physical rows.
Stable order is attitude, factor, duration. Use the existing worst-mass cap
thrust_derate*(1-declared_robustness_margin)*max_thrust/(dry_mass+max_fuel).
Invert acceleration using the existing held-pair throttle conversion with actual
mass; propagate actual fuel and rate-limited attitude. Retain conversion and
physical failures without clamping an infeasible desired acceleration.

After powered flight explicitly command throttle zero and target attitude zero
at every update. Examine 360 boundaries H=E+B+2*k, k=1..360, with H+240 no later
than the original deadline. One complete idle hold precedes the first boundary.
Propagate one trace through at most 720+240 coast ticks and reuse its windows.
Stop at first contact, nonfinite state, domain/reserve failure or original
deadline. A later failure does not invalidate an earlier completed certificate.
Retain every row, each boundary decision and trace stop cause; rows excluded by
entry admission are not counted as physically propagated rows.

## Local acceptance and selection

Every actual local state must be contact-free, within the strict terrain domain
and above the unchanged conservative rotated-body 5 m reserve. At H require the
unchanged airborne direct admission predicate, without requiring a generated
proposal. Require x_H >= max(x_E,x_first_conflict)+D_body, where D_body is twice
the largest COM-to-hull-corner or COM-to-foot distance. This is progress, not the
terrain feature's far edge; a handoff above a continuing plateau is legal.

An explicit separate 240-tick idle/upright continuation must maintain full
reserve, positive fuel, forward motion and all unchanged airborne admission
conditions at every aligned boundary, including its endpoint. It is a finite
nominal certificate, not eventual landing, robust viability or compute-time
proof. Offline planning pauses simulation. Do not consume this branch and then
replan from a stale H.

Choose each row's earliest eligible H, then select globally by latest eligible
entry, earliest H, least actual fuel burn and stable row identity. Subsequent
nominal search, terrain audit and landing cannot change this selection. Do not
embed analytical examples or old winning rows as expected answers.

## Execution and evidence

Bind the policy, full actual context including terrain, full entry snapshot,
original deadline, goal, row, all commands and query trajectory into a separate
local proposal. It is not a FlightProgramV1 and has no fabricated contact or
checkpoint goal. Validate clocks, coverage and bindings before execution.

Ordinary live execution continues the retained entry. Independently execute and
replay from the original source using the existing bounded APIs. Supply canonical
commands strictly before E and local commands E through H-2. Coverage ends at H;
hard deadline remains the original deadline. The separate continuation proof
adds H through H+240-2. Both must stop CoverageExhausted, Flying/InProgress/Running,
with no contact. Compare complete final snapshots, IncomingContact options,
actions, events and existing-cadence samples across live, source bounded run and
official bounded replay. Keep absolute controller ordinals and no endpoint
command. A flying endpoint need not be a sampled tick.

The source guard retains the existing source-pad launch exception only for the
canonical prefix. Local entry and all subsequent local flight require full
reserve. Post-step phase uses the last consumed update strictly before its tick.
Do not modify core physics, contact classification, legacy replay or defaults.

At the selected actual H invoke the unchanged airborne nominal generator without
a baseline suffix or clearing reference. Audit its single selected proposal on
actual terrain. Direct and parity-backed actual TerrainBlocked pass composition;
Unknown or unsupported stops at a composition coverage gap while preserving
successful local proof. Invalid binding, missing commands or unrelated audit
errors are not TerrainBlocked. Do not rerank or generate a second waypoint.

For Direct, additionally execute and replay landing using the real combined
canonical prefix before E, clearing E before H and newly generated suffix from H.
Compare complete final snapshot and incoming contact with audit and official
replay. Never extract the obsolete canonical prefix through H.

## Gates and closure

Gates are baseline_and_entries, local_generation, local_physical_proof,
handoff_compatibility and closure_and_preservation. First decisive failure stops
downstream physical work; later gates are not_evaluated. Ordinary rejected rows
do not stop the finite search. Preserve all partial evidence and precise causes.

Complete the integrated source before the first measured run and bind a recursive
source closure plus all protocol/input seals before and after each run. Repeat
in two create-only roots on unchanged source. Compare complete deterministic
payloads; only named observational timing fields and output-root paths may differ.
Do not ignore full states, contacts, commands, searches or physical decisions.

The new experiment has no timing exclusions. New envelope exclusions are its
`elapsed_wall_time_us`, preservation-stage `elapsed_wall_time_us`, and raw
observed file hashes whose bytes include the declared timings. Reread the actual
preserved files when comparing repeats; recorded hashes alone are insufficient.
Require full source envelopes and deterministic identities to agree between the
two final-source runs.

Legacy canonical timing paths are case and twin `elapsed_wall_time_us`, plus
capture `search_wall_time_us`, `audit_wall_time_us` and
`stitched_replay_wall_time_us`. Its discriminator's flat control, late broad
case and old terrain-aware control also exclude `elapsed_wall_time_us`; the
old control's `compute` excludes the same five nominal-direct compute timing
fields named below. Legacy airborne paths are its five named
`baseline_compute` fields and capture `search_wall_time_us` and
`nominal_audit_wall_time_us`. Legacy source-rest paths are the three root compute
fields, case `case_wall_time_us`, and its five named generation, verification,
execution, action-replay and artifact-writing compute fields. In performance
files exclude those same five fields, `wall_time_us` and `thread_cpu_time_us`;
in controller-update records exclude only `compute_time_us`, never the frames.

For legacy HTML, compare its static shell and complete embedded `reportData`,
excluding only `runPerformance.wallTimeMs`, `threadCpuTimeMs`,
`cpuTimePerTickUs`, `simRateX`, `physicsStepsPerS`, sample `computeTimeMs`, and
`botStats.totalComputeMs`, `meanComputeMs`, `p95ComputeMs`, `maxComputeMs` and
`controlDutyCyclePct`, and optional `plannerCompute.wallTimeUs` when present.
Keep control cadence and every physical field unchanged.
No additional timing-shaped field is automatically exempt.

Historical comparisons may exclude the old source-binding envelopes, their
per-case before/after binding hashes and derived outer evidence identities when
the source closure expands. They may not exclude proposal/search identities or
physical payloads. That historical exception does not apply between final runs.
Record the exact applied exclusion paths with every historical comparison.

Rerun and compare eight accepted canonical controls, twelve prior airborne
continuations and twenty-four source-rest controls against accepted archives.
Expanded source envelopes may differ; deterministic physical payloads may not.
Retain original evidence unchanged. Run focused negatives, workspace tests,
strict all-target Clippy, formatting and whitespace checks. Structural negative
tests are not additional physical obstacle coverage.

No measured-result retuning, geometry change, relaxed margin, additional family,
repeated waypoint, default promotion, commit, push or deployment is authorized.
Report finite exhaustion as LocalUnknown, not physical impossibility. If local
clearing succeeds but regeneration does not, the next decision concerns that
actual handoff coverage rather than reopening launch or landing physics.
