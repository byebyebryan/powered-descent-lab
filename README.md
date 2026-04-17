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
- one primary goal per scenario
- formal controller API with full immutable scenario context at setup time and a
  compact per-tick observation
- authored scenario packs and regression suites
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

## Scope

`pd-lab` owns:

- deterministic simulation
- controller and bot development
- scenario packs
- evaluation and benchmarking
- telemetry, traces, and replay artifacts

`pd-lab` does not own:

- player progression or economy systems
- content-heavy mission design
- final game UX
- browser-first runtime constraints

## Docs

- [Architecture](docs/architecture.md)
- [Roadmap](docs/roadmap.md)
- [Early Design Scratchpad](docs/early_design.md)
