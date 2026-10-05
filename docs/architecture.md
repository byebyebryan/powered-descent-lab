# Powered Descent Lab Architecture

## 1. Purpose

Powered Descent Lab is a control and simulation lab for 2D rocket flight.

It exists to support:

- deterministic simulation
- controller and bot development
- scenario design
- batch evaluation
- telemetry, traces, and replay analysis

It is not the player-facing game.

## 2. Design Principles

### Core is the source of truth

The simulation core owns world state and state transitions. Controllers,
evaluation tooling, and viewers sit around that core.

### Native-first, reports on web

The primary runtime is native. Any web output should consume generated artifacts
as reports and inspection surfaces, not act as an authoritative runtime.

### Deterministic by default

Repeatability is a product feature. Every scenario, run configuration, and
controller configuration should be reproducible from explicit inputs.

### Stable contracts over ad hoc access

Controllers should work through documented inputs and outputs, not by reaching
into engine internals.

### Lightweight hot path, rich offline artifacts

Per-tick observation should stay compact. Rich trace data, plots, and debug
artifacts belong in optional capture paths and offline analysis.

### Scenarios are data-first

Scenario identity should come from authored data and metadata. CLI selector
syntax is useful, but it should remain a thin layer over named scenarios and
scenario packs.

The lab should distinguish between:

- concrete scenarios, which are fully resolved and directly runnable
- scenario families, which define a curated perturbation space plus seed policy

`pd-core` should consume only resolved concrete scenarios. Seed expansion,
scenario-family expansion, and randomized coverage belong in `pd-eval`.

### Concrete v1 stack

The implemented stack is:

- Rust cargo workspace
- `clap` for native CLI entry points
- `serde` for scenario, replay, trace, and report artifacts
- `rayon` for bounded batch parallelism in `pd-eval`, not as a core simulation
  contract assumption

There is no frontend framework decision in v1 because there is no interactive
frontend requirement in v1.

Reporting is static-output-first:

- machine-readable run artifacts
- aggregate CSV/JSON summaries
- generated static HTML reports

The browser is a report viewer, not an authoritative runtime.

This project borrows ideas, scenario concepts, telemetry concepts, and behavior
lessons from `pylander`, but not its implementation or module boundaries.

The intended telemetry/reporting stance is hybrid:

- own the canonical artifact schemas
- use lightweight OSS tools for generic analytics and profiling
- keep custom report UX only where the domain is genuinely specific

## 3. Problem Model

### World

The lab models a 2D side-view flight problem:

- gravity
- static terrain
- landing targets
- separate static-obstacle entities as a future world-model extension; current
  V2 obstruction cases use terrain relief
- vehicle and mission constraints

### Vehicle

The vehicle model should support the use cases discovered in `pylander` without
locking the lab to one game-specific ship fantasy:

- dry mass and propellant mass
- thrust limits
- throttle floor and ceiling
- attitude and attitude-rate limits
- landing constraints

### Mission

A mission describes what the controller is trying to do:

- initial vehicle state
- designated target site for landing scenarios
- evaluation goal
- success and failure conditions
- optional non-landing termination checkpoint
- run-time disturbances as a future mission-model capability

The current native V2 evaluation pack supplies static scenarios and terrain;
it adds neither a separate obstacle-entity API nor a runtime-disturbance
payload.

For v1, a mission should have exactly one primary goal.

Examples:

- land firmly on this target pad
- end at boost cutoff and validate the projected post-burn trajectory

Secondary metrics still matter, but the run should have one authoritative
success predicate.

### Scenario

A scenario is the packaged unit of evaluation:

- world specification
- vehicle specification
- mission specification
- deterministic seed
- metadata and tags

Each concrete scenario should stay small and specific. The lab does not need to
optimize for huge worlds or streaming content in v1.

Examples of scenario families:

- terminal descent
- point-to-point transfer
- preplanned waypoint transfer and ordered waypoint contracts
- experimental terrain diagnostics outside maintained guidance gates
- named regressions for past failures

### Transfer Missions

Transfer guidance is now a first-class mission family rather than a terminal
condition. A transfer mission can carry an optional `transfer_route` alongside
the single authoritative evaluation goal:

- `source_pad_id`
- `target_pad_id`
- `route_angle_deg`
- `route_radius_m`
- ordered `waypoints`, each with a spatial capture and outbound-state envelope

The transfer route describes source-to-target setup and controller context. The
maintained reliability goal remains landing on the target pad. Focused
`waypoint_handoff` and `waypoint_sequence` goals are separate early-stop probes,
not simultaneous secondary objectives. Each scenario still has one
authoritative goal, so physical landing and route-contract evidence remain
explicit rather than being blended into one outcome.

The first transfer matrix uses a signed one-sided route arc with the target at
`(0, 0)` and the source resolved from route angle and radius. Positive route
angle means the target is uphill from the source. The matrix records
transfer-specific report axes (`route_family`, `route_angle`, `radius_tier`) and
uses terminal-compatible aliases only as report plumbing.

Waypoint work splits planning from guidance.

Waypoint planning is distinct from authored-route guidance. The maintained
native V2 evaluator constructs a terrain-blind nominal transfer, audits the fixed
program against actual heightfield terrain, applies local correction where
possible, and replans from the actual handoff state. This offline loop may hand
off at useful safe progress before a feature's far edge; it does not require a
landing suffix. It is evaluator-owned, not a `pd-control` update-loop
capability. The earlier chord-based V1 setup search is retired; saved route
contracts remain readable. See the [current V2 evaluation status](waypoint_planning.md#current-v2-evaluation-status-2026-10-04).

Waypoint guidance assumes the waypoint list is already planned, follows the
currently active route leg, and enters a bounded handoff window at the waypoint
capture radius. The plan provides the desired handoff tangent and energy
envelope. Guidance keeps the current leg active until that contract passes or
the craft reaches the waypoint plane, then switches to the next leg or final
landing target. A waypoint is not a stop-and-land objective or an isolated
point target; it is a switching surface and handoff contract between route legs.

Final `landing_on_pad` remains the product-level score. Ordered arrival quality
is preserved as handoff telemetry and can be scored independently with
`EvaluationGoal::WaypointSequence`: the core advances through waypoint
contracts in order, stops at the first failed contract, and succeeds only after
the final handoff. Run summaries preserve passed/total and first-failure index;
batch artifacts preserve each handoff's window-entry state, resolution reason,
planned-tangent error, closest approach, cross-track miss, speed, vertical rate,
turn margin, and replan count. This keeps one primary goal per scenario while
preventing a later landing from hiding poor route guidance.

Waypoint guidance v1 is closed against a preplanned maintained corpus spanning
turn and ordered routes, full nominal seeds, and route-radius tiers. Initial
launch energy is regulated from immutable inbound-leg geometry, while final
handoff selection and direct terminal entry use terrain-blind recoverability.
Retained V1 batch schema `36` adds optional planner provenance, route diagnostics,
planned-versus-sampled clearance, and per-solve monotonic wall-time evidence to
the schema-34 guidance evidence. Planner timing is excluded from deterministic
plan and batch identity. The planner owns terrain-valid placement, leg
ordering, and arrival-envelope
design; guidance must not infer obstacle classes or repair a structurally bad
route. Legacy authored runs retain their existing behavior with the planner
fields absent. Retained focused captures close at `54 / 54` generated-route
landings and `36 / 36` handoff/ordered-contract runs with zero invalidations.
These are historical generated-route results, not current V2 flight records.

The controller implementation mirrors this ownership. `pd-control` keeps the
registry and legacy controllers in `controllers.rs`, shared state-target math in
`guidance.rs`, terminal guidance under `terminal/`, and transfer guidance under
`transfer/`. Pure waypoint geometry and contract prediction live in
`transfer/waypoint.rs`; transfer and waypoint metric/marker emission lives in
`transfer/telemetry.rs`; maintained endpoint scoring lives in
`transfer/scoring.rs`. Rejected boost scorers are retired; old false-default
configuration fields remain readable, while enabling them fails explicitly.
Terminal and transfer tests live in sibling test
modules instead of production files. These are internal module boundaries, not
changes to controller JSON, IDs, telemetry, phase strings, or report artifacts.

## 4. Terrain Direction

The canonical terrain model should remain a 1D piecewise-linear heightfield in
v1.

That choice preserves several advantages:

- cheap deterministic queries
- easy authoring and debugging
- good fit for descent and transfer problems
- continuity with the flight and control problem explored in `pylander`, not
  with its implementation shape

The important change is not the terrain foundation. The important change is the
query layer built on top of it.

The implemented heightfield query layer provides:

- height and slope at `x`
- local surface normal
- full immutable terrain through the setup-time `RunContext`
- strict, non-clamping height lookup for planning validation
- exact segment-to-heightfield corridor clearance for a linearly varying
  conservative envelope
- endpoint-aware route validation and stable minimum-clearance diagnostics

Closest-point and ray queries remain useful later for collision warnings and
reactive guardrails, but they are not prerequisites for the bounded planner.

Controllers are therefore not forced to rediscover the world through only
per-frame local sensors, while planner-specific geometry can be added without
bloating the hot observation path.

This is a simple controller-facing terrain package, not a hidden view
over engine internals and not a sensor-gated approximation.

Terrain avoidance is not a core terminal-controller objective for the current
lab slice. The terminal controller should assume a reachable, terrain-valid
approach corridor or target and focus on the high-frequency work it already owns:
braking, lateral cleanup, descent-rate control, attitude, and touchdown.

Terrain responsibilities sit above that controller boundary:

- co-pilot use should favor target/route validation and collision-course
  warnings before handoff to terminal guidance
- pure non-human bots should eventually use waypoint or corridor planning so
  terrain clearance is handled before terminal landing begins
- waypoint guidance should be terrain-blind in v1. Terrain avoidance should be
  encoded indirectly by the planned waypoint positions and arrival envelopes,
  not by controller branches that detect obstacle labels or special cases.
- any reactive avoidance that remains inside terminal guidance should be small
  and local, such as holding descent or rejecting an unsafe candidate, not
  rerouting around arbitrary terrain

Reactive terrain diagnostics should still be expressed as terrain geometry and
clearance observations, not scenario-specific controller modes. Scenario
metadata can identify a fixture as a backstop or descent-clip case for reports,
but the controller should not branch on those labels.

Optional obstacle layers can be added later for structures or hazards without
discarding the heightfield as the canonical ground model.

Current V2 terrain obstructions are represented by heightfield relief; the
workflow does not define a separate obstacle-entity API.

`pd-lab` should not start with:

- SDF as the source of truth
- fully arbitrary polygonal cave systems
- procedural complexity for its own sake

Those are valid future experiments, but they would expand the problem faster
than the controller lab needs.

The lab also does not need terrain LOD in v1. Scenarios are small enough that a
single canonical terrain representation is the right default.

## 5. Layer Model

### 5.1 `pd-core`

`pd-core` owns the authoritative simulation.

Responsibilities:

- world and vehicle state
- deterministic stepping
- heightfield terrain and terrain-derived clearance queries, including exact
  conservative corridor clearance
- neutral serialized route-planning contracts and shared route validation
- mission setup
- observation generation
- action validation and actuation rules
- event emission
- replay and trace schemas

`pd-core` should not own:

- controller strategy
- batch orchestration
- plotting
- frontend code

### 5.2 `pd-control`

`pd-control` owns controller interfaces and built-in controllers.

Responsibilities:

- controller trait
- controller identity and factory/registry
- controller configuration schemas
- baseline controller implementations
- shared planning and optimization helpers
- controller-local telemetry
- controller-facing status, phase, and report/debug markers

Controllers should consume:

- immutable run context at setup time
- compact per-tick observation during stepping

Controllers should produce:

- vehicle command
- optional debug and telemetry payloads

The new bot framework should borrow the useful parts of `pylander`'s bot layer:
rich setup-time environment, compact per-tick state, and controller-owned
inspection data. It should not move mission success authority back into the
controller layer.

### 5.3 `pd-plan`

`pd-plan` owns current deterministic planning math and sealed policies.

Responsibilities:

- discrete ballistic bridge and coast kinematics
- canonical initial-transfer basis construction
- supported vehicle input and conservative bridge margins
- local-clearing finite policy and candidate math
- versioned policy-3 bounds, correction limits and clocks

`pd-plan` depends only on neutral contracts and terrain queries in `pd-core`. It
must not depend on `pd-control`, select controller IDs, execute simulations, or
own evaluator/report policy. Its math is always available, without a research
feature gate. V1 visibility search, `pd_plan::plan` and research candidate
exposure are retired. Neutral saved route DTOs stay in `pd-core`; current physical
realization and the policy-3 flight loop stay in `pd-eval`. See
[Waypoint Planning](waypoint_planning.md) and the
[retirement results](planner_retirement_cleanup_results.md).

### 5.4 `pd-cli`

`pd-cli` is the single-run native entry point.

Responsibilities:

- run one scenario with one controller
- emit readable console summaries
- export replay and trace artifacts
- support targeted debugging and inspection

This is the primary developer entry point for targeted native runs and replaces
the old mixed interactive/headless shell role.

### 5.5 `pd-eval`

`pd-eval` owns repeated execution and analysis.

Responsibilities:

- batch scenario packs
- scenario-family expansion
- seeded coverage and regression sweeps
- deterministic native parallel execution
- baseline comparisons
- regression suites
- telemetry aggregation
- performance profiling hooks
- batch summary and report orchestration

The eval layer should orchestrate runs around the same core and controller
contracts used by `pd-cli`.

The current `planner_v2_lab_suite` is a separate native evaluator path with
policy 3 selected by the default `run-pack` workflow. It records its own V2
batch schema and complete flight evidence; it does not create a V2 controller
variant or change ordinary `pd-cli run` behavior.

`pd-eval::WaypointV2Session` also exposes that same owned, synchronous flight
loop to the default-off `pd-cli` `planner-v2` feature. The session retains the
actual live handoff state between whole-piece advances and requires final
source replay before success. This is an explicit lab adapter, not a per-tick
controller, externally supplied snapshot restart or evaluator-free game API.
See [Guidance Architecture](guidance.md#current-v2-design-and-support) for the
V2 module boundaries and supported envelope.

Its implementation is split by responsibility rather than pack family:

- `model.rs` owns persisted batch, cache, comparison, and review DTOs
- `resolution.rs` validates packs and expands them into concrete runs
- `execution.rs` schedules resolved runs and classifies analytic feasibility
- `runtime.rs` owns scenario/controller loading plus artifact and cache support
- `comparison.rs` and `review.rs` derive cross-run and per-run evidence
- `report.rs` is the batch-report shell; its `report/` children own overview,
  diagnostics, review-tree, and comparison presentation
- `planner_flight/` owns current nominal input/preflight, canonical initial
  source fitting, state-derived airborne acquisition, terminal realization and
  contact/terrain audit, with pure ballistic math delegated to `pd-plan`
- `waypoint_v2/` owns the piecewise session, live execution, persisted records
  and replay-safe proofs; `local_clearing.rs` owns current correction helpers
- `waypoint_v2_pack/` separates native models, input expansion,
  execution/aggregation, capture validation, provenance and common presentation
- `evidence_io.rs` provides neutral exact hashing and create-only writing;
  callers retain their own trust and provenance checks

These are internal boundaries. Current entrypoints, persisted JSON schemas,
cache layout and stable report paths remain compatibility surfaces. Obsolete
research Rust exports and executable frontdoors are intentionally retired,
not kept as migration shims. Serialized legacy-looking DTO names required by
current or saved records are not renamed for tidiness.

Recommended parallelism boundary:

- `pd-core` stays single-run and deterministic
- `pd-eval` expands packs and seed sweeps into concrete runs
- `pd-eval` may execute independent runs in parallel on native threads
- immutable scenario data such as compiled terrain can be shared across worker
  threads where useful
- output ordering and aggregate reports should still be written in a stable,
  deterministic order

### 5.6 `pd-report` and the static report viewer

A minimal inspection/report path is part of the core bot-lab workflow, not only
late polish.

The current split is:

- `pd-report` owns reusable single-run static report and trajectory rendering,
  plus dedicated setup-only analytical planning reports
- `pd-report::batch` owns the shared full batch shell, sections, review-tree
  rows, and interactions used by controller batches and native V2
- `pd-report::site` owns stable report paths, latest links, and shared site
  index generation
- `pd-cli` invokes that path for targeted one-run inspection
- `pd-eval` owns domain aggregation, batch adapters and publication over the
  same captured artifacts; it supplies native V2 data to the shared batch
  renderer and also orchestrates deterministic setup reports from feature-gated
  planner projections without invoking a controller or simulator
- `fixtures/reports/guidance_catalog.json` declares the curated terminal,
  direct-transfer, and waypoint evidence scorecards without making generated
  outputs source-controlled truth

Responsibilities:

- summary report generation
- trace and replay inspection pages
- single-run trajectory and state inspection
- batch summary and candidate-vs-baseline compare pages
- setup-only analytical mission geometry and candidate classification
- lightweight interaction over precomputed run data
- trajectory scrubbing, hover, or drag-based state inspection

They should not own:

- authoritative simulation
- controller execution
- benchmark orchestration

The implemented report baseline answers basic controller questions without
requiring raw JSON inspection:

- where the vehicle flew relative to terrain and target
- how altitude, clearance, velocity, attitude, and throttle evolved
- where discrete events and controller phase/status changes happened
- how a candidate batch changed relative to a known baseline over shared runs

Report navigation is topic-first, with evidence boundaries visible within each
topic. The root and `/reports/` share one home. Waypoint planning groups the
active policy-3 V2 evaluation batch, the legacy V1 planner baseline and related
analytical studies. Flight and landing control groups terminal landing, direct
transfers and following authored waypoint routes. Research/history, a
searchable report library and raw data are secondary choices. Report type and
review status are metadata rather than peer top-level subjects.

`fixtures/reports/report_navigation.json` is the explicit topic map; unknown
stable report entrypoints remain visible as Unclassified. Navigation never scans
every research directory or chooses current evidence from modification time.
The active V2 entry resolves the selected native batch capture; the older
`navigation_preview.json` fixture pins a historical presentation edition and
does not select current evidence. Historical flight provenance and report
editions remain separate.

Evidence presentation keeps these distinctions:

- `outputs/reports/guidance/` preserves the maintained cross-guidance scorecard
- each guidance group separates primary smoke evidence from supporting
  full-seed evidence
- `outputs/reports/library/` is the friendly report inventory, including missing
  fixture captures and unclassified stable reports; `/reports/eval/` is its
  compatibility entrypoint
- `outputs/reports/setups/` contains deterministic analytical setup evidence
  that is explicitly separate from controller runs and simulation claims
- the retained conservative-ballistic setup contains the V2 direct-bridge
  certificate and one bounded ridge canary; the canary keeps flat-derived
  nominal-lane status, optimistic post-commit correction evidence, derived
  blocking terrain, terrain-derived waypoint evidence, and higher global-
  replan diagnostics distinct, and its report path is versioned so superseded
  analytical projections cannot be mistaken for current evidence; its obsolete
  standalone generator is retired, while saved pages remain linked and unchanged
- batch pages lead with outcome totals and selector coverage, while provenance,
  context, and guidance-specific diagnostics remain available in collapsed
  sections
- run pages lead with mission outcome and trajectory evidence; controller
  guides, scalar overlays, and vectors are explicit plot modes rather than
  default clutter

`pd-eval refresh-reports` is a presentation-only operation. It deserializes
captured run bundles and batch summaries, regenerates static HTML in parallel,
and reuses a recorded comparison only when that comparison directory remains
readable. It must not execute controllers or rewrite authoritative batch
summaries.

`pd-eval refresh-navigation` regenerates navigation and maintained scorecard
indexes only. In topic-hierarchy mode `ReportSite` is the sole owner of home,
topic, history, library and compatibility indexes; the evaluator catalogue owns
guidance scorecards but does not compete for the eval index. Both refresh orders
must preserve the same navigation. `refresh-reports --home-only` regenerates
the site navigation without scorecards or detailed report bodies.

The active V2 batch is published at
`/reports/eval/planner_v2_lab_suite/`; `current.json` selects its create-only
capture under `outputs/eval/planner_v2_lab_suite/`. Its batch page uses the
shared full batch template, and supported details retain the rich flight
renderer with additive executed-handoff annotations. Unsupported cases use a
status page without invented flight plots. The evaluator-owned native
acceptance gate binds the complete frozen pack, requires ordinary target
landings and verified evidence, and keeps diagnostic expectations separate.
Failed captures retain their evidence and reports but cannot replace the
accepted current site; the checked CLI reports a failure outcome. Report
regeneration is presentation-only: it preserves raw capture evidence and records the selected
summary, renderer, and page hashes. Older preview editions remain historical
and do not replace the active batch.

### 5.6 Telemetry and reporting stack

`pd-lab` should not center itself on a production metrics stack such as
Graphite/Grafana or Prometheus/Grafana.

Those systems are optimized for long-lived services and live operational
metrics. `pd-lab` is centered on run artifacts, batch comparison, and offline
inspection.

Recommended ownership boundary:

- `pd-lab` owns run, sample, event, and summary schemas
- `pd-lab` owns domain-specific report pages and trajectory inspection UX
- external tools are used for generic analytics and profiling

Recommended supporting tools:

- `DuckDB` for local analytics over Parquet/JSON artifacts
- `Perfetto` for profiling and timeline-style trace inspection
- `Vega-Lite` plus `vega-embed` for generated summary charts in static reports

Optional convenience layer:

- `Aim` or a similar local experiment tracker can be added later for run
  browsing, but it should not become the source of truth

Build-vs-buy stance:

- keep custom per-run flight inspection because terrain, trajectory, touchdown,
  and controller overlays are domain-specific
- avoid rebuilding generic aggregation and trace-analysis plumbing when mature
  local tools already exist

This is the main tradeoff relative to late `pylander`: keep the custom parts
that genuinely need to be custom, and replace the generic reporting/analytics
plumbing around them.

Practical sequencing rule:

- invest in minimal inspection early
- defer only the richer and more polished report UX

## 6. Current Repo Shape

The implemented workspace shape is:

```text
powered-descent-lab/
  README.md
  docs/
  fixtures/
    packs/
    scenarios/
  pd-core/
  pd-plan/
  pd-control/
  pd-report/
  pd-cli/
  pd-eval/
  scripts/
  outputs/      # generated and ignored
```

Notes:

- `fixtures/scenarios` stores authored concrete scenarios
- `fixtures/packs` stores batch matrices, comparison fixtures, and maintained
  regression gates
- `pd-report` provides reusable per-run rendering; `pd-eval` owns batch report
  assembly and comparison UX
- `outputs/` stores generated run bundles, cache entries, stable report aliases,
  and report indexes; it is not source-controlled truth
- `pd-plan` depends only on `pd-core`; controller selection, simulation, cache
  orchestration, and evidence presentation remain outside the planner

## 7. Contracts

## 7.1 Canonical Command Surface

The core should consume actuator-space commands, not planner-friendly idealized
accelerations.

Canonical command examples:

- throttle command or normalized throttle target
- attitude or gimbal target
- optional engine mode flags if required by the plant model

Reason:

- the plant should own actuation limits and lag
- human, scripted, and autonomous controllers can share the same command surface
- controllers remain free to plan in acceleration space internally without
  forcing that abstraction into the core contract

### Controller cadence

The simulation should step at a fixed physics rate, and controller updates may
run at a lower fixed cadence.

Recommended v1 stance:

- physics uses a fixed-step update
- controller updates may be less frequent than physics
- the latest controller command is held constant between controller ticks

This preserves determinism while allowing controller cost to stay decoupled from
the finest physics step.

## 7.1.1 Controller output frame

The controller contract should be richer than `Command` alone.

Recommended shape:

- `command`: authoritative actuation request consumed by the core
- `status`: short human-readable mode string
- `phase`: optional structured controller phase label
- `metrics`: controller-owned numeric or categorical diagnostics
- `markers`: optional discrete annotations for reports and replay

Only `command` is authoritative for simulation. The rest exists to make
controller behavior explainable during evaluation and reporting.

This is one of the main places to borrow ideas from `pylander`'s bot framework
without copying its exact interface shape.

## 7.2 Observation Surface

Per-tick observation should remain compact and hot-path friendly.

Examples:

- pose and velocity
- orientation and angular rate
- mass and fuel state
- target-relative geometry
- touchdown-clearance and contact-style signals
- controller-visible mission and timing metadata

The full terrain should not be copied into every observation frame.

Target-relative geometry should be defined explicitly as convenience data in the
designated landing target's local frame, for example:

- `dx`, `dy` from vehicle reference point to target center
- target pad half-width
- target surface height, tangent, and normal
- along-track and cross-track error relative to the target surface

This is a convenience layer, not a replacement for world-frame pose or full
terrain access.

Non-landing evaluation scenarios can expose separate goal-specific convenience
fields when useful, but they should not overload the meaning of `landing`.

## 7.3 Vehicle Geometry and Reference Frames

The lab should separate four geometry concepts that were too easy to blur
together in `pylander`:

- dynamics reference frame
- collision hull
- touchdown footprint
- render shape

### Dynamics reference frame

The authoritative body state should be expressed at the vehicle's center of mass
or another clearly defined inertial reference point.

This is the right frame for:

- integration
- control
- mass properties
- thrust application

It is not automatically the right frame for altitude, touchdown, or contact
reasoning.

### Collision hull

The collision hull should be a simple convex shape owned by the simulation.

Recommended stance:

- do not hard-code a triangle as the canonical vehicle shape
- do not use a circle as the only physical shape for landing logic
- prefer a convex polygon or box-like hull in v1

A broad-phase bounding circle or AABB is still fine as an acceleration
structure, but it should not define actual touchdown behavior.

### Touchdown footprint

Landing should use dedicated touchdown geometry rather than the body origin.

Recommended v1 model:

- a landing segment or two touchdown points in body-local coordinates
- optional future support for multi-leg footprints

This footprint is what should drive:

- touchdown clearance
- footprint-over-pad checks
- touchdown contact classification

### Render shape

Rendered art is separate. A vehicle can be drawn as a triangle, box, or
something more detailed without redefining the authoritative contact model.

### Ground-reference metrics

The lab should avoid using "ship position" ambiguously in ground interaction
logic.

At minimum, runs should distinguish:

- body origin pose
- touchdown footprint world pose
- minimum hull clearance to terrain

Altitude-like convenience values in observations should be based on touchdown
geometry or explicit ground-clearance definitions, not on the mistaken
assumption that the body origin is the contact point.

## 7.4 Setup-Time Run Context

Controllers need more than a per-frame snapshot. At reset time they should
receive immutable run context with:

- vehicle limits
- mission goal
- target geometry
- full terrain definition
- terrain query helpers
- scenario metadata

This follows one of the useful lessons from late `pylander` work: rich
environment access is useful, but it should not bloat the hot path.

Passing the whole terrain at setup time is more general than forcing the bot to
infer the world from local samples alone, and it keeps per-tick observations
small.

## 7.5 Events and Outcomes

The core should emit structured events for notable state transitions:

- touchdown
- crash
- fuel exhaustion
- boundary violation
- controller reset or run abort conditions
- mission success and failure reasons

Events should be usable in both human-readable summaries and replay artifacts.

Controller markers are separate from core events. They belong in a controller
namespace so built-in and future controllers can annotate runs without changing
the core event contract.

## 7.6 Landing Success Contract

Landing success should not be defined only by world-frame `vx` and `vy`.

The authoritative touchdown check should be expressed in the contact frame of
the touched surface:

- valid landing-surface contact
- touchdown footprint overlaps the allowed landing area
- closing speed along surface normal is below threshold
- tangential or shear speed along surface tangent is below threshold
- attitude error relative to the landing surface normal is below threshold
- angular rate at touchdown is below threshold

For v1, `landing` should mean stable touchdown on the designated target pad.

This is preferable to using collision force or impulse as the primary rule in
v1 because impulse is more solver- and timestep-dependent.

Impulse or impact-energy-style values can still be recorded as telemetry.

Recommended outcome split:

- `landed_success`
- `touchdown_off_target` for stable off-target ground contact, if recorded
- `crashed`

Off-target touchdown is not counted as `landing`. It is a separate physical
outcome and a mission failure for landing scenarios.

That preserves the difference between mission failure and total destruction
without weakening the meaning of `landing`.

Some scenarios may terminate before any touchdown classification exists at all.
For example, a boost evaluation may end at boost cutoff and judge the projected
post-burn trajectory instead of the final touchdown.

## 7.7 Trace and Replay Artifacts

Trace data should be a first-class output of the lab.

Recommended authoritative artifact split:

- one scenario spec or equivalent scenario snapshot to make replay bundles
  portable across machines and worktrees
- one run manifest with scenario, controller, config, result, and summary
  metrics
- one action log sufficient to replay controller outputs over time
- one event stream for discrete events and controller debug markers
- one optional profiling trace for runtime and solver timing
- one optional sampled trace cache for report generation and debugging

Recommended encoding direction:

- project-owned canonical schemas
- Parquet where columnar aggregate analysis is useful
- JSON or JSONL where portability and debugging are more important
- profiling traces emitted in a format that existing trace tooling can inspect

Sampled state/observation traces are still useful, but they should be treated
as optional caches for reports and debugging rather than the sole authoritative
source of replay.

Recommended stance:

- a replay bundle should be runnable without an external scenario file
- actions and events are the primary replay inputs
- sampled traces are decimated or event-focused by default
- dense sampled traces are a debug mode, not the default contract
- per-run bundle and report paths should prefer stable semantic keys such as
  scenario ID, controller ID, family, and seed
- commit hashes, config digests, or timestamps should only enter artifact names
  when they are actually needed for cache identity, collision avoidance, or
  compare workflows

The current compatibility baseline uses versioned JSON for manifests, actions,
events, samples, controller updates, and summaries. A later columnar derivative
may optimize large-scale analysis, but JSON remains authoritative for native
tooling and static reports.

If controller output frames include status, phase, metrics, or markers, those
should be captured in controller-namespaced artifacts rather than flattened into
the core manifest schema.

## 7.8 Telemetry namespaces

Telemetry should stay split between:

- lab-owned generic metrics and events
- controller-owned metrics in a controller namespace
- profiler/runtime traces that are separate from authoritative sim outputs

This keeps evaluation stable while allowing controller-specific diagnostics to
grow without polluting the core result contract.

## 7.9 Controller Phase Ownership

The core should not hard-code one guidance decomposition such as `boost`,
`coast`, `terminal`, and `touchdown`.

Those are useful controller concepts, but they are not fundamental plant state.

Recommended boundary:

- the core owns physics, events, goals, and constraints
- controllers may implement staged or unified guidance internally
- controller phase changes can be reported through controller telemetry, not as a
  mandatory core state machine

This keeps the lab open to multiple controller styles instead of baking the
current `pdg` mental model into the simulation layer.

## 8. Determinism Policy

Determinism should be specified in tiers rather than hand-waved.

### Tier 1: same build, same target, same seed

This is the required guarantee for v1. Re-running the same scenario with the
same controller configuration should produce identical results on the same
platform and build.

### Tier 2: cross-machine native consistency

This is desirable but secondary. It should be tested explicitly, not assumed.

### Tier 3: replay and report fidelity

Generated reports and replay viewers should faithfully reflect captured run
artifacts. If any visualization layer derives secondary values, those
derivations should be documented and bounded rather than treated as
authoritative sim state.

Operational rules:

- fixed-step simulation
- seeded randomness only through explicit scenario inputs
- eval-side parallel execution must not change per-run results
- no wall-clock time in authoritative state transitions
- profiling and compute timing are observed outputs, not simulation inputs

## 9. Scenario and Pack Model

Scenario authoring should avoid another positional-selector trap.

Recommended model:

- every concrete scenario has a canonical ID
- scenario metadata carries family, tags, difficulty, seed, and notes
- scenario families define curated randomized perturbations, not unbounded fuzz
- packs group scenarios by explicit inclusion or tag queries
- packs may also expand scenario families over explicit seed sets or seed-sweep
  ranges
- CLI selectors are convenience syntax on top of data-backed identity

That lets the lab keep human-friendly handles without baking too much taxonomy
into one parser.

For terminal guidance specifically, the selector model should be more explicit
than one flat family name:

- physical-case hierarchy:
  - `mission`
  - `arrival_family`
  - `condition_set`
  - `vehicle_variant`
- dense matrix axes inside a physical case:
  - `arc_point`
  - `velocity_band`
- small local variation:
  - `seed`
- controller comparison lane:
  - `controller`

This is intentionally not a pure tree. The hierarchy axes answer "what class of
terminal case is this?", while the matrix axes answer "where inside the arrival
profile space did it land?". Controller choice should stay separate from the
physical-case selector so the same case can be compared across multiple lanes.

Example resolved selector:

- `mission=terminal_guidance`
- `arrival_family=half_arc_terminal_v1`
- `condition_set=clean`
- `vehicle_variant=half`
- `arc_point=a30`
- `velocity_band=mid`
- `seed=0006`
- `controller=terminal_pdg_v1`

The pack implementation now represents this model directly:

- terminal-matrix entries declare physical selectors, seed tier, vehicle
  variant, expectation tier, and controller lanes
- resolution records explicit selectors and perturbation parameters per run
- batch reports render the hierarchy, matrix axes, lane, and seed as distinct
  levels
- maintained clean and trajectory-error packs execute only the `current` lane;
  historical comparison is supplied by cached result packs

The maintained terminal corpus covers `empty`, `half`, and `full` payload tiers
over clean plus small/large undershoot and overshoot conditions. Experimental
terrain backstops remain outside the maintained scorecard. Authored expectation
tiers describe corpus intent; analytic `scored`, `frontier`, and `impossible`
classification is a separate evaluator result.

Output-path stance:

- single-run iteration should favor stable, human-readable output paths so the
  same run can be regenerated in place and refreshed through a stable URL
- batch caches and compare artifacts may use short digests derived from the
  resolved pack, controller/config inputs, and commit/workspace identity
- planner-backed cache identity must also include planner algorithm, complete
  resolved policy, and plan digest; selector equality alone is insufficient
- cross-report case matching should retain stable physical request identity and
  route provenance so a changed generated plan remains comparable rather than
  appearing as unrelated coverage
- timestamps are useful for ad hoc archival bundles, but they should not be the
  primary identity mechanism for regression workflows

Recommended ownership split:

- `pd-core` runs one resolved concrete scenario
- `pd-eval` expands family plus seed specifications into concrete runs
- artifacts record both family identity and the resolved seed/parameters used for
  that run

Scenario families stay narrow and high-value:

- terminal descent
- direct transfer with boost/coast/terminal behavior
- preplanned waypoint turn and ordered-route guidance
- experimental terrain diagnostics outside maintained guidance gates
- small pinned bug reproductions

Do not start with a giant parameter cross-product or random fuzz catalog.
Start with the curated scenario shapes that proved useful in `pylander`, then
re-author them in the new data model.

## 10. Telemetry Model

Telemetry should have two namespaces:

- core metrics owned by the lab
- controller metrics owned by the controller

Core metrics examples:

- run outcome
- mission success
- touchdown normal speed
- touchdown tangential speed
- touchdown angular rate
- landing offset
- fuel consumed
- sim time
- controller step count
- profile or solver links to external trace artifacts where available

Controller metrics examples:

- planner solve counts
- fallback counts
- stage transitions
- terrain-divert diagnostics
- guidance mode or status strings
- report markers such as cutoff points or notable planner decisions

Controller metrics should be namespaced by controller ID so the lab can compare
multiple approaches without schema collisions.

## 11. Report Stance

`pd-lab` does not need a full interactive frontend.

The visualization target should be generated reports over captured artifacts,
not a new game shell or browser runtime.

But it does need enough inspection support early to make controller iteration
practical. A bot lab without a usable inspection path turns every run into
manual log reading.

An extended target is reasonable:

- hover or scrub along a trajectory
- drag a cursor across the flight path
- inspect precise vehicle state at sampled times

But that interaction should sit on top of precomputed run data.

Recommended split for report UX:

- use generated charts and tables for generic summaries
- keep a custom trajectory/detail viewer for per-run inspection where the domain
  is unique
- avoid turning the report layer into a second simulation or analysis backend

Current baseline:

- guidance scorecards over a declarative terminal/direct-transfer/waypoint
  report catalog
- selector-aware batch review trees, guidance-specific diagnostics, cached
  result-pack comparison, and regression-policy status
- single-run terrain, trajectory, target, waypoint, event, controller-guide,
  time-series, and vector inspection modes
- report-only regeneration over existing JSON evidence without simulation

Reason:

- the lab's primary users are developers and controllers
- deterministic artifacts matter more than runtime polish
- report tooling gives inspection value without reintroducing old runtime
  constraints

## 12. Deliberately Deferred Decisions

The workspace, JSON contracts, and static report baseline are established. The
remaining deferred choices should be made only when a concrete workload needs
them:

- optimization backend details for future optimizer-based controllers
- whether high-volume analytics justify a columnar derivative of the owned JSON
  artifacts
- whether report UX remains generated HTML or later deserves a dedicated small
  static app
- whether an optional experiment-tracker layer is useful beyond the owned
  artifact/report path
