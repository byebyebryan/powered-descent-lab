# Powered Descent Lab Roadmap

[Documentation home](README.md) · [Current guidance](guidance.md) · [Research archive](history.md)

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

### Active planner evaluation and reporting

The accepted native V2 evaluation workflow is `planner_v2_lab_suite`: omitting
the pack from `pd-eval run-pack` selects policy 3, and
`pd-eval waypoint-v2-flight` also defaults to policy 3. This offline workflow
covers the supported vehicle, Earth gravity, and 120 Hz physics / 60 Hz command
setup; it is not a game-loop timing guarantee. Policy 3 is also the Rust policy
default and sole executable planner. V1 search and policy-1/2 selectors are
retired; saved contracts/known identities remain readable. The ordinary
`pd-cli run --controller baseline` default is unchanged.

The accepted 44-case capture has 36/36 core target landings (11 direct, 25
corrected) and eight separate diagnostics (two landed, four zero-command and
zero-step `NoClearing` stops before departure, and two unsupported). All 44
passed integrity; all 42 supported results passed final-source replay. Its
canonical batch and rich mission details use the shared report templates at
`/reports/eval/planner_v2_lab_suite/`. See the
[activation results](waypoint_v2_eval_activation_results.md) and
[common-template results](planner_v2_common_report_templates_results.md) for
the earlier capture and report checkpoints. Two final-source
[reliability captures](planner_v2_reliability_results.md) reproduced the same
outcomes before the accepted
[session/CLI replacement](waypoint_v2_session_repair_results.md). That latest
checkpoint passes native, real CLI and CLI-repeat matrices with exact retained
flight parity and 42 saved-source CLI replays. Its native capture supplies the
current published evidence. A reusable acceptance gate binds the
frozen inputs, requires all 36 ordinary landings and integrity/replay evidence,
and keeps diagnostic expectations separate. Failed captures retain their rich
reports but cannot replace the accepted current site. The checked native batch
workflow returns failure for an acceptance miss.

The owned synchronous session and `pd-cli` adapter behind the default-off
`planner-v2` Cargo feature are implemented and accepted, not a pending design.
They do not provide an externally driven per-tick controller or arbitrary
snapshot restart. Behavior-preserving review, reconciliation and the
[core cleanup checkpoint](waypoint_v2_core_cleanup_results.md) are now complete;
they do not themselves expand terrain coverage. The subsequent
[retirement and consolidation](planner_retirement_cleanup_results.md) removes
obsolete execution lanes, extracts the current runtime, and reconciles tools/docs
while preserving complete flight numerics and saved evidence.
Seeded procedural terrain evaluation and a concrete game-host integration are
separate possible follow-ups, not prerequisites for using the current lab
planner and not selected by this reconciliation.

The phase and research notes below retain their original evidence scope. Their
date-local “next” steps and stop rules are not an ordered prerequisite list for
the accepted V2 evaluator workflow.

### Retained phase and research checkpoint detail

Historical implementation/evidence status (retired V1/research executables are
not current entrypoints):

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
  - the opt-in
    [waypoint direct-route characterization](waypoint_direct_characterization.md)
    freezes a separate continuous-terrain mismatch: V1 rejects flat, uphill,
    and downhill direct rows while the unchanged direct controller lands them
    with positive sampled en-route clearance. This is research evidence for a
    future bounded per-leg profile or certificate, not a V1 rewrite, default
    selection change, or selected V2 model
  - the bounded
    [direct-leg primitive research pass](waypoint_direct_primitive_research.md)
    mapped a single 90 s V2 bridge/coast policy to the five characterization
    inputs, then certified direct routes for all 24 fixed obstacle cells.
    The unchanged direct controller landed on the 12 lower cells and crashed
    on the 12 taller cells; no analytical one-waypoint row was eligible.
    This historical controller split does not establish certificate execution
    or justify inferring waypoint demand from controller crashes
  - the opt-in known-flat
    [complete-flat acceptance checkpoint](waypoint_direct_complete_flat_acceptance_protocol.md)
    accepts four new launch-aware held-60 Hz direct witnesses without a floor
    cutaway and rejects five shorter target-contact crashes before ranking.
    It closes known-flat nominal acceptance only. The subsequent
    [input-driven generation checkpoint](waypoint_direct_generation_results.md)
    passes all six uncut flat/uphill/downhill cases, and the
    [obstacle-discrimination checkpoint](waypoint_direct_obstacle_discrimination_results.md)
    accepts complete direct witnesses in six of eight cases. The two high
    obstacles retain terrain-clear schedules rejected at target contact,
    rather than proof of waypoint demand. Robustness, arbitrary incoming
    waypoint states, composition and controller/default integration remain
    open. V1 and F6 are unchanged; the analytical boundary study remains
    distinct from these nominal simulator acceptance results
  - the [terminal diagnostic](waypoint_direct_terminal_admissibility_results.md)
    reproduces all 76 frozen schedules and separates 32 inadmissible ideal
    contact references from ten cadence-sensitive failures. Terminal-only
    120 Hz execution lands those ten, preserves three accepted controls and
    still cannot land the six high-case tails despite nearly exact tracking.
    It isolates the reference and cadence problems; its results and old ledgers
    remain unchanged
  - the latest [body-aware terminal prototype](waypoint_direct_body_aware_terminal_results.md)
    accepts all fourteen development and ten newly sealed Direct cases at the
    intended held-60 Hz cadence without cutaways or waypoints. All 152 / 100
    available development / fresh source schedules pass, while historical
    baseline bytes and launch/source commands remain unchanged. The nominal
    terminal question is closed; selected hull-contact margin remains
    millimetre-scale and robustness, setup cost and default authority are open.
    Next is a separately scoped opt-in direct-first witness
    adapter, not more source fitting or automatic waypoint expansion
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
    implemented: the feature-gated `pd-plan` runtime-route projection carries
    the selected controller route, exact handoff evidence, structural
    validation, stable identities, and typed fail-closed reasons. The
    controller-shadow remains schema `v4` / semantic identity
    `a69de7872ad039fd` with its three frozen outcomes, and ordinary route
    validation, production planner wiring, and controller behavior are
    unchanged. `pd-eval` retains the ordinary source-taper comparison outside
    planner acceptance as a frozen canary artifact-integrity/preflight gate.
    H0 closed the separately reviewed same-family held-out design with two
    frozen ridge-progress probes (`rho = 0.50` and `0.68`, labels
    `ridge_progress_050_probe` and `ridge_progress_068_probe`). H1's generic
    input/recompute/projector boundary and H2a's identity-bound raw-input and
    prediction manifests are implemented. H2b completed an analytical-only
    reveal and recorded a deterministic `STOP/NARROW` result:
    `ridge_progress_050_probe` passed its frozen analytical/runtime
    expectations, while `ridge_progress_068_probe` passed every candidate-level
    prediction but stopped at the typed runtime `handoff_contract_failed`
    boundary. The tracked result is `24804a74cb303625`; H3's six controller
    lanes were not run, so there are no held-out controller outcomes. The
    subsequent F0-F3 controller-free development checkpoint preserved that
    historical stop and added the generic runtime V2 selector; `050` remains
    byte-compatible with V1 and `068` selects exit step `3960` under the
    unchanged handoff contract. F4 then completed green as an exposed
    development full-controller check using the unchanged `120 Hz` physics /
    `60 Hz` controller / `180 s` configuration: flat-direct target-landed,
    mesa-direct crashed within the derived mesa, and mesa-waypoint captured
    its contract once and target-landed. Its artifact is
    `conservative-ballistic-handoff-controller-development-v1`, identity
    `8ecd42d0bf113f5d`, with `deterministic_repeat: true`; F3 identity remains
    `aeb7338fd71c73e5` and frozen controller-shadow v4 remains
    `a69de7872ad039fd`. This is not held-out or physical proof and does not
    establish universal necessity or robustness; no controller, planner,
    handoff-threshold, or terrain tuning occurred. F5a now locks a fresh
    held-out pair, `ridge_progress_056_probe` (`rho = 0.56`) and
    `ridge_progress_072_probe` (`rho = 0.72`), with predeclared qualitative
    analytical and controller predictions before any reveal. F5b is now
    source-sealed in the evaluator-owned F5 module and two versioned fixtures;
    the ordered input identities are `fnv1a64:1685aab304642342` and
    `fnv1a64:b0dcdcbefac383b6`, with input-manifest identity
    `5a654dd8762a1438` and prediction-manifest identity `f3d3ff2ee6e83403`.
    The seal validates raw inputs and qualitative-prediction identity, order,
    and binding only. F5c subsequently completed as a mixed analytical
    checkpoint: `ridge_progress_056_probe` advanced, while
    `ridge_progress_072_probe` stopped at finite waypoint-search exhaustion.
    F5d then ran only the three eligible `056` controller lanes; flat-direct
    landed, mesa-direct crashed within the derived mesa, and mesa-waypoint
    captured exactly once before landing. The controller compact result is
    `d08eb8ca50f34561`, binds analytical result `6ebab8240feaea3e`, and is
    deterministic. F5e is therefore complete as **NARROW**, not a full-green
    PASS: `056` demonstrates the scoped capability while `072` remains
    analytically stopped and unrun by the controller. This does not satisfy
    the full-pass condition for an automatic production-integration design
    review. In that separate conservative-ballistic research lane, production
    wiring, runtime replanning, broader waypoint counts or topologies, D2-D5,
    and tuning remain deferred. F6 then closed the retained
    `ridge_progress_056_probe` as a scoped opt-in **PASS**: its identity-bound
    analytical decision injected one derived-mesa waypoint into the unchanged
    real controller, which captured once and target-landed.
    `ridge_progress_072_probe` remains immutable historical evidence and is
    excluded from F6 fixtures, gates, tuning, and capability claims. This
    proves one development seam only; it does not establish mission-family
    coverage, arbitrary terrain support, multiple waypoints, runtime
    replanning, or production/default planner selection. F6 stops at this
    single-mission capability; no F7/neighborhood checkpoint is scheduled.
    Expansion reopens only for a concrete gameplay mission or
    production/default-selection need;
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

The accepted policy-3 V2 core is consolidated without reopening nominal fitting
or retuning terrain cases. Persisted records and replay safety have separate
ownership, and the flight loop delegates to named planning and execution phases.
The [core cleanup results](waypoint_v2_core_cleanup_results.md) record exact
44-case numerical parity, the final workspace gate and unchanged saved evidence.
The subsequent [retirement and housekeeping](planner_retirement_cleanup_results.md)
completed the current pack-responsibility split and common rich-template
extraction. Optional further domain splits or report-reference math consolidation
need a concrete maintenance reason; neither is a flight defect or prerequisite.

The [current guidance boundary](guidance.md#current-v2-design-and-support) and
[session/CLI replacement result](waypoint_v2_session_repair_results.md) define
where we are. Choose a bounded procedural-terrain evaluation or a concrete
host-consumer requirement as the next substantive pass. Do not automatically add a
new solver, extract a standalone crate, expand vehicles/gravities or promote
controller defaults. No new terrain experiment or publication is selected here.

### Retained V1 and research-track checkpoint history

The notes below preserve earlier V1, source-departure, bounded-witness, and
conservative-ballistic checkpoints. Their local “current” and “next” wording
refers to the checkpoint date; their stop rules apply only to the named
research lane. D1, W1-W4, and F6 are retained or stopped research, not scheduled
prerequisites for the accepted V2 evaluator workflow.

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
   crossing without changing production planner wiring. CB3 is implemented:
   `pd-plan` owns the feature-gated runtime-route projection over that
   certificate and crossing, while `pd-eval` retains the ordinary source-taper
   comparison outside planner acceptance as a frozen artifact-integrity/
   preflight gate. The controller-shadow remains schema `v4` /
   `a69de7872ad039fd` with its three frozen outcomes; production behavior,
   generic source-departure D2, and D4-D5 are unchanged. H0 closed the
   separately reviewed same-family held-out design: two frozen
   ridge-progress probes at `rho = 0.50` and `0.68` have explicit analytical
   predictions and H1-H4 ordering. H1's input/recompute/projector boundary and
   H2a's raw-input and prediction manifests are implemented. H2b completed an
   analytical-only reveal and recorded a deterministic `STOP/NARROW` result:
   `ridge_progress_050_probe` passed its frozen analytical/runtime
   expectations, while `ridge_progress_068_probe` passed every candidate-level
   prediction but stopped at typed runtime `handoff_contract_failed`. H3 was
   not run, so there are no held-out controller outcomes and the scoped
   execution-compatibility claim does not advance. The subsequent F0-F3
   controller-free development checkpoint preserves that historical V1 stop
   and adds a versioned generic runtime V2 selector: it retains the exact
   virtual-anchor crossing attempt and, only when its canonical handoff fails,
   tries the certified intermediate-bridge exit/target-leg apex seam. `050`
   remains byte-compatible with V1; `068` selects exit step `3960` and passes
   the unchanged handoff contract. The deterministic development artifact is
   `aeb7338fd71c73e5`; controller-shadow v4 remains `a69de7872ad039fd`. F4
   completed green as the development full-controller test of that route, with
   flat-direct landing, mesa-direct terrain crash, and mesa-waypoint contract
   capture followed by target landing. This is exposed development evidence,
   not held-out or physical proof. F5a locked genuinely new predeclared inputs
   and predictions, F5b sealed their source-controlled identities, and F5c
   completed as a mixed analytical checkpoint: `056` advanced while `072`
   stopped at finite waypoint-search exhaustion. F5d subsequently confirmed
   all three frozen controller expectations for `056` and did not run `072`;
   compact controller result `d08eb8ca50f34561` binds analytical result
   `6ebab8240feaea3e`. F5e closes the sequence as **NARROW**, not full green. A
   materially distinct topology remains deferred;
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
and controller semantics downstream. CB3 now completes the feature-gated
`pd-plan` runtime-route projection over that candidate decision and crossing;
ordinary route validation, the controller-shadow v4 artifact, and its three
frozen outcomes remain unchanged. H0 closed the same-family held-out
matrix design with `ridge_progress_050_probe` and
`ridge_progress_068_probe`, including analytical predictions, identity
bindings, and the H1-H4 freeze order. H1's generic boundary and H2a's frozen
raw-input and prediction manifests are implemented; H2b has now recorded the
analytical-only `STOP/NARROW` result above, before any H3 controller run. The
ordinary source-taper comparison remains an evaluator artifact-integrity/
preflight gate outside planner acceptance. A materially distinct topology is
deferred until the handoff boundary is understood; production planner wiring
remains unauthorized.

### F5 held-out checkpoint (closed: NARROW)

F5 is the completed bounded gameplay evidence sequence after the green F4
development run. F5a's documentation-only design lock and F5b's
source-controlled input/prediction seal were completed before either reveal.
F5a defined two fresh, same-family ridge-progress probes before any new result
was inspected:

| Label | `rho` | `c` | Raw ridge points `(x, y)` |
| --- | ---: | ---: | --- |
| `ridge_progress_056_probe` | `0.56` | `2247.92` | `(2122.92, 0)`, `(2172.92, 1200)`, `(2322.92, 1200)`, `(2422.92, 0)` |
| `ridge_progress_072_probe` | `0.72` | `2885.04` | `(2760.04, 0)`, `(2810.04, 1200)`, `(2960.04, 1200)`, `(3060.04, 0)` |

Both retain the existing policy, vehicle, pads, domain `[-40, 4040]`, source
`x = 18`, target `x = 4000`, initial rest state, and ridge shape/height. The
`056` case interpolates the demonstrated progress range; `072` is a mild
extrapolation. Predictions cover flat-direct, mesa-direct, and mesa-waypoint
qualitative outcomes, including exactly one certified forward-progressing
waypoint result and a structurally valid runtime-V2 handoff. The exact
`primary_crossing` versus `intermediate_bridge_exit`, exact times/points/fuel,
and derived identities are intentionally not frozen.

F5b seals those inputs and predictions in
`fixtures/manifests/conservative_ballistic_ridge_f5_inputs_v1.json` and
`fixtures/manifests/conservative_ballistic_ridge_f5_predictions_v1.json`,
validated by `pd-eval/src/conservative_ballistic_f5_seal.rs`. The ordered raw
input identities are `fnv1a64:1685aab304642342` and
`fnv1a64:b0dcdcbefac383b6`; the input-manifest identity is
`5a654dd8762a1438`, and the prediction-manifest identity is
`f3d3ff2ee6e83403`. The seal validates raw input and qualitative-prediction
identity, order, and binding only. No F5 analytical, runtime-projection,
controller, simulator, or report outcome was included in those seals, and the
historical H2/H4 manifests and results remain immutable.

F5c completed as a mixed analytical checkpoint. The tracked compact
result is
`fixtures/manifests/conservative_ballistic_ridge_f5_analytical_result_v1.json`,
identity `6ebab8240feaea3e`; it binds the input seal `5a654dd8762a1438` and
prediction seal `f3d3ff2ee6e83403`. The detailed ignored artifact identity is
`ae9f3d7f01b1afdd`, with summary/report directories under
`outputs/eval/conservative-ballistic-ridge-f5-analytical-v1/` and
`outputs/reports/eval/conservative-ballistic-ridge-f5-analytical-v1/`. The
root status is `one_or_more_cases_stopped` with `deterministic_repeat: true`.

- `ridge_progress_056_probe` passed all candidate and runtime checks and is
  `eligible_for_controller`; runtime V2 observed `primary_crossing`, attempt
  `0`, with a selected waypoint near `(1913.111172, 1509.228848) m`.
- `ridge_progress_072_probe` stopped independently at
  `candidate_projection` / `derived_mesa` with typed `Unsupported` reason
  `finite_waypoint_search_exhausted`; no certified waypoint was found. Its
  flat-direct and nominal terrain-clearance checks passed, but the one-waypoint
  predictions did not.

This analytical result was not `INVALID SETUP`, but it prevented an overall
F5 PASS/full-green result and restricted F5d to the three `056` controller
lanes. `072` did not run controller lanes.

F5d is now complete. Its tracked compact result is
`fixtures/manifests/conservative_ballistic_ridge_f5_controller_result_v1.json`,
identity `d08eb8ca50f34561`; the ignored detailed artifact identity is
`ef550886127d079d`. It binds the F5c result and both source seals, reports
`deterministic_repeat: true`, and records `072` as
`analytically_ineligible_not_run` with no controller scenario or lane data.
For `056`, all launches passed; flat-direct target-landed, mesa-direct crashed
at a validated reconstructed contact within the derived mesa, and
mesa-waypoint captured exactly one passing contract before target landing.
The controller status is `controller_predictions_matched`.

The first invocation stopped before writing evidence because the compact
validator expected a planner-style identity prefix for evaluator lane digests.
The wiring-only repair in `ea23268` changed no frozen experiment input,
planner, route, controller, threshold, terrain, prediction, or acceptance rule.

The phase order is F5a design lock, F5b source-controlled raw-input and
qualitative-prediction seals, F5c analytical reveal, F5d controller reveal only
for analytically eligible cases after analytical results are committed, and
F5e decision. Cases advance independently, but a mismatch or `Unsupported`
stops that case and prevents a full green. PASS requires both cases to pass
analytical and controller expectations; NARROW means one remains useful while
the other fails/stops; STOP means neither demonstrates the capability or both
share a failure; INVALID SETUP covers provenance/scenario/launch/determinism
and permits wiring repair only. F5e therefore closes this experiment as
**NARROW**: `056` demonstrates the scoped capability and `072` remains stopped.
This is not a full-green PASS and does not satisfy the frozen full-pass
condition for an automatic production-integration design review. A limited
operating envelope, another independently frozen same-family confirmation, or
stopping the capability now requires a separate product/design checkpoint.
Runtime replanning, multiple waypoints, new topologies, D2-D5, production
wiring, and tuning remain deferred.

### F6 limited integration checkpoint (complete: scoped PASS)

F6 accepts the useful `056` result as one bounded capability and does not seek
a replacement held-out partner for `072`. The F5 seals, results, and historical
`NARROW` label remain unchanged. `072` is retained only as experiment history:
its derived blocker leaves an unrepresentative terminal-recovery region, so it
is excluded from every F6 fixture, gate, tuning decision, and claimed
operating-envelope boundary. Broader coverage remains unmeasured.

The integration target is deliberately smaller than production planner
promotion. A new explicit evaluator lane consumes the existing `pd-plan`
generic analytical and runtime-V2 projections for the source-controlled `056`
input, selects the derived-mesa route through a typed adapter, and injects that
exact `TransferRouteSpec` into the controller scenario. The adapter records
input, analytical, runtime, candidate, route, terrain, topology, waypoint, and
disposition provenance. Valid bounded exhaustion is `unsupported`; malformed
or mismatched evidence is invalid. Both stop before controller execution, and
fallback remains a future caller decision.

F6 retains exact `source`/`target` IDs, source-pad rest/upright state, frozen
cadence, the derived-mesa terrain, and at most one waypoint as V1 applicability
conditions. These are structural checks, not label branches or new geometric
thresholds. The ordinary `pd_plan::plan()` API and result identities,
`pd_core::validate_route`, controller behavior and parameters, F5 artifacts,
runtime replanning, arbitrary raw terrain, multiple waypoints, broader
topologies, and D2-D5 remain unchanged.

The ordered implementation is F6a documentation lock; F6b single-case fixture,
typed adapter, deterministic provenance, and fail-closed tests; F6c integrated
controller run and visual report through the injected route; and F6d full
non-regression/closure. Passing F6 establishes only that this opt-in
development seam is sound. Default production selection and any neighborhood
expansion require another explicit product checkpoint.

F6 is now complete. Fixture `feefa81872db45eb` produced typed decision
`4de1b2a1fca1cf9f`, application `ea87905351c8a2e5`, and selected route
`c2823e8c4c6c7515`. The one injected `transfer_waypoint_pdg` lane captured one
passing waypoint contract at `33.25 s` and target-landed at `85.7 s`. Detailed
artifact `d8e7bf750237ae28` and compact result `9c9876bc34eeca01` are
deterministic; the latter is tracked at
`fixtures/manifests/conservative_ballistic_ridge_f6_integration_result_v1.json`.
The corresponding report is under
`outputs/reports/eval/conservative-ballistic-ridge-f6-integration-v1/`, and an
explicit before/after test preserves the ordinary planner result.

The deferrals and later-arrival proposal below apply only to this historical
conservative-ballistic/F6 research lane; they do not constrain the separate
accepted native V2 evaluator loop described above.

The next product decision is therefore not more F6 implementation or a new
neighborhood checkpoint. F6 stops at the demonstrated single-mission
capability, with no F7 scheduled. A broader mission-family or default-planner
path reopens only for a concrete gameplay mission or production/default-
selection need, with its mission contract and predictions source-controlled
before evaluation. Default production selection, arbitrary raw-terrain
adaptation, multiple waypoints, runtime replanning, tuning, and D2-D5 remain
out of scope until explicitly reopened.

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
