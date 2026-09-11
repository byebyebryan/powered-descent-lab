# Conservative Ballistic Route Planning

## Status

The CB0 gameplay audit and private CB1 analytical canary gate are implemented
and complete. CB0 adds an evaluator-owned final-landing audit, while CB1 adds
only test-private `pd-plan` inputs and analytical evidence. Neither changes
production planner behavior, controller behavior, existing fixtures, or
held-out evidence. The bounded one-waypoint solver in CB2 is now the next
design-bounded capability checkpoint rather than a repair of existing missions.

On the exact current `36` maintained and `24` already-seen diagnostic inputs,
all `60 / 60` ordinary full-simulation runs reached the target pad and none
reported fuel depletion. An independent repeat produced the same summary bytes
and all case identities. The first stop rule therefore applies to claims that
the current corpus needs repair.

A subsequent product decision authorized one forward-looking capability path:
define intentionally synthetic game missions where a conservative direct
ballistic leg is unsuitable but a single staging gate is suitable. This does
not turn the clean CB0 rows into failures, reopen research-grade D2, or
authorize production integration. CB1 now establishes the route-necessity
canaries below without using controller outcomes. CB2 is authorized only as a
private bounded solver; later phases remain gated on its result.

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

## Route-Necessity Canary Contract

The existing `single_mid_ridge` fixture proves that the straight-segment
visibility planner selects one waypoint. It does not prove that a direct
ballistic coast needs staging: a ballistic arc can already loft over a narrow
obstacle. The new canaries must isolate the additional powered transition that
a waypoint provides.

For a one-waypoint-solvable canary, the frozen analytical policy must establish:

```text
accepted zero-waypoint candidates = 0
accepted one-waypoint candidates  > 0
```

The initial corpus is deliberately limited to four input-only cases:

| Canary | Required classification | Purpose |
| --- | --- | --- |
| `clear_direct_control` | at least one direct candidate passes; select zero waypoints | prove the policy does not add unnecessary staging |
| `long_span_capture_split` | every direct duration fails the source/terminal energy envelope; an authored midpoint staging gate passes | isolate the value of a powered energy reset without terrain |
| `late_ridge_capture_split` | every direct duration either violates terrain clearance or terminal capture; an authored gate near or just beyond the crest passes | prove a terrain-driven clearance/capture conflict can be repaired |
| `insufficient_authority_control` | neither direct nor one-waypoint candidates pass | prove the policy rejects instead of forcing a waypoint |

All four use the same vehicle and one frozen policy. Scenario geometry may
change, but no case may carry a private duration set, threshold override,
controller identity, route-family branch, or expected-outcome exception. Red
and green classifications must clear a declared nonzero robustness margin;
cases that sit on floating-point or policy thresholds are invalid canaries.

CB1 may carry the successful one-waypoint gates as authored property oracles.
Their coordinates are not planner goldens: they prove only that at least one
gate inside the policy exists. CB2 must rediscover a suitable gate from its
bounded, failure-guided candidate set. Every rejected direct duration retains
its decisive analytical reason, and every accepted one-waypoint result retains
separate source-leg, transition, target-leg, and terminal-capture margins.

Controller simulation does not define whether a canary is analytically red or
green. After the policy and solver are frozen, ordinary full simulation checks
whether each accepted route lands with the existing topology-compatible game
controller. A failed simulation can reject the route policy, but it may not be
fed back as a per-case planner threshold or candidate exception.

Target craters, source-side walls, broad mesas, payload/radius matrices, and
two-waypoint missions are deferred. They mix endpoint ownership, duplicate the
first terrain/energy interaction, or expand scope before the basic direct
versus one-staging-gate distinction is proven.

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

**Result (2026-09-10): complete; stop.** `pd-eval final-landing-audit` applies
only an exact `landing_on_pad` goal overlay, then validates the unchanged
scenario, route-plan, controller, manifest, and raw persisted execution bundle
for every case. The sealed result was:

- source D0 input: `fnv1a64:4f3b0eb97bce8209`;
- resolved maintained input: `36` cases,
  `fnv1a64:ba258fb11d2fc36e`;
- resolved diagnostic input: `24` cases,
  `fnv1a64:85236d37129f5efd`;
- terminal outcomes: `60` target touchdowns and no other terminal class;
- fuel-depletion diagnostic: `0`; and
- summary identity: `be08b90305456370`.

Two fresh output roots produced byte-identical `summary.json` files with SHA-256
`1bafe7028b7d51f7a27fada5c7c3136e9d3453842c7a99a16dba438de6e0df9f`.
The strict intermediate-contract shortfall is therefore not a current gameplay
landing failure. No current-corpus failure set authorizes planner work; CB1 is
separately authorized only by the forward-looking route-necessity decision, and
CB2 remains gated on CB1.

### CB1 — analytical kernel and canary gate

- Add a private `pd-plan` spike for discrete ballistic leg construction,
  per-step exact corridor clearance, state gates, and conservative transition
  checks.
- Freeze one policy and a tiny duration-candidate set before running any
  controller simulation.
- Materialize the four route-necessity canaries above as input-only test data.
- Evaluate direct candidates and authored one-gate property oracles, retaining
  decisive rejection reasons and component margins.

**Exit:** the direct control has a zero-waypoint survivor; both solvable cases
have zero direct survivors and at least one robust authored one-gate survivor;
the authority control has neither; every result and reason recomputes exactly;
and no controller type or configuration enters `pd-plan`. Otherwise stop and
revise the model before search work.

**Result (2026-09-11): complete.** The private, test-only CB1 kernel uses the
canonical Earth-gravity vehicle, four shared duration multipliers, exact
semi-implicit ballistic legs, and exact piecewise-linear corridor queries. Its
powered phases are explicit, non-overlapping source, intermediate, and terminal
polylines: coast legs meet those regions only at their entry or exit anchors,
so no route distance is counted as both exact coast and maneuver room. Powered
maneuver clearance inflates the vehicle corridor by the shared `10 m` gate
radius; only the upright source lift uses the hull corridor with an explicit
supported-pad release boundary.

| Canary | Direct survivors | Authored one-gate survivors | Decisive result |
| --- | ---: | ---: | --- |
| `clear_direct_control` | `4 / 4` | not applicable | direct green |
| `long_span_capture_split` | `0 / 4` | `8 / 16` | direct terminal capture fails; staged route passes |
| `late_ridge_capture_split` | `0 / 4` | `2 / 16` | direct terrain/terminal conflict; staged route passes |
| `insufficient_authority_control` | `0 / 4` | `0 / 16` | direct and staged authority reject |

Every accepted candidate has a minimum normalized component margin of at least
`0.10`, strictly above the declared `0.075` robustness threshold and the test's
`0.10` anti-threshold floor. Every direct candidate in either solvable-red case, and
every candidate in the authority control, is robustly rejected. The complete
duration cross-products, powered-path anchors and inflation, vehicle and policy,
component evidence, result ordering, and evaluation are identity-bound and
reject tampering.

The review rejected an earlier false-green draft because it lent whole coast
legs to powered transitions without shortening those coasts. The accepted CB1
model removes that double allocation. It remains a conservative
maneuver-room screen rather than command integration or a physical-feasibility
proof; CB3 still owns the real-controller landing claim after CB2 freezes a
solver.

### CB2 — bounded one-waypoint solver

- Reuse the existing graph and route validator.
- Add failure-guided staging candidates and an exact bounded reverse search
  over zero or one waypoint only.
- Emit candidate `TransferRouteSpec` values and private analytical diagnostics;
  do not replace production planning or change fixtures.

**Exit:** the solver chooses direct for the direct control, rediscovers a
policy-valid one-gate route for both solvable canaries without reading their
authored coordinates, and rejects the authority control. Search bounds,
ordering, and deterministic identities are explicit. Two-waypoint search does
not begin in this checkpoint.

### CB3 — frozen full-simulation validation

- Freeze the CB2 algorithm, policy, canary inputs, and analytical results first.
- Run the selected routes for the direct control and two one-waypoint canaries
  with the existing topology-compatible controller and ordinary full
  simulation.
- Require target touchdown for all three accepted routes and rerun the existing
  maintained gates as no-regression evidence. Treat intermediate waypoint
  contracts, tracking, clearance, fuel, and time as diagnostics.

**Exit:** advance only if the analytical acceptances reliably predict real
controller landings and the staged routes demonstrate practical value.
Otherwise retain the negative spike and stop; do not tune by route label or
individual outcome.

### CB4 — production integration decision

- Review whether successful private types belong in `pd-core` and whether the
  current policy stays available as a fallback/version.
- Add public algorithm and policy identities, digest coverage, resolver wiring,
  reporting, and focused regression fixtures only after that review.
- Keep runtime replanning, arbitrary waypoint counts, randomized terrain, and
  controller-specific branches out of V1.

**Exit:** the selected production planner is deterministic, bounded,
controller-independent in its inputs, and covered by ordinary landing evidence.

### CB5 — optional scope expansion

Only after CB4 is worthwhile, review whether two-waypoint search, target
craters, source-side walls, broad mesas, payload/radius variation, or a small
genuinely uninspected gameplay matrix has product value. Any broader matrix
measures accepted-route landing precision and conservative rejection; it is
not a revival of the broader D2 physical-feasibility program.

## Recommended Checkpoint

CB0 remains complete and establishes that the current development corpus needs
no route repair. CB1 now robustly distinguishes zero-waypoint,
one-waypoint-solvable, and honest-rejection cases. The next sensible
forward-looking checkpoint is **CB2 only**: reuse the private kernel in a
bounded zero/one-waypoint solver that must rediscover suitable gates without
reading the authored property-oracle coordinates.

This remains capability development for intentional future game content, not
remediation of the CB0 cases. CB3 remains a separate review gate because it
opens controller outcomes only after the planner inputs, policy, and solver are
frozen.

## Stop Rules

Stop or narrow the work when any of the following occurs:

- work is presented as repair of the current corpus despite the clean CB0
  final-landing audit;
- the frozen policy cannot robustly separate the direct and one-waypoint
  canaries without per-case thresholds;
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
