# Waypoint V2 unified nominal design and plan

## Decision and current state

Keep the implemented direct proposal, terrain audit, local clearing and actual
handoff replanning loop. The next design change is inside nominal trajectory
construction: use a common current-state-to-target model for pad rest and
airborne starts, preserve already acceptable motion, and estimate the acquisition
needed to meet an admissible landing approach. The reference arc is a minimum
acceptable construction baseline, not a profile that every flight must track.

This 2026-10-01 checkpoint authorizes research and documentation, not another
runtime policy revision. It supersedes the earlier suggestion to tune local
ranking before examining the nominal constructor. The original
[implementation plan](waypoint_v2_practical_plan.md) and
[measured results](waypoint_v2_practical_results.md) remain the historical baseline.
The [goal amendment](waypoint_v2_goal_amendment.md) retains the usable-planner
objective and records the changed next phase.

Both retained policy 2 matrices land 8/8 clear controls and 12/16 ordinary
obstacle cases. The 13/16 floor is unmet. Four missions clear terrain safely,
then exhaust the airborne nominal family. Its 56 choices are eight coast-time
fractions times seven terminal-time factors; each trajectory is freshly computed
from actual state. They are not stored routes, but their timing and trajectory
family are restrictive. Across the four misses, 216/224 attempts reject excessive
thrust demand and eight reject the shape rule. Neither recoverability nor
physical impossibility has been established for these states.

The source-rest path already fits and simulates its finite launch/acquisition;
it does not assume instantaneous departure velocity. The opportunity is to
share inexpensive construction and admissibility logic, not to remove state
continuity or executable-flight validation.

The subsequent [characterization](waypoint_v2_nominal_characterization_results.md)
and [primary review](waypoint_v2_nominal_characterization_review.md) now complete
the first checkpoint below. All 39 retained airborne states have free-space
landing witnesses, and the four former NoNominal handoffs also land on their
real terrain. Only 2/8 clear ground starts pass the studied shared family. The
review therefore rejects runtime replacement for now; the staged implementation
sections remain conditional, not an instruction to start a writer.

## Clarified nominal contract

Inputs are the actual position, velocity, attitude, angular rate, held command,
fuel/mass and absolute clock; target pad geometry/elevation; vehicle/gravity;
and remaining original deadline. Grounded mode additionally supplies the source
support geometry. Interior terrain is not an input to nominal choice.

Both coordinates and both velocity components are fixed at the actual start.
The lateral reference connects current x to target x; this does not fix flight
time or prohibit lateral velocity correction. Vertical freedom means choosing
future height/apex and approach shape, not changing the current height or
inventing a different incoming velocity.

Define acceptance at a declared terminal-entry boundary above and approaching
the target. Require descending entry, adequate geometry/height, a sufficient
downward approach angle, and speed/braking demand within the landing capability.
Angle alone is insufficient. Entry is a region of admissible states, not an
equality to one reference height, velocity or apex. Touchdown still uses the
unchanged physical landing predicate and execution backend.

For an entry velocity, the descent angle is
`atan2(-v_y, abs(v_x))`. A steeper-than-minimum approach is not a reference error
if braking, attitude, fuel and time are also acceptable. Excessive downward
energy still requires braking. An already-safe descending state needs no newly
manufactured future apex. If unsafe geometry genuinely needs lift, powered
acquisition may change that motion; a blanket ban on ascent after any preceding
descent is not an appropriate substitute for phase-specific admissibility.

Use the lowest adequate profile when new vertical shaping is necessary. When
the current vertical motion is already admissible, do not lower the apex or
make descent shallower merely to match that baseline. Terminal braking and
coupled changes necessary for lateral correction are not forbidden. Reassess
arrival after lateral correction changes timing; do not claim independent axes.

The angle measurement location, entry region, energy bounds and numerical policy
must be settled together at the first checkpoint below. Do not silently equate
the tangent at a virtual pad-height intercept with the actual terminal-entry
angle, or invent an unvalidated global angle threshold.

## Common construction with finite acquisition

One constructor should predict existing ballistic motion, test admissibility,
and correct only violated conditions. Ground launch and airborne acquisition
have different initial support constraints, but share target geometry, arrival
conditions, acquisition estimates, candidate ranking and result semantics.

1. Predict the no-correction trajectory and potential terminal entry from the
   actual state. If already admissible, preserve it; zero acquisition remains
   distinct from the powered terminal landing needed afterward.
2. If lateral intercept is wrong, construct a correction with a future join
   position and velocity. Preserve an admissible vertical profile unless the
   combined thrust/time constraints make a change necessary.
3. If entry is too shallow, below the target approach, or outside the braking
   envelope, adjust the minimum required vertical shape or energy. Do not raise
   the profile to clear an interior obstacle.
4. Estimate the powered transition from actual state to that future arc state,
   including displacement, gravity, attitude transition, fuel and deadline.
   Recompute the arc from the predicted join and make bounded consistency updates.
5. Rank nominal candidates without interior terrain. Convert and validate a
   bounded shortlist with supplied commands. Only then audit the selected
   executable proposal against terrain.

The complete trajectory begins at the actual current position/velocity and
includes acquisition. The subsequent ballistic coast begins at the predicted,
then physically validated join. Do not erase the prefix by reanchoring the
whole flight at that join or changing velocity instantaneously.

Ground start retains upright launch and source-plane constraints. Airborne
start may coast, brake, redirect or boost, but must not replay the grounded
launch schedule or obtain its source-pad clearance exception. Reuse the current
source fitter as an actuator-validation building block where appropriate, not
the entire old source-only selection algorithm as a second nominal definition.

A common function that simply dispatches to the old two generators is not this
change. Nor is enlarging the existing 56-row grid sufficient evidence of the
clarified admissibility and preserve-safe-motion semantics.

## Cheap acquisition estimates and their limits

For a proposed burn duration tau and desired velocity at its future endpoint,
start with a constant-average-acceleration model:

```text
a_thrust = (v_join - v_now) / tau - gravity_vector
p_join = p_now + 0.5 * (v_now + v_join) * tau
required_thrust = incoming_mass * length(a_thrust)
estimated_fuel = burn_rate_at_estimated_throttle * tau
```

Screen the vector thrust against the declared derated budget, not separate
full-thrust budgets for x and y. Estimate orientation delay, propagate motion
during it, and include subsequent turn/slew demand. Approximate fuel and delay
need explicit reserve; longer duration is not universally better because it
changes position, gravity loss and target overshoot.

These equations assume constant average net acceleration. They are planning
estimates, not physical certificates. [NASA's motion equations](https://www1.grc.nasa.gov/beginners-guide-to-aeronautics/equations-of-motion/)
state the constant-force/mass assumptions; its
[rocket acceleration explanation](https://www.grc.nasa.gov/www/k-12/VirtualAero/BottleRocket/airplane/rktalo.html)
accounts for gravity and warns about changing mass. We must compare the proposed
estimator with the actual mass/held-command plant before trusting its margins.

If velocity-only estimation leaves excessive position error, use the existing
closed-form affine bridge coefficients and endpoint thrust extrema. This joins
both position and velocity with no materialized tick trace. The current
`bridge_coefficients` / `primitive_precheck_failure` in
`pd-plan/src/conservative_ballistic_bridge.rs` provide reusable mathematics,
but their wrapper budgets use worst-case mass and source/terminal conventions;
they are not a drop-in airborne certificate. Keep the analytical model and
physical validation separate.

Bounded time/control parameterization is compatible with this approach;
[MIT's trajectory formulation](https://underactuated.mit.edu/trajopt.html)
fixes the initial state and distinguishes dynamics/input constraints from
trajectory objectives. A general optimizer is not required by this proposal.

The constructor should report which conditions needed correction. Feasible
candidates that preserve admissible vertical motion should not lose to one that
merely matches the minimum reference better. Then prefer less required
acquisition effort/extra shaping, acceptable mission time/fuel, and a stable
identity. Freeze the precise ordering at the characterization checkpoint;
never rank by terrain clearance, obstacle avoidance or a local row's suffix.

## Analytical check on the current misses

The [diagnostic data](waypoint_v2_nominal_handoff_analysis.json) use actual retained
policy 2 handoffs, not restored execution. The following is a point-mass scale
check for complete horizontal stopping, not a required entry velocity or a
proposed maneuver. Constant mass, no geometry/contact, no fuel validation and
no vertical support are assumed.

Let A be incoming `max_thrust / mass`, and let the comparison budget be
`0.81 * 0.925 * A`, using the existing source derating and robustness factors
only as an illustrative screen. It is not a newly chosen airborne policy.
Assume zero thrust while turning at maximum rate to horizontal braking, then
constant horizontal deceleration. Ignore incoming angular-rate transients.
Stopping distance is `v_x * turn_time + v_x^2 / (2 * acceleration)`.

| Actual handoff | Target distance m | Turn time s | Raw thrust stop distance m | Source budget stop distance m |
| --- | ---: | ---: | ---: | ---: |
| Late ridge | 288.850 | 1.000 | 235.859 | 289.042 |
| Late plateau | 276.025 | 1.000 | 235.859 | 289.042 |
| Successive rising ridges | 288.864 | 1.100 | 179.764 | 216.352 |
| Successive plateaus | 276.221 | 1.317 | 219.469 | 262.236 |

All four appear to have room under the raw-thrust model; the late pair become
marginal or short under the source-budget/coast-then-turn assumptions. Neither
column proves feasibility or impossibility. Braking during a turn, vertical
support, changing mass, body geometry and terminal capture can change the result.
In particular, apparent stopping room must not authorize flying through terrain.

The unpowered target-x crossing is approximately 228 to 330 m above the pad
surface in these inputs. That suggests studying lateral timing/braking before
assuming that every miss needs more lift. It does not establish an acceptable
entry or landing; the descent angle and speed remain coupled to capture timing.

This check motivates explicit turn/displacement modeling and cautious estimator
classification. It does not demonstrate that unification will recover all four
misses or that local ranking never needs revision.

## Work to keep and work to retire

Keep the reusable V2 loop, actual-state ledger, original clock/fuel/deadline,
independent whole-source replay, odd-contact coverage handling, unmodified
terrain, local position-plus-velocity handoffs and separate two-second guard.
The uncut direct controls and repeated plateau landing remain useful foundations.
Do not reopen the floor-cutaway or first-step-crash investigation without a new
reproduced defect.

Retire the 56-row airborne family as the intended final nominal model, exact
reference-profile matching as a general objective, and global shape rejection
that ignores acquisition versus coast versus terminal phases. Preserve those
old implementations and results as versioned controls; retire their authority
over the new design, not their code or evidence in this documentation pass.

Defer changes to local entry spacing, maneuver templates and ranking. The next
comparison must isolate the nominal constructor while retaining policy 2 local
behavior. Immediate landing after a local waypoint and a terrain feature's far
edge remain outside local acceptance.

## Staged execution plan

### First characterize the estimator and entry conditions

Build a deterministic analytical study of the eight retained clear starts, all
actual local handoffs in the final policy 2 suite, and the twelve historical
airborne captures. Include the four misses without giving them special controls.
Snapshots may be analytical inputs only; physical checks must reproduce live
states through their original commands, never restore them into an executor.

Add focused condition tests: already-safe steeper/higher arrival, safe descent
without a new apex, lateral miss with acceptable vertical motion, too-shallow
entry, excessive downward energy despite a steep angle, upward target, low fuel,
large attitude turn and overshoot. Synthetic cases are labeled separately from
retained physical handoffs.

Compare the constant-average estimate and the closed-form position/velocity
screen against neutral supplied-command propagation with interior contact
excluded from nominal ranking. Record time, position, velocity, fuel and angle
errors; distinguish estimated acceptance from physical acceptance. A screen
rejecting a case does not prove no feasible maneuver exists.

Exit this checkpoint with one specified terminal-entry region, angle measurement
location/minimum, energy/braking budget, estimator margin, nominal rank,
seed-selection rule and finite budgets. Establish their provenance from existing
landing behavior and the supported vehicle; do not use a new magic angle to
erase the four failures. Decide whether the simple estimator is adequate or the
closed-form screen is needed. Report unresolved physical cases honestly.

Initial budget proposal: at most 16 geometry/time seeds per cycle, three
consistency updates per seed and three executable-witness candidates. Keep the
existing finite fitter bound if reused. With at most seven cycles, this allows
336 analytical updates and 21 witness attempts, not an unbounded optimizer.
These limits are proposals to freeze at this checkpoint, not measured latency.

### Then review and freeze the implementation contract

Perform a separate primary review before starting a runtime writer. Check
one-sided acceptance, shared x/y thrust, future join position/velocity, incoming
attitude/rate/fuel, ground-only support, phase-specific shape/clearance rules,
finite failure labels and initial-terrain independence. No independent-agent
review is claimed by this document.

The review must resolve the numerical entry/budget choices and select a feasible
supplied-command realization. Entry-envelope acceptance alone cannot replace
landing proof. If the selected backend fails despite estimated entry acceptance,
record a model/backend gap rather than declaring the state physically impossible
or selecting a terrain-dependent alternative.

Nonfinite state, invalid binding, an unexpected command gap or disagreement
between claimed validated execution and independent replay remains an
ImplementationError. A matched finite witness rejection is not such an error.

### Implement an explicit new nominal policy after approval

Keep pure state/envelope/estimate contracts in pd-plan and actual command
generation/validation in pd-eval. Use one constructor interface for ground and
airborne inputs; reuse existing plant, bridge and body queries. Ground prep is a
mode-specific adapter, not a second target-selection definition.

Add an explicit new policy identity and CLI selection; do not repurpose versions
1 or 2 or silently change their default. The nominal acquisition is part of the
nominal segment, not a terrain waypoint or a new mission. Ground S denotes the
actual nominal acquisition end used by the unchanged entry formulas.

The generation interface must return a timed supplied-command proposal before
the existing V2 loop executes it. Preserve ledger intervals, held-pair cadence,
raw contact, deadline and replay ownership. Acquisition needs ordinary nominal
reserve and cannot claim the terminal contact corridor merely by its label;
only grounded initial prep gets the existing source exception. Ensure the guard
and query understand any new acquisition phase explicitly.

NoNominal remains a finite constructor miss with stage/reason diagnostics.
TerrainBlocked still requires an actual selected trajectory and terrain conflict.
Never insert a clearing waypoint just to hide estimator exhaustion. If no
realized candidate passes, preserve the incomplete actual state.

### Validate the complete loop on the unchanged suite

Use the same 32 expanded inputs, with eight clear controls, sixteen ordinary
obstacles and eight diagnostics. Keep 8/8 clear zero-correction landings,
13/16 ordinary and at least 2/4 per family; require the reference to land with
repeated corrections and at least twelve initially terrain-blocked ordinary
nominal proposals. Do not alter terrain or the denominator after measuring.

Keep six terrain corrections, the final seventh nominal opportunity, the
original min(scenario horizon, 80 s) deadline, unchanged local reserve/guard,
and the median <=2 s / nearest-rank p95 <=5 s planning target over all 24 common
attempts including failures. Count analytical construction and executable-witness
validation in planning time; report replay, execution and output separately.

New-policy clear commands and reference handoffs may differ because this is a
deliberately different constructor. They must preserve state continuity and
nominal identity across interior-terrain twins. Old modes/canaries must retain
their physical payloads; do not rewrite the old reference winner or use its
state as an execution seed. The first handoff need not be numerically identical
in the new policy, but the unchanged reference must still demonstrate repetition.

Repeat all complete decisions/commands/state/contact/replay payloads on final
source under named observational exclusions. Preserve the eight old canonical,
twelve airborne, sealed local-clearing and 24 source-rest controls. Run workspace
tests, strict Clippy, formatting and whitespace checks. No commit, push,
deployment or default promotion is implicit.

Use one predeclared new constructor policy for that matrix. Further tuning is a
new decision, not another automatic iteration of the exhausted old calibration
budget. If coverage still misses 13/16, report which stage dominates and decide
whether further investment is justified. A four-handoff or analytical-only pass
does not substitute for complete planner acceptance.

## Current review verdict and immediate next action

The original characterization completed with an integration no-go because its
ground family passed only 2/8 clear controls. The subsequent terminal-time
checkpoint below resolves that constructor blocker on the retained inputs,
without acquisition changes or reserve relaxation. Runtime integration and the
complete repeated-handoff gate remain separately planned work.

The [ground diagnostic](waypoint_v2_ground_diagnostic_results.md) is now complete:
all eight original flights reproduce; original terminal durations pass at their
actual entries, but all eight unchanged research braking-time references first
reject on lateral reversal. The three preselected shadow probes land; flat/uphill
require about 0.993 terminal throttle and still violate the 0.925 budget. Their
nominal rejection and the research 2/8 ground coverage remain unchanged.

The [separate primary review](waypoint_v2_ground_diagnostic_review.md) recommends
one bounded coupled-state terminal-time construction, retaining the reference
backend, physical checks and budget. Ground acquisition compatibility remains
unresolved; do not change acquisition simultaneously or weaken reserve on the
basis of shadow landings. Freeze a small terrain-blind candidate rule before
that next execution. Shared target/entry/preservation semantics do not demand
identical ground/air actuator templates or a universal analytical envelope.
This is not an approved runtime fix. Preserve airborne behavior; constructor
validation and the unchanged full 32-case runtime gate remain later approvals.

The [terminal time construction plan](waypoint_v2_terminal_time_plan.md) has now
completed under a new opt-in research identity. Retain a valid baseline time,
otherwise try two algebra-derived horizontal shapes under exact discrete
no-reversal and remaining-deadline limits. Acquisition, margins and three witness
attempts remain unchanged. The [measured results](waypoint_v2_terminal_time_results.md)
pass all eight proven-entry gates and recover 8/8 new actual ground landings,
while retaining 39/39 airborne witnesses and all four designated terrain
recoveries. Valid baseline screens and acquisition ledgers remain identical;
each airborne first-valid terrain/reserve classification is preserved.

The [separate primary review](waypoint_v2_terminal_time_review.md) accepts this
constructor checkpoint and recommends planning the smallest opt-in runtime
adapter next, rather than continuing terminal-time/acquisition research. Keep
the existing direct-first/terrain/local-clearing loop and old runtime paths;
distinguish constructor miss from obstruction. Freeze integration and failure
semantics before the unchanged 32-case repeated-handoff gate. Newly generated
handoffs and the 13/16 ordinary floor remain unmeasured by this corpus. No
runtime/default promotion or implementation of that later phase starts here.

The subsequent [simplified integration plan](waypoint_v2_airborne_integration_plan.md)
now recommends retaining canonical uncut ground launch and policy 2 local
clearing while replacing airborne nominal regeneration first. Research ground
8/8 remains valid evidence, but changing its startup and intervention landmarks
is not needed to address the measured runtime gap. This preserves shared
target/state/safety semantics without requiring identical actuator templates.
First-valid executable selection replaces materializing all three research
witnesses; the three-trial cap, tested family and terrain-independent choice
remain. The [planning review](waypoint_v2_airborne_integration_review.md) is ready
for separate approval, not a claim that policy 3 or new full-loop coverage exists.

## Pre-characterization design verdict

The clarified architecture is consistent with the intended direct-first loop
and existing state-continuity evidence. Cheap acquisition estimates are useful
construction tools; they are not yet calibrated safety or landing certificates.
The simple handoff screen exposes sensitivity to turn time and available thrust,
so recovery claims would be premature.

At the earlier design checkpoint, this plan was ready for the bounded
estimator/entry characterization and its
separate review. It is not yet a frozen numerical runtime implementation
contract: angle/entry location, coupled energy bounds, estimator errors,
ranking/seed details and executable realization remain explicit first-checkpoint
decisions. No Rust, runtime policy, fixture, local ranking or physical evidence
changes in this design pass.

## Historical design-pass validation

The four analytical rows reproduce from their retained handoffs to the recorded
three-decimal precision. The diagnostic binds its policy fixture and eight
flight/scenario files with verified SHA-256 hashes. All 80 relative links across
the affected documents resolve, and whitespace checks pass. The existing
frozen-suite structural/input checker passes again: 30 supported inputs, two
expected unsupported diagnostics, no simulation created and no flight acceptance
evaluated. It does not validate the proposed new constructor or entry envelope.

Before/after hashes match for 204 monitored Rust/manifest/top-level fixture/script
files and six retained evidence artifacts. The physical matrices, full workspace
tests and historical preservation runs above are existing evidence, not new
measurements in this documentation/analytical pass. No commit is made.
