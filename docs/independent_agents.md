# Independent agents on the shared platform

[Architecture](architecture.md) · [Procedural missions](procedural_missions.md) · [Implementation record](platform_restructure_results.md)

The reusable path is `pd-core` → `pd-run` → `pd-eval`, with `pd-missions`
providing resolved inputs and `pd-report` rendering saved evidence. None of
these crates imports `pd-control`, `pd-plan`, `pd-planner` or `pd-eval-cli`,
including with all features. The developer gate enforces that boundary.

## Implementing an approach

Implement `pd_run::Agent` in your own crate. Its descriptor records an open
implementation identity, configuration JSON and optional checkpoint SHA-256.
`reset` and `update` receive a physical `MissionView`: simulation clocks, terrain,
vehicle, initial state, mission and physical target. Scenario labels, seeds,
tags and collection membership are excluded. `update` returns an `AgentFrame`
with a command and optional telemetry/markers. No existing registry entry is
required. Strategy-specific rewards, forecasts and training logic belong to
your implementation.

`run_agent` and `run_prepared_program` both use the authoritative ordinary
bounded plant host. Commands start at physics tick zero, are held at the declared
control cadence, and stop at the actual terminal or finite endpoint. Invalid
decisions retain a typed execution-invalid prefix. Prepared schedules bind all
physical/mission inputs and every covered control tick; they are command data,
not a feasibility certificate.

The maintained controllers retain their existing `Controller` interface and
serialized specifications. `pd-control::run_controller` delegates neutral frame
recording to `pd-run`, preserving reset order, command clamping and core execution.

## Evaluation and capture

`pd_eval::evaluate_agent` executes the actual mission, binds the executable and
explicit source/checkpoint input scope before and after execution, and replays
the native command log. Source scope is declared by the composition application;
it must include its own agent implementation and runtime dependencies. The
executable is always bound. An empty source list means binary-only provenance,
not complete source provenance. Frozen-input identifiers and collection role
never become decision-time observations.

Physical prefix verification is distinct from complete episode verification.
An unavailable policy decision can have a verified physical prefix while its
native failure reason remains separate. A verified target landing requires the
physical landing, successful mission, stable integrity, complete bounded replay
and a terminal stop without an execution failure. Successful early checkpoints
and finite planning stops are not landings.

`save_evaluation` reserves a fresh directory and writes `evaluation.json` and
`seal.json` create-only. `load_verified_evaluation` checks exact saved bytes,
physical identity, accounting and authoritative command replay without loading
the policy implementation or checkpoint. Existing captures and selectors are
untouched.

`evaluate_collection` accepts an explicit attempt schedule and a fresh-agent
factory. Repeated indices are separate attempts with one unique-mission count.
Input, initialization, execution and capture failures each retain a disposition;
they do not disappear from the denominator. It records a collection exposure
receipt before execution. Existing built-in workflows remain application-owned
in `pd-eval-cli`; the installed binary is still named `pd-eval`.

`evaluate_collection_with_executor` admits offline approaches through a
composition callback, without importing a strategy into evaluation. Unsupported
inputs retain a separate disposition in the full scheduled denominator. Each
episode binds its selected mission and explicit checkpoint/source scope; the
whole collection is bound once before/after the campaign. Overall campaign
integrity must pass before using its aggregate landing counts. There is no
mutable evidence cache or whole-collection snapshot inside each episode.

`assess_execution` applies the same physical/mission/integrity/replay contract
to an externally driven ordinary native episode. The thin `pd-python` binding
uses it for the [independent PPO experiment](../experiments/rl/README.md).
Its observation/action/reward and launch choices are approach-owned; there is
no controller/planner dependency or fallback.

## Rich reporting

`pd_report::render_agent_run_report` uses the maintained common run template,
plots, telemetry, navigation and configuration panel. Evaluation supplies an
`AgentReportEvidenceV1` projection and capture links. This rendering API writes
nothing; the caller persists a new report create-only. Odd or unsampled endpoints
are displayed from the saved final-state projection, with an explicit caption;
raw samples, commands and proof records stay unchanged.

Report configuration parameters are JSON snapshots. Existing applications
serialize their unchanged `ControllerSpec` at the rendering boundary. Historical
configuration decoding therefore does not import executable controllers into
the renderer.

`pd_report::campaign` renders common campaign projections on the maintained
batch shell, with coverage groups, an interactive attempt tree, configuration,
native proof dispositions and individual run links. Failed campaign integrity
prevents aggregate promotion even when an earlier individual capture verifies.

## Reset/step and capture modes

`pd_run::Episode::reset` constructs state from a resolved scenario.
`Episode::step` advances one held action to the next control boundary, stopping
earlier at actual contact/progress, a numeric failure or a finite endpoint.
State cannot be replaced by an arbitrary snapshot. Completed episodes require
an explicit reset, and the runtime emits no reward.

`CaptureModeV1::Summary` retains final state, manifest and stop disposition without
action, event, sample or update vectors. It supplies physical/mission outcomes
for an external training loop, not saved-command landing proof. Captured mode
retains native records. Both modes use the same incremental host, ordinary core
transitions and core artifact projections. The existing guarded bounded core
backend remains an explicit capability seam, with its own guard ownership.

`run_agent_with_capture_mode` drives an open agent through this host; the regular
`run_agent` and prepared-program runner use captured mode. Tests compare both
modes at every decision boundary against the direct ordinary core path, including
zero/odd endpoints, contact, mission checkpoints, scenario timeout and failed
decisions. Summary mode retains zero trace records throughout.

The [independent checkpoint example](../examples/checkpoint-policy/README.md)
provides a runnable workflow with explicit observation/action mapping. Its
hand-written weights demonstrate the interface; no trained RL competence is
claimed. Maintained solver execution is now separately owned by `pd-planner`.

The launch integration produced 66 saved attempts/reports on 64 procedural
missions, with complete integrity/replay and exact timing-excluded repeats.
The demo rotates while resting on its source pad and crashes on the first tick;
the example preserves those failed outcomes and has not been tuned after them.
Its timing sample measures early-stop overhead. A separate fixed airborne trace
exercises sustained reset/step execution and capture cost:

```sh
rtk proxy cargo run --release -p pd-run --example held-trace-benchmark -- outputs/my-held-trace.json
```

This instrumentation derives a synthetic input without modifying its fixture.
It records actual ticks, endpoint, summary/capture equivalence, exact native
replay, serialization and process RSS. The [run record](platform_restructure_results.md)
keeps its measurements separate from procedural input coverage and policy ability.
