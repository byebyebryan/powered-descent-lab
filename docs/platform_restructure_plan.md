# Platform restructure plan

[Documentation home](README.md) · [Current guidance](guidance.md) · [Roadmap](roadmap.md#7-recommended-immediate-next-step)

Status: reviewed design, October 10, 2026; see the
[review and validation record](platform_restructure_review.md). Refactor
implementation has not started. Work stays in this checkout; controller tuning
is parked at the [combined safety checkpoint](ballistic_combined_validation_results.md).
The [unattended execution plan](platform_restructure_execution.md) expands the
next run across milestones, with an independent-agent workflow as the first
major integration target. A small first slice is a checkpoint within that run.

The lab should support independent approaches to complete missions on procedural
terrain. A new RL agent must be able to use the same simulator, mission inputs,
evaluation and reports without importing the existing controller or planner.
Authored missions remain useful diagnostics, while procedural mission
distributions become the main source of training and coverage evaluation.

## Why restructure

The crate dependency graph is acyclic and the plant already has a useful neutral
boundary. The main problem is ownership above the plant. `pd-eval` owns substantial
planner execution and recovery logic as well as experiments, mission expansion,
verification and reports. The ordinary evaluation descriptor also selects from
the closed `ControllerSpec` registry. A second approach would inherit these
choices unless the shared platform contracts become independent of them.

The procedural terrain study produces real terrain, but its complete missions
still use tightly constrained templates and external reference generation.
More seeds from the same four recipes improve sampling without establishing
coverage of a wider mission distribution. The original development 1k and the
now-observed fresh 1k are regression collections, not permanently unseen tests.

The original and fresh combined-safety results are 866/1000 and 824/1000. These
documented results motivate broader coverage; their difference alone does not
identify overfitting or its cause. Preserve those captures and their denominators.

## Current ownership and extraction targets

| Current area | Existing responsibility or coupling | Proposed destination |
| --- | --- | --- |
| `pd-core` | Physics, contact, mission progress, clocks, terrain, neutral artifacts and replay | Keep authoritative plant and mission contracts here |
| `pd-control` | Controller trait, capture records, closed registry, terminal and transfer algorithms | Extract neutral runtime records; keep feedback algorithms and complete configuration |
| `pd-plan` | Pure ballistic and local-clearing calculations | Keep pure planning calculations |
| `pd-eval` | V2 session, nominal realization, clearing, acquisition, recovery, execution, packs, proof and reports | Extract solver orchestration and mission generation; retain generic evaluation and verification |
| `pd-report` | Rich saved-data rendering with imports from `pd-control` | Consume neutral trace contracts and optional diagnostic payloads |
| Terrain study scripts | External generator extraction, fixed recipes and mission templates | Retain historical recipes; add a reusable versioned mission generator |
| CLI and scripts | Built-in selection, capture and report workflows | Compose maintained approaches above the neutral platform |

Concrete extraction sources include
[controller execution](../pd-control/src/lib.rs),
[controller registration](../pd-control/src/controllers.rs),
[V2 sessions](../pd-planner/src/waypoint_v2.rs),
[nominal realization](../pd-planner/src/planner_flight.rs),
[ordinary clearing execution](../pd-planner/src/local_clearing.rs),
[ballistic feedback](../pd-eval-cli/src/ballistic_feedback.rs),
[pack resolution](../pd-eval-cli/src/resolution.rs), and
[terrain refinement](../studies/terrain_profiles/refinement.py).

## Target responsibilities

The proposed new crates are `pd-run`, `pd-missions` and `pd-planner`. Names
are provisional; their ownership boundaries are the design requirement. Keep
one repository and one authoritative simulator. An RL adapter may be a separate
consumer package or Python integration without duplicating the plant.

- `pd-core` owns the physical world, actuator semantics, contact classification,
  mission progress and physical termination. It defines what a mission means.
- `pd-run` owns actual episode execution: command scheduling, observations,
  supported lifecycle modes, capture policy and neutral approach contracts.
  Replay continues to use the authoritative core implementation.
- `pd-missions` owns procedural generation, structural validation, resolved
  mission identities, immutable collections and their usage roles. It creates
  missions without consulting a controller or planner.
- `pd-control` owns current feedback controllers and their configuration.
- `pd-plan` owns pure planning calculations. `pd-planner` owns extracted
  acquisition, correction, recovery, terminal composition and solver stops.
  It may compose `pd-control` algorithms through the shared host; this dependency
  is confined to the existing planning approach.
- `pd-eval` owns generic evaluation, comparison, evidence verification and
  resource accounting. It receives an approach factory rather than selecting a
  built-in enum inside its execution engine.
- `pd-report` renders saved results through maintained rich templates. Optional
  approach diagnostics extend those views without becoming execution inputs.
- Application adapters own registries, CLI compatibility and dependencies on
  concrete controllers, planners or RL frameworks.

The neutral platform must have no normal dependency on `pd-control`, planning
algorithms or an RL framework. An independent agent should require only neutral
contracts and runtime services. Strategy-specific forecasts and guards belong
to that strategy; mandatory physical and mission rules remain shared.

### Dependency direction and application packaging

These are proposed internal dependencies, with external serialization and
utility libraries omitted. `pd-eval` becomes a library; report creation is
composed above it rather than called by its execution engine.

| Package | Allowed internal dependencies |
| --- | --- |
| `pd-core` | None |
| `pd-run` | `pd-core` |
| `pd-missions` | `pd-core` |
| `pd-control` | `pd-core`, `pd-run` |
| `pd-plan` | `pd-core` |
| `pd-planner` | `pd-core`, `pd-run`, `pd-plan`, `pd-control` |
| `pd-eval` library | `pd-core`, `pd-run`, `pd-missions` |
| `pd-report` | `pd-core`, `pd-run`; neutral saved-data projections |
| Evaluation application | Neutral packages plus concrete approaches and reports |

Move the evaluation binary into a composition package, provisionally
`pd-eval-cli`, retaining the installed binary name `pd-eval`. Move built-in pack
registration, strategy diagnostics and explicit experiment commands with it.
This deliberately changes Cargo package invocation from `-p pd-eval` to
`-p pd-eval-cli`; update maintained callers and workflows together at that stage.
Do not introduce a dependency cycle through a compatibility binary in the
generic library package. Installed command names, selectors, defaults and
create-only output rules remain compatibility requirements.

Keep ordinary `pd-cli` free of evaluator and planner dependencies when its
`planner-v2` feature is off. Enable the extracted planner adapter and evaluation
services explicitly with that feature. Independent agent consumers must build
without either application registry. Check normal transitive dependencies,
including report imports, rather than relying on folder names for independence.

Saved controller descriptors retain isolated decoding adapters and their exact
historical JSON shapes. Neutral execution uses a new versioned approach
descriptor and opaque configuration payload. Readers must not depend on
executable controller implementations to decode old data. Solver-specific DTOs
move with their solver; evaluation stores their versioned diagnostic payloads.

## Runtime contract

Do not force the synchronous V2 solver into a per-tick controller interface.
Support both observation-driven command policies and prepared command programs
through one authoritative host. Planning may query copies of the plant, but
only the host advances the actual episode and writes its command history.
Programs retain their original command timing and segment boundaries.

The contract has three cooperating interfaces: an observation-driven policy
that resets and returns a command decision, a session driver that proposes a
bounded command chunk, and a host that validates and executes those decisions.
The session can inspect actual state and request prediction on an independent
copy. It cannot replace the live state with a predicted or saved snapshot.
Rust signatures are settled by the first slice's compile and execution checks.

For the existing 120/60 Hz paths, retain the time-zero observation and command,
two physical transitions per command interval, original sample/event ordering,
and termination before issuing a further command. Do not renumber chunk-local
updates as a fresh episode. Preserve existing online clamping and prepared
program validation through their adapters. The native throttle and attitude
commands remain the actuator interface; an RL action transform must be explicit.

Approach identity includes an implementation version and complete resolved
configuration. Learned approaches also bind checkpoint identity. Decision
inputs describe the physical mission and available observations, without
evaluation labels, expected outcomes or collection membership. Information
availability is an explicit comparison condition.

The new decision view excludes scenario names, tags and generator seeds; policy
randomness has its own recorded seed. Legacy adapters retain their existing
`RunContext` contract during preservation checks rather than silently changing
inputs while moving execution.

Preserve the distinction between physical termination, mission disposition and
an algorithm's decision to stop. A planner can stop while the craft is still
flying. Its finite search failure does not establish physical impossibility.
The current solver's planning reserve is solver policy, not an environment-wide
terminal allowance or a reason to extend the mission clock.

The current 90-second terrain missions have an 80-second planning bound, with the
ballistic loop retaining its 24-tick query horizon inside that bound. Keep this
existing distinction during extraction. Episode completion must also distinguish
physical/mission termination from algorithm stop and evaluator resource timeout;
an RL adapter exposes these reasons rather than inferring success from `done`.

Training may suppress detailed capture and expensive proof generation, but it
must execute the same actuation, contact, progress and timeout semantics as a
verified episode. The physics-only diagnostic stepping seam is insufficient
for actual training episodes. Core replay remains authoritative for captured
commands; historical restart and prefix-proof restrictions remain intact.

Fresh airborne missions use a validated initial-state contract; they do not
relax the proof required to resume a historical flight from its retained prefix.
Zero-command stops need their existing bounded proof path rather than the
ordinary full-flight replay API, which requires an initial action.

Capture levels are summary, command trace and full evidence. They vary retained
records and post-run work, not decision inputs or physical transitions. Summary
training outcomes are not verified landing claims. A policy's own recurrent
state and decision diagnostics remain available regardless of retained capture.

## Procedural missions and management

Separate three operations: propose a mission, validate its structural contract,
and assign the resolved mission to a collection. Structural validity is not a
claim that every agent can solve the mission.

Validation applies `ScenarioSpec`/`RunContext` checks plus declared construction
invariants: finite ordered terrain, domain coverage, legal pad geometry and
preparation, an admissible initial pose, and consistent mission targets and
clocks. Distributions have bounded structural rejection with a deterministic
proposal index and typed reasons. Exhausting that bound produces a generation
failure, not another seed selected by solver performance. Track proposed,
rejected and materialized counts separately from mission execution counts.
Analytical difficulty estimates may annotate missions; a finite solver failure
cannot classify a mission as impossible or remove it from the collection.

Start with the current vehicle and gravity so generator diversity can be
assessed separately from new physical models. Vary terrain shape and scale,
source and destination geometry, travel distance and direction, pad elevation
and width, initial state, fuel and mission budget through declared distributions.
Introduce airborne starts and wider physical envelopes as explicit tiers with
visible capability declarations.

Generate the terrain continuously between pads. Local pad preparation must be
part of the recipe, bounded and recorded. Do not flatten routes, relocate
obstacles after a failure, or reject missions because the current solver fails.
Keep authored route contracts separate from complete pad-to-pad missions where
an approach chooses its own route.

The generator should be self-contained. Preserve the external Pylander study
as a historical recipe and reference; a port that changes numerical output gets
a new generator version. Persist complete resolved scenario bytes, generator
version, seed, distribution parameters and content identity. Regeneration is
convenience; stored resolved content is the evaluation authority.

Version the PRNG and component substreams so generation order and worker count
cannot change mission content. Test float round trips and reject non-finite
values. A byte digest binds the saved input; a separately specified physical
mission digest excludes labels and provenance, exposing identical missions with
different names or seeds. Both identities include all relevant physical values,
mission rules and clocks in their appropriate representation. Historical
identities continue to use their original algorithms.

Training uses a reproducible stream that can keep producing missions.
Evaluation uses immutable ordered collections sealed before results are
inspected. Maintain development, locked regression and unseen evaluation roles,
plus a usage record showing when a collection has been observed. Seed separation
alone is insufficient when resolved inputs duplicate another collection.

Keep collection manifests immutable and record usage separately. Training and
development roles may be declared from creation; an unseen collection becomes
observed when used to inspect results or select an approach. Subsequent tuning
makes it regression/development evidence. Moving a label cannot restore unseen
status. Reserve separate evaluation distributions for physical dimensions
outside training support; a larger sample of the same recipes measures only
that distribution. Predeclare sample sizes or a stopping rule and report
uncertainty alongside rates, keeping repeat invocations separate from unique
mission counts.

Report coverage by declared mission dimensions and retain all attempted runs,
including unsupported inputs, failures and missing evidence. A capability filter
may define a secondary comparison slice, but must not silently shrink the full
collection denominator. Wider sampling is not evidence of arbitrary terrain
reliability.

## Migration sequence

The stages are acceptance milestones, not a mandatory linear execution order.
The unattended run prioritizes Stages 0, 1 and 2, then generic evaluation and an
independent adapter from Stages 4 and 6, before the deepest solver extraction
and retirement in Stages 3 and 5. Application-owned transitional modules keep
legacy execution available without entering the neutral dependency graph;
their final extraction remains a completion requirement. See the
[work order](platform_restructure_execution.md#work-order).

### Stage 0 Freeze the migration baseline

Inventory maintained execution paths, historical readers and obsolete experiment
frontdoors. Bind active approach versions and resolved configurations to their
input collections and captures. Protect existing dirty report/server work and
all retained evidence. Identify exact default, opt-in and diagnostic behavior
before moving files.

Retain Policy 3, ordinary supported controllers, the combined-safety opt-in
reference and acquisition-only comparator. Map earlier experimental executables
to retained readers or explicitly justified tests before retirement; their
historical reports are not a requirement to keep every algorithm executable.

### Stage 1 Extract neutral execution contracts

Add the approach and host contracts, then adapt one existing execution path
without changing numerical decisions. Extract neutral traces used by evaluation
and reports. Allow an independent trivial agent to run without controller or
planner imports. Preserve command order, actuator timing, physical ticks,
clocks, fuel, observations, algorithm stops and source replay.

Begin by delegating through the existing core run functions; do not rewrite
stepping or add low-capture execution in this first slice. Add explicit stop and
prepared-program interfaces without extracting the full planner yet.

Enforce dependency rules for each new neutral crate immediately; existing
evaluator/report couplings are removed at their explicit migration stages.

### Stage 2 Add procedural mission generation

Build generation, validation and collection management independently of solver
extraction. Add a small versioned distribution covering declared dimensions,
deterministic identities and immutable collection materialization. Characterize
its coverage before promising a large campaign size. Retain authored fixtures
and the observed 1k collections as distinct diagnostics and regressions.

### Stage 3 Extract maintained solver orchestration

Move Policy 3 and the current ballistic development reference separately.
Extract nominal realization, clearing, acquisition, recovery and terminal
composition from the evaluator. Make currently hidden experimental switches
part of complete versioned configuration while preserving legacy defaults and
serialized identities through explicit compatibility adapters. Migration must
not incorporate an algorithm improvement.

Use `pd-planner` for orchestration, with an independently owned actual host.
Retain candidate ordering, cached programs, guard/query origins, correction caps
and retained-prefix proofs. Export complete resolved switches, including the
coupled effects of existing builder methods, under a new configuration version;
do not append fields to a hashed historical descriptor and call it unchanged.

### Stage 4 Make evaluation and reporting generic

Accept supplied approach factories and neutral records. Separate execution,
capture, verification and rendering, allowing training without report assets,
repository hashing or duplicated execution per episode. Keep physical, mission,
algorithm, unsupported, integrity and replay dispositions distinct. Preserve
rich navigation, diagnostic panels and saved-data readers.

Generic verification checks actual command replay, physical/mission outcomes
and evidence completeness. Strategy-specific certificates use supplied versioned
verification adapters; they do not require importing a solver into the evaluator.
An approach's reported success or a rendered annotation cannot substitute for
the complete physical, mission, integrity and replay tuple.

Move PDG phase/shape diagnostics into registered diagnostic adapters. The
existing parabolic reference is a documented benchmark diagnostic; distinguish
that versioned reference from recorded controller intent rather than changing
its meaning. Neither can affect generic success classification.

### Stage 5 Retire superseded execution paths

Remove old executables only after mapping their callers and acceptance tests to
the replacement. Preserve historical decoding, frozen inputs, captures and
reports. Add dependency-boundary checks and consolidate current CLI/workflow
instructions. Ordinary tests must not depend on locally retained captures;
historical numerical parity remains an explicit opt-in check.

### Stage 6 Prove an independent RL adapter

Expose reset, observation, action and episode completion from the shared host.
Reward shaping, feature encoding and framework integration belong to the RL
adapter. First exercise that boundary with a small policy stub and recorded
weights on generated missions and a sealed collection, using ordinary command
replay and rich reports. Measure simulation throughput and captured/uncaptured
physical equivalence before selecting a training algorithm or launching
substantial runs. Learned performance is a separate experiment after this
integration contract passes.

Record checkpoint, observation/action transforms, recurrent-state reset, policy
randomness and inference versions for evaluation. Compare information access,
cadence and physical limits explicitly. Account for forecast steps and solver
work as well as actual episode steps; separate proof/render cost from approach
compute. GPU execution need not promise bitwise policy reruns: captured native
commands still require deterministic plant replay, with policy-repeat evidence
reported separately.

## Acceptance checks

| Boundary | Required evidence during implementation |
| --- | --- |
| Independent approaches | A small external approach builds and executes without current controller/planner dependencies |
| Shared plant | Captured and uncaptured execution produce the same authoritative state and outcomes for the same commands |
| Preserved algorithms | Existing adapters retain command, state, clock, fuel, handoff and outcome parity; differences are investigated rather than normalized |
| Mission generation | Same recipe produces identical resolved missions and identities; coverage and structural rejection are characterized independently of any solver |
| Collection integrity | Input order and bytes are sealed; observation history and overlapping resolved missions are visible |
| Honest evaluation | Every scheduled attempt has a typed disposition; missing evidence cannot count as a verified landing |
| Reporting | Existing rich views and old capture readers remain usable; strategy metrics are optional |
| Architecture | Cargo dependency checks reject strategy imports into the neutral platform |

Implementation uses the [maintained development gate](development.md), plus
focused contract and generator checks. The retained 44-case exact parity check
is explicit opt-in and retains its original source provenance. No mission or
historical outcome changes merely to satisfy a migration check.

## First implementation slice

After design review, the first slice establishes neutral execution contracts and
adapts one ordinary controller. The second introduces mission generation and
collections. Keep solver extraction and RL training outside these first slices.
Each slice needs an owned file list, explicit compatibility mapping and bounded
acceptance checks before code changes begin.
The launched unattended goal continues to later accepted slices without asking
for routine per-slice approval. The first slice alone does not complete it.

The first slice may add `pd-run` contracts and their focused tests, adapt
`pd-control/src/lib.rs`, and register the new workspace package. Leave planner
implementations, mission fixtures, report templates and CLI selection behavior
unchanged in that slice. A prepared-program contract can be exercised with a
small synthetic program through the existing bounded core runner.

Its completion requires a trivial independent consumer, paired execution of the
adapted ordinary controller, zero-command stop and command-boundary checks,
exact deterministic trace/outcome equality apart from measured compute time,
and the maintained gate. Optional local retained parity is additional evidence.
These checks validate the first boundary; later solver/report extraction still
needs its own preservation checks.

The reviewed decisions and remaining implementation measurements are recorded
in the [review notes](platform_restructure_review.md). Throughput, exact public
Rust signatures, initial distribution ranges and an RL library remain bounded
implementation decisions. They do not reopen the ownership or evidence rules.
