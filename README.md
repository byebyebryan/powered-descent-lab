# Powered Descent Lab

Powered Descent Lab (`pd-lab`) is a native Rust lab for deterministic 2D rocket
flight, guidance, waypoint planning, evaluation and replay. It is not the
player-facing game: the lab owns flight behavior and evidence; a future host
owns presentation, input and game feel.

## Current checkpoint

- Policy 3 is the sole executable planner. It constructs a terrain-blind nominal
  flight, audits that fixed path against terrain, applies a local
  clearing correction when needed, and replans from the actual handoff state.
  Airborne proposals can use long powered landing segments; the
  [ballistic aim and correction revision](docs/ballistic_aim_correction_plan.md)
  has an opt-in candidate and [local waypoint results](docs/ballistic_local_waypoint_results.md),
  but is not the accepted planner.
- The accepted planner pack has 36/36 core target landings: 11 direct and 25
  corrected. Eight diagnostics remain separate: two landings, four zero-step
  `NoClearing` stops and two unsupported inputs. All 44 inputs pass integrity;
  all 42 supported results pass recorded source replay.
- The supported envelope is the tested vehicle, Earth gravity, 120 Hz physics
  and 60 Hz commands. This is synchronous offline planning, not arbitrary-terrain
  reliability, arbitrary airborne restart or a per-tick game-controller guarantee.
- Terminal, direct-transfer and authored-waypoint controllers remain supported.
  Ordinary `pd-cli run` defaults are unchanged; its V2 commands require the
  default-off `planner-v2` feature.

The [accepted session/CLI results](docs/waypoint_v2_session_repair_results.md)
own measured flight evidence. The
[cleanup results](docs/planner_retirement_cleanup_results.md) record subsequent
behavior-preserving source work; cleanup does not relabel captured provenance.

<a id="docs"></a>

## Start here

| You want to… | Read |
| --- | --- |
| Build, test or change the repository | [Development workflow](docs/development.md) |
| Run planner or controller evaluations | [Evaluation workflow](docs/evaluation.md) |
| Browse, serve or refresh reports | [Report workflow](docs/reports.md) |
| Understand current flight ownership and limits | [Guidance](docs/guidance.md), [architecture](docs/architecture.md) |
| Choose the next product pass | [Current roadmap](docs/roadmap.md#7-recommended-immediate-next-step) |
| Find all docs or past experiments | [Documentation home](docs/README.md), [history](docs/history.md) |

<a id="planner-evaluation-v2-default"></a>
<a id="single-planner-flight"></a>
<a id="batch-eval"></a>

## Quick start

Run commands from the repository root. The maintained tooling uses Rust/Cargo,
Node.js, Git and RTK; report serving also needs Python 3 and tmux. See
[development prerequisites](docs/development.md#prerequisites).

Build and run the maintained local gate:

```sh
rtk proxy cargo build --workspace
rtk proxy node scripts/check-planner-development.mjs
```

Run the current planner pack:

```sh
rtk proxy cargo run --release -p pd-eval -- run-pack --workers 4 --enforce-regression-policy
```

This creates a new capture and publishes the current report only if acceptance
passes. A completed capture or successful collection exit is not itself a landing.
For read-only inspection of already-saved evidence:

```sh
rtk proxy cargo build --release -p pd-eval
rtk proxy node scripts/check-planner-v2-workflow.mjs
```

Saved captures are local, ignored data; a fresh checkout does not contain them.
The ordinary developer gate needs no external capture.

<a id="report-serving"></a>

Serve locally generated reports:

```sh
rtk proxy ./scripts/serve-reports start
```

Start at `/` or `/reports/`, then **Waypoint planning → Current V2 batch → Mission
detail**. The canonical batch is `/reports/eval/planner_v2_lab_suite/`.
Terminal, transfer and planner reports share the detailed templates and review
tree; executed waypoint handoffs extend those views rather than replace them.
See [report operations](docs/reports.md) before refreshing or publishing pages.

## Repository map

| Area | Responsibility |
| --- | --- |
| `pd-core` | Deterministic plant, missions, terrain queries and neutral persisted contracts |
| `pd-plan` | Pure current ballistic and local-clearing calculations |
| `pd-control` | Terminal, direct-transfer and authored-waypoint guidance |
| `pd-eval` | Owned V2 session, pack expansion, acceptance, evidence and batch reporting |
| `pd-cli` | Ordinary run/replay/report and feature-gated V2 flight/replay adapters |
| `pd-report` | Common rich detail and batch templates, annotations and report navigation |
| [fixtures](fixtures/README.md) | Maintained inputs, frozen research data and deliberately retained archive metadata |
| [scripts](scripts/README.md) | Maintained checks, explicit capture tooling and report operations |
| `docs` | Current contracts, workflows and dated development records |
| `outputs`, `target` | Local generated data and build products, ignored by Git |

<a id="direction"></a>
<a id="why-reboot"></a>
<a id="scope"></a>

[Project direction and scope](docs/project_direction.md) preserves the original
lab/game split and reboot rationale. [Repository guidance](AGENTS.md) records
safe change boundaries for coding agents.
