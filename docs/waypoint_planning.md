# Waypoint Planning

[Documentation home](README.md) · [Current evaluation](evaluation.md#planner-evaluation-v2-default) · [Research archive](history.md)

<a id="current-v2-evaluation-status-2026-10-04"></a>

## Current V2 Evaluation Status (2026-10-05)

The active native planner workflow is the evaluator-owned
`planner_v2_lab_suite`: `pd-eval run-pack` without an explicit pack selects
policy 3, and `pd-eval waypoint-v2-flight` also defaults to policy 3. This is
offline evaluation on the tested vehicle, Earth-gravity, 120 Hz physics / 60 Hz
command setup; it is not a 60 Hz game-loop guarantee. Policy 3 is the Rust
default and sole executable planner; V1 search and policy-1/2 selectors are
retired. Known saved identities/contracts remain readable. The ordinary
`pd-cli run` controller default is unchanged, with no V2 controller added to `pd-control`.

The accepted 44-case capture has 36/36 core target landings (11 direct and 25
corrected). Eight diagnostics remain separate: two landings, four zero-command,
zero-step `NoClearing` stops before departure, and two unsupported inputs. All
44 inputs passed integrity; all 42 supported results passed final-source replay.
The accepted [session/CLI replacement](waypoint_v2_session_repair_results.md)
reproduces these results through native, real CLI and CLI-repeat matrices with
exact retained flight parity and 42 saved-source CLI replays. Its native capture
supplies current provenance. The
[reliability captures](planner_v2_reliability_results.md) and
[activation results](waypoint_v2_eval_activation_results.md) retain earlier
checkpoints. The active batch and mission details are available
at `/reports/eval/planner_v2_lab_suite/` and use the
[shared report templates](planner_v2_common_report_templates_results.md).

V2 constructs a terrain-blind nominal transfer, audits that fixed program
against actual terrain, applies local correction where possible, then replans
from the actual handoff state. A useful safe handoff need not reach a literal
feature far edge or include a landing suffix. A reusable acceptance gate now
binds the frozen 44-case pack, keeps diagnostics separate and prevents failed
captures from replacing current evidence. The owned
[session and optional CLI adapter](guidance.md#optional-v2-session-and-cli-integration)
are now implemented and accepted; they remain synchronous lab execution rather
than a per-tick game controller. See
[current V2 design and support](guidance.md#current-v2-design-and-support) for
the supported envelope, source ownership and reconciliation of earlier research.
The [retirement/consolidation checkpoint](planner_retirement_cleanup_results.md)
closes the approved housekeeping batch without tuning behavior or changing saved
evidence. Broader terrain validation or concrete host integration are separate
possible next choices. The roadmap's D1, W1-W4 and F6 entries are historical or
parked research, not prerequisites for this accepted V2 path.

## Implementation Status

This section records V1 and dated development checkpoints. Its historical
"next" steps are not an outstanding V2 task list; current V2 status is above.
V1 search, generated-route execution and superseded research frontdoors described
below are now retired. Preserve the historical claims at their measured scope.

V1 implementation phases 1-5 are complete. `pd-core` owns the serialized planning
contracts, strict heightfield corridor queries, endpoint-shaped safety profile,
and shared all-leg route validator. `pd-plan` owns the deterministic bounded
search and stable plan identity. `pd-eval` resolves the focused generated-route
matrices before simulation and carries planner identity through schema-36 cache
and artifact contracts; `pd-report` and batch reports render optional planner
evidence without changing legacy authored runs.

Fresh retained captures close the focused generated-route matrix at `54 / 54`
landings and `36 / 36` handoff/ordered-contract runs, both with zero
invalidations. Schema 36 persists the elapsed monotonic wall time spent inside
each single `pd_plan::plan` call without making timing part of deterministic
plan or batch identity. The maintained terminal, direct-transfer, and authored
waypoint gates also reproduced their declared baselines.

An opt-in post-closure
[direct-route characterization](waypoint_direct_characterization.md) now
records a narrower modeling mismatch without revising those retained V1
results. V1 rejects continuous flat, uphill, and downhill terrain because its
exact direct corridor follows the endpoint-shaped pad-to-pad chord, while the
unchanged direct controller lands all three by flying a lofted trajectory. The
same artifact retains bounded ridge and mesa controls, separates planner and
controller claims, and deliberately does not choose or implement a V2 leg
profile or certificate.

The follow-on opt-in
[direct-leg primitive research](waypoint_direct_primitive_research.md) mapped
the existing V2 source-bridge/coast/terminal-bridge certificate onto those
five exact inputs with their 90 s mission horizon. Flat, uphill, and downhill
all passed the direct-certificate gate. A fixed 24-cell obstacle sweep then
certified a direct route in every cell, while the unchanged `transfer_pdg`
controller landed on the 12 lower obstacles and crashed on the 12 taller
ones. The analytical profile is a candidate for direct-leg work; the observed
split is a controller/profile-tracking question, not demonstrated waypoint
demand or a production planner decision.

### Revised direct and local waypoint design 2026 09 29

The revised design separates nominal transfer generation from terrain handling.
A direct proposal should be a canonical idealized ballistic transfer derived
from the current flight state, destination, and vehicle feasibility envelope.
Its apex and terminal entry are chosen for reaching and landing at the target,
not raised until interior obstacles disappear. Audit that selected proposal
against the real, unmodified terrain as a separate step. A finite generator
`Unknown` is distinct from an actual terrain obstruction.

If terrain blocks the nominal proposal, seek a bounded local clearing maneuver.
Generate its feasible trajectory and derive the waypoint position and arrival
velocity together. The goal is to bring the flight out of the obstructed region
with useful progress and a finite safe continuation for replanning, not to prove
that it can land directly afterward. The original clearance-conflict interval's
exit is a search hint, not a mandatory terrain-feature far edge. A useful
handoff can be above a long plateau before that interval ends, and another
waypoint can follow.

At a handoff, regenerate from the actual observed state while retaining global
time, fuel, attitude/rate, and command cadence. Do not reset a mission, invent
checkpoint success, execute restored evidence snapshots, or replay an old
suffix as if it were a new plan. Existing authored-waypoint transfer guidance
already advances through handoffs; the missing capability is dynamically
generating and auditing the next terrain-aware leg.

The first bounded foundation is the opt-in
[airborne regeneration canary](nominal_airborne_direct_canary_protocol.md):
regenerate a finite coast-plus-terminal family from real ascending, near-apex,
and descending states on four accepted uncut-terrain flights; require independent
whole-flight replay and identical nominal proposals across interior-terrain
twins. This family tests state continuity and regeneration coverage, not the
complete canonical ballistic generator or a local obstacle-clearing maneuver.
The production V1 planner remains unchanged. Source-rest launch and body-aware
terminal lessons remain applicable; the separate one-update completion-reserve
design is parked rather than silently implemented by this pass.

The [canary results](nominal_airborne_direct_canary_results.md) pass all twelve
airborne captures and all twelve unchanged-proposal obstacle-twin checks in two
final-source runs, with full state/contact and whole-source replay parity. The
four source-rest baselines and the 24 earlier controls are preserved. This closes
the bounded regeneration foundation. At that checkpoint, terrain-blind canonical
initial selection and actual local clearing remained the next separate capabilities.

### Canonical initial transfer foundation 2026 09 30

The [canonical initial-transfer canary](canonical_initial_direct_canary_results.md)
now passes all six sealed evaluator gates in two final-source runs. Eight uncut
flat/uphill/downhill source-rest flights land safely. All four blocking and four
nonblocking interior-terrain twins preserve the complete nominal search and
selected commands; the fixed proposal is audited independently on actual terrain.
The 900 m late/broad case rejects the canonical choice even though a freshly
regenerated old terrain-aware control safely flies a higher direct arc. All twelve
real coast captures from the new initial flights regenerate accepted continuations
with full-state/contact and whole-source replay parity. The prior airborne canary
and 24-control source-rest regression retain their physical evidence.

This closes the declared terrain-blind initial-transfer foundation, not production
planner wiring or local waypoint generation. The finite policy chooses the lowest
complete-flight peak within its declared family; terrain rejection does not choose
a higher alternative and finite exhaustion remains Unknown. Production V1,
contact physics, existing generator semantics, and the parked completion reserve
remain unchanged. Offline timing does not establish real-time planning safety.

The next separate capability is one local clearing maneuver, with waypoint position
and velocity derived from its feasible trajectory, useful progress, finite safe
continuation, and replanning from the actual handoff. Plan early enough to retain
control authority: a retained first-conflict snapshot identifies the problem but
is not automatically a safe state from which to correct it. Do not require immediate
direct landing or a literal far-edge handoff. Arbitrary powered handoff coverage
and repeated waypoints remain later evidence boundaries.

The [phase plan](local_clearing_canary_plan.md) specifies a sealed one-obstruction
canary, bounded intervention/control-template search with trajectory-derived
coast handoff timing, flying-state replay using existing bounded execution, and
separate actual-handoff regeneration. An analytical review replaces the initial
fixed short coast with a finite search window.

### One local clearing maneuver 2026 09 30

The [local clearing results](local_clearing_canary_results.md) pass all five sealed
gates in two corrected final-source runs. The 168-row search selects a six-second
upright boost from a real powered entry at 12.6 s, then a trajectory-derived idle
handoff at 23.5 s, position (-337.771, 382.821) m and velocity (38.313, -10.903) m/s.
Independent whole-source bounded execution and replay prove that handoff and its
separate two-second safe continuation, with minimum local body clearance 21.313 m.

The verdict is `LocalClearingThenTerrainBlocked`, not landing: the unchanged
nominal generator produces a fresh proposal from actual H, but its fixed terrain
audit encounters the same continuing plateau again. That unsafe suffix is tested
only on diagnostic branches, not executed on the active flight. This closes the
first local clearing/replanning seam without requiring immediate direct landing
or the terrain feature's far edge. Another clearing iteration remains a separate
design and evidence boundary. All eight canonical, twelve prior airborne and
24 source-rest controls are preserved; production V1 and defaults stay unchanged.

### Practical V2 design 2026 10 01

The user clarifies that V2 should cover most ordinary game-like missions, not
guarantee all edge cases. The [practical plan](waypoint_v2_practical_plan.md) now
targets a reusable bounded correction/replanning loop on the current tested
vehicle/gravity/rates. It proposes eight clear controls, sixteen ordinary terrain
cases with a 13/16 success floor and per-family minimum, and eight diagnostics.
Immediate landing and the feature far edge remain outside local ranking.

The [design review](waypoint_v2_practical_plan_review.md) and input-only readiness
checks pass: thirty supported scenarios plus two expected unsupported diagnostics,
no simulations created. Live-origin conflict queries, segment-aware guards and
honest finite failures are explicit implementation requirements. Coverage,
complete obstacle flight and interactive cost still need measurement. This is
implementation-ready design, not an implemented V2 or default promotion.

### Practical V2 implementation 2026 10 01

The subsequent [implementation results](waypoint_v2_practical_results.md) now
prove the complete bounded loop and actual reference landing through three
corrections. Eight clear controls land directly on uncut terrain. The single
uniform initial-entry revision improves ordinary coverage from 8/16 to 12/16,
with every family meeting its floor and complete source replay agreement.
Repeated release planning median is about 0.398 s and p95 below 0.681 s.

The implementation is an experimental reusable mode, not accepted usable V2:
the declared 13/16 ordinary floor is still missed. Four ordinary cases exhaust
the unchanged airborne nominal family after locally safe handoffs, mostly at
the thrust-demand limit. Further entry/maneuver/ranking revisions need another
decision; the approved one-revision budget is spent. This no longer requires
another floor-cutaway or first-step crash study. Production V1 and legacy
controller/default behavior remain separate and unchanged.

### Unified nominal construction design 2026 10 01

The current [next-phase plan](waypoint_v2_unified_nominal_plan.md) keeps the
implemented direct/audit/local-clearing/replan loop and changes its nominal
construction contract. Ground and airborne planning should share a current-state
model with finite acquisition. Current position/velocity remain fixed; lateral
reference endpoints connect current position to target, while future vertical
shape is chosen only to obtain an admissible above-target approach and energy.

The minimum-profile arc is a construction baseline, not an exact tracking
target. Preserve an already admissible higher/steeper approach; do not lower it,
shallow it or create a new future apex solely to match that baseline. Excessive
energy still needs correction, and lateral retargeting changes vertical timing.
Acquisition estimates must include future join position, gravity, turning, fuel
and original time rather than just instantaneous velocity replacement.

The [analytical handoff screen](waypoint_v2_nominal_handoff_analysis.json) shows
why a cheap estimate is useful but not a recovery proof. Apparent stopping room
in the four misses changes substantially when orientation delay and source
derating are included. First characterize the estimator and landing-entry
conditions, then review and freeze their numerical/command-realization contract
before an explicitly approved new policy. Local ranking remains unchanged for
that comparison; the 13/16 gate and frozen suite remain intact.

The [goal amendment](waypoint_v2_goal_amendment.md) records the intended update.
At this design checkpoint the stored old goal was blocked because the available
tools could not edit its objective. It was externally cleared before the
subsequent research loop; it was not falsely completed. The design pass made
no runtime/default/physics change and did not call the 12/16 implementation usable.

### Unified nominal characterization 2026 10 01

The [research results](waypoint_v2_nominal_characterization_results.md) now cover
eight fresh clear starts, all 27 actual local handoffs and twelve historical
airborne captures, plus separately labeled synthetic conditions. All 39 airborne
states reproduce exactly from their full original command prefixes and have
independently replayed free-space target-plane landing witnesses. The four
former NoNominal handoffs also land on their unchanged terrain without measured
noncontact reserve violations; three preserve natural vertical motion.

The shared research construction passes only 2/8 clear ground starts, however.
All eight ground preparation programs lift off safely and replay; six reject
analytical acquisition/entry screens before any physical witness attempt.
This is not another first-step crash, and does not supersede the working old
8/8 uncut ground constructor. Some airborne witnesses remain terrain-blocked,
which is a demand for another local piece, not terrain-aware nominal lofting.

The [separate primary review](waypoint_v2_nominal_characterization_review.md)
concludes no-go for runtime replacement and proposes comparing the new screen
against proven ground entries to isolate screen conservatism from acquisition
span. Two final-source artifacts are byte-identical; integrity, workspace and
input/evidence preservation checks pass. This closes the research checkpoint,
not the unchanged complete-loop 13/16 usability gate. No policy/default/local
acceptance/physics/contact change or commit is included.

### Ground diagnostic 2026 10 02

The [bounded diagnostic results](waypoint_v2_ground_diagnostic_results.md)
reproduce all eight known-good uncut controls from fresh H0 and compare sixteen
references at their actual terminal entries. Original durations pass; the
unchanged research vertical braking-time formula produces lateral reversal in
all eight matched comparisons. This localizes a construction restriction, not
another source-contact crash or a requirement to cut away the floor.

The three preselected shadow probes all land, but flat/uphill need approximately
0.993 terminal throttle and fail the unchanged 0.925 budget. Their nominal
rejection remains unchanged; the research ground family is still 2/8 versus the
working old 8/8. The [primary review](waypoint_v2_ground_diagnostic_review.md)
recommends bounded coupled-state terminal timing first, keeping the backend and
margin. Whether research acquisition reaches compatible entries remains open.
No runtime fix, new full-loop coverage or default promotion occurs here.

The [terminal timing plan](waypoint_v2_terminal_time_plan.md) was subsequently
implemented under its new opt-in research identity. The
[results](waypoint_v2_terminal_time_results.md) recover 8/8 uncut ground landings
while retaining all 39 airborne witnesses and four designated terrain recoveries.
A valid baseline duration stays unchanged; at most two derived horizontal times
follow a finite rejection, under exact discrete no-reversal and deadline caps.
Acquisition, reference backend, ranking semantics and 0.925 budget stay unchanged.
The [primary review](waypoint_v2_terminal_time_review.md) accepts the constructor
checkpoint. Runtime policies/defaults remain unchanged; next is separately
planned integration and the complete 32-case gate, not another timing search.

The [simplified integration plan](waypoint_v2_airborne_integration_plan.md) narrows
that next pass to airborne regeneration. Keep the already-working uncut launch
and policy 2 entry/local behavior, select the first executable state-aware
proposal within the tested three-trial cap, then audit terrain exactly once.
Launch unification and unusual recovery modes are deferred. The
[planning review](waypoint_v2_airborne_integration_review.md) keeps the unchanged
mission/performance gates and finds this ready only after separate approval;
no new runtime policy or flight result is delivered by the design pass.

### Usable opt in airborne integration 2026 10 02

The [implementation results](waypoint_v2_airborne_integration_results.md)
reach the complete supported game-oriented V2 gate under explicit policy 3:
8/8 uncut direct controls and 16/16 ordinary terrain missions, with the reference
plateau still requiring three corrections. Both final-source matrices match
all 64 per-case payloads and pass existing safety/replay and planning latency
limits. All thirty supported initial cycles remain exactly policy 2's;
canonical launch and local clearing are unchanged. Only airborne nominal
construction and its complete phase-aware audit are replaced.

The [primary review](waypoint_v2_airborne_integration_acceptance.md) accepts
the first usable supported opt-in checkpoint. Late fail-closed guard corrections
pass nine focused tests and fresh 39-row equivalence without numerical changes.
After the original matrix allowance was used, the user explicitly approved one
corrected-source policy 2 preservation matrix and two policy 3 repeats; all pass
their respective gates. The final workspace passes 915 tests. Earlier complete
flights remain preliminary evidence, not relabeled current-build acceptance.
No GUI/controller/default promotion occurs.
Policy 1 remains default and policy 2's historical payloads
are preserved. Four difficult diagnostics retain NoClearing with honest partial
flight evidence; other vehicle/gravity setups reject without simulation.
Ground unification, unusual recovery and perfect diagnostic coverage are not
prerequisites or automatic next work.

### Preceding direct-planning checkpoint (2026-09-28)

The next [one-update completion-reserve contract](nominal_direct_terminal_completion_reserve_contract.md)
and [implementation protocol](nominal_direct_terminal_completion_reserve_protocol.md)
are design-only. They specify one exact final-command update beyond saved
coverage, clipped by the original planned end, without changing `FlightProgramV1`
or strict execution. Fresh 735/915 m flat and 915 m ±75 m slope inputs are sealed
and pass input-only readiness, not flight validation. The narrow shared segment
seam uses freshly captured in-memory terminal-entry state, recorded vertical
counterfactuals and independent replay; persisted snapshots remain query-only.
No reserve capability, new flight measurement, accepted continuation, operating
tolerance or default/direct-first planner change is introduced by this design.

The opt-in
[strict saved-coverage executor](nominal_direct_operational_execution_results.md)
is implemented and validated: all 24 exposed controls retain exact nominal
proof, and four inputs sealed before implementation (685/845 m flat and 845 m
uphill/downhill ±75 m) are Direct, completed-safe and Match in two final-source
runs. It separates actual contact/outcome from exact nominal comparison,
retains incoming contact and truthful partial evidence, and recomputes bounded
replay from the full program. An airborne run stops before an uncovered update
or hard deadline; no idle, extra command or last-command hold is supplied. All
809 workspace tests and strict lint pass; source and historical archive bytes
are unchanged during measurements. Exact nominal verification stays strict.
The [completion contract](nominal_direct_execution_completion_contract.md)
does not recover the 48 diagnostic late-contact observations. Robustness,
direct-first default wiring and airborne waypoint composition remain separate.

The latest diagnostic is the evaluator-only
[first-contact and command-coverage study](nominal_direct_contact_phase_results.md).
All 24 nominal baselines reproduce exactly before vertical-only terminal-entry
offsets. Two complete 216-row runs agree: saved programs reach 168 safe contacts
and stop at missing commands in 48 rows; bounded diagnostic final-command hold
reaches safe target contact in all 216, within the original planned end. No
unsafe contact, clearance/domain violation or parity failure occurs. This is
not general robustness or an approved fallback. The operational pass now
implements honest finite coverage/completion without promoting that diagnostic
hold. Exact nominal proof stays strict; no new source fit or obstacle sweep is
required to close this boundary.

The preceding exact-nominal baseline remains the opt-in
[nominal direct flight integration](nominal_direct_flight_integration_results.md).
It executes complete accepted timed programs through the ordinary controller
and simulator path from mission inputs. All 24 now-exposed body-aware controls
have exact full-generation, selected-command, safe-contact and action-replay
parity in two independent release runs. No saved command schedule is a generator
input, no floor cutaway or waypoint is added, and production V1/default behavior
is unchanged. The CLI returns finite Unknown without fallback; valid unsupported
inputs and malformed bindings are separately typed and execute no flight.

At that preceding checkpoint, local release medians were about 944 ms for the
unchanged research generator and 662 ms for selected verification, versus 1.86 ms
for ordinary
execution of the entire offline flight. This includes old-policy comparison and
source-family regeneration; it is not optimized production setup cost.
Contact-predicate margins are not obstacle clearances or an operating tolerance;
general perturbation robustness remains unproven by the vertical-only study.

The preceding
[body-aware terminal prototype](waypoint_direct_body_aware_terminal_results.md)
was not a production planner/controller change. Under one generic fixed policy,
all fourteen development cases and all ten cases sealed before implementation
have complete accepted Direct flights without cutaways or waypoints. All 152
development and 100 fresh source schedules pass; every previously accepted
development row is retained. Both exposed and both fresh high obstacles now
land directly at the intended global held-60 Hz cadence under 120 Hz physics.
Launch/source commands and all fourteen historical baseline bytes are unchanged.

The new terminal reference adds a zero final horizontal acceleration constraint
and audits actual first core contact; the executor averages adjacent thrust
vectors and inverts throttle against both post-burn masses. Complete ordinary
and neutral replay validates body clearance, real attitude joins and command
clock, fuel and time budgets, and stable safe target contact. Upper-foot margin
is about 0.154 m and incoming normal speed about 1.5 m/s, but selected
hull-penetration margin remains only 6.29–7.50 mm. Nominal acceptance is not
perturbation robustness, swept-path proof or real-time planning authority.

The preceding
[terminal-admissibility and cadence diagnostic](waypoint_direct_terminal_admissibility_results.md),
was diagnostic-only, not a planner/controller change. All 76 retained schedules
reproduced their frozen contact results. Of 42 rejected schedules, 32 have inadmissible
ideal first-contact foot geometry, while ten have admissible reference poses
but fail under held-60 Hz execution. Matched-entry terminal-only 120 Hz commands
land those ten and preserve all three accepted controls, but the six high-case
tails still fail upper-foot contact geometry despite essentially exact tracking.
Two independent runs retain byte-identical complete evidence. This separates
reference construction from execution cadence; it does not promote new witnesses.

The preceding
[frozen obstacle-discrimination pass](waypoint_direct_obstacle_discrimination_results.md),
also did not change `pd_plan::plan()`. Four development controls and all eight
sealed 700/900 m cases passed their declared gates without retuning. At that
checkpoint, six cases had complete Direct witnesses and two high obstacles
were finite Unknown; all 160 rows and both-run byte parity are retained. The 900 m
late/broad obstacle blocks the selected flat program but the unchanged
generator accepts a different direct arc. No waypoint was added.

The preceding
[input-driven nominal direct generator](waypoint_direct_generation_results.md)
generates a contact-safe launch, reseeded
source bridge with paired held-60 Hz commands, ballistic coast, and terminal
bridge from scenario inputs alone. Gate A exactly reproduced the
[known-flat acceptance checkpoint](waypoint_direct_complete_flat_acceptance_protocol.md):
four of nine flown wrappers accepted, with a `34.35 s` planned winner and
`34.275 s` stable target contact. Historical adapters retain their original
artifact bytes.

After Gate A and code freeze, all six predeclared 600/1,000 m flat/uphill/
downhill cases passed under one unchanged policy, without cutaways or
retuning. The finite family retained 120 rows, 76 full schedules, and 28
accepted witnesses. Forty-eight target-contact crashes were rejected before
ranking. Two fresh runs produced byte-identical complete artifacts.

Keep the evidence levels separate: the earlier V2 analytical certificates
retain their original classifications, the original source-contact failures
remain valid, and the new launch-aware wrappers are nominal simulator witnesses
rather than V2 certificates or robustness proofs. The original generation
checkpoint's smallest selected fresh-case hull-penetration margin was only
about `+2.245 mm`; nominal success
must not be promoted to robust or production/default authority. The
[direct-first boundary protocol](ballistic_direct_first_decision_protocol.md)
also records an analytical direct-to-waypoint boundary, but its controller
execution slice has not passed its exact-trajectory compatibility gate.

The input-driven nominal baseline, bounded obstacle discrimination, terminal
causal diagnostic, body-aware nominal terminal pass, opt-in complete-program
flight integration, fixed contact-phase diagnostic and strict saved-coverage
operational execution are closed at their declared gates. Do not restart source
fitting or cadence-only experiments. The production V1 chord model is still
unchanged. Setup cost is now measured for the research backend, not optimized
or accepted against a production budget. Commands/recovery beyond saved coverage,
general robust contact, arbitrary incoming
waypoint states, waypoint composition and production/default selection remain
open; changing flight policy requires a new
boundary and held-out seal, not retuning inspected cases.
Neither a controller crash nor finite direct-family exhaustion proves that
every physically possible direct transfer is blocked; the latter remains an
explicit `unknown`.

### Post-closure expansion finding

The first diagnostic nominal-radius `r-60 | r+60` expansion exposed a boundary
above planner V1 rather than reopening V1 closure. Its disposable local
contract snapshot closed at `11 / 24`, while the established focused planner
contract remains `36 / 36`. A research-only sequential quintic trajectory-tube
spike could not preserve that maintained baseline while rejecting the expansion
failures, and its terrain-limited mismatch diagnostics localized to the
undeclared source-contact/departure transition.

The result is a production stop rule for that model, not a new planner
rejection claim. Further coverage expansion first requires a neutral
source-departure/acquisition capability and a progress-indexed route-relative
envelope or paired-executor contract, validated on held-out evidence without
route-label tuning.
The model, corrected results, limitations, and escalation gate are recorded in
[Trajectory-tube shadow spike](trajectory_tube_spike.md).
The follow-on [source-departure/acquisition execution contract](source_departure_execution_contract.md)
closes the design prerequisites with an initial-state pad-departure/acquisition
contract, neutral source and route-wide evidence artifacts, explicit tri-state
phase composition, and a blinded implementation sequence. D0a/D0b observational
evidence is implemented; the planner-facing predictive capability from D2
onward remains blocked and unimplemented. This does not reopen V1 closure.

This document defines the first waypoint-planning slice above the closed
terminal, direct-transfer, and preplanned-waypoint guidance stack. It owns the
planner contract, bounded search policy, evidence model, and implementation
sequence. Guidance behavior and the maintained preplanned corpus remain owned
by [Guidance Architecture](guidance.md) and
[Transfer Suite Design](transfer_suite.md).

## Decision

Waypoint planning v1 is a deterministic, setup-time, pad-to-pad planner for the
existing static 1D heightfield world.

It will:

- plan once before simulation, never inside a controller update
- accept source and target pad identities plus immutable world, vehicle, and
  initial-state context
- emit either a direct route or at most two ordered pass-through waypoints
- preserve monotonic source-to-target progress after left/right normalization
- use terrain geometry, vehicle clearance, a versioned route policy, and a
  conservative authority screen
- persist enough provenance to reproduce the exact resolved route

This is a bounded route constructor, not a general terrain-navigation system or
a trajectory optimizer. Planner success means that a route passes the declared
geometric and authority screens. Only simulation can establish that guidance
flies the route, satisfies every handoff, preserves fuel, clears terrain with
the actual hull, and lands safely.

## Goals

- Turn terrain-valid route setup into a production component rather than an
  evaluator-only authored profile.
- Reuse the existing waypoint handoff contract instead of adding a second
  arrival language.
- Keep terminal and waypoint guidance terrain-blind and free of obstacle-name,
  route-profile, payload, seed, and mission-time branches.
- Produce deterministic artifacts that can be cached, compared, and reviewed.
- Prove usefulness on a small curated corpus before expanding route radius,
  terrain complexity, or runtime behavior.

## Non-Goals

V1 does not include:

- runtime replanning or reactive obstacle avoidance
- random terrain, overhangs, caves, moving obstacles, or obstacle layers
- arbitrary start/end states that are not associated with landing pads
- reverse-progress paths or more than two generated waypoints
- fuel-, time-, or globally optimal trajectories
- a convex trajectory optimizer or a replacement for the guidance controller
- a proof of full dynamic feasibility from analytic screens alone
- exact coordinate matching against authored waypoint profiles

The `pylander` terrain experiments are a warning here: long-horizon route
choice and short-horizon execution guards should not collapse into one large
scenario-specific controller state machine.

## Ownership And Data Flow

The planned workspace boundary is:

- `pd-core` owns neutral serialized planning contracts and reusable terrain
  clearance queries.
- a new `pd-plan` crate owns the deterministic planning algorithm and depends
  only on `pd-core`.
- `pd-control` remains a consumer of the resolved `TransferRouteSpec`; it does
  not call the planner or share controller configuration with it.
- `pd-eval` resolves planner-backed fixtures, compares generated routes with
  authored property oracles, runs guidance evidence, and owns batch/cache
  orchestration.
- `pd-report` renders optional planner provenance and route diagnostics from
  captured artifacts.

The setup flow is:

1. Resolve the world, vehicle, initial state, source pad, target pad, and
   versioned planner policy.
2. Call `pd-plan` once.
3. Validate and persist the complete `RoutePlan`.
4. Put its `TransferRouteSpec` into the concrete mission.
5. Select a compatible direct or waypoint guidance controller outside the
   planner.
6. Run the existing simulation and evaluation contracts unchanged.

The planner does not choose a controller ID. A zero-waypoint plan is a direct
route; a non-empty plan is a waypoint route. That topology is data for the
caller, not a controller-selection side effect inside planning.

## Planning Contract

The public request is pad-to-pad even though it carries enough physical context
to screen the route. The logical input is:

- immutable `WorldSpec`, including the full validated heightfield and pads
- immutable `VehicleSpec` and initial vehicle state
- `source_pad_id` and `target_pad_id`
- an explicit `RoutePlanningPolicy`

The request does not accept route angle, route radius, controller configuration,
scenario labels, seeds as behavior switches, or simulation time remaining.
Route angle and radius are derived from the two pad centers for reporting and
`TransferRouteSpec` compatibility.

A successful `RoutePlan` contains:

- `algorithm_id`, initially `heightfield_visibility_v1`
- the complete policy version and resolved policy values
- stable request and plan digests
- route topology: `direct` or `waypoint`
- the resolved `TransferRouteSpec`
- normalized source/target geometry and selected candidate-node identities
- direct-path result, waypoint count, route length, peak extra loft, and minimum
  planned clearance
- per-leg clearance, stopping, turn-authority, and handoff-envelope diagnostics

The artifact is more than `TransferRouteSpec`: the route is the guidance input,
while the surrounding provenance explains why the planner selected it.

Planner algorithm, policy, and plan digests are part of the resolved descriptor
and exact batch-cache reuse digest. A generated route must never reuse a cache
entry merely because its pack selectors and controller happen to match an older
plan. Cross-report matching still uses stable physical request/corpus identity
plus route provenance, so a changed generated plan can be compared with the
previous generated plan for the same case. The report surfaces the changed plan
digest instead of turning it into a coverage mismatch. `authored` versus
`generated` is route provenance, not a controller lane or comparison role.

## V1 Policy

Every resolved request persists its complete policy. The initial policy is
intentionally small and independently versioned from controller configuration.

| Policy field | Initial value | Purpose |
| --- | ---: | --- |
| `max_waypoints` | `2` | Bounds route and guidance complexity. |
| `flight_clearance_margin_m` | `24` | Adds operational margin outside endpoint transition windows. |
| `endpoint_transition_m` | `96` maximum | Tapers from pad-contact geometry to full en-route clearance. |
| `max_extra_loft_ratio` | `0.45` | Caps route height above the direct source-target chord relative to direct distance. |
| `max_continuation_ratio` | `0.75` | Preserves the maintained stopping/turn authority margin. |
| `max_handoff_speed_mps` | `130` | Retains the maintained upper envelope before authority clamping. |
| `min_handoff_speed_mps` | `10` | Avoids turning a waypoint into a stop target. |
| `min_outbound_progress_mps` | `8` | Requires useful continuation into the next leg. |
| `max_outbound_heading_error_rad` | `0.35` | Reuses the maintained pass-through heading contract. |
| `max_outbound_cross_speed_mps` | `20` | Bounds lateral energy at handoff. |

The `24m` and `96m` starting values align with existing transfer-clearance
experience, but the planner owns its copies. It must not read hidden defaults
from `pd-control`.

The endpoint transition is capped at one quarter of the usable horizontal span
after each touchdown footprint has cleared its pad. The full en-route
requirement is an orientation-independent vehicle bounding radius plus
`flight_clearance_margin_m`. The canonical centerline remains at the source
touchdown reference until the contact footprint clears the source pad, blends
to the original pad-center chord as the envelope expands, follows that chord
through the full-envelope middle unless explicit waypoints loft it, then uses
the symmetric target blend and contact plateau. These implicit endpoint points
are persisted diagnostics, not emitted waypoints. Unsupported near-vertical or
overlapping endpoint geometry fails explicitly rather than silently weakening
clearance.

The loft cap is required for meaningful rejection semantics. Without it, a
static heightfield route can usually be made geometrically clear by adding an
arbitrarily high apex. The cap is measured above the direct source-target chord
at the same normalized progress, with the allowance equal to
`max_extra_loft_ratio * direct_distance_m`.

## Geometry And Search

Planning runs in a normalized frame where source-to-target horizontal progress
is positive. Output is transformed back to world coordinates once, and that
side choice never changes mid-route.

The algorithm is:

1. Validate terrain, pads, finite policy values, non-overlapping endpoint
   windows, and source/target horizontal separation.
2. Build a conservative vehicle-center safety profile from the piecewise-linear
   heightfield, orientation-independent vehicle envelope, planner clearance,
   and endpoint taper.
3. Create a finite directed candidate graph from source, target, safety-profile
   breakpoints, and endpoint-window boundaries.
4. Add only forward-progress edges whose complete segment passes the shared
   centerline-to-heightfield clearance predicate.
5. Accept a clear source-to-target edge as a zero-waypoint plan.
6. Otherwise select a path lexicographically by waypoint count, descending
   minimum conservative handoff-authority cap, total polyline length, peak
   extra loft, and stable candidate identity.
7. Enforce the loft and two-waypoint limits, then construct handoff tangents and
   authority-clamped arrival envelopes.
8. Run the same route-property validator used for authored oracle routes before
   returning the plan.

The search is exact over this finite conservative candidate graph, not a claim
of continuous-space or dynamic optimality. Every accepted edge gets an exact
deterministic clearance check against the canonical heightfield; sampled plot
geometry is never the acceptance authority.

Candidate and edge identities are assigned from canonical terrain order. All
inputs are finite-validated, numeric comparisons use a documented total order,
and graph traversal never depends on hash iteration. Digests use canonical
serialized values rather than debug text or artifact paths.

Authority ranking maps only the comparison key to a fixed `1e-9 m/s` grid. The
observed `4e-15 m/s` numerical difference therefore shares a stable key and
falls through to the later length and loft tie-breaks. Emitted arrival-envelope
caps, exact route validation, policy thresholds, and persisted diagnostics
remain unquantized.

The shared generic validator must check source-to-first, intermediate, and
last-to-target legs with the endpoint taper. The current evaluator validator
intentionally skips endpoint legs for authored guidance profiles because their
flown endpoints are shaped dynamically; carrying that exception into planning
would allow a nominally valid route to intersect terrain before the first or
after the final waypoint. Exact profile-coordinate expectations remain
evaluator/oracle checks and do not belong in the generic planner contract.

The orientation-independent vehicle envelope is intentionally conservative.
An attitude-aware corridor can recover rejected routes later, but v1 must not
claim clearance by assuming the guidance controller will hold a favorable
attitude.

## Arrival Envelopes And Authority

For each generated waypoint:

- capture radius starts at `8%` of direct route distance, clamped to the
  maintained `35m` to `95m` range
- adjacent capture regions must not overlap
- handoff tangent is the normalized inbound/outbound angle bisector
- heading, outbound progress, cross-speed, and minimum-speed bounds use the
  versioned policy values
- maximum handoff speed is the lower of the policy cap and the
  authority-derived continuation cap

The analytic screen uses worst-case initial mass and gravity-taxed authority:

```text
thrust_acceleration = max_thrust / initial_mass
conservative_net_acceleration = thrust_acceleration - gravity
```

For each side of a handoff, the stopping cap is derived from:

```text
speed_cap = sqrt(
    2 * conservative_net_acceleration
      * max_continuation_ratio
      * available_distance
)
```

The turn cap inverts the maintained deflection-distance relation with the same
gravity-taxed acceleration and ratio. The waypoint cap is the minimum of the
policy cap and every inbound/outbound stopping and turn cap. Zero-deflection
turns add no turn cap.

The plan is rejected when conservative net acceleration is not positive or when
no handoff speed at or above the policy minimum satisfies the stopping and turn
ratios. This is deliberately more conservative than the current authored-route
screen, which uses gross thrust acceleration.

The screen is a necessary policy check, not a sufficient dynamic proof. It does
not model attitude transients, coupled vertical/lateral authority, finite burn
timing, or fuel sufficiency. Those remain simulation evidence. V1 makes no
analytic fuel-feasibility claim.

## Failure Semantics

Planner rejection is typed and stable enough for fixtures and reports:

- `invalid_request`: malformed policy, unknown/equal pads, invalid terrain, or
  non-finite physical input
- `unsupported_geometry`: endpoint windows overlap, horizontal progress is too
  small, or the request is outside the monotone-heightfield v1 contract
- `loft_limit_exceeded`: a graph path exists only above the policy loft ceiling
- `route_complexity_exceeded`: a path within the loft ceiling needs more than
  the allowed waypoint count
- `insufficient_authority`: the conservative authority screen cannot construct
  a valid handoff envelope
- `no_route_within_policy`: the bounded conservative candidate graph contains
  no acceptable path for another declared policy reason

These codes mean “rejected by planner v1 policy,” not “physically impossible.”
In particular, a higher-loft, higher-complexity, attitude-aware, or optimized
planner may accept the same mission later.

Expected planner rejections are contract fixtures and unit/integration tests,
not fake simulation rows. If an expected-solvable planner-backed pack cannot
resolve, pack execution fails before simulation. This avoids changing the batch
outcome model merely to display a route that was never run.

## Evidence Model

Planner evidence is layered because no one metric answers the product question.

### Contract evidence

- deterministic output and digest stability
- waypoint count and strict forward ordering
- finite, non-overlapping route geometry
- full-edge terrain clearance under the declared planner envelope
- loft and authority policy compliance
- stable rejection codes for unsupported requests

### Guidance evidence

- every generated waypoint passes the existing handoff or ordered-sequence
  contract; a zero-waypoint route instead runs the direct-transfer gate
- final target entry remains terminal-recoverable where applicable
- existing preplanned waypoint closure packs do not regress

### Physical evidence

- final landing outcome
- actual minimum hull clearance along the flown trace
- fuel use and simulation duration as diagnostics
- route length, peak loft, and excess path length as diagnostics

Generated-route efficiency is not an initial pass/fail threshold. The structural
limits prevent degenerate routes; length and loft distributions should first be
captured so a later threshold is based on evidence rather than aesthetics.

Authored routes are property oracles, not coordinate goldens. They demonstrate
that a case has a route inside the v1 topology and policy bounds and provide
expected topology/clearance properties. The generated route may choose different
coordinates if it passes the same contract and simulation evidence.

## Initial Corpus

The first accepted geometry classes are curated and non-random:

- `clear_direct`: the direct corridor is clear and the planner emits no waypoint
- `single_mid_ridge`: direct flight is blocked and one waypoint is sufficient
- `double_separated_ridge`: the loft cap makes two ordered waypoints the bounded
  route instead of one excessive apex

Initial rejection fixtures are:

- `three_ridge_complexity`: requires more than two candidate waypoints under the
  v1 loft policy
- `authority_limited_short_climb`: geometric route exists but the gravity-taxed
  handoff screen fails
- malformed policy, terrain-domain, and endpoint-window cases

The first simulation matrix stays focused:

- route angles: `r-30 | r00 | r+30`
- radius: nominal only
- payloads: `empty | full`
- seeds: the three smoke seeds
- route source: paired authored-oracle and generated-plan fixtures where useful

Accepted terrain features stay outside endpoint transition windows. Source-pad
shoulders, target-side terminal traps, steep `r+60/r+80` routes, all radius
tiers, randomized terrain, and runtime disturbances are later expansions. They
would otherwise conflate planner geometry, endpoint ownership, and guidance
before the basic contract is proven.

The existing maintained gates remain mandatory no-regression evidence:

- terminal bot-lab and trajectory-error smoke packs
- direct route-angle/radius transfer
- paired waypoint turn landing/contract packs
- paired ordered waypoint landing/contract packs

## Reports And Cache Identity

Planner-backed run artifacts should expose:

- route provenance, algorithm ID, policy version, and plan digest
- direct-path rejection reason and selected waypoint count
- safe profile, selected centerline, waypoint envelopes, and actual trajectory
- planned and actual minimum clearance
- route length, direct distance, excess length, peak loft, and authority ratios
- elapsed monotonic planner wall time for the single setup-time solve

The aggregate preview should remain readable: terrain, pads, generated
waypoints, and actual trajectories are enough. Detailed per-seed views may add
the safety profile and selected centerline.

Planner provenance is orthogonal to `current` versus cached comparison. Reports
must not label an authored route as baseline or a generated route as current.
The result-pack comparison model continues to compare complete controller and
planner states over matching resolved cases.

## Implementation Sequence

The implementation sequence is:

1. **Contracts and geometry primitives**
   - add neutral planning policy/result/error contracts to `pd-core`
   - add exact segment/corridor clearance queries and characterization tests
   - centralize generic route-property validation across every leg while
     keeping profile-specific oracle assertions in `pd-eval`
2. **Deterministic planner**
   - add `pd-plan` with normalized safety-profile construction, candidate graph,
     bounded search, envelope construction, digests, and rejection fixtures
   - no evaluator, controller, or report behavior changes in this commit
3. **Evaluator integration**
   - resolve planner-backed scenario families in `pd-eval`
   - include algorithm/policy/plan identity in descriptors and cache digests
   - add the focused authored-oracle/generated-plan corpus
4. **Evidence and presentation**
   - persist planner provenance and diagnostics in run/batch artifacts
   - add contract, landing, and actual-clearance scorecards plus route overlays
   - recapture only the new planner packs; refresh existing report HTML without
     changing maintained evidence
5. **Closure**
   - run the planner matrix and all maintained guidance no-regression gates
   - record accepted/rejected coverage, residuals, compute cost, and any policy
     changes before expanding the corpus

Steps 1-5 are complete. The focused captures close at `54 / 54` landings and
`36 / 36` contracts with zero invalidations, and every maintained regression
pack reproduced its declared baseline. Further route angles, radii, terrain
classes, or runtime behavior require a separate checkpoint.

## Design Basis

The finite visibility-graph direction fits the small, static, piecewise-linear
world and keeps behavior deterministic. Standard shortest-path geometry also
explains why obstacle inflation must happen before visibility testing rather
than hoping a point path implies vehicle clearance; see LaValle's
[visibility-graph discussion](https://lavalle.pl/planning/node271.html).

The planner deliberately stops short of kinodynamic completeness. Dynamic
motion planning with acceleration constraints is materially harder than a
geometric route search, so v1 uses conservative screens plus simulation rather
than presenting a geometric result as a proof; see Donald et al.,
[Kinodynamic Motion Planning](https://scholars.duke.edu/publication/768953).

This boundary also preserves the useful result from the earlier project:
planning owns long-horizon route choice, while any future reactive avoidance is
an execution guardrail that preserves the route instead of replacing it.
