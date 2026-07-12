# Powered Descent Lab Roadmap

This roadmap is intentionally design-first. It describes the order of work after
the reboot without committing to implementation in this pass.

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
- Phase 2 is now a late-stage bot workflow phase rather than a scaffolding
  phase:
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
- Phase 3 is active as an evaluation and controller workbench:
  - `timed_checkpoint` remains available as an early-termination contract probe
  - `signed_route_arc_transfer_v1` now exists as the first source-to-target
    matrix family
  - `transfer_pdg_v1` provides the first staged launch/boost/coast/terminal
    handoff controller
  - direct transfer is clean across the maintained route-angle/radius matrix
  - `transfer_waypoint_pdg_v1` closes the normalized balanced terrain-blind
    pass-through handoff and paired landing corpora
  - ordered two-waypoint evaluation, first-trigger prediction, and paired smoke
    corpora now exist; normalized route-frame fixtures establish a
    `24 / 54` route-contract and `54 / 54` final-landing checkpoint

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

- late in the phase, but not closed yet
- the tooling/reporting side is largely in place now; the remaining work is
  mostly controller robustness, corpus expansion, and evaluation policy
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
    `684 / 720` scored successes, `36` scored failures,
    `36` impossible warnings, `48` frontier annotations
  - clean full-pack `empty` and `half` tiers are solved on the maintained
    Earth corpus; clean full-payload issues are scored frontier failures plus
    analytically impossible warnings
  - trajectory-error full current lane:
    `2751 / 2880` scored successes, `129` scored failures,
    `144` impossible warnings, `192` frontier annotations
  - trajectory-error `empty` is solved; `half` has only three high-energy
    overshoot-large outliers; `full` is represented as the main scored
    authority-frontier tier
  - terrain avoidance is parked outside the maintained terminal guidance
    scorecard; the latest experimental backstop full snapshot was
    `228 / 288` scored successes with `60` scored failures
  - the first generic terminal terrain-clearance candidate constraint remains
    available as telemetry/diagnostic plumbing, not a Phase 2 blocker
  - `terrain_clip` and backstop containment are parked until terrain work is
    reframed as approach-corridor validation, collision warning, or waypoint
    planning
- still missing:
  - optional targeted controller robustness work on the two remaining
    half-payload trajectory-error outliers, but this should not block Phase 2
    corpus/evaluation progress
  - broader feasibility/frontier classification while keeping
    authority-frontier cells scored
  - future terrain boundary definition above terminal guidance:
    - valid approach-corridor checks for target/route selection
    - collision-course warnings for co-pilot use
    - waypoint/path planning for pure bots
  - deeper report polish that depends on real matrix scenarios and controller
    signal rather than the old provisional corpus

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
  workbench: three `23deg` through `62deg` turn profiles, three representative
  route angles, all payloads, nominal radius, and smoke seeds
- `transfer_waypoint_sequence_smoke` and
  `transfer_waypoint_sequence_contract_smoke` are the first paired ordered
  route workbench: two two-waypoint profiles, `r-30 | r00 | r+30`, all
  payloads, nominal radius, and smoke seeds
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
  evidence. Batch schema `28` retains ordered handoff histories, route-level
  status, completed-leg target/deadline debt, and predicted first-trigger state
  while preserving waypoint-zero compatibility fields.
- batch review metrics now capture transfer final phase, first terminal handoff,
  boost/cutoff quality, boost burn stats, and Pylander-inspired shape metrics
  per run, including post-handoff apex gain, time-to-apex, and apex lateral
  offset
- batch reports now put `Transfer Handoff Triage` ahead of shape triage so
  controller tuning starts from entry kind, handoff gate, height/speed,
  projected `dx`, cutoff quality, and worst seed before visual-shape RMSE
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
- current ordered waypoint-sequence checkpoint:
  - final landing is `54 / 54`; ordered route success is `24 / 54`
  - passed-handoff distribution is `0:3 | 1:27 | 2:24`
  - both sequence profiles now land `27 / 27`; route-contract debt remains
    concentrated in handoff shaping rather than touchdown recovery
  - first-trigger projection, roundoff-safe envelope validation, and bounded
    local replacement remain the current controller mechanism
- next transfer slice should improve guidance against the corrected corpus:
  - keep waypoints preplanned and terrain avoidance encoded in the plan
  - address upstream/two-leg feasibility where the next active leg has no
    dynamically feasible trigger-pass candidate, especially late-bend index one
  - use handoff packs as guidance targets and paired landing packs as
    recovery/reliability regression gates
  - expand route count and radius coverage only after these nominal mechanisms
    improve without terrain reaction or route/profile-specific branches
- one early-stop evaluation primitive (`timed_checkpoint`) remains available as
  a contract probe only, not as the transfer v1 scoring goal

### Phase 4: Terrain-aware lab

Target:

- add the terrain-query richness needed for real route and guardrail work

Planned scope:

- setup-time terrain query API
- closest-point, ray, and clearance queries
- curated terrain-reactive scenarios after approach-corridor or waypoint
  semantics exist
- terrain-focused telemetry and replay markers

Exit criteria:

- terrain-aware guidance can be evaluated without exposing engine internals
- terrain failures are explainable from captured artifacts

Status:

- initial backstop terrain fixtures exist as experimental, non-blocking packs
- first-pass generic controller-side terrain-clearance evaluation is in place as
  telemetry/diagnostic plumbing
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

## 5. First Comparison Corpus

The first cross-check set should stay small and high signal:

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

Baseline comparison should be treated as a first-class reporting mode, not only
as a post-hoc analysis utility.

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

Keep Phase 2 terminal work conservative while using the new Phase 3 transfer
workbench to expose launch, boost, coast, and terminal-handoff issues.

The next useful work is:

1. Treat the current terminal controller as the Phase 2 baseline unless a
   specific, general controller hypothesis can clear a smoke-suite
   no-regression gate.
2. Use the thresholded regression policy so future controller changes have an
   explicit pass/fail bar.
3. Keep refining feasibility/frontier semantics where the vehicle is authority
   limited, while keeping frontier cells scored so regressions do not disappear
   into warning buckets.
4. Keep terrain avoidance out of terminal-guidance pass/fail gates until the lab
   has a higher-level approach-corridor or waypoint-planning layer.
5. Keep a later terminal-arrival extension on the roadmap: a signed
   climb/descent arrival family that expands the current one-sided quarter-arc
   into a half-arc around the target and exercises climbing arrivals.
6. Keep `transfer_bot_lab_suite` and `transfer_route_angle_radius_suite` as
   direct-transfer regression gates. Preserve balanced handoffs at `81 / 81`,
   balanced landing at `81 / 81`, sequence landing at `54 / 54`, and ordered
   sequence handoffs at or above `24 / 54` before
   waypoint planning or terrain-aware routing.

The immediate controller direction should stay conservative. Direct transfer,
balanced pass-through handoff, and paired final landing are clean. Ordered
route success remains `24 / 54` despite `54 / 54` sequence landings, with
late-bend continuation still dominant. The next hypotheses should propagate
next-leg feasibility upstream, not retune terminal recovery, widen the
prediction horizon, or add route/profile policy. Radius tiers should follow
once that nominal mechanism is credible.
