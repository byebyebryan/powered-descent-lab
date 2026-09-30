# Nominal airborne direct regeneration canary protocol V1

## Status and scope

This protocol freezes a bounded evaluator-only canary before measurement. It
does not change `FlightProgramV1`, planner defaults, controller behavior, or
ordinary mission execution. The canary tests whether a newly generated coast
plus powered-terminal segment can be proposed from real, retained airborne
state and then executed with the original global clock, fuel, contact handling,
and whole-flight action replay.

The search family is deliberately narrower than the existing source-to-target
ballistic-direct generator. It tests airborne regeneration plumbing and
finite proposal coverage only. Its powered terminal is not a canonical ideal-
ballistic planner, obstacle-clearing policy, powered redirect, or proof that
the original ballistic-direct planner can regenerate from arbitrary waypoint
handoffs. Terrain waypoints and replanning at later handoffs remain future work;
this canary neither requires nor proves a waypoint landing/checkpoint.

Every result is opt-in and bounded to the four fresh physical controls loaded
by `load_nominal_direct_operational_fresh_inputs`. Historical outputs and
saved schedules are not generation inputs. A finite `Unknown`, unsupported
incoming state, unsafe proposal, replay mismatch, changed terrain-twin search,
or failure of the terrain twin to reject is retained as evidence and
contributes to a failed compatibility verdict. The expected independent
rejection on the obstructed twin is a passing discrimination check. No result
is discarded, replaced, or retuned. Such a result does not establish physical
impossibility or waypoint necessity.

## Frozen input and baseline construction

Use only the four exposed controls in the sealed fresh-input manifest:

`fixtures/research/nominal_direct_operational_fresh_inputs_v1.json`

The loader verifies its sealed SHA-256, schema, order, pads, profiles, and
default generation/terminal policies. Preserve the input scenario exactly.
Regenerate each baseline at runtime with
`evaluate_nominal_direct_flight(request, BodyAwareTerminalPolicyV1::default())`.
Do not load an archived generation, selected row, program, witness, schedule,
or expected outcome. Only a fresh `Direct` decision supplies a baseline
program; all other typed decisions remain recorded compatibility outcomes.

For every fresh `Direct` decision, independently execute the selected source-
rest program through `execute_nominal_direct_flight_program`, preserving its
full safety, ordinary execution, exact command/clock, contact/fuel, and
action-replay evidence. Also retain the ordinary baseline run artifacts as a
separate source-rest reference. A passing prefix capture does not substitute
for this complete baseline check.

Create each baseline live prefix from a fresh `SimulationState::new(context)`.
For every baseline update before the selected capture boundary, call
`set_command` at its absolute pre-step physics tick and advance with
`step_with_contact_report` at every physics tick. Stop before consuming the
command scheduled at the capture tick. Retain the full in-memory
`SimulationState`; serialized snapshots are query evidence only and are never
restored into executable state. Do not reset simulation time, physics step,
fuel, held command, mission outcome, or accumulated state during capture or
continuation.

## Capture boundaries and admission

Capture three distinct globally aligned 60 Hz boundaries from the generated
program's `ballistic_coast` interval:

1. The first capture is after one complete coast control hold: the first
   ballistic-coast command is applied at its ordinary absolute update tick,
   then two 120 Hz physics transitions run; capture the resulting state before
   the next update. In particular, do not treat the source-handoff state as a
   coast state, because its held command can still be powered.
2. The near-apex capture is the eligible coast boundary with minimum actual
   `abs(vy)`, choosing the earliest tick on an exact tie. This is a relative
   extremum over observed coast boundaries, not a tuned vertical-speed
   threshold.
3. The last capture is the final aligned pre-step state before the first
   `terminal_bridge` update.

The first and last rules apply before classifying the observed vertical motion.
Each of the three chosen ticks must be distinct and must exhibit its intended
role in actual state: first capture ascending (`vy > 0`), argmin capture near
the observed apex by the rule above, and last capture descending (`vy < 0`).
If the coast interval cannot provide three distinct states with those roles,
retain the finite compatibility failure; do not substitute another tick or
mission.

Each proposed incoming state must be the captured, full live state at an
absolute 120/60 Hz aligned control boundary and satisfy the proposer admission
contract: physical `Flying`, mission `InProgress`, end `Running`, no incoming
contact, exact held throttle `0.0`, positive forward `vx`, positive remaining
fuel, and a route-free `LandingOnPad` mission under the original 120/60 clock.
Preserve the full state clone. The compact airborne state stored in proposal
evidence is not a restoration format and deliberately excludes cumulative
clearance extrema.

## Finite proposal and independent terrain audit

For each eligible capture, call the airborne proposer with only the original
`RunContext`, retained live state, and the absolute scenario budget tick
`floor(mission_budget_s * 120)`. Do not pass the baseline suffix, source row,
saved schedule, terrain-twin data, or expected result. The proposer has a fixed
8-by-7 family (56 maximum attempts): coast fractions
`[0, .125, .25, .375, .5, .625, .75, .875]`; terminal-duration factors
`[1, 1.5, 2, 2.5, 3, 3.5, 4]` of remaining horizontal crossing time; and
round-up to an even number of 120 Hz ticks. Candidate updates are absolute
pre-step global ticks. The original horizon/deadline and incoming fuel remain
in force. Nominal candidate contact is target-pad-only; interior terrain
contact classifications and terrain-derived extrema do not select or rerank a
proposal. Rank accepted nominal proposals by lowest future peak COM height,
then earliest planned end tick, then identity. The actual powered continuation
is restricted to at most one future vertical apex: after actual nominal `vy`
first becomes non-positive, any later positive `vy` rejects that proposal.
This prevents endpoint-only ranking from accepting a trajectory that dives
below the target and then powers upward to re-emerge.

Audit exactly the selected proposal against actual geometry and unchanged
terrain, with a fixed 5 m minimum airborne clearance and the existing target
pad contact exception. The first actual target contact must be stable and
descending and enter from above: the previous rotated-body minimum clearance
to the target plane must be positive. Record ordinary/neutral parity, command coverage, actual
first contact, clearance scan, final snapshot, and all rejection reasons.
Audit failure cannot trigger another selection or terrain-aware reranking.

For every selected proposal, construct one independent terrain twin. Preserve
both original source and target shelves and every physical input other than
the terrain profile. Insert one triangular obstruction ahead of the incoming
body and before the target shelf, with peak strictly above the selected
proposal's future peak COM height by more than 50 m. Reevaluate the nominal
search from the same retained live clone and counterfactual context. Require
the complete search identity/payload, selected proposal identity, and selected
commands to be identical to the original-terrain search. Then independently
audit that same selected proposal on the twin and require rejection by the
actual terrain audit. Do not select a replacement. Incoming query snapshots'
cumulative terrain-clearance extrema are context-dependent evidence and are
not compared as proposal inputs.

## Whole-flight execution and replay

For each selected nominal proposal, continue from its retained full live clone
using `set_command` at each supplied absolute proposal update and
`step_with_contact_report` for each ordinary physics transition. Preserve the
single global clock and actual fuel consumption. Do not reconstruct from a
snapshot, invoke `SimulationState::new` at a capture, rewrite the source
program, add an artificial checkpoint, normalize the held command, duplicate
the boundary action, or provide any command after the finite proposal's last
update. If the supplied updates do not reach terminal contact, stop and retain
that failure; do not allow replay to extend a held command as an unsupplied
fallback.

This first canary is an offline/synchronous pause: proposal computation occurs
between ordinary simulation stepping, with retained `sim_time_s`, physics
step, fuel, and held command unchanged. Record monotonic proposal wall time;
do not advance physics to simulate computation latency. This establishes no
in-flight compute-delay, finite hold, real-time replanning, or bounded
controller-tick latency safety. A future local-clearing handoff still needs a
separately designed finite safe-planning continuation.

Construct one absolute-time action list from the baseline program updates
strictly before capture tick `E`, followed by the actual supplied candidate
updates from `E` through the first terminal contact. Each action has its
absolute pre-step physics tick, exact `sim_time_s = tick / 120`, contiguous
controller-update ordinal, and original command. Independently call
`replay_simulation` from the original source scenario on this stitched list.
Require no duplicate/missing tick and exact action/command parity. In addition
to the official `replay_simulation` result, run the stitched global action list
from the original source with ordinary contact-report stepping so the full
final in-memory snapshot and incoming contact state are retained. Require
exact action/event/sample parity between those independent ordinary replays,
the complete final `SimulationStateSnapshotV1` to match the live-clone audit,
and incoming contact classification/state to match the audit. Check final
manifest parity for contact/outcome, global tick, time, and remaining fuel.
Record proposal audit and whole-prefix replay as distinct checks: neither a
compact state payload nor the proposer's internal candidate simulation can
substitute for full action replay.

## Artifacts and verdict

Invoke `pd-eval nominal-airborne-direct-canary --output-dir NEW_OUTPUT_DIR`;
there is no default output path and the root/files are create-only. Store the
protocol and sealed input identities, current source hashes, fresh baseline
generation/decision, full capture snapshots, each finite search ledger,
selected proposal, actual audit, terrain-twin preservation/search/audit,
stitched actions and ordinary replay artifacts, and all finite failures.
Source-bind at least the nested airborne proposer, its body-aware terminal
math, complete-flat clearance implementation, core simulation/model/evaluator
code, this harness, CLI, and this protocol. A failed case keeps its typed
evidence and contributes to the bounded compatibility summary.
Reuse the existing recursive nominal-direct source closure and append this
protocol and the fresh input manifest. Case identity excludes only the
explicitly retained observational generation/execution/replay/search/audit
wall-time fields; policy, dynamics, commands, state, and outcome evidence are
identity-bearing.

Acceptance requires all four freshly generated baselines to be `Direct`, all
three actual captures per case to satisfy the frozen capture/admission rules,
each selected proposal to pass actual-terrain audit and whole-flight replay,
and each terrain twin to preserve the entire nominal search while failing its
independent actual clearance audit. A case without a selected proposal remains
a finite failure, not a waypoint-necessity or landing claim. Passing establishes
only this fixed airborne coast-plus-terminal regeneration canary; it does not
establish a general execution API, canonical planner, continuous replanning,
obstacle avoidance, arbitrary waypoint-state coverage, realtime cost, or a
default behavior change.
