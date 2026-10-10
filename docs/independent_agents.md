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

The independent checkpoint-backed runnable example, reset/step API and measured
throughput are the next integration milestone. This page describes implemented
APIs; it does not claim a trained RL policy or completed solver migration.
