# Waypoint Planning V1

## Implementation Status

Implementation phases 1-5 are complete. `pd-core` owns the serialized planning
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
phase composition, and a blinded implementation sequence. It remains
unimplemented and does not reopen V1 closure.

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
