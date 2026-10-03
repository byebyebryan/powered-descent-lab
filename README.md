# Powered Descent Lab

Powered Descent Lab (`pd-lab`) grows out of the bot and simulation work explored
in `pylander`: a native-first lab for deterministic 2D rocket flight,
controller development, scenario design, benchmarking, and replay/telemetry
analysis.

This project is not the eventual player-facing game. The split is intentional:

- the lab optimizes for determinism, throughput, interfaces, and evaluation
- the game can later optimize for feel, UX, content, and presentation

## Direction

Current design direction:

- Rust cargo workspace, native-first
- `clap` + `serde` native tooling, with web reserved for static report viewing
  rather than an interactive runtime
- fixed-step deterministic simulation
- controller updates may run at a lower fixed rate than physics, with commands
  held between controller ticks
- a proper bot framework, not only a thin `Observation -> Command` callback
- one primary goal per scenario
- formal controller API with full immutable scenario context at setup time and a
  compact per-tick observation
- controller outputs that can include status, phase, metrics, and report/debug
  markers in addition to vehicle commands
- authored scenario packs, curated scenario families, and seeded regression
  sweeps
- 1D heightfield terrain as the canonical world model, with richer query APIs
  layered on top
- no LOD in v1 for controller-facing terrain data
- landing means stable touchdown on the designated target, based on
  touchdown/contact-frame metrics rather than only world-frame `vx`/`vy`
- replay and trace artifacts as first-class outputs
- project-owned artifact schemas with lightweight OSS analysis tools layered on
  top, rather than a production observability stack
- action and event logs as authoritative replay inputs, with sampled traces kept
  as optional report/debug caches
- native multithreaded batch evaluation in `pd-eval`, especially for seeded
  scenario sweeps
- static inspection/reporting as part of the controller workflow, not only a
  late polish phase
- static web reports over captured artifacts, with lightweight trajectory
  inspection modes but no browser runtime target

## Why Reboot

`pylander` proved the problem was interesting, but it also mixed too many
concerns in one place:

- game runtime and presentation
- controller logic
- evaluation and benchmark orchestration
- plotting and trace tooling
- browser and Pygame delivery constraints

What mattered from that broader experimentation was the project split:

- native core
- controller layer
- evaluation and reporting layer
- thin presentation over generated artifacts

`pd-lab` borrows ideas, concepts, scenario lessons, and telemetry vocabulary
from `pylander`, but not its implementation or module layout.

It should also reuse the scenario lessons that proved useful in `pylander`
without treating the old scenario files as fixtures to transliterate directly.

## Scope

`pd-lab` owns:

- deterministic simulation
- controller and bot development
- scenario packs
- evaluation and benchmarking
- telemetry, traces, and replay artifacts
- controller telemetry and report/debug artifacts

`pd-lab` does not own:

- player progression or economy systems
- content-heavy mission design
- final game UX
- browser-first runtime constraints

## Docs

- [Architecture](docs/architecture.md) owns system boundaries and persisted
  contracts.
- [Guidance Architecture](docs/guidance.md) owns current terminal, transfer,
  waypoint, and planner/guidance responsibilities.
- [Waypoint Planning V1](docs/waypoint_planning.md) owns the implemented bounded
  planner contract, search policy, evidence model, and closure sequence.
- [Waypoint direct-route characterization](docs/waypoint_direct_characterization.md)
  records the opt-in continuous-terrain mismatch between V1's exact direct
  chord and the unchanged direct controller; it does not select or implement a
  replacement leg-profile model.
- [Direct-leg primitive research](docs/waypoint_direct_primitive_research.md)
  records the fixed 90 s analytical baseline, 24-cell obstacle sweep, and
  unchanged-controller comparison. It locates a tracking gap on taller terrain
  without demonstrating a need for an operational waypoint.
- [Complete flat direct acceptance](docs/waypoint_direct_complete_flat_acceptance_protocol.md)
  records the known-flat milestone: four complete launch-aware, held-60 Hz
  direct witnesses land on uncut flat terrain. It accepts before ranking and
  does not change planner defaults or establish held-out coverage.
- [Input-driven direct generation](docs/waypoint_direct_generation_results.md)
  records six uncut flat/uphill/downhill cases; the subsequent
  [obstacle discrimination](docs/waypoint_direct_obstacle_discrimination_results.md)
  separates blocked chosen arcs, accepted alternative direct arcs, and finite
  unknowns. The [terminal diagnostic](docs/waypoint_direct_terminal_admissibility_results.md)
  separates inadmissible ideal contact references from cadence-sensitive
  execution. The latest [body-aware terminal prototype](docs/waypoint_direct_body_aware_terminal_results.md)
  accepts all fourteen development and ten newly sealed direct-flight cases
  at held 60 Hz, without cutaways, waypoints or planner/controller/default changes.
- [Nominal direct flight integration](docs/nominal_direct_flight_integration_results.md)
  adds an opt-in mission-input CLI and complete timed-program executor through
  the ordinary controller/simulator path. All 24 now-exposed checkpoint cases
  have exact parity in two release runs. Defaults remain unchanged; robustness,
  useful setup cost and waypoint composition remain separate gates.
- [Nominal direct operational execution](docs/nominal_direct_operational_execution_results.md)
  implements strict saved-command coverage with separate actual outcomes,
  exact nominal comparison and truthful partial-run/replay evidence. All 24
  exposed controls and four presealed uncut flat/uphill/downhill missions are
  Direct, completed-safe and Match in two final-source runs. No continuation,
  robustness claim or default planner/controller change is introduced.
- [One-update terminal completion design](docs/nominal_direct_terminal_completion_reserve_contract.md)
  specifies a separately versioned completion reserve and explicit counterfactual
  segment/replay boundary. Four new physical inputs are input-only sealed; the
  reserve is not implemented or flown, and current execution/defaults are unchanged.
- [Waypoint planner V2 integration](docs/waypoint_v2_airborne_integration_results.md)
  and its [acceptance review](docs/waypoint_v2_airborne_integration_acceptance.md)
  record the first usable supported opt-in checkpoint: terrain-blind direct
  construction, local clearing and replanning from actual handoff state.
  Policy 3 passes eight uncut clear controls and sixteen ordinary terrain
  missions. Remaining diagnostics are recorded limits; normal controller
  integration and default promotion are separate decisions.
- [Conservative Ballistic Route Planning](docs/conservative_ballistic_route_planning.md)
  owns the game-oriented analytical contingency, its completed CB0 gameplay
  audit, the V2 direct-ballistic bridge certificate, and the bounded ridge
  canary. The canary establishes a scoped flat-control-green, committed-
  nominal-direct-red, terrain-derived-one-waypoint-green result while retaining
  higher direct arcs as global-replan diagnostics. Its completed F6 opt-in
  integration lane also demonstrates the typed analytical-decision to real-
  controller route seam for the retained `ridge_progress_056_probe` mission;
  this remains a one-mission development capability, not production planner
  wiring or mission-family coverage.
- [Bounded Trajectory Witness V1](docs/bounded_trajectory_witness.md) owns the
  locked post-R1 controller-neutral exact-witness contract and its implemented
  W1-W4 gates.
- [W3 Finite Proposal Spike](docs/bounded_trajectory_proposal_spike.md) owns the
  fixed untrusted command-template backend and its exact-search budget;
  proposal exhaustion remains `unknown`.
- [W4 Physical/Executor Comparison](docs/physical_executor_comparison.md) owns
  the outcome-isolated already-seen result and its strict selected-route join.
- [Roadmap](docs/roadmap.md) owns current phase status and the next execution
  slice.
- [Terminal Suite Design](docs/terminal_suite.md) and
  [Transfer Suite Design](docs/transfer_suite.md) own maintained corpus shape
  and current evidence interpretation.
- [Progress](docs/progress.md) is append-only checkpoint history; older results
  there are not current claims.
- [Early Design Scratchpad](docs/early_design.md) is retained exploratory
  history and may contain superseded directions.

## Report Serving

Generated reports live under `outputs/` and can be served locally with:

```bash
./scripts/serve-reports start
```

The script starts a simple HTTP server inside a named detached `tmux` session
and serves `outputs/` on `0.0.0.0:8000` by default. The root URL now lands on a
generated `outputs/index.html` page, and `/reports/` remains the clean
report-only subtree. The printed LAN URL resolves automatically when available.

Start at `/` or `/reports/`, then browse by subject:

- **Waypoint planning** groups the selected V2 preview, maintained planner
  baseline and related analytical studies.
- **Flight and landing control** groups terminal landing, direct transfers and
  following authored waypoint routes.
- **Research and history**, **Browse all reports** and **Raw data** are
  secondary routes. The library exposes type, status and availability; analytical
  setups are not simulation results.

The existing `/reports/guidance/` scorecards remain available within this
hierarchy. `/reports/eval/` is a compatibility entrypoint for the report library.

The guidance catalog treats smoke matrices as the primary controller-iteration
surface. Full-seed packs are supporting reliability evidence; focused frontier
and experimental packs remain visible in the all-reports index without
overstating them as the project scorecard.

Useful commands:

```bash
./scripts/serve-reports status
./scripts/serve-reports attach
./scripts/serve-reports stop
```

This is intentionally explicit. Agent skills or local tooling can call the same
script when they need a report server, but the repo-owned script remains the
canonical entrypoint.

Generated single-run, replay, and batch outputs also maintain `latest` links
under `outputs/` when written through the project CLIs, for example:

- `outputs/runs/latest/report.html`
- `outputs/replays/latest/report.html`
- `outputs/eval/latest/summary.json`
- `outputs/eval/<pack>/runs/latest/report.html`

Stable HTML entrypoints also live under `outputs/reports/`, for example:

- `outputs/reports/index.html`
- `outputs/reports/guidance/index.html`
- `outputs/reports/guidance/terminal/index.html`
- `outputs/reports/guidance/transfer/index.html`
- `outputs/reports/guidance/waypoint/index.html`
- `outputs/reports/guidance/planner/index.html`
- `outputs/reports/eval/index.html`
- `outputs/reports/eval/conservative-ballistic-ridge-f6-integration-v1/index.html`
- `outputs/reports/setups/conservative-ballistic-direct-bridge-v2/index.html`
- `outputs/reports/runs/latest/`
- `outputs/reports/eval/latest/`

The root and report-home URLs use the same topic hierarchy:

- `outputs/index.html`
- `outputs/reports/index.html`

Start with **Waypoint planning** for terrain-aware route selection, or **Flight
and landing control** for landing, direct transfer and following authored
waypoints. Research/history, the searchable report library and raw data are
secondary destinations. Report type and review status are shown separately.

To apply the current report templates to existing captures without running
simulations:

```bash
cargo run -p pd-eval -- refresh-reports
cargo run -p pd-eval -- refresh-reports --all
```

To refresh navigation and maintained scorecard indexes without rewriting
detailed report bodies, flight captures or outcome summaries:

```bash
cargo run -p pd-eval -- refresh-navigation
```

The narrower home-navigation refresh also regenerates configured topic, library,
history and raw-data indexes, but does not regenerate maintained scorecards:

```bash
cargo run -p pd-eval -- refresh-reports --home-only
```

`fixtures/reports/report_navigation.json` describes the explicit topic map.
The library also lists stable report entries with unknown topics as Unclassified;
it does not recursively publish research directories.
`fixtures/reports/navigation_preview.json` pins the under-review preview links
relative to `outputs/reports/`. This selection does not change planner defaults
or designate an accepted flight capture. Missing preview files are shown as
unavailable, without substituting historical reports.

The V2 navigation edition preserves the rich report plots and payloads and adds
return/previous/next links in new copies. Only Late ridge has waypoint annotations
at this checkpoint; the other full reports are labelled as not enhanced. Earlier
report editions and original captures remain unchanged.
New V2 flight captures still use the existing rich report writer. Annotated
retained previews are explicit; future-capture annotation integration is deferred.

To recompute the four V2 direct-ballistic probes, the bounded ridge canary, and
their setup-only HTML/SVG report without running a controller or simulation:

```bash
cargo run -p pd-eval -- conservative-ballistic-report
```

When run, this writes the reloadable analytical summary under
`outputs/setups/conservative-ballistic-direct-bridge-v2/` and the stable visual
entrypoint under
`outputs/reports/setups/conservative-ballistic-direct-bridge-v2/`. These are
setup evidence only, not controller or simulation results. The ridge result is
scoped to the flat-derived nominal lane plus its optimistic post-commit local-
correction envelope; it does not claim that every global direct replan fails.

The completed F6 opt-in integration lane is a separate command:

```bash
cargo run -p pd-eval -- conservative-ballistic-f6-integration
```

It consumes the retained `ridge_progress_056_probe` fixture, injects the
identity-bound one-waypoint route, and runs the real controller/full
simulation. Its stable report is
`outputs/reports/eval/conservative-ballistic-ridge-f6-integration-v1/index.html`.
This is one-mission development/regression evidence; it does not promote a
default planner path or claim arbitrary terrain, multiple waypoints, runtime
replanning, or mission-family coverage.

The default refresh covers packs in the guidance catalog. `--all` covers every
captured fixture-backed pack. Both rebuild batch and per-run HTML from existing
JSON bundles, preserve a recorded comparison when its basis is still readable,
and leave `summary.json` evidence unchanged.

## Opt-in Nominal Direct Flight

`pd-eval nominal-direct-flight` generates, independently verifies and executes
a complete accepted ballistic-direct program from a full scenario. It does not
use stored successful commands or call the V1 chord planner. Supply exact pad
IDs from that scenario and a new output directory:

```bash
cargo run --release -p pd-eval -- nominal-direct-flight \
  --scenario SCENARIO.json --source-pad-id SOURCE_PAD_ID \
  --target-pad-id TARGET_PAD_ID --output-dir NEW_FLIGHT_ROOT
```

Replace the uppercase arguments with real values. Use `--preflight-only`
instead of `--output-dir` for read-only validation. Direct output includes the
program, safety/contact audit, ordinary replay artifacts and `report.html`.
Unknown, Unsupported and Invalid execute no flight; Unknown is finite-family
exhaustion, not a waypoint-necessity claim or automatic fallback. Output roots
are create-only. See the [results and measured costs](docs/nominal_direct_flight_integration_results.md)
for the 24-control regression command and current nominal limits.

## Batch Eval

`pd-eval` owns scenario packs, scenario-family expansion, seed sweeps, and
native multithreaded execution.

Example:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_suite.json --workers 4
```

`run-pack` now writes stable review output to `outputs/eval/<pack>/`, but stores
the actual batch artifacts under:

- `outputs/eval/cache/<workspace-or-commit-key>/<batch-stem>/`

By default it will:

- reuse a complete candidate cache when the resolved pack digest matches
- try `--compare-ref auto`
- on a dirty workspace, compare against the clean `HEAD` cache if it exists
- on a clean workspace, compare against the previous clean commit cache if it
  exists

Add `--enforce-regression-policy` when a run should exit nonzero if the
resolved compare target fails the default regression gate. That flag requires
an explicit or cached compare baseline.

After a dirty run becomes the new checkpoint, promote it into the clean commit
key:

```bash
cargo run -p pd-eval -- promote-cache fixtures/packs/terminal_bot_lab_suite.json
```

Run the same matrix with the full seed tier:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_full.json --workers 8
```

Run the trajectory-error matrix:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_traj_err_suite.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_traj_err_full.json --workers 8
```

Run the experimental terrain backstop diagnostics:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/experimental_terrain_backstop_suite.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/experimental_terrain_backstop_full.json --workers 8
```

Run the first transfer-guidance smoke matrix:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_bot_lab_suite.json --workers 8
```

Run the nominal-radius route-angle diagnostic matrix:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_suite.json --workers 8
```

Run the transfer radius-tier diagnostics:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_radius_tier_suite.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_radius_suite.json --workers 8
```

Run the full-seed transfer reliability and frontier packs:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_radius_full_solved.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_route_angle_radius_frontier_full.json --workers 8
```

Run the route-wide waypoint landing and contract packs:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_route_angle_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_contract_route_angle_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_route_angle_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_contract_route_angle_smoke.json --workers 8
```

Run the focused planner-generated landing and route-contract packs:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/planner_generated_route_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/planner_generated_route_contract_smoke.json --workers 8
```

Run the full-seed nominal and all-radius waypoint closure packs:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_route_angle_full.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_contract_route_angle_full.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_route_angle_full.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_contract_route_angle_full.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_route_angle_radius_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_turn_contract_route_angle_radius_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_route_angle_radius_smoke.json --workers 8
cargo run -p pd-eval -- run-pack fixtures/packs/transfer_waypoint_sequence_contract_route_angle_radius_smoke.json --workers 8
```

Force a rerun and skip cache reuse if needed:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_suite.json --workers 8 --no-reuse
```

Use `terminal_bot_lab_suite` as the primary controller workbench. It is the
smoke-tier Earth `half_arc_terminal_v1` matrix over:

- `condition_set = clean`
- `vehicle_variant = empty`
  - `pylander`-aligned Earth baseline hardware with empty payload
- `vehicle_variant = half`
  - the same hardware with half payload
- `vehicle_variant = full`
  - the same hardware with full payload
- `arc_point x velocity_band`
- one maintained `current` controller lane
- optional cached comparison against an earlier result pack; comparison role is
  report provenance, not another physical-case axis

The maintained terminal baseline now matches the core `pylander`
vehicle/engine envelope closely enough to reason about directly:

- `8m x 10m` hull
- `7200kg` dry mass
- `6300kg` max fuel
- `240000N` max thrust
- `25%` ignited minimum throttle
- `90 deg/s` max rotation rate

The one intentional simplification is fuel use:

- `pd-lab` does not yet model `pylander` overdrive or the nonlinear burn
  penalty above nominal thrust
- fuel burn currently scales linearly between minimum and maximum thrust

In maintained terminal packs, `current` resolves to `terminal_pdg_v1`. Batch
reports prefer cached current-controller history when a compatible clean cache
exists. The older heuristic `baseline_v1` controller remains available for
explicit comparison fixtures, but it is not executed by the maintained clean
or trajectory-error packs and is not the project progress signal.

Use `terminal_bot_lab_full` when the same matrix should run with the full
seed tier for spread measurement. The `terminal_compare_*_fixture` packs are
only for smoke-testing pack-vs-pack compare output.

Use `terminal_traj_err_suite` and `terminal_traj_err_full` when the same
Earth/payload matrix should exercise projected miss conditions. These packs use
current-lane-only runs over:

- `traj_undershoot_small`
- `traj_undershoot_large`
- `traj_overshoot_small`
- `traj_overshoot_large`

The clean matrix keeps small seed-level radial/speed jitter. The trajectory
error matrix instead owns the lateral miss as a condition-set perturbation:
undershoot stays short on the approach side, overshoot crosses to the far side,
and the configured small/large projected miss magnitudes are recorded in each
resolved run.

Use `experimental_terrain_backstop_suite` and
`experimental_terrain_backstop_full` only for non-blocking terrain diagnostics.
They run the same Earth terminal matrix machinery, but they are explicitly not
part of the maintained terminal guidance scorecard. These packs are
current-lane-only over `empty` and `half` payload tiers, use the `diagnostic`
expectation tier, and include condition sets:

- `terrain_backstop_wall`
- `terrain_backstop_slanted`

The two backstop variants are shape variants, not low/medium height bands: both
use a `400m` terrain rise so they behave more like a wall or cliff than a small
obstacle.

The experimental terrain packs intentionally prune terrain-blind high-arc cells:
backstop entries keep `a70/a80`. Both `terrain_clip` and backstop containment
are parked as terminal-controller objectives until terrain work is reframed as
approach-corridor validation, waypoint planning, or collision-course warning.

The terminal controller contract is narrower: given a reachable, terrain-valid
approach corridor or target, land safely. Terrain condition metadata is for
diagnostics and reports, not controller mode switches.

`transfer_bot_lab_suite` is the first Phase 3 source-to-target matrix and the
fast transfer smoke gate. It uses `transfer_matrix =
signed_route_arc_transfer_v1`, the default nominal `800m` route radius,
route-angle labels from `r-60` through `r+60` in smoke tier, and the staged
`transfer_pdg_v1` controller. Transfer reports label the matrix axes as route
and radius instead of terminal arc and velocity band.

`transfer_route_angle_suite` runs the same controller, payload tiers, and fixed
nominal radius, but expands to all 11 signed route angles from `r-80` through
`r+80` with smoke seeds. It is the nominal-radius route-shape diagnostic pack.

`transfer_radius_tier_suite` keeps the smoke route-angle set and expands across
`short = 400m`, `nominal = 800m`, and `long = 1200m` radius tiers. It is the
fast distance-sensitivity gate.

`transfer_route_angle_radius_suite` combines all 11 route angles with all three
radius tiers for a 297-run wide smoke diagnostic.

`transfer_route_angle_radius_full_solved` expands the solved direct-transfer
region to full seeds: all route angles from `r-80` through `r+60`, all three
radius tiers, and all three payload tiers. It intentionally excludes the known
`r+80` frontier so the pack can answer whether the solved region is reliable.

`transfer_route_angle_radius_frontier_full` keeps the excluded `r+80` route
visible as a separate full-seed frontier watch across all radius and payload
tiers. It is not a pass/fail gate for direct-transfer controller reliability.


That writes:

- `pack.json`
- `resolved_runs.json`
- `summary.json`
- `report.html`
- optional `compare.json`
- per-run bundles under `outputs/eval/<pack>/runs/`

Batch output keeps stable semantic run directories for inspection while also
recording stable digests for the resolved pack and resolved run set.

The batch report is intentionally compare-friendly:

- a selector-aware review tree as the main drill-down surface
- explicit report context near the top of the page:
  - standalone
  - lane compare
  - external compare
  - compare basis
  - scope resolution
  - compare status
- a regression-policy panel and overview chip for compare runs
- optional candidate-vs-baseline deltas over shared run IDs
- stable links back to per-run detail reports and bundles

At this point the batch/single-run reporting stack and cache workflow are good
enough for real controller iteration. The evaluator can now:

- reuse and promote batch caches
- prefer cached current-lane history compare by default
- classify analytically impossible terminal runs separately from scored
  failures
- annotate low-thrust/high-energy frontier cells without removing them from
  scoring
- evaluate a default thresholded regression policy over compare runs, scoped to
  the preferred current controller lane when both reports contain one
- record transfer handoff diagnostics in per-run review metrics, including
  terminal entry kind, handoff gate, handoff height/speed, handoff projected
  `dx`, handoff angle, boost-cutoff quality/projected `dx`, and
  Pylander-inspired shape metrics
- render transfer-specific `Transfer Handoff Triage` and `Transfer Shape
  Triage` sections ahead of the Review Tree so transfer tuning starts from
  handoff/gate/cutoff quality before visual shape

Current checkpoint on the maintained Earth payload tiers:

- `terminal_bot_lab_suite`
  - `current`: `171 / 180` scored successes, `9` scored failures,
    `9` impossible warnings, `12` frontier annotations
- `terminal_bot_lab_full`
  - `current`: `686 / 720` scored successes, `34` scored failures,
    `36` impossible warnings, `48` frontier annotations
  - by vehicle tier:
    - `empty`: `252 / 252`
    - `half`: `252 / 252`
    - `full`: `182 / 216` scored, `34` fail, `36` impossible warnings,
      `48` frontier annotations

Trajectory-error checkpoint:

- `terminal_traj_err_suite`
  - `current`: `694 / 720` scored successes, `26` frontier failures,
    `0` core failures, `36` impossible warnings, `48` frontier annotations
- `terminal_traj_err_full`
  - `current`: `2772 / 2880` scored successes, `107` frontier failures,
    `1` core failure, `144` impossible warnings, `192` frontier annotations
  - by condition:
    - `traj_undershoot_small`: `698 / 720` scored, `22` frontier failures,
      `36` impossible warnings, `48` frontier annotations
    - `traj_undershoot_large`: `708 / 720` scored, `12` frontier failures,
      `36` impossible warnings, `48` frontier annotations
    - `traj_overshoot_small`: `684 / 720` scored, `36` frontier failures,
      `36` impossible warnings, `48` frontier annotations
    - `traj_overshoot_large`: `682 / 720` scored, `37` frontier failures,
      `1` core failure, `36` impossible warnings, `48` frontier annotations
  - by vehicle tier:
    - `empty`: `1008 / 1008`
    - `half`: `1007 / 1008`, `1` core failure
    - `full`: `757 / 864` scored, `107` frontier failures,
      `144` impossible warnings, `192` frontier annotations

Experimental terrain diagnostic snapshot:

- `experimental_terrain_backstop_suite`
  - `current`: `57 / 72` scored successes, `15` scored failures
  - `3.24s` wall clock with `8` workers
- `experimental_terrain_backstop_full`
  - `current`: `228 / 288` scored successes, `60` scored failures
  - `12.73s` wall clock with `8` workers
  - the first generic terrain-clearance candidate constraint is in place
  - `terrain_clip` is parked until it can test localized avoidance without
    forcing route-level replanning
  - the backstop packs are also parked outside the maintained terminal guidance
    scorecard

Transfer route-angle checkpoint:

- `transfer_route_angle_radius_suite`
  - `current`: `297 / 297` successes, `0` crashes, `0` invalidations
  - the maintained smoke matrix is clean across all route angles, radius tiers,
    payloads, and seeds
- `transfer_route_angle_radius_frontier_full`
  - `current`: `108 / 108` successes and `0` invalidations
  - the historical frontier name now denotes a focused steep-uphill regression,
    not a current failure region
- `transfer_waypoint_turn_contract_smoke`
  - `current`: `81 / 81` pass-through handoff successes
  - every normalized gentle, medium, and sharp waypoint contract passes without
    route/profile controller branches
- `transfer_waypoint_turn_smoke`
  - `current`: `81 / 81` final landings
  - retained terminal horizons now release when their attitude-aware vertical
    braking margin is exhausted; waypoint contract quality remains `81 / 81`
- `transfer_waypoint_sequence_smoke`
  - `current`: `27 / 27` final landings across the maintained double-bend corpus
- `transfer_waypoint_sequence_contract_smoke`
  - `current`: `27 / 27` ordered sequence successes
  - all `54` handoffs satisfy the planned tangent/energy contract at window
    entry and resolve as `contract_pass`
- `transfer_waypoint_turn_contract_route_angle_smoke`
  - `current`: `135 / 135` handoff successes over
    `r-60 | r-30 | r00 | r+30 | r+60`
- `transfer_waypoint_turn_route_angle_smoke`
  - `current`: `135 / 135` final landings with `0` invalidations
  - final-waypoint states are ranked by terrain-blind terminal recoverability
    after satisfying the waypoint contract
- `transfer_waypoint_sequence_contract_route_angle_smoke`
  - `current`: `45 / 45` ordered sequence successes
- `transfer_waypoint_sequence_route_angle_smoke`
  - `current`: `45 / 45` final landings with `0` invalidations
  - all `90` ordered handoffs and all `45` final landings complete cleanly
- full-seed nominal waypoint closure:
  - turn landing and contract: `540 / 540` for both goals
  - ordered landing and contract: `180 / 180` for both goals
- all-radius waypoint closure:
  - turn contract and paired landing: `405 / 405` for both goals
  - ordered contract and landing: `135 / 135` for both goals
  - bounded final authority-recovery search closes the former
    `single_gentle_bend_v1/full/r-30/short/seed 02` landing residual
- `transfer_waypoint_sequence_late_bend_diagnostic`
  - `current`: `27 / 27` final landings and complete route telemetry
  - `27 / 54` handoffs enter the capture radius outside the envelope, then
    recover before the waypoint plane; this profile is diagnostic, not a gate
- smoother `r+80` bend reset:
  - landing: `15 / 27` smoke and `54 / 108` full
  - handoff contract: `21 / 27` smoke and `89 / 108` full

The waypoint corpus uses fixed route-frame geometry rather than silently lifting
waypoints in world Y. Maintained ordered waypoints carry an explicit handoff
tangent: the normalized inbound/outbound angle bisector. Spatial radius entry
opens an acceptance window; guidance keeps the active leg until the contract
passes or the craft reaches the waypoint plane. Maintained handoff envelopes
also cap energy and reject fixtures whose optimistic stopping-distance ratio
exceeds `0.75`. Schema `34` reports the immutable plan tangent, window-entry
snapshot, final resolution reason, window duration, and final-terminal
recoverability separately. Full-seed nominal closure is `540 / 540` for turn
landing/contracts and `180 / 180` for ordered landing/contracts. All-radius
contracts are `405 / 405` turn and `135 / 135` ordered; paired landings are
`405 / 405` and `135 / 135`. Final-state ranking is terrain-blind and uses
terminal braking authority rather than route/profile labels. Controller compute
remains below the `1ms` p99 budget.

The old `single_dogleg_v1` packs and the full-matrix `late_bend_v1` pack remain
parked diagnostic history rather than acceptance gates.
Terrain-blind waypoint guidance v1 is closed over the preplanned maintained
corpus. The bounded deterministic pad-to-pad planner defined in
`docs/waypoint_planning.md` is now implemented through evaluator integration
and schema-36 evidence rendering. Retained focused captures close at `54 / 54`
generated-route landings and `36 / 36` handoff/ordered contracts with zero
invalidations, and fresh maintained no-regression packs reproduced their
declared baselines. General runtime avoidance remains parked at the
planning/collision-warning layer; broader angles, radii, or terrain classes
require a separate planner checkpoint.
The guidance implementation now follows the ownership boundaries in
`docs/guidance.md`: terminal and transfer are separate modules, pure waypoint
geometry is isolated from controller lifecycle state, telemetry emission is
separate from control decisions, controller tests live outside production
modules, and rejected boost scorers remain reproducible diagnostics rather than
maintained modes.
Detailed checkpoint history lives in `docs/progress.md`,
`docs/transfer_suite.md`, and `docs/terminal_suite.md`.
