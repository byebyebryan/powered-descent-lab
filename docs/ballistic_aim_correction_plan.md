# Ballistic aim and correction plan

Date: 2026-10-08. Status: first bounded construction pass complete; full campaign
deferred. The [construction results](ballistic_feedback_results.md) retain the
opt-in candidate, positive direct/correction controls and negative waypoint
verdict. The stages below remain the reviewed design, not a claim that every
validation/integration requirement has passed. Publication is not authorized.

The subsequent [bounded replan pass](ballistic_replan_results.md) implements
active-waypoint replacement and same-goal reacquisition, with 385/1000 on the
original worlds and no losses of the first candidate's 360 wins. That completes
the bounded loop fix, not the full acceptance stages below. Short-triggered
correction recovery and continuation/exit-state capability remain open.
The [terrain-aware correction follow-up](ballistic_terrain_correction_results.md)
now implements same-goal clearance recovery separately from arc-blocked waypoint
selection. Its 401/1000 preserves all 385 earlier complete successful flights.
This closes that bounded implementation pass, not the full acceptance stages.

The planner should aim for a ballistic transfer, correct toward it from the
actual state, and reassess during correction. If terrain blocks the target arc,
insert a local pass-through waypoint and aim for that instead. Landing contributes
simple approach safeguards; it does not supply a long powered transfer that
makes an otherwise misdirected ballistic trajectory acceptable.

This supersedes the earlier conversation draft's terminal-tail restrictions,
coast-entry search and complete acquisition/coast/landing program construction
as the proposed fix. Historical captures and their original result records remain
unchanged. The [planning-cycle review](terrain_planning_cycle_review.md) supplies
the diagnostic context; the departure lift/advance proposal remains parked.

## Current gap

Mission `715` reaches actual H1 at 20.67 s, position (323.71, 195.54) m and velocity
(30.54, 44.77) m/s. Its saved next proposal has no initial acquisition burn: it
coasts for six seconds, then uses about 33 seconds of powered terminal flight to
reach the destination. At powered entry, x is about 507 m; the target is at
x=1200 m. This is a proposed continuation, not the route actually flown.

The current zero-acquisition construction records its virtual target miss but
can still admit the candidate through the powered endpoint solver. Executable
landing evidence therefore does not prove that each transfer was ballistic
and aimed at the target. Also, the current airborne runtime rejects powered
incoming states and audits fixed command programs. Rechecking during correction
requires a genuine feedback execution path, not relabeling that constructor.

## Simple planning loop

1. At launch, handoff or a scheduled correction refresh, use actual position,
   velocity, attitude, held command, mass/fuel and the original clock. Compute
   a terrain-blind ballistic aim toward the active goal and its required velocity.
2. For the destination, choose the lowest adequate vertical profile when shaping
   is needed. Preserve an already-safe higher or steeper profile; lateral
   correction can change timing, so reassess the combined approach afterward.
3. Check the resulting ballistic arc against terrain. A genuine obstruction
   selects a local clearing waypoint, then the same aiming logic targets that
   waypoint. Do not raise the destination arc because an obstacle blocks it.
4. Apply bounded powered velocity correction. Periodically recompute the aim
   from the new actual position and velocity, rather than chasing a departure
   velocity calculated at an obsolete origin. Cheap estimates need not exactly
   predict the whole acquisition, but must include gravity and actuator limits.
5. When current motion already gives an acceptable ballistic continuation,
   stop correcting and coast. Reassess at waypoint handoff or when validity
   changes. Enter landing execution only at an accepted destination approach.

Keep the active waypoint stable unless its trajectory becomes invalid or its
handoff passes. A routine aim refresh is neither a new waypoint nor an executed
correction handoff. Local admission still needs body clearance, useful progress
and a finite safe continuation from the actual state, not a landing suffix.
Waypoint arrival is pass-through: do not demand zero velocity or landing angle.
If its arc is blocked again, another local waypoint may be needed within the
unchanged correction budget.

Keep a missing feasible aim distinct from a terrain-blocked aim. Fuel, time or
actuator exhaustion is not an obstacle and must not automatically insert a
waypoint. A finite construction miss is not proof of physical impossibility.

Ground launch retains its source-support and departure safety rules. Ground and
airborne flight share aiming semantics, not necessarily identical actuator
templates. No instantaneous velocity assignment, grounded exception in midair,
snapshot-restoration execution or externally supplied arbitrary-start frontdoor.

## Landing safeguards

Apply these to the predicted approach near the destination, not merely to a
current state that is somewhere above target height:

- Enter the landing region from above while descending; never approach from below.
- Avoid shallow side entry with a simple minimum downward approach angle.
- Preserve an acceptable steeper approach instead of forcing a preferred arc.
- Keep a cheap speed/braking-demand bound so a steep but unrecoverably fast
  arrival is not accepted on angle alone.

Before implementation flights, declare the approach evaluation location, angle
rule and approximate braking bound together, using the maintained landing
capability as the basis. These are game-oriented admission checks, not proof of
a full landing. Intermediate waypoints do not inherit destination landing rules.
Landing execution owns braking and touchdown under the existing physical landing
predicate. Do not introduce a landing-suffix search, coast-percentage threshold,
fixed terminal-distance limit or terminal-shape optimizer.

### First candidate's frozen development controls

The first opt-in `ballistic_feedback_v1` candidate uses the existing 120/60 Hz
clock, 24-tick (0.2 s) aim refreshes, and a physically propagated 24-tick held
command check before issuing the next two-tick update. Active goal and absolute
arrival/cutoff clocks remain stable between refreshes. First test the current
unpowered continuation: acceptable motion must command zero thrust, including
descending motion whose apex is in the past. A refresh is not a new waypoint.

Destination admission evaluates the downward angle at the unpowered target-COM
height crossing, using the existing transfer minimum of 45 degrees. Lateral
miss tolerance uses half the pad's usable body-centre width. Approximate braking
room is evaluated at the first descending state (apex or current state), using
the existing derated horizontal braking estimate and a vertical stopping-room
screen. These checks do not certify a landing. Actual descending landing entry
uses an explicit adapter to the unchanged maintained terminal controller; entry
requires an already accepted ballistic continuation.

Finite correction estimates retain gravity, turn duration, paired post-burn
mass/throttle inversion and a constant-acceleration burn followed by coast.
Select the shortest admissible burn at a terrain-blind time/profile, with at
most 32 later half-second profile trials on dynamics/approach rejection. Terrain
rejection cannot advance that profile search. Intermediate goals have no
landing angle or zero-velocity constraint. Their passage requires actual forward
progress and the existing two-second body-clear continuation, not a landing suffix.
The waypoint arrival region allows rising or already-higher motion; it is not a
descending height-crossing predicate. Retained correction clocks require turn
time to remain consistent within one two-tick command interval. Upright source
clearance invalidates the earlier acquisition estimate before the first tilt.

Development allowance: at most 24 retained diagnostic attempts before source
freeze (four neutral `715`, six original-terrain `715`, the eight-world review
panel, two `715` repeats and four construction controls). The two unused neutral
slots are reassigned to final-source flat/uphill controls after those checks
exposed the paired-tick geometric touchdown-query endpoint. The total remains 24;
no population inputs or expected outcomes change. Each attempt has a
60-second wall limit. Ordinary finite stops remain evidence; no measured sweep
starts until the correction/cutoff and source-replay construction checks pass.
The full regression ledger, set inputs, binaries and source identities must be
sealed separately before submitting its four-worker campaign, with a two-hour
campaign bound. Existing frozen set contracts supply controls/repeat ceilings.
No default promotion, accepted-report replacement, commits or pushes are included.

## Implementation and review stages

### Stage A resolve the execution boundary

Keep this as one bounded evaluator candidate with a new explicit source/behavior
identity. Preserve ordinary defaults and historical readers until acceptance.
Review three integration decisions before measured flights:

1. A minimal ballistic aim and velocity corrector using existing discrete math
   and command allocation, with one declared refresh cadence and correction
   tolerance. Refresh must accept the actual powered state. Throttle, minimum
   burn, rotation, fuel and deadline guards still govern issued commands.
2. A concrete waypoint goal and actual handoff test using the existing local
   clearance/progress/continuation rules where applicable. The present local
   maneuver's predicted H cannot simply be renamed a new ballistic waypoint
   or treated as an already-executed arrival.
3. An explicit adapter/handoff to maintained landing logic. The current complete
   `terminal_bridge` proposal is not that adapter; do not silently retain its
   long powered transfer or retune the landing controller in this pass.

Pure aiming math belongs with the existing ballistic helpers. The evaluator
session owns live state, active goal, correction/coast/landing lifecycle and
decision recording. Keep the game-host/per-tick API and a standalone planner
crate out of scope.

Check the actual short powered correction before committing commands, not just
the ideal arc. A predicted arc is not a collision certificate for the burn used
to establish it. Define the short command horizon and body-reserve query in the
review, and keep every query separate from actual execution.

Record each refresh's actual origin, goal, desired velocity/arc, decision reason,
issued commands and resulting state. Preserve exact command ordering and source
replay. Old complete-proposal audit metadata cannot certify new feedback flight;
new evidence must bind the decisions and commands actually executed. The normal
handoff and optional early-exit paths must use the same aiming contract, with
queued decisions revalidated against their exact actual origin.

### Stage B demonstrate correction on mission 715

Reconstruct the retained original prefix to H1 and match its complete state.
First demonstrate an executable aim/correct/recheck continuation in terrain-neutral
conditions, preserving target geometry. Then run the unchanged real-terrain
mission from its original source. The neutral comparison is diagnostic, not a
new terrain input in any regression population.

Show the desired arc at H1 and after correction refreshes, actual drift, remaining
velocity error, active waypoint and any destination landing handoff. Success
requires an established target-directed ballistic continuation, not merely
rejecting the old powered tail. Real terrain may require another waypoint; a
direct landing on `715` is not the construction test. A bounded constructor miss
must be explained as such, not called physical impossibility.

### Stage C validate and freeze

Test already-correct motion, lateral misses, safe higher/steeper motion, unsafe
shallow approach, flat/uphill/downhill endpoints without cutaways, rising/apex/
descending handoffs, powered refresh drift, waypoint stability and repeated
handoffs. Include already-valid landing entry, fuel/deadline/rotation rejection,
terrain twins with identical destination aims, actual correction clearance,
decision/command tampering and deterministic replay.

Use `715/280/983/327/999/000/030/565` as the fixed initial review panel, not as
case-specific code branches. The full campaign also covers the portable ten-case
diagnostic pack. Run the maintained developer gate and relevant terrain-tooling
tests. Review the actual diff and common rich views, then freeze source, policy,
cadence, safeguards, inputs, binary and a finite attempt ledger before collection.

## Full regression campaign

Retest all the following frozen sets. Keep each denominator separate; diagnostic
cases can overlap the 1k population and are not additional independent worlds.

| Set | Primary cases | Comparison purpose |
| --- | ---: | --- |
| Maintained planner pack | 44 | 36 mandatory landings and eight separate diagnostics |
| Portable terrain diagnostics | 10 | Departure, acquisition, progress and successful comparisons |
| Original easy random terrain | 100 | Direct-flight sanity preservation |
| Original harder development terrain | 100 | Earlier terrain regression coverage |
| Full original procedural population | 1000 | Paired comparison to 817/1000 |
| Separate observed validation population | 100 | Paired comparison to 77/100 |

The [1k baseline](terrain_early_exit_sweep_results.md) and
[100-world validation baseline](terrain_departure_probe_results.md) remain
receipt-pinned. These sets are already observed; rerunning them is paired
regression, not fresh held-out validation. Copy scenario inputs unchanged and
report new initial classifications alongside the original cohorts.

The paired 1k and separate 100-world comparison retain cap 24 and the existing
early-exit timing/guard policy. Normal maintained-pack acceptance retains cap 6.
The new aiming logic necessarily changes continuations and may change realized
early exits; it must not silently introduce another exit search or relax guards.
Do not promote a correction cap or change vehicle, gravity, clocks, terrain,
pad preparation, landing predicate or physical controller tuning alongside this fix.

Predeclare controls and repeats using the existing set contracts, adding `715`
to repeat coverage. Pin capture/binary/source identities, four-worker collection,
case/campaign wall limits and all attempt ceilings in the new ledger. Run the
44-case native and real CLI paths with a same-source repeat and saved-source
CLI replay. Run maintained terminal/trajectory-error, direct-transfer and authored
waypoint smoke gates; broaden those controller matrices if shared controller
implementation changes.

Finish the fixed populations despite ordinary finite misses or regressions.
Integrity/replay errors, source/input drift, infrastructure failures and exhausted
wall bounds stop further submission and retain all launched evidence. Do not tune
or retry a mission inside a source-frozen campaign. A repair needs a new source
seal and explicit remaining-attempt plan, never a mixed-source capture.

## Acceptance and report review

Judge both ballistic-first compliance and actual mission outcomes. Require:

- Recorded aims and refreshes show correction from actual state toward an
  acceptable ballistic transfer, with no powered landing substitute for a miss.
- Complete physical/mission/integrity/source-replay evidence for claimed landings;
  all 36 core pack landings, with its clear/corrected and diagnostic rules intact.
- Exact candidate native/CLI/repeat agreement apart from declared wall timings.
  Old full-flight numerical parity is not a gate for this intentional logic change.
- Clear-control preservation, all gained/lost landings, per-recipe/cohort results,
  actual failures versus finite planning stops, waypoint counts, fuel and planning
  cost. No reinterpretation of rejected queries as actual crashes or handoffs.

The proposed usability target remains at least 817/1000 and 77/100, preserving
the old clear landing cohorts. Missing these targets is a negative promotion
verdict, not a reason to restore the powered-transfer shortcut, weaken safeguards
or fit exceptions to inspected seeds. Every lost landing needs explicit review
even if aggregate gains offset it; perfection on arbitrary terrain is not required.

Extend the common detailed report with active goal, desired ballistic arc,
correction refreshes, waypoint handoffs and landing entry. Distinguish current
unpowered projection, desired arc, short command prediction, actually flown
commands and counterfactual/query traces. Keep rich plots, batch trees and report
navigation. Candidate captures/previews must not replace accepted pages or
selectors without a separate publication decision.

Finish with a reviewed implementation/validation verdict and the explicit list
of remaining mechanisms. This plan does not authorize commits, pushes, delegation,
server operations, cleanup of retained outputs or default/report promotion.
