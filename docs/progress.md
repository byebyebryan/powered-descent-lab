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

### Active implementation focus

1. Tighten the action/event-first replay contract before adding richer reports.
2. Start separating mission termination/evaluation from pure touchdown logic.
3. Keep artifacts simple and authoritative:
   - run manifest
   - action log
   - event log
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

- Mission goals are still limited to landing-on-pad scenarios.
- Contact/landing logic is intentionally simple and will need refinement before
  richer terrain or transfer-style evaluations.
- Replay verification is being added now, but mission/eval logic still lives
  inside the simulation loop instead of a cleaner termination/evaluation layer.

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
