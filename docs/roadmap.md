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
- Phase 2 has started with a minimal `pd-eval` pack runner and aggregate JSON
  summaries, but the phase is now explicitly framed around building a proper bot
  workflow rather than only adding more packs.
- One Phase 3 contract probe has been pulled forward intentionally:
  `timed_checkpoint`, to validate early-termination mission evaluation without
  committing to the full transfer stack yet.

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

- started
- current implementation includes:
  - a first named pack runner and summary counts
  - controller config/spec plumbing for built-in controllers
  - controller-local status, phase, metrics, and markers
  - a first static single-run inspection report in `pd-cli`
  - scenario-family expansion with explicit seeds and seed ranges
  - deterministic perturbation resolution recorded per run
  - native multithreaded execution in `pd-eval`
  - richer batch summaries grouped by entry and family, including failed seeds
    and slowest runs
- still missing:
  - stronger comparison/regression reporting
  - polished batch-level report UX
  - a real digest-backed compare/cache workflow over the new batch identities

### Phase 3: Transfer guidance

Target:

- support the full point-to-point transfer problem that made late `pylander`
  interesting

Planned scope:

- source/target transfers
- boost/coast/terminal mission definitions
- early-stop evaluation checkpoints such as boost-cutoff trajectory validation
- richer target geometry
- controller telemetry for staged or unified guidance

Exit criteria:

- the lab supports both terminal and transfer evaluation under the same core
  contracts

Status:

- not started as a full phase
- one early-stop evaluation primitive (`timed_checkpoint`) is in place as a
  contract probe only

### Phase 4: Terrain-aware lab

Target:

- add the terrain-query richness needed for real route and guardrail work

Planned scope:

- setup-time terrain query API
- closest-point, ray, and clearance queries
- curated terrain-reactive scenarios
- terrain-focused telemetry and replay markers

Exit criteria:

- terrain-aware guidance can be evaluated without exposing engine internals
- terrain failures are explainable from captured artifacts

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
path already exists in Phase 2.

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

Before any implementation, lock these decisions:

- repo and crate naming
- command and observation contract direction
- scenario identity model
- vehicle geometry and touchdown reference model
- landing success envelope
- artifact model for runs and traces
- telemetry/reporting stack boundaries: what the project owns vs what existing
  tools should handle

Once those are accepted, the first implementation pass can start with a narrow
Phase 1 vertical slice instead of another open-ended prototype.
