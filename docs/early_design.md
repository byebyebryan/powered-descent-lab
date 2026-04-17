# Project Design Notes

This file is the original exploratory scratchpad. It contains superseded ideas,
including older web/Wasm-oriented notes. The current working docs are
[`architecture.md`](architecture.md) and [`roadmap.md`](roadmap.md).

## 1. Powered Descent Lab

### Purpose

Powered Descent Lab is the technical project: a control/simulation lab for developing, benchmarking, and analyzing autonomous rocket flight and landing behavior.

It exists to support:

* deterministic simulation
* controller and bot development
* scenario design
* benchmarking and telemetry
* trace capture and replay
* rapid iteration on control logic

It is not primarily a player-facing game.

### Why it exists

The original Python/Pygame implementation began as a fast way to prototype a lunar lander-style game and test AI coding agents. Over time, the project evolved into a much more serious bot-development environment, with increasingly sophisticated simulation, evaluation, telemetry, and controller logic.

As that happened, the game layer and the lab layer started to pull in different directions:

* the lab wants determinism, throughput, and instrumentation
* the game wants player feel, UX, and content

Powered Descent Lab is the answer to that split.

### Core goals

* Build a native simulation core for high-throughput iteration
* Support complex control logic for rocket flight in difficult terrain
* Provide deterministic evaluation scenarios
* Capture traces, metrics, and plots for controller development
* Support future browser deployment through Wasm where useful
* Serve as the source of advanced autopilot logic that can later inform the game

### Scope

#### In scope

* Terrain generation and terrain queries
* Physics and game rules relevant to rocket motion and landing
* Deterministic world stepping
* Sensor / observation modeling
* Bot/controller interfaces
* Evaluation harnesses and scenario packs
* Benchmark runner and profiling
* Telemetry, tracepacks, and plotting
* Optional browser-safe demo or replay builds

#### Out of scope

* Economy systems
* Content-heavy mission design
* Full player progression
* Rich game UX as a primary goal
* Heavy dependence on desktop-specific presentation layers

### Terrain model direction

#### Current decision

For now, both Powered Descent Lab and the eventual game should keep a **1D heightfield terrain foundation**.

This remains the preferred baseline because it provides:

* cheap and deterministic terrain queries
* simple procedural and authored generation
* easy debugging and scenario explanation
* good fit for control and landing problems
* straightforward rendering and LOD strategies

The current conclusion is that a heightfield is still the best default world foundation even though the bot increasingly needs richer spatial queries than just `height(x)`.

#### Why the heightfield still works

A pure heightfield does not have to mean the bot only knows altitude at a single x-position.
The simulation/query layer can still support richer spatial reasoning on top of heightfield terrain, including:

* nearest-point or closest-distance queries against the terrain polyline
* side-clearance checks
* raycasts against terrain segments
* local surface-normal estimation
* obstacle proximity and time-to-impact style queries

This matches the later evolution of the Python implementation, which moved beyond simple step tracing toward more complete ray intersection against polygonal representations of terrain.

#### Planned extension path

The preferred evolution path is:

* keep the canonical terrain as a 1D heightfield
* allow semantic tags / annotations / metadata on terrain regions or points
* support optional authored/static obstacle layers for richer gameplay and navigation cases
* expose richer query APIs so bots and physics can reason about clearance, approach geometry, and side collisions

This gives most of the practical benefits of more complex terrain without immediately forcing the project into a fully arbitrary 2D geometry model.

#### Why not switch to SDF / implicit terrain yet

Signed distance field terrain was considered because it naturally supports:

* distance queries
* surface normals
* inside/outside checks
* ray marching
* caves and overhangs

However, the current conclusion is that SDF should **not** be the primary terrain model at this stage.

Reasons:

* the project world is still mostly static, sparse, and structured
* exact geometric queries against segments/polylines are often simpler and more trustworthy for a control/sim lab
* SDF introduces approximation, resolution, and tolerance concerns
* SDF-first terrain adds complexity that is not yet justified by current gameplay or lab needs

If distance fields become useful later, they should be introduced as a **derived helper or acceleration layer**, not necessarily the canonical source of terrain truth.

#### Why not fully arbitrary 2D terrain yet

Full polygonal or implicit terrain would allow caves, ceilings, tunnels, and overhangs as first-class terrain features.

That remains interesting for future experimentation, but it would also:

* increase collision and query complexity
* make bot reasoning harder
* complicate procedural generation and debugging
* expand the lab from a descent/guidance problem toward a more general 2D navigation problem

For now, that complexity is not considered worth the tradeoff.

#### Practical implication for the lab

Powered Descent Lab should primarily use:

* flat ground
* slopes
* piecewise linear terrain
* authored terrain motifs
* terrain tags/metadata
* optionally, sparse static obstacles where needed

Complex procedural noise terrain is not necessary for most lab work. Explicit scenario construction is generally more useful for debugging, benchmarking, and regression analysis.

#### Practical implication for the game

The game can remain heightfield-based while still becoming much richer through:

* biome tagging
* authored terrain features
* pads, structures, towers, bridges, and floating hazards
* optional obstacle layers

This allows richer flying and landing gameplay without discarding the simplicity and strengths of the heightfield model.

### Proposed architecture

#### Native sim core

Owns:

* terrain generation
* world state
* simulation rules
* physics
* sensors / observations
* deterministic stepping
* trace emission hooks
* action/state schemas

The sim core should not own UI concerns. It should consume a clean action/input contract and emit updated state, events, and observations.

#### Native bot runtime

Owns:

* controller logic
* planning and optimization
* state/observation -> action mapping
* bot-local debugging and telemetry

The bot should interact through formal interfaces, not by reaching into engine internals.

#### Evaluation framework

Owns:

* scenario definitions and packs
* benchmarking
* regression tests
* telemetry aggregation
* plots and reports
* tracepack generation
* profiling and development tooling

This layer orchestrates repeated runs around the same sim core and bot contracts.

#### Frontends

Possible frontends include:

* lightweight web viewer
* replay viewer
* optional browser demo
* temporary legacy Pygame shell

The frontend should not own rules. It should render state, accept human input where needed, and inspect live or recorded runs.

### Deployment model

#### Native-first

The main execution mode should be native:

* fastest benchmarking
* easiest concurrency and threading
* least browser constraint friction
* best fit for iterative bot development

#### Optional Wasm target

Parts of the lab may later be compiled to Wasm for:

* browser demos
* replay viewers with local stepping
* lightweight sandbox scenarios
* reduced-feature controller demos

Wasm is a deployment target, not the primary environment for full evaluation workflows.

### Language direction

Rust is the current leading candidate for the lab because it fits:

* deterministic native core work
* strong interface boundaries
* safe concurrency
* future Wasm packaging
* architecture-heavy development

C++ remains viable, especially if ultimate low-level freedom or specific native ecosystems become more important.

### Migration strategy from Pylander

Pylander should remain the reference implementation and behavior oracle.

Recommended path:

1. Freeze Pylander as the baseline/reference
2. Start a new repo for Powered Descent Lab
3. Rebuild a minimal vertical slice in the new architecture
4. Compare behavior against Pylander on selected scenarios
5. Port benchmark concepts and telemetry intentionally, rather than transliterating the old code wholesale

### Success criteria

Powered Descent Lab succeeds if it provides:

* fast deterministic simulation
* clean bot-development workflows
* useful metrics, traces, and plots
* a stable base for solving more general terrain and flight problems
* a source of reusable control logic for future game systems

---

## 2. Approach Vector

### Purpose

Approach Vector is the player-facing game project.

It grows out of the original lunar-lander inspiration, but expands beyond pure landing into a broader rocket-flight and cargo-hauling game with assisted navigation, missions, upgrades, and light systemic gameplay.

### Core fantasy

The player is no longer just operating a dedicated lunar lander. The fantasy is closer to:

* piloting a rocket-powered ship
* moving between locations
* boosting, coasting, approaching, and landing
* hauling cargo
* managing risk on routes
* gradually unlocking smarter assistance systems

The game begins close to classic lunar lander gameplay, but evolves into a broader flight game.

### Terrain model direction

#### Current decision

Approach Vector should currently share the same baseline terrain philosophy as the lab:

* **1D heightfield terrain as the world foundation**
* optional semantic tags and metadata for biome and terrain meaning
* optional authored/static obstacles for richer spatial gameplay

This keeps the terrain model simple, expressive, and compatible with the control-focused lineage of the project.

#### Why this is the right default for the game

A heightfield still supports a lot of compelling gameplay:

* cliffs
* valleys
* canyons
* ridges
* narrow landing zones
* visually distinct biomes
* authored pads and structures

The lack of true overhangs or caves is a limitation, but not one that currently justifies abandoning the simpler terrain foundation.

#### What would be added before replacing the terrain model

Before moving to full arbitrary 2D terrain, the preferred extension path is:

* richer terrain annotations and tags
* authored structures and obstacles
* floating hazards
* towers, bridges, bays, ruins, and landing infrastructure
* optional obstacle layers for special mission spaces

This gives the game a lot more navigational and visual variety while preserving a terrain model that remains easy to reason about and compatible with lab-derived control logic.

#### Relationship to bot/autopilot logic

The bot and assist systems increasingly need more than just altitude-at-x queries. They also need:

* side-clearance awareness
* closest-terrain checks
* raycast-style obstacle and terrain probing
* local surface normals and approach geometry

The current direction is to support these needs through a richer terrain-query layer, not by replacing the terrain foundation with SDF or full implicit geometry.

#### SDF / implicit terrain status

Signed distance fields and implicit terrain were explored conceptually because they would naturally support:

* distance queries
* surface normals
* ray marching
* caves and overhangs

The current conclusion is that this is interesting, but premature.
For now:

* SDF should not be the canonical terrain model
* if used later, it should likely be as a derived/helper representation
* marching-squares style contour extraction would be the relevant 2D rendering approach rather than marching cubes

#### Full arbitrary geometry status

Full polygonal or implicit 2D terrain remains a future option if the game later proves that caves, ceilings, tunnels, and overhangs must become first-class gameplay features.

At the current stage, that complexity is considered too costly relative to the likely gain.

### Long-term game vision

Potential pillars:

* simple but interesting terrain, biomes, and scenery
* mission loop based on hauling cargo for credits
* basic economy loop: buy, sell, upgrade, route choice
* light combat/risk systems such as pirates, turrets, or missiles
* ship upgrades and progression
* landing/navigation assist systems inspired by autopilot ideas

### Gameplay arc

#### Early game

* manual piloting
* classic lander challenge
* basic point-to-point travel
* player skill focused on thrust, velocity, and landing control

#### Mid/late game

* navigation and landing assists
* partial automation
* route planning and efficiency
* higher-stakes terrain and mission challenges
* richer ship roles and upgrades

### Assist philosophy

The game should not simply automate itself away.
The inspiration is closer to flight-sim autopilot assistance:

* the player still makes strategic decisions
* assists reduce workload and expand capability
* higher-end systems provide better approach, landing, and navigation help

Possible assist progression:

* collision / wall-protection style reactive systems
* approach stabilization aids
* auto-position above landing zones
* assisted landing execution
* route guidance or point-to-point auto-nav

### Relationship to Powered Descent Lab

Approach Vector should borrow from the lab, but not be constrained by it.

Principles:

* reuse ideas and controllers where they are useful
* simplify or adapt lab logic for gameplay needs
* do not hard-bind the game to the heaviest experimental bot stack
* allow lighter NPC/autopilot controllers for ordinary actors

The lab discovers advanced behaviors.
The game productizes selected behaviors.

### NPC and AI design implications

The current heavy controller logic may be too expensive or too specialized for all game actors.
The game will likely need several controller tiers:

* player assist/autopilot controllers
* lightweight NPC flight controllers
* simplified landing/navigation logic for ambient actors
* richer tactical logic for enemies or specialized mission actors

### Architecture direction

Approach Vector should remain a separate project from the lab.

It may eventually share:

* concepts
* trace or replay ideas
* scenario schemas
* selected control modules
* terrain/navigation techniques

But shared code should be introduced only when the abstractions are truly stable.

### Frontend and accessibility

Because browser accessibility proved unexpectedly valuable, the game should strongly consider a web-first or web-friendly presentation layer.

Potential frontends:

* primary web frontend
* optional native/desktop packaging later

The game should be optimized for player experience first, not benchmark throughput.

### What makes the game distinct from the lab

The lab optimizes for:

* determinism
* throughput
* instrumentation
* experiment velocity

The game optimizes for:

* feel
* readability
* progression
* content
* UX
* player-facing systems

Keeping the projects separate prevents these goals from fighting each other.

### Naming rationale

The lab and the game now have different identities.

Recommended naming:

* technical project: **Powered Descent Lab**
* player-facing game: **Approach Vector**

This works because:

* Powered Descent Lab names the specific control problem
* Approach Vector names the broader piloting/navigation fantasy
* the two remain related without forcing them to be the same project

### Success criteria

Approach Vector succeeds if it becomes:

* a compelling rocket-flight game
* a richer evolution of the original lunar lander idea
* a game where assists and autopilot deepen progression rather than removing play
* a project that can selectively benefit from the research done in Powered Descent Lab
