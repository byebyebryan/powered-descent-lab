# Conservative Ballistic Route Planning

## Status

This is a design-only checkpoint. It defines a simpler, game-oriented route
planning direction after the finite bounded-trajectory proposal catalog found
no development witnesses. It adds no planner behavior, controller behavior,
artifact schema, fixture, dependency, or held-out evidence.

The earlier [bounded trajectory witness](bounded_trajectory_witness.md) remains
a valid positive-only research contract, and its W4 result remains valid: the
finite proposal backend had zero useful coverage, not proof that any route was
physically impossible. D2 of the research-grade capability program remains
blocked. This document deliberately does not make another proposal backend the
next product milestone.

## Decision

The next planner experiment should be a deterministic analytical route policy,
not a simulated reference pilot and not another command-trace search.

For each mission it will:

1. construct a conservative source-acquisition gate and terminal-capture gate;
2. try a small fixed set of direct ballistic coast legs between those gates;
3. disprove each candidate with exact terrain-clearance and conservative
   source, energy, turn, capture, time, and fuel screens;
4. if no direct candidate survives, introduce a small deterministic set of
   staging-gate candidates associated with the decisive failures;
5. search backward from the terminal gate for a composable chain with at most
   two emitted waypoints; and
6. return the easiest surviving route under explicit margin-first ranking.

The analytical result is a planner-policy decision: the route appears easy to
execute under the declared vehicle envelope. It is not a controller-independent
physical-feasibility proof. Ordinary full simulation with the real controller
remains the authority for landing, contact, fuel use, and timeout.

This gives the desired separation without inventing a stronger implication
than the evidence can support:

- `pd-plan` may use world, vehicle, initial-state, pad, and versioned policy
  inputs;
- it may not read a controller ID, controller configuration, phase marker,
  recorded outcome, route-family label, or seed-specific exception;
- `pd-eval` may later test a frozen planner against the real controller; and
- “a real controller will most likely pass” becomes a measured validation
  result, not an assumption embedded in the planner.

## Product Question

The product question is intentionally narrower than the earlier physical
witness question:

> Can the setup-time planner produce a bounded, terrain-clear route whose
> transfer legs and transitions are conservative enough that the maintained
> game controller usually lands it?

The intended use is route generation for a deterministic 2D video game. The
planner does not need to prove arbitrary physical reachability, synthesize
120 Hz commands, or cover every feasible mission. Conservative rejection is
acceptable. Accepting routes the real controller routinely cannot fly is not.

## First Check: Is There Still a Gameplay Problem?

The strict waypoint-contract expansion is not itself a final-landing failure.
Before implementing a new planner, reconstruct ordinary `landing_on_pad` runs
for the exact current `36` maintained and `24` already-seen diagnostic physical
inputs while preserving their resolved routes, controller specs, and other
execution inputs.

This audit is the first stop rule:

- if the current tree lands all relevant cases safely, do not build this
  planner merely to improve an intermediate checkpoint score;
- if landing failures remain, classify contact, timeout, fuel, and terminal
  outcomes before attributing them to route geometry; and
- proceed only where a simpler route or staging gate can plausibly remove the
  observed failure.

Historical results suggest this audit may collapse the problem substantially,
but historical output is not current acceptance evidence.

## Planning Model

### Normalized frame and endpoints

Planning keeps the existing normalized frame: source-to-target horizontal
progress is positive, and every generated gate is strictly forward ordered.

The ballistic problem does not run from pad center to pad center. Its endpoints
are:

- a **source-acquisition gate** above the source pad, reached by a simple
  upright departure reserve; and
- a **terminal-capture gate** above and on the approach side of the target,
  with a deliberately slow arrival envelope suitable for terminal recovery.

These gates are planner policy, not copies of current controller thresholds.
Their values must be versioned and validated against generic vehicle limits.
The source and target pads remain the route identities emitted to the existing
`TransferRouteSpec`.

### State gates

A staging point must be more than a position. Internally, each candidate gate
needs enough state to compose adjacent legs:

```text
BallisticStateGateV1
  anchor position
  capture / maneuver radius
  outbound tangent
  minimum and maximum outbound speed
  maximum outbound cross speed
  minimum and maximum vertical speed
  conservative transition-distance reserve
```

An accepted intermediate gate compiles down to the existing
`TransferWaypointSpec` position, capture radius, tangent, cross-speed, total-
speed, progress, and vertical-speed fields. The first spike should keep richer
state-gate data private to `pd-plan`; a new public serialized type is justified
only if the shadow experiment succeeds.

Attitude and angular rate do not need to become waypoint-contract fields in the
first version. Their cost is represented by a worst-case slew reserve in the
transition screen. If that reserve is too coarse to distinguish obvious
canaries, the spike stops rather than importing controller state into planning.

### Exact discrete ballistic coast

During a coast the current plant has constant gravity, no drag, and no
horizontal acceleration. With physics step `dt`, gravity `g`, and `N` discrete
steps, the authoritative semi-implicit update gives:

```text
x_N  = x_0 + N dt vx_0
y_N  = y_0 + N dt vy_0 - 0.5 g dt^2 N(N + 1)

vx_0 = (x_N - x_0) / (N dt)
vy_0 = (y_N - y_0) / (N dt) + 0.5 g dt (N + 1)
vy_N = vy_0 - g N dt
```

The planner chooses `N` from a small, fixed, versioned set derived from the
leg span and policy speed bounds. It does not optimize continuous time. Every
candidate records its exact departure velocity, arrival velocity, apex, and
step count.

Terrain clearance can initially reuse the existing exact piecewise-linear
corridor query on each adjacent pair of analytical step positions. The coast
arc is concave downward, so the chord between consecutive positions lies at or
below the corresponding continuous parabola; clearing the lower chord is a
conservative check. This is simpler than introducing another sampled tube or a
new polynomial-clearance oracle. Coalescing intervals is a later performance
optimization only if profiling requires it.

### Conservative powered transitions

Power is modeled only at source acquisition, intermediate redirect gates, and
terminal capture. V1 uses simple bounds rather than a reference controller:

```text
conservative_thrust_accel = thrust_derate * max_thrust / worst_case_mass
omnidirectional_net_accel = conservative_thrust_accel - gravity
```

The policy is out of scope when `omnidirectional_net_accel <= 0`. For each
transition, the screen accounts for:

- the velocity change between the incoming and outgoing ballistic legs;
- a worst-case or gate-bounded attitude-slew delay;
- stopping and redirect distance under `omnidirectional_net_accel`;
- an inflated terrain-clear maneuver region;
- an upper bound on full-throttle burn time and fuel; and
- a remaining mission-time reserve.

Source acquisition is a distinct upright-lift primitive before any lateral
slew. Terminal capture ends at the conservative terminal state gate rather than
pretending that a ballistic coast lands on the pad. Intermediate gates reserve
space to redirect before the emitted waypoint handoff is considered complete.

These inequalities may reject flyable routes. That is acceptable. They may not
claim exact fuel use or exact controller behavior.

## Bounded Backward Search

The user's backward-repair idea is retained, with one guard against a brittle
greedy implementation.

Search starts from the terminal state gate and constructs only suffixes that
can still be captured. It first evaluates all fixed-duration direct candidates.
When they fail, the first decisive failure determines which finite staging
candidates are tried first:

| Failure | Repair candidates |
| --- | --- |
| terrain clearance | lofted gates around the blocking terrain breakpoint |
| excessive arrival energy | an upstream or higher braking gate |
| excessive turn / invalid angle | a slower transition gate before the bend |
| source acquisition | the fixed source staging alternatives |
| fuel or mission time | reject; another waypoint is not an automatic repair |

Failure guidance affects ordering, not soundness. Within the declared candidate
and two-waypoint bounds, the planner evaluates the finite alternatives rather
than committing to the first locally plausible repair. This avoids three common
greedy failures: choosing the wrong side of an obstacle, leaving no room for the
next capture, and missing a valid two-gate chain.

The current terrain-breakpoint graph, endpoint windows, exact corridor
machinery, monotone-progress rule, and two-waypoint cap should be reused. The
new work is the state-bearing ballistic edge and transition composition, not a
second unrelated graph planner.

## Selection Policy

Only candidates that pass every hard analytical screen enter ranking. Ranking
then prefers controller ease:

1. greatest minimum normalized safety margin across clearance, energy,
   redirect, capture, fuel, and time;
2. fewer waypoints;
3. lower total transition delta-v;
4. shorter route and lower excess loft; and
5. stable candidate identity.

This intentionally differs from the current geometry-first comparator, which
cannot see whether a candidate's transitions are easy. The margin formulas and
normalization are versioned policy. Candidate outcomes from the real controller
may validate or reject a frozen version, but may not change ranking within the
same run.

## Result Semantics

A successful result means:

> `planned/conservative_ballistic_policy_v1`: a route inside the bounded search
> passed every declared analytical policy screen.

A rejection means only that no route survived this policy. It never means the
mission is physically impossible. Diagnostics retain the first decisive reason
and phase, using categories such as:

- invalid or out-of-scope request;
- no fixed-duration coast candidate;
- terrain clearance;
- source acquisition;
- transition authority or angle;
- terminal capture;
- fuel or mission-time budget;
- route-complexity limit; and
- bounded search exhausted.

The production-facing rejection can continue to use the existing broad planner
codes until the experiment demonstrates that finer persisted codes have value.

## Evidence Boundary

The planner remains independent of `pd-control`. Validation deliberately uses
the real controller because the product claim is about useful game routes:

```text
world + vehicle + mission
          |
          v
  frozen analytical planner ----> TransferRouteSpec
                                      |
                                      v
                       existing controller + full simulation
                                      |
                                      v
                    landing, clearance, fuel, time evidence
```

The same controller on the same resolved physical case should execute both the
current and candidate plans. Final landing is the primary result. Waypoint
contract outcomes, tracking error, saturation, clearance, fuel, and elapsed time
are secondary explanations.

This is not D2's research-grade held-out feasibility claim. A small frozen
gameplay validation matrix is enough to decide whether the route policy is
useful. Broader held-out work is optional after product value is demonstrated.

## Execution Plan

### CB0 — current gameplay audit

- Reconstruct ordinary final-landing evaluation for the exact current 36-case
  maintained and 24-case already-seen diagnostic inputs.
- Verify physical input, route plan, and controller identity rather than
  relying on old branch output.
- Classify every non-landing before blaming the planner.

**Exit:** either stop because the gameplay objective already passes, or publish
a concrete set of current landing failures for which route shape is a plausible
cause. No planner code changes.

### CB1 — analytical kernel spike

- Add a private `pd-plan` spike for discrete ballistic leg construction,
  per-step exact corridor clearance, state gates, and conservative transition
  checks.
- Freeze a tiny duration-candidate set before running controller comparisons.
- Cover unobstructed direct transfer, one blocking ridge, excessive arrival
  speed, invalid redirect, insufficient authority, fuel/time exhaustion, and
  deterministic repeat canaries.

**Exit:** obvious easy cases pass, obvious impossible-under-policy cases reject
for the correct reason, all accepted witnesses recompute exactly, and no
controller type or configuration enters `pd-plan`. Otherwise stop and revise
the model before search work.

### CB2 — bounded backward route spike

- Reuse the existing graph and route validator.
- Add failure-guided staging candidates and an exact bounded reverse search
  over zero, one, or two waypoints.
- Emit candidate `TransferRouteSpec` values and private analytical diagnostics;
  do not replace production planning or change fixtures.

**Exit:** synthetic direct, one-waypoint, and two-waypoint missions resolve
deterministically; unsafe and out-of-scope canaries reject; search bounds and
ordering are explicit.

### CB3 — already-seen full-simulation shadow comparison

- Freeze the CB2 algorithm and policy first.
- Compare current and candidate routes on the CB0 cases with the same real
  controller and ordinary full simulation.
- Require zero maintained landing regressions. If CB0 exposed current landing
  failures, require at least one additional landing without introducing a new
  failure. Treat intermediate contract scores as diagnostics, not the primary
  gate.

**Exit:** advance only if the analytical acceptances reliably predict real
controller landings and the new plan has practical value. Otherwise retain the
negative spike and stop; do not tune by route label or individual outcome.

### CB4 — production integration decision

- Review whether successful private types belong in `pd-core` and whether the
  current policy stays available as a fallback/version.
- Add public algorithm and policy identities, digest coverage, resolver wiring,
  reporting, and focused regression fixtures only after that review.
- Keep runtime replanning, arbitrary waypoint counts, randomized terrain, and
  controller-specific branches out of V1.

**Exit:** the selected production planner is deterministic, bounded,
controller-independent in its inputs, and covered by ordinary landing evidence.

### CB5 — optional broader validation

Only after CB4 is worthwhile, freeze a small genuinely uninspected gameplay
matrix to measure accepted-route landing precision and conservative rejection.
This is confidence-building for the game planner, not a revival of the broader
D2 physical-feasibility program.

## Recommended Checkpoint

The next sensible checkpoint is **CB0 only**. It is cheap, answers whether the
gameplay problem still exists under final-landing semantics, and can prevent
another unnecessary planner spike.

If CB0 finds route-caused landing failures, CB1 and CB2 form the next coherent
implementation loop. CB3 is a separate review gate because it opens real
controller outcomes and decides whether the frozen analytical model was useful.

## Stop Rules

Stop or narrow the work when any of the following occurs:

- the current final-landing audit is already clean;
- failures are controller/contact bugs rather than route-shape problems;
- the analytical kernel needs controller IDs, phase names, or per-route
  exceptions to classify basic canaries;
- useful coverage requires more than two waypoints or unbounded duration
  search;
- accepted routes regress maintained landings;
- the new planner changes only strict checkpoint scores without improving
  landing, safety, fuel, or time; or
- the candidate needs repeated outcome-guided retuning to appear useful.

These rules make “one path that could work” the goal and keep the earlier
research program available without allowing it to dominate the next gameplay
milestone.
