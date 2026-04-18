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

The intended v1 stack is:

- Rust cargo workspace
- `clap` for native CLI entry points
- `serde` for scenario, replay, trace, and report artifacts
- `tracing` for structured native logs
- optional native parallelism in eval only, not as a core contract assumption

There is no frontend framework decision in v1 because there is no interactive
frontend requirement in v1.

Reporting should be static-output-first:

- machine-readable run artifacts
- aggregate CSV/JSON summaries
- optional generated static HTML reports

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
- optional static obstacles
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
- optional run-time disturbances

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
- terrain-reactive transfer
- named regressions for past failures

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

The lab should expose richer read-only terrain queries such as:

- height and slope at `x`
- local surface normal
- closest point / closest distance
- ray and segment intersection
- corridor or path-clearance sampling

In v1, the bot should receive the full immutable terrain definition through the
setup-time run context. Query helpers are still useful, but the controller
should not be forced to rediscover the world through only per-frame local
sensors.

This should be a simple controller-facing terrain package, not a hidden view
over engine internals and not a sensor-gated approximation.

Optional obstacle layers can be added later for structures or hazards without
discarding the heightfield as the canonical ground model.

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
- terrain and obstacle queries
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

### 5.3 `pd-cli`

`pd-cli` is the single-run native entry point.

Responsibilities:

- run one scenario with one controller
- emit readable console summaries
- export replay and trace artifacts
- support targeted debugging and inspection

This should replace the old mixed interactive/headless shell as the primary
developer entry point.

### 5.4 `pd-eval`

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
- report generation

The eval layer should orchestrate runs around the same core and controller
contracts used by `pd-cli`.

Recommended parallelism boundary:

- `pd-core` stays single-run and deterministic
- `pd-eval` expands packs and seed sweeps into concrete runs
- `pd-eval` may execute independent runs in parallel on native threads
- immutable scenario data such as compiled terrain can be shared across worker
  threads where useful
- output ordering and aggregate reports should still be written in a stable,
  deterministic order

### 5.5 Report generation and report viewer

A minimal inspection/report path is part of the core bot-lab workflow, not only
late polish.

Likely responsibilities:

- summary report generation
- trace and replay inspection pages
- single-run trajectory and state inspection
- lightweight interaction over precomputed run data
- trajectory scrubbing, hover, or drag-based state inspection

They should not own:

- authoritative simulation
- controller execution
- benchmark orchestration

The first report milestone should be enough to answer basic controller
questions without reading raw JSON:

- where the vehicle flew relative to terrain and target
- how altitude, clearance, velocity, attitude, and throttle evolved
- where discrete events and controller phase/status changes happened

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

## 6. Proposed Repo Shape

The intended repo shape is:

```text
pd-lab/
  README.md
  docs/
  fixtures/
    scenarios/
    baselines/
  pd-core/
  pd-control/
  pd-cli/
  pd-eval/
  report/       # later, only if report UX grows enough to justify it
```

Notes:

- `fixtures/scenarios` stores authored scenario definitions and pack manifests
- `fixtures/baselines` stores known-good summaries or comparison references
- `report/` is optional later report UX, not a browser runtime target

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

The exact encoding can be finalized later, but it should be friendly to both
native tooling and a later static report viewer.

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

Recommended ownership split:

- `pd-core` runs one resolved concrete scenario
- `pd-eval` expands family plus seed specifications into concrete runs
- artifacts record both family identity and the resolved seed/parameters used for
  that run

The first scenario families should stay narrow and high-value:

- terminal descent
- transfer with boost/coast/terminal behavior
- terrain-reactive regression cases
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

Near-term minimum:

- a single-run view with terrain, trajectory, target, and event markers
- time-series plots for the main state and control signals
- batch summaries that make seeded regressions visible without replaying every
  run

Reason:

- the lab's primary users are developers and controllers
- deterministic artifacts matter more than runtime polish
- report tooling gives inspection value without reintroducing old runtime
  constraints

## 12. Deliberately Deferred Decisions

These are important, but they should not block the repo reboot:

- exact on-disk scenario syntax
- exact trace encoding format
- optimization backend details for baseline controllers
- whether report UX remains generated HTML or later deserves a dedicated small
  static app
- whether an optional experiment-tracker layer is useful beyond the owned
  artifact/report path

Those choices should be made after the workspace exists and the first vertical
slice proves the boundaries.
