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
- `clap` + `serde` + `tracing` style native tooling, with web reserved for static
  report viewing rather than an interactive runtime
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
- basic static inspection/reporting as part of the near-term controller workflow,
  not only a late polish phase
- static web reports over captured artifacts, with optional lightweight
  trajectory inspection later, but no browser runtime target

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

- [Architecture](docs/architecture.md)
- [Roadmap](docs/roadmap.md)
- [Terminal Suite Design](docs/terminal_suite.md)
- [Early Design Scratchpad](docs/early_design.md)

## Report Serving

Generated reports live under `outputs/` and can be served locally with:

```bash
./scripts/serve-reports start
```

The script starts a simple HTTP server inside a named detached `tmux` session
and serves `outputs/` on `0.0.0.0:8000` by default. The root URL now lands on a
generated `outputs/index.html` page, and `/reports/` remains the clean
report-only subtree. The printed LAN URL resolves automatically when available.

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
- `outputs/reports/runs/latest/`
- `outputs/reports/eval/latest/`

The root landing page is:

- `outputs/index.html`

## Batch Eval

`pd-eval` owns scenario packs, scenario-family expansion, seed sweeps, and
native multithreaded execution.

Example:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_suite.json --workers 4
```

Run the same matrix with the full seed tier:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_bot_lab_full.json --workers 8
```

Compare a candidate batch against a recorded baseline:

```bash
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_compare_baseline_fixture.json --workers 4
cargo run -p pd-eval -- run-pack fixtures/packs/terminal_compare_regression_fixture.json --workers 4 --baseline-dir outputs/eval/terminal_compare_baseline_fixture
```

Use `terminal_bot_lab_suite` as the primary controller workbench. It is the
smoke-tier Earth `half_arc_terminal_v1` matrix over:

- `condition_set = clean`
- `vehicle_variant = nominal`
  - `pylander`-aligned Earth baseline hardware with half payload
- `vehicle_variant = heavy_cargo`
  - the same hardware with full payload
- `arc_point x velocity_band`
- `baseline` and `current` controller lanes

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

At the moment:

- `baseline` means the older heuristic baseline controller
- `current` means `terminal_pdg_v1`, the first serious terminal-only PDG-shaped
  native Rust controller lane

Use `terminal_bot_lab_full` when the same matrix should run with the full
seed tier for spread measurement. The `terminal_compare_*_fixture` packs are
only for smoke-testing pack-vs-pack compare output.

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
- optional candidate-vs-baseline deltas over shared run IDs
- stable links back to per-run detail reports and bundles

At this point the batch/single-run reporting stack is good enough for real
controller iteration. The main next bottleneck is now controller viability on
the fully aligned Earth baseline: once the terminal suite was moved onto the
heavier `pylander`-style vehicle/engine envelope, both current bot-lab lanes
regressed to `0` successes on the smoke and full matrices, so controller
retuning is again the main job before the suite expands further.
