# Progress

## 2026-04-17

### Current status

- Repo scaffolded as a Cargo workspace.
- Initial crates created:
  - `pd-core`
  - `pd-control`
  - `pd-cli`
  - `pd-eval`
- `fixtures/scenarios/` created for authored scenario inputs.
- First `pd-core` contract slice is now implemented and compiling.
- First single-run path is now implemented through `pd-cli`.
- Early-termination mission evaluation now has a first concrete timed-checkpoint
  goal shape.
- Replay bundles now include `scenario.json` and can be replayed without an
  external scenario path.
- `pd-eval` now has a first minimal pack runner and summary output path.
- The docs are now realigned around the next real milestone: turning the thin
  controller loop into a proper bot framework with inspection, seeded coverage,
  and eval-side native parallelism.
- The controller layer now emits structured controller frames with:
  - command
  - status
  - phase
  - metrics
  - report/debug markers
- `pd-cli` now writes controller config and controller-update traces into
  artifact bundles and can generate a first static single-run inspection report.
- CLI defaults now write repo-local artifacts under `outputs/` so generated
  bundles and reports are easier to find during iteration.

### Active implementation focus

1. Extend `pd-eval` with scenario families, seeded coverage, and deterministic
   native multithreading.
2. Tighten the first report path beyond a single static run page:
   - richer event/marker inspection
   - better batch-level summaries
   - easier regeneration from batch bundles
3. Keep artifacts simple and authoritative:
   - run manifest
   - action log
   - event log
   - controller update trace
   - optional sampled/debug trace

### Notes

- Start contract-first. Avoid building physics details before command, terrain,
  mission, and artifact types are stable.
- Use a fixed physics step with lower-rate controller updates and held commands.
- Treat sampled traces as optional report/debug caches, not the replay source of
  truth.

### Checkpoints

#### Checkpoint 1: `pd-core` contracts and loop

- Added the first shared domain types:
  - sim config and cadence validation
  - terrain and landing pad specs
  - vehicle geometry/specs
  - scenario/run context
  - observation, command, event, action, sample, and manifest artifacts
- Implemented a minimal deterministic simulation loop with:
  - fixed-rate physics stepping
  - lower-rate controller updates via held commands
  - simple hull/touchdown contact classification
  - terminal outcomes for on-target landing, off-target landing, crash, and timeout
- Added `pd-core` smoke tests for:
  - cadence validation
  - terrain interpolation
  - authoritative action/event/sample capture

### Known limitations

- Contact/landing logic is intentionally simple and will need refinement before
  richer terrain or transfer-style evaluations.
- `pd-eval` currently produces only JSON bundle output and summary counts. It
  does not yet provide baseline-to-baseline diffs, thresholds, or richer
  aggregate metrics.
- Scenario authoring is still pinned-case only. Scenario families, seed sweeps,
  and randomized coverage have not been implemented yet.
- `pd-eval` is still single-threaded. Native parallel execution for multi-seed
  and multi-scenario runs is still design-only.
- Replay bundles now capture controller config and controller traces, but replay
  still treats those as carried-through inspection data rather than as
  authoritative replay inputs.
- The first report path is single-run only. `pd-eval` bundles can be rendered
  through `pd-cli report`, but batch output does not yet generate polished
  report pages directly.

#### Checkpoint 2: baseline runner path

- Added `pd-control` controller traits plus:
  - `IdleController` for debugging
  - `BaselineController` for the first closed-loop landing path
- Added `pd-cli run <scenario>` with:
  - scenario JSON loading
  - controller selection
  - optional artifact file output
  - manifest printing for quick inspection
- Added the first authored scenario fixture:
  - `fixtures/scenarios/flat_terminal_descent.json`
- Replaced leftover crate stubs so the workspace shape better matches the repo
  design docs.
- Validated the first authored scenario by running the baseline controller to an
  on-target landing through `pd-cli`.

### Implementation notes

- The touchdown footprint is now treated as independent from the hull bounds.
  Gear can extend below or wider than the body, which matches the design docs
  better than constraining touchdown points inside the hull box.
- The landing gate uses a small touchdown settle band instead of requiring both
  points to hit in the exact same fixed step. That keeps the first discrete-time
  loop from misclassifying an otherwise safe touchdown as a crash.

#### Checkpoint 3: replay contract hardening

- Added a replay path that consumes logged controller actions instead of a live
  controller callback.
- Added artifact bundle output as separate files:
  - `manifest.json`
  - `actions.json`
  - `events.json`
  - `samples.json`
- Verified replay by comparing the reproduced manifest and event stream against
  the original bundle.

#### Checkpoint 4: mission evaluation split

- Added a dedicated `pd-core` evaluation module so contact detection and mission
  outcome mapping are no longer the same function.
- The simulation loop now:
  - detects contact classification from physics state
  - routes that classification through mission evaluation
  - keeps timeout handling in the same outcome layer
- This does not add new mission types yet, but it makes the next step
  clearer: early-termination goals should plug into the evaluation side rather
  than into low-level contact code.

#### Checkpoint 5: first early-termination goal

- Added `timed_checkpoint` as the first concrete non-landing mission type.
- The goal evaluates an in-flight state envelope at a configured end time,
  relative to the designated target pad.
- Added an authored scenario fixture:
  - `fixtures/scenarios/timed_checkpoint_idle.json`

#### Checkpoint 6: self-contained replay bundles

- Added `seed`, `tags`, and `metadata` to `ScenarioSpec`.
- Replay bundles now write `scenario.json` alongside:
  - `manifest.json`
  - `actions.json`
  - `events.json`
  - `samples.json`
- `pd-cli replay` can now reconstruct from the bundle alone without requiring a
  separate scenario path.

#### Checkpoint 7: minimal batch evaluation

- Added the first `pd-eval` batch runner and `pd-eval run-pack`.
- Added a pack spec and the first named suite:
  - `fixtures/packs/core_suite.json`
- Batch eval now emits per-run bundles plus one JSON summary with:
  - total runs
  - success count
  - mean sim time
  - mission-outcome counts
  - end-reason counts

#### Checkpoint 8: bot-framework realignment

- Reviewed the new project against `pylander`'s older bot framework and captured
  the gap explicitly:
  - the new repo shape and replay/eval boundaries are cleaner
  - the controller-facing contract, telemetry, and inspection path are still too
    thin for a real bot lab
- Reordered the design priorities accordingly:
  - minimal report/inspection moves into the near-term workflow
  - seeded scenario coverage becomes a first-class evaluation concern
  - eval-side native multithreading is now part of the intended batch-execution
    model
- Clarified the migration stance:
  - reuse scenario ideas and proven scenario shapes from `pylander`
  - do not transliterate the old bot interface or scenario files directly

#### Checkpoint 9: first proper bot-framework slice

- Replaced the thin controller callback with a richer controller frame contract:
  - `command`
  - `status`
  - `phase`
  - `metrics`
  - `markers`
- Added controller config/spec support for built-in controllers plus JSON-loaded
  controller configs in `pd-cli` and `pd-eval`.
- Updated bundle outputs to include:
  - `controller.json`
  - `controller_updates.json`
- Added the first static single-run inspection report:
  - terrain and trajectory view
  - event and marker overlays
  - sampled state inspection
  - basic time-series charts for altitude, velocity, throttle, and attitude
- Added `pd-cli report --bundle-dir ...` so report generation can be rerun over
  captured bundles.

#### Checkpoint 10: repo-local output defaults

- `pd-cli run` now defaults to `outputs/runs/<scenario>__<controller>/` when no
  output path is specified.
- `pd-cli replay` now defaults to `outputs/replays/<bundle>/` when no output
  path is specified.
- `pd-eval run-pack` now defaults to `outputs/eval/<pack>/`.
- `/outputs` is ignored in git so generated artifacts stay local.
