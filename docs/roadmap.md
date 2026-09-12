# Powered Descent Lab Roadmap

This roadmap began as the design-first reboot sequence. It now tracks the
implemented phase boundaries, maintained evidence, and the next work above the
closed guidance baseline.

## 1. Reboot Goals

The reboot should produce a lab that is easier to evolve than `pylander`, not a
line-by-line rewrite.

Success means:

- clear project boundaries
- a native-first architecture
- scenario and telemetry models that survive multiple controller generations
- a migration path that uses `pylander` as a source of concepts and behavior
  references rather than as code or structure to port

## 1.1 Current status

Current implementation status:

- Phase 0 is complete.
- Phase 1 is functionally complete for the first usable landing slice.
- Phase 2 is complete for the maintained guidance workflow; future evaluator
  and report additions are evidence-driven extensions rather than closure
  blockers:
  - shared controller kit and multiple built-in controller styles
  - seeded packs and native multithreaded batch execution
  - single-run and batch static reports
  - lane-compare and external compare reporting
  - a curated terminal bot-lab suite as the main controller workbench
  - a first serious native terminal controller lane, `terminal_pdg_v1`
  - projected trajectory-error packs over the same maintained Earth terminal
    matrix
  - cache reuse / promotion / current-lane history compare for batch reports
  - analytic impossible-run classification for clearly unrecoverable terminal
    cells
  - scored authority-frontier annotations for low-thrust/high-energy cells
  - default thresholded regression policy over batch comparisons, scoped to the
    preferred current controller lane when both reports contain one
- Phase 3 guidance is complete over the maintained direct-transfer and
  preplanned-waypoint corpora; waypoint-planning implementation phases 1-5 are
  complete:
  - `timed_checkpoint` remains available as an early-termination contract probe
  - `signed_route_arc_transfer_v1` now exists as the first source-to-target
    matrix family
  - `transfer_pdg_v1` provides the first staged launch/boost/coast/terminal
    handoff controller
  - direct transfer is clean across the maintained route-angle/radius matrix
  - `transfer_waypoint_pdg_v1` closes terrain-blind waypoint guidance v1 over
    the preplanned maintained turn and ordered corpora
  - full-seed nominal waypoint contracts and landings are clean at `540 / 540`
    turn runs and `180 / 180` ordered runs
  - all-radius waypoint contracts are clean at `405 / 405` turn runs and
    `135 / 135` ordered runs; paired landings are also `405 / 405` and
    `135 / 135`
  - bounded final authority-recovery search closes the former short-radius
    landing residual
  - `pd-core` now owns neutral planner contracts, exact corridor queries, and
    all-leg route validation; `pd-plan` implements the deterministic bounded
    pad-to-pad search without a `pd-control` dependency
  - `pd-eval` resolves the focused generated-route matrices before simulation,
    includes complete plan identity in exact cache reuse, and persists schema-36
    planner and compute evidence for `pd-report` and batch reports
  - retained focused captures close at `54 / 54` landings and `36 / 36`
    handoff/ordered contracts with zero invalidations; every maintained
    no-regression pack reproduced its declared baseline
  - the first nominal-radius `r-60 | r+60` expansion remains diagnostic rather
    than accepted: its disposable local contract snapshot closed at `11 / 24`,
    and the research-only center-to-center trajectory-tube model failed the
    maintained-baseline preservation gate
  - the source-departure/acquisition and route-execution evidence prerequisites
    are implemented through D0a/D0b: all `60` development cases pass cadence
    parity, produce complete neutral source and route artifacts, and replay
    deterministically with zero invalidations; D1a/D1b and the R1 bounded
    candidate-replay diagnostic are complete, and the post-R1 positive-only
    [bounded trajectory witness V1](bounded_trajectory_witness.md) W1-W4 gates
    are implemented; the full finite catalog found zero supported development
    witnesses. Research-grade D2 stays blocked. The evaluator-owned CB0 audit
    in
    [Conservative Ballistic Route Planning](conservative_ballistic_route_planning.md)
    is complete: the exact `36` maintained and `24` already-seen diagnostic
    inputs produced `60 / 60` target touchdowns and zero fuel-depletion
    diagnostics in two byte-identical summary runs. That closes claims of a
    current-corpus repair need. The forward-looking V2 checkpoint now combines
    a private-by-default, controller-independent direct-ballistic bridge
    certificate with one bounded ridge canary. The original direct rows remain
    `clear_direct_probe` `3 / 4`, `long_span_probe` `3 / 4`,
    `long_range_probe` `3 / 4`, and `ridge_probe` `2 / 4`. Above them, a flat
    twin selects the shortest robust `1.0x` lane, an optimistic physics-derived
    post-commit envelope bounds local correction, and a derived broad mesa
    blocks the exact center-crossing cut. A finite terrain-derived search finds
    a forward-progressing one-waypoint certificate while `1.25x` and `1.5x`
    remain green global-replan diagnostics. This is a scoped nominal-direct-red/
    one-waypoint-green result, not universal direct infeasibility. The
    recomputable setup projection feeds dedicated HTML/SVG reports without
    running a controller or simulation. The single canary is now frozen behind
    focused correctness and visual gates. The corrected frozen full-controller
    shadow now supplies all three scoped lanes: the flat twin lands, the mesa
    direct twin makes reconstructed rotated-hull contact with the derived left
    wall, and the unchanged mesa-waypoint controller passes its waypoint
    contract and lands. An evaluator-scoped composed preflight selects an exact
    traversed intermediate-bridge state rather than the virtual ballistic
    anchor, while retaining the ordinary validator's identical zero-/one-
    waypoint source-taper rejection as a separate diagnostic. The subsequent
    feature-gated experimental candidate API now preserves this finite policy,
    exact crossing, deterministic ordering, and fail-closed reasons in
    `pd-plan`; the evaluator consumes it without report-shape coupling. CB3 is
    design-closed, with implementation still unstarted: a narrow feature-gated
    `pd-plan` runtime-route projection will carry the selected controller route
    and exact handoff evidence without altering ordinary route validation,
    production planner wiring, or controller behavior. `pd-eval` retains the
    ordinary source-taper comparison outside planner acceptance as a frozen
    canary artifact-integrity/preflight gate. A frozen same-family held-out gate
    follows implementation; materially distinct topology work is deferred;
    see
    [Trajectory-tube shadow spike](trajectory_tube_spike.md) and the
    [source-departure/acquisition execution contract](source_departure_execution_contract.md)
  - see [Waypoint Planning V1](waypoint_planning.md)

## 2. What Not To Build First

Do not start by rebuilding:

- a Pygame shell
- a browser runtime
- economy or progression systems
- complex procedural terrain
- the full late-stage `pdg` stack
- every old benchmark family

Those are the easiest ways to drag the reboot back into the old shape.

## 3. Phase Plan

### Phase 0: Documentation and naming

Deliverables:

- project brief
- architecture doc
- roadmap
- explicit repo shape and crate boundaries
- explicit vehicle geometry, touchdown-footprint, and landing-success contract
- explicit telemetry/reporting ownership boundary and supporting tool choices

Exit criteria:

- the lab has a stable direction
- future implementation work has named modules and contracts

### Phase 1: Minimum usable lab

Target:

- smallest vertical slice that proves the repo split

Planned scope:

- `pd-core`, `pd-control`, `pd-cli`
- one vehicle model
- fixed-step simulation
- lower-rate controller updates over a fixed-rate physics loop
- flat or simple piecewise-linear terrain
- terminal-descent scenarios only
- one baseline controller path
- one explicit collision hull plus touchdown-footprint model
- run manifest, action log, event log, and basic optional trace capture

Exit criteria:

- one scenario can be run repeatedly with deterministic results
- one controller can be iterated on without touching frontend concerns
- run output is structured enough for later comparison tooling

Status:

- complete for the initial landing slice
- artifact bundles are now self-contained enough to replay from the bundle alone

### Phase 2: Bot workflow and evaluation

Target:

- move from single-run debugging to a proper controller lab workflow

Planned scope:

- `pd-eval`
- controller config schemas and named controller instances
- controller-local telemetry, status, phase, and report/debug markers
- minimal single-run inspection and report outputs
- scenario packs
- curated scenario families based on useful `pylander` lessons
- seeded coverage and regression sweeps
- native multithreaded execution in `pd-eval`
- baseline comparison reports
- aggregate metrics
- profiling hooks
- local analytics over owned artifacts rather than ad hoc JSON walkers

Exit criteria:

- controller changes can be checked against a small named suite plus seeded
  coverage runs
- controller behavior is explainable without manual raw-JSON inspection
- result regressions are visible without replaying every run by hand

Status:

- complete for the maintained terminal-guidance and controller-iteration scope
- tooling and reporting are in place; additional terminal-controller tuning is
  optional, hypothesis-driven work and does not block waypoint planning
- current implementation includes:
  - a real named pack runner and summary path in `pd-eval`
  - controller config/spec plumbing for built-in controllers
  - controller-local status, phase, metrics, and markers
  - a shared controller helper kit over target-relative state and terrain
  - a second built-in controller style to keep the framework from collapsing
    into one heuristic path
  - static single-run inspection reports in `pd-cli`
  - scenario-family expansion with explicit seeds and seed ranges
  - deterministic perturbation resolution recorded per run
  - native multithreaded execution in `pd-eval`
  - richer batch summaries and review metrics over generated artifacts
  - static batch reports with:
    - selector-aware review trees
    - lane-compare and external-compare views
    - explicit report context/provenance near the top of the page
  - first-class candidate-vs-baseline comparison for batch outputs
  - cache reuse, promotion, and current-lane history compare over stable batch
    identities
  - first-class terminal-guidance selector support in the execution model:
    - hierarchy axes such as mission, arrival family, condition set, and
      vehicle variant
    - matrix axes such as arc point and velocity band
    - lane-aware expansion over the same resolved physical cases
  - a real Earth `half_arc_terminal_v1` bot-lab corpus:
    - `terminal_bot_lab_suite` as the smoke matrix
    - `terminal_bot_lab_full` as the full-seed matrix
    - current-controller-only execution, with historical comparison supplied
      by cached result packs
    - maintained payload tiers:
      - `empty`
      - `half`
      - `full`
  - a first projected trajectory-error corpus on the same Earth matrix:
    - `terminal_traj_err_suite` as the smoke matrix
    - `terminal_traj_err_full` as the full-seed matrix
    - split condition sets for undershoot/overshoot and small/large projected
      miss distances
  - experimental terrain diagnostics outside the maintained terminal guidance
    scorecard:
    - `experimental_terrain_backstop_suite` as the smoke matrix
    - `experimental_terrain_backstop_full` as the full-seed matrix
    - current-lane-only `empty` and `half` payload tiers
    - backstop terrain fixtures that remain scenario geometry, not controller
      mode switches
  - batch review trees that surface the terminal matrix directly:
    - `mission -> arrival_family -> condition_set`
    - `arc_point -> velocity_band -> vehicle_variant -> lane -> seed`
  - analytic impossible-run classification for clearly unrecoverable terminal
    cells based on controller-independent vertical and coupled terminal stop
    bounds
  - low-altitude dwell and low-altitude unsafe-recovery metrics for diagnosing
    landing-time tuning without baking those diagnostics into controller logic
- latest terminal checkpoint:
  - clean smoke current lane:
    `171 / 180` scored successes, `9` scored failures,
    `9` impossible warnings, `12` frontier annotations
  - clean full-pack current lane:
    `686 / 720` scored successes, `34` scored failures,
    `36` impossible warnings, `48` frontier annotations
  - clean full-pack `empty` and `half` tiers are solved on the maintained
    Earth corpus; clean full-payload issues are scored frontier failures plus
    analytically impossible warnings
  - trajectory-error full current lane:
    `2772 / 2880` scored successes, `107` frontier failures and `1` core
    failure,
    `144` impossible warnings, `192` frontier annotations
  - trajectory-error `empty` is solved; `half` has one high-energy
    overshoot-large core outlier; `full` is represented as the main scored
    authority-frontier tier
  - terrain avoidance is parked outside the maintained terminal guidance
    scorecard; the latest experimental backstop full snapshot was
    `228 / 288` scored successes with `60` scored failures
  - the first generic terminal terrain-clearance candidate constraint remains
    available as telemetry/diagnostic plumbing, not a Phase 2 blocker
  - `terrain_clip` and backstop containment are parked until terrain work is
    reframed as approach-corridor validation, collision warning, or waypoint
    planning
- deferred or optional follow-ups:
  - optional targeted controller robustness work on the remaining half-payload
    trajectory-error core outlier, but this does not block the planner slice
  - broader feasibility/frontier classification while keeping
    authority-frontier cells scored
  - future terrain boundary definition above terminal guidance:
    - valid approach-corridor checks for target/route selection
    - collision-course warnings for co-pilot use
    - waypoint/path planning for pure bots
  - optional report refinements driven by new planner evidence rather than
    speculative UI work

### Phase 3: Transfer guidance

Target:

- support the full point-to-point transfer problem that made late `pylander`
  interesting

Planned scope:

- source/target transfers
- one-sided signed route-arc scenarios around the target, covering descent,
  flat transfer, and climb without duplicating left/right sides
- route-angle labels such as `r-80`, `r00`, and `r+80`, where positive route
  angle means the target is uphill from the source
- a fixed target pad at `(0, 0)` with the source pad resolved from
  `source = (-radius * cos(route_angle), -radius * sin(route_angle))` after
  side normalization and route-angle label conversion
- route radius as an explicit axis, because travel distance materially changes
  transfer shape and difficulty
- optional simple monotonic source-to-target slope terrain for physical
  miss/crash containment, not terrain-avoidance behavior
- boost/coast/terminal mission definitions
- early-stop evaluation checkpoints such as boost-cutoff trajectory validation
- richer target geometry
- controller telemetry for staged or unified guidance
- waypoint guidance semantics before waypoint planning: preplanned active route
  legs, pass-through waypoint envelopes, and next-leg viability diagnostics

Exit criteria:

- the lab supports both terminal and transfer evaluation under the same core
  contracts

Status:

- first-class transfer matrix infrastructure exists for
  `signed_route_arc_transfer_v1`
- `MissionSpec` can carry an optional source-to-target `transfer_route`
- `transfer_pdg_v1` provides the first staged launch/boost/coast/terminal
  handoff controller
- `transfer_bot_lab_suite` is the smoke workbench for the initial route family
- `transfer_route_angle_suite` is the nominal-radius route-shape diagnostic
  workbench: nominal `800m` radius tier, deterministic smoke-seed radius
  perturbations, and all 11 signed route angles across `empty`, `half`, and
  `full` payload tiers
- `transfer_radius_tier_suite` is the fast distance-sensitivity gate over smoke
  route angles and `short`, `nominal`, and `long` radius tiers
- `transfer_route_angle_radius_suite` is the current wide route/radius
  diagnostic: 297 smoke-seed runs over all 11 route angles and all 3 radius
  tiers
- `transfer_route_angle_radius_full_solved` is the full-seed reliability gate
  for the historical non-`r+80` partition: all included route angles, all
  radius tiers, all payload tiers, and all 12 transfer seeds
- `transfer_route_angle_radius_frontier_full` retains the historical name but
  is now the focused full-seed `r+80` steep-uphill regression
- `transfer_waypoint_rpos80_smoke` and `transfer_waypoint_rpos80_full` are the
  first waypoint-guidance probes for the steep `r+80` stress geometry, using a
  preplanned `single_dogleg_v1` waypoint profile rather than terrain-aware
  waypoint planning. They are retained as hairpin/stress probes.
- `transfer_waypoint_contract_rpos80_smoke` and
  `transfer_waypoint_contract_rpos80_full` score the same dogleg route at the
  first waypoint handoff instead of after final-landing recovery
- `transfer_waypoint_bend_rpos80_smoke` and
  `transfer_waypoint_bend_rpos80_full` are the focused smoother
  `single_bend_v1` regressions for the same `r+80` axes
- `transfer_waypoint_bend_contract_rpos80_smoke` and
  `transfer_waypoint_bend_contract_rpos80_full` score the smoother bend profile
  at the first waypoint handoff
- `transfer_waypoint_turn_smoke` and
  `transfer_waypoint_turn_contract_smoke` are the paired broad waypoint
  workbench: three `27deg` through `62deg` turn profiles, three representative
  route angles, all payloads, nominal radius, and smoke seeds
- `transfer_waypoint_turn_route_angle_smoke` and its paired contract pack extend
  the same profiles to `r-60 | r-30 | r00 | r+30 | r+60` without replacing the
  faster maintained gate
- `transfer_waypoint_turn_route_angle_full` and its paired contract pack expand
  the same nominal-radius matrix to all `12` seeds
- `transfer_waypoint_turn_route_angle_radius_smoke` and its paired contract pack
  cover `short | nominal | long` radius tiers across the five route angles,
  three turn profiles, all payloads, and smoke seeds
- `transfer_waypoint_sequence_smoke` and
  `transfer_waypoint_sequence_contract_smoke` are the first paired ordered
  route workbench: the maintained `double_bend_v1` two-waypoint profile,
  `r-30 | r00 | r+30`, all payloads, nominal radius, and smoke seeds. The full
  `late_bend_v1` matrix is retained separately as diagnostic evidence.
- `transfer_waypoint_sequence_route_angle_smoke` and its paired contract pack
  extend `double_bend_v1` to the same five smoke route angles while preserving
  nominal radius and three smoke seeds
- `transfer_waypoint_sequence_route_angle_full` and its paired contract pack
  expand that nominal-radius matrix to all `12` seeds
- `transfer_waypoint_sequence_route_angle_radius_smoke` and its paired contract
  pack cover all three radius tiers over the same five route angles, payloads,
  and smoke seeds
- waypoint profiles and handoff envelopes are separate selectors. The balanced
  corpus uses one `pass_through_v1` route-relative envelope across every turn
  profile so geometry and contract difficulty are not conflated.
- `transfer_waypoint_pdg_v1` provides the first terrain-blind waypoint guidance
  variant: powered state-target guidance reaches the fixed waypoint endpoint
  with an outbound-envelope velocity, then resumes the final target leg
- `TransferWaypointSpec::assess_handoff` centralizes handoff semantics across
  core evaluation, controller capture, and reporting; contract probes evaluate
  on controller observation boundaries
- `EvaluationGoal::WaypointSequence` evaluates every route waypoint in order,
  stops at the first failed contract, and persists passed/total/first-failure
  evidence. Batch schema `34` retains ordered handoff histories and route-level
  status while separating the planned tangent, immutable window-entry state,
  and final handoff resolution. It also exposes final-handoff terminal
  recoverability evidence.
- batch review metrics now capture transfer final phase, first terminal handoff,
  boost/cutoff quality, boost burn stats, and Pylander-inspired shape metrics
  per run, including post-handoff apex gain, time-to-apex, and apex lateral
  offset. Low-altitude rebound gain plus origin distance separates near-pad
  terminal chatter from legitimate recovery climbs farther along the route.
- batch reports now put `Transfer Handoff Triage` ahead of shape triage so
  controller tuning starts from entry kind, handoff gate, height/speed,
  projected `dx`, cutoff quality, terminal rebound, and worst seed before
  visual-shape RMSE
- current direct-transfer checkpoint:
  - `transfer_route_angle_radius_suite`: `297 / 297` successes and `0`
    invalidations across all route angles, radii, payloads, and smoke seeds
  - `transfer_route_angle_radius_frontier_full`: `108 / 108` successes and `0`
    invalidations across the full-seed `r+80` partition
  - the route-local uphill-corridor brake closes the old near-vertical failure
    without route-label branching or a regression elsewhere in the wide matrix
  - the historical `near_vertical_transfer_route` annotation and frontier pack
    remain useful stress labels, but direct transfer is no longer the active
    Phase 3 blocker
- current waypoint corpus policy:
  - maintained fixtures are exact route-frame contracts, not world-Y-adjusted
    hints. Resolution validates signed turns, forward ordering, capture/terrain
    clearance, and an optimistic continuation stopping ratio at or below
    `0.75`.
  - report Plan cells expose progress, signed offset, signed turn, envelope,
    speed cap, and worst continuation ratio before controller behavior is judged
  - `single_dogleg_v1` and its four packs are parked diagnostic history;
    validation permits them only under `expectation_tier = diagnostic`
- current smooth-bend `r+80` checkpoint:
  - landing is `15 / 27` smoke and `54 / 108` full
  - handoff contract is `21 / 27` smoke and `89 / 108` full
  - worst continuation ratio is `0.742`, so failures are controller outcomes,
    not analytically over-energetic plans
- current balanced waypoint-guidance checkpoint:
  - `transfer_waypoint_turn_contract_smoke`: `81 / 81` contract successes
  - `transfer_waypoint_turn_smoke`: `81 / 81` final landings
  - retained terminal horizons release to receding recovery when their
    attitude-aware vertical braking margin reaches zero
  - fixed endpoint geometry, outbound target velocity, geometry-derived
    time-to-go candidates, and bounded path correction remain free of sim-time,
    route-angle, and profile branches
- current route-wide waypoint checkpoint:
  - full-seed nominal turn contract and landing are both `540 / 540`
  - full-seed nominal ordered contract and landing are both `180 / 180`
  - all-radius turn contract and landing are both `405 / 405`
  - all-radius ordered contract and landing are both `135 / 135`
  - final-waypoint states are ranked by terrain-blind terminal recoverability;
    direct transfer remains `297 / 297`
  - final authority-recovery plans retry reachable-state selection only after
    material progress, closing the former post-contract landing residual
- current ordered waypoint-sequence checkpoint:
  - maintained double-bend landing and ordered contract are both `27 / 27`
  - each planned waypoint carries the normalized inbound/outbound angle-bisector
    tangent; contract heading and energy are assessed in that frame
  - capture-radius entry opens a window instead of resolving the handoff;
    guidance retains the active leg until contract pass or waypoint-plane
    deadline
  - schema `34` separates plan tangent, window-entry state, final resolution,
    final-terminal recoverability, and low-altitude rebound in JSON and HTML
    reports
  - the full `late_bend_v1` matrix is parked as a 27-run diagnostic: it lands
    `27 / 27`, with `27 / 54` initially bad entries recovering in-window
  - ordered-contract compute remains within budget at `434us` p99
- waypoint-planner closure is complete:
  - the neutral contracts, exact route-clearance primitives, shared validator,
    and controller-independent `pd-plan` implementation are in place
  - setup-time planning emits a direct route or at most two monotone
    pass-through waypoints under an explicit versioned clearance, endpoint,
    loft, and authority policy
  - schema-36 evaluator/report integration preserves the distinction between a
    geometrically accepted route and handoff, actual-clearance, fuel, and final
    landing evidence while adding per-solve monotonic wall-time evidence
  - retained generated-route captures close at `54 / 54` landings and `36 / 36`
    contracts; the full-seed and all-radius maintained corpus remains the
    waypoint-guidance v1 regression baseline
  - keep future mechanisms independent of route/profile labels and mission
    timeout; use planned geometry, state, authority, and envelope margins
  - use handoff packs as guidance targets and paired landing packs as
    recovery/reliability regression gates
  - retain the former short-radius post-contract crash as a final-recovery
    regression watch without weakening waypoint contracts
  - the complete contract, corpus, failure semantics, and commit sequence live
    in [Waypoint Planning V1](waypoint_planning.md)
- one early-stop evaluation primitive (`timed_checkpoint`) remains available as
  a contract probe only, not as the transfer v1 scoring goal

### Phase 4: Terrain-aware lab

Target:

- add the terrain-query richness needed for real route and guardrail work

Planned scope:

- terrain query APIs beyond the segment/corridor clearance required by waypoint
  planning
- closest-point and ray queries for warnings and execution guardrails
- curated terrain-reactive scenarios after approach-corridor or waypoint
  semantics exist
- terrain-focused telemetry and replay markers

Exit criteria:

- terrain-aware guidance can be evaluated without exposing engine internals
- terrain failures are explainable from captured artifacts

Status:

- full immutable heightfield terrain is already available through `RunContext`
- deterministic height, slope, and surface-normal queries are implemented
- initial backstop terrain fixtures exist as experimental, non-blocking packs
- first-pass generic controller-side terrain-clearance evaluation is in place as
  telemetry/diagnostic plumbing
- segment and route-corridor clearance are pulled into the Phase 3 planner
  prerequisite; closest-point and ray queries remain Phase 4 work
- terrain-aware guidance is parked until approach-corridor validation,
  collision-course warnings, or waypoint planning define the higher-level
  boundary

### Phase 5: Report UX

Target:

- deepen report UX over captured artifacts, not core ownership

Planned scope:

- static HTML report pages
- trace and replay inspection
- lightweight interaction over captured trajectory data
- hover, scrub, or drag-based state inspection
- generated summary charts built on top of the owned artifact schema

This phase is for richer and more polished report UX after a minimal inspection
path already exists in Phase 2. The current state is enough for real
controller/batch iteration; this phase is about deeper visualization and
workflow polish after the scenario corpus is more mature.

Exit criteria:

- captured runs are easy to inspect without turning the browser into a runtime

Status:

- the guidance overview now provides separate terminal, direct-transfer, and
  waypoint scorecards over a declarative report catalog
- the complete eval index groups maintained evidence separately from
  diagnostics and fixtures
- batch reports lead with outcomes and selector coverage, while context and
  guidance diagnostics remain available without dominating the page
- single-run reports provide mission-first summaries, readable selectors,
  phase context, and explicit trajectory inspection modes
- existing captures can be rerendered without simulation through
  `pd-eval refresh-reports`; authoritative summary artifacts remain unchanged
- the current corpus meets the Phase 5 inspection goal. Further report work
  should respond to concrete waypoint-planning evidence rather than block it.

## 4. Migration Strategy From `pylander`

`pylander` should be treated as a source of concepts, scenario ideas, telemetry
ideas, and behavior references, not as an implementation to transliterate.

Recommended migration posture:

1. Freeze `pylander` conceptually as the baseline for expected behavior.
2. Rebuild the smallest useful slice in the new architecture.
3. Compare new runs against `pylander` on a short list of pinned scenarios.
4. Port ideas intentionally, not mechanically: scenario semantics, success
   criteria, telemetry vocabulary, and debugging lessons.
5. Only expand scope after the new boundaries hold under real use.

Important rule:

Do not port old module boundaries just because the old code already exists.

## 5. Cross-Implementation Comparison Posture

The original `pylander` cross-check intent remains useful as a small behavior
reference, not as the current acceptance gate. A focused set should stay small
and high signal:

- one nominal terminal descent
- one off-nominal terminal case
- one short transfer
- one terrain-reactive regression once terrain queries exist

These should be re-authored from the scenario shapes that proved useful in
`pylander`, not copied over mechanically as file-for-file ports.

Each case should have:

- a pinned scenario ID
- a pinned controller config
- expected success and failure interpretation
- baseline metrics that matter

The point is not perfect numeric parity. The point is to know whether the new
lab matches the intended behavior envelope closely enough to trust iteration.

Result-pack comparison is now a first-class reporting mode. Cross-implementation
`pylander` parity remains conceptual and scenario-level rather than a requirement
for numeric trace equality.

## 5.1 Coverage and seeds

Pinned scenarios are necessary but not sufficient.

The lab should also support curated randomized coverage:

- one scenario family definition
- multiple explicit seeds or seed-sweep ranges
- stable recorded resolved parameters per run

This is how the lab should validate controller robustness without exploding into
unbounded fuzzing.

## 6. Risks To Control Early

### Recreating `pylander` in Rust

This is the biggest trap. The new project should inherit lessons, not old
entanglement, and it should not copy late-stage module structure or code shape.

### Overfitting the core to one controller

`pdg` is a strong reference, but the lab should support multiple controller
styles. The contracts must stay general enough for optimization, heuristic, and
future learned controllers.

### Letting scenario grammar become architecture

Scenario identity should live in data. CLI convenience syntax should remain a
thin wrapper.

### Reintroducing frontend pressure too early

If the lab needs a browser runtime before the core is stable, the split has
already failed.

## 7. Recommended Immediate Next Step

[Waypoint Planning V1](waypoint_planning.md) is closed above the reconciled
guidance stack. Implementation phases 1-5 now provide the contracts, exact
geometry, bounded planner, evaluator/cache integration, focused corpus,
schema-36 run and batch evidence, retained `54 / 54` landing and `36 / 36`
contract captures, and fresh maintained no-regression evidence.

The attempted nominal-radius `r-60 | r+60` expansion shows that broader
setup-time coverage is not a mechanical next step. Its disposable local
contract snapshot closed only `11 / 24`, and the
[trajectory-tube shadow spike](trajectory_tube_spike.md) could not preserve the
`36 / 36` maintained baseline while rejecting the expansion failures. Do not
integrate or tune that center-only model.

The neutral
[source-departure/acquisition execution contract](source_departure_execution_contract.md)
now has validated D0a source-transition and D0b route-execution evidence without
implementing a predictor. The remaining evidence-first sequence is:

1. D0a (complete): the input-only development manifest, neutral `pd-core`
   point-envelope clearance query, and physics-rate evidence across
   `initial_state -> contact_exit -> tracking_entry` pass the full `60`-case
   development fidelity gate;
2. D0b (complete): the same `60` bundles produce complete route-wide leg and
   waypoint contract-window/deadline evidence with zero invalidations and
   deterministic repeat digests; controller markers remain audit-only;
3. D1 design (complete): the neutral input/artifact boundary, full state-set
   composition, asymmetric tri-state proof rules, narrow empirical domain,
   leave-one-input-digest-out development gate, 32-bin envelope baseline, and
   paired research-oracle boundary are locked;
4. D1 implementation (in progress): D1a common artifacts and the
   outcome-isolated development harness are complete. D1b's fixed
   interval-envelope candidate is also complete and deterministically fails:
   exact terrain identity leaves every out-of-fold query without a physical
   stratum, so all 60 predictions abstain. The current exact-float overlay is
   `11 / 13`, not the design-locked `10 / 14`; the gate records that drift
   without retuning. A subsequent input-only terrain-equivalence spike also
   rejects the simple A2 unblock: its route-relative relief motif yields eight
   groups, only `30 / 60` eligible rows, eight maintained hull exclusions, and
   `139` required cells with zero leave-one-out support. The evaluator-only R1
   candidate-selection diagnostic is also complete: bounded production-planner
   exposure plus up to eight exact paired replays found `47` selected-pair
   compatible cases, `3` fixed-executor selection-gap witnesses, `10`
   incomplete cases, and zero invalid cases. The witnesses show that route
   selection contributes to three already-seen failures, but they are neither a
   feasibility certificate nor a planner-ranking candidate. The previously
   declared D1c remains an optional paired-oracle benchmark, not a prerequisite
   or the current next slice; D1d cannot advance the failed Alternative A.
   Neither Alternative A, the motif-based A2 concept, nor R1 advances to D2.
   The post-R1 `bounded_trajectory_witness_v1` design now locks
   an independent positive-only physical lane: one exact command trace must
   replay in the `120 Hz` plant under a `60 Hz`, `130 s` bound; failure to find
   one is `unknown`, never physical `unsupported`. Its W1 typed artifacts and
   exact verifier, minimum W2 conformance checkpoint, fixed W3 finite-template
   proposal spike, and outcome-isolated W4 comparison are complete. W4 found
   zero supported physical witnesses and retains all 60 results as `unknown`;
   the research lane stops there unless a separate need justifies resuming it.
   The subsequent conservative-ballistic CB0 final-landing audit is also
   complete: all 60 exact current development inputs land on target, so it
   closes the repair claim. The separate forward-looking V2 ballistic-first
   checkpoint now freezes one scoped ridge canary with a nominal-direct-red,
   one-waypoint-green analytical certificate. Its corrected full-controller
   shadow validates flat-direct-green, mesa-direct-red, and mesa-waypoint-green
   through an evaluator-scoped composed preflight over an exact powered-bridge
   state. A feature-gated experimental `pd-plan` candidate projection now
   exposes the frozen direct/one-waypoint/unsupported decision and exact
   crossing without changing production planner wiring. CB3 now design-closes
   the next implementation: `pd-plan` will own a feature-gated runtime-route
   projection over that certificate and crossing, while `pd-eval` retains the
   ordinary source-taper comparison outside planner acceptance as a frozen
   artifact-integrity/preflight gate. The implementation must preserve
   controller-shadow v4 / `a69de7872ad039fd`, its three frozen outcomes, and
   production behavior. It does not advance generic source-departure D2 or
   D4-D5. After those gates pass, a small predeclared same-family held-out
   matrix will seal cases, predictions, and identities before simulation; only
   then is a materially distinct topology reconsidered;
5. D2-D3 (blocked): freeze genuinely uninspected inputs and predictions only
   after W1-W4 are reviewed and a separate advancement checkpoint defines an
   integration-eligible claim and held-out gate;
6. D4-D5: promote a separately reviewed candidate interface or capability into
   planner-facing evaluation, and consider planner integration only after an
   alternative passes every declared gate. The evaluator-consumed R1
   exposure does not satisfy this phase.

The terrain-equivalence spike closes the empirical-envelope salvage question
for the tested motif without outcome-guided tuning. R1 additionally proves that
three already-seen rows have an executor-compatible route inside the current
planner's first eight candidates, while ten rows remain deliberately
inconclusive at that budget. The subsequent design review rejected an
unqualified equivalence between physical feasibility and frozen-executor
compatibility. The locked
[bounded trajectory witness V1](bounded_trajectory_witness.md) therefore makes
only a positive controller-neutral existence claim, verified against the exact
discrete plant, and keeps R1 as the separate executor axis. W1 through W4 are
now complete. The outcome-isolated W4 run exhausted the full finite catalog on
all 60 already-seen rows, producing 47 `unknown/supported` and 13
`unknown/unsupported` physical/executor joins with zero invalid comparisons.
This closes the comparison mechanism but supplies no physical coverage or
negative certificate. Research-grade D2 remains blocked. The separately
versioned
[Conservative Ballistic Route Planning](conservative_ballistic_route_planning.md)
design now distinguishes two claims: CB0 found no current final-landing
failure, while intentional future game content may still require staging. The
V2 direct rows remain neutral diagnostics, and the bounded ridge canary now
adds a controller-independent scoped result: the shortest robust flat-derived
nominal lane plus optimistic post-commit correction is red on a derived mesa,
while a finite terrain-derived one-waypoint witness is green. Higher global
direct replans remain possible. The corrected full-controller shadow now
validates the complete scoped gameplay distinction: the flat twin lands, the
mesa direct twin crashes on the derived wall, and the mesa waypoint twin passes
its runtime handoff contract and lands. The composed evaluator preflight uses
an exact traversed intermediate-bridge state and preserves the ordinary full-
route source-taper rejection as a separate diagnostic. The feature-gated
experimental planner-owned projection now completes that extraction, preserves
the scoped nominal policy and candidate identities, and leaves runtime route
and controller semantics downstream. CB3 design-closes the next feature-gated
`pd-plan` runtime-route projection over that candidate decision and crossing;
implementation remains unstarted and must preserve ordinary route validation,
the controller-shadow v4 artifact, and its three frozen outcomes. A small
same-family held-out matrix follows successful implementation, with analytical
cases, predictions, and identities sealed before simulation; its ordinary
source-taper comparison remains an evaluator artifact-integrity/preflight gate
outside planner acceptance. A materially distinct topology is deferred until
after that gate; production planner wiring remains unauthorized.

Do not expand the research lane to runtime replanning, randomized terrain, more
than two waypoints, or route/profile controller branches. The new analytical
design may use a conservative fuel upper bound as planner policy, but ordinary
simulation remains authoritative for actual fuel use. Keep a later
terminal-arrival extension on the roadmap: a signed climb/descent arrival
family that expands the current one-sided quarter-arc into a half-arc around
the target and exercises climbing arrivals.

Direct transfer, authored waypoint guidance, and the focused generated-route
matrix are clean across their maintained scopes. Schema-34 window and
terminal-recovery evidence keeps contract quality separate from final touchdown
reliability; schema 36 adds planner provenance and compute evidence without
changing that boundary. Any next expansion should remain planner-generated
geometry backed by a versioned analytical execution policy and ordinary
full-simulation landing evidence, not route-specific guidance recovery
heuristics.
