# Progress

## 2026-04-28

### Current status

- The latest controller pass adds lateral-cushion preservation when
  projected touchdown is outside the safe touchdown-footprint center.
- Phase 2 remains the active phase, but the center of gravity has moved:
  - report/cache/review-tree infrastructure is no longer the bottleneck
  - the clean `empty` and `half` payload tiers have no scored current-lane
    failures
  - low-thrust/high-energy frontier cells are scored and annotated, not
    excluded from failure counts
  - the trajectory-error matrix is now the main stress corpus above the clean
    payload tiers
- The important distinction in reports is now:
  - `current` means the latest `terminal_pdg_v1` run
  - compare/baseline provenance may point at a cached prior current run or at
    the internal heuristic baseline controller
  - `baseline` as a report lane is not a physical scenario axis

### Current clean-matrix checkpoint

Latest local reports:

- `outputs/eval/terminal_bot_lab_suite/summary.json`
- `outputs/eval/terminal_bot_lab_full/summary.json`

Latest local wall-clock signal with `8` workers:

- `terminal_bot_lab_suite`: `7.13s`
- `terminal_bot_lab_full`: `28.99s`

Smoke tier:

- `terminal_bot_lab_suite`
  - `current`: `168 / 180` scored successes, `12` scored failures,
    `9` impossible warnings, `12` frontier annotations
  - `baseline`: `33 / 180` scored successes, `147` scored failures,
    `9` impossible warnings, `12` frontier annotations

Full pack:

- `terminal_bot_lab_full`
  - `current`: `676 / 720` scored successes, `44` scored failures,
    `36` impossible warnings, `48` frontier annotations
  - `baseline`: `135 / 720` scored successes, `585` scored failures,
    `36` impossible warnings, `48` frontier annotations

`terminal_bot_lab_full` current-lane split by payload tier:

- `empty`: `252 / 252`
- `half`: `252 / 252`
- `full`: `172 / 216` scored, `44` scored failures,
  `36` impossible warnings, `48` frontier annotations

The clean matrix read is now:

- `empty` is solved on the maintained Earth corpus
- `half` is solved on the maintained clean Earth corpus
- `full` is the clean-matrix low-thrust/high-energy frontier tier; failed
  frontier cells remain scored failures

### Trajectory-error matrix checkpoint

Latest local reports:

- `outputs/eval/terminal_traj_err_suite/summary.json`
- `outputs/eval/terminal_traj_err_full/summary.json`

Latest local wall-clock signal with `8` workers:

- `terminal_traj_err_suite`: `15.33s`
- `terminal_traj_err_full`: `58.51s`

Smoke tier:

- `terminal_traj_err_suite`
  - `current`: `686 / 720` scored successes, `34` scored failures,
    `36` impossible warnings, `48` frontier annotations

Full pack:

- `terminal_traj_err_full`
  - `current`: `2741 / 2880` scored successes, `139` scored failures,
    `144` impossible warnings, `192` frontier annotations

`terminal_traj_err_full` current-lane split by condition:

- `traj_undershoot_small`: `693 / 720` scored, `27` scored failures,
  `36` impossible warnings, `48` frontier annotations
- `traj_undershoot_large`: `707 / 720` scored, `13` scored failures,
  `36` impossible warnings, `48` frontier annotations
- `traj_overshoot_small`: `672 / 720` scored, `48` scored failures,
  `36` impossible warnings, `48` frontier annotations
- `traj_overshoot_large`: `669 / 720` scored, `51` scored failures,
  `36` impossible warnings, `48` frontier annotations

`terminal_traj_err_full` current-lane split by payload tier:

- `empty`: `1008 / 1008`
- `half`: `1005 / 1008`, `3` scored failures
- `full`: `728 / 864` scored, `136` scored failures,
  `144` impossible warnings, `192` frontier annotations

The trajectory-error read is now:

- `empty` is solved across the projected-miss corpus
- `half` is nearly solved, with only sparse high-energy overshoot-large outliers
  still standing out
- `full` is represented as a scored low-thrust/high-energy frontier stress tier
- the remaining scored failures are real stress cases, not report artifacts:
  - `traj_overshoot_large / half / high`: `3` failures across `a60 / a80`
  - low-thrust/high-energy `full / high` frontier failures across clean and
    trajectory-error conditions

### Latest tuning note

The latest tuning loops deliberately rejected several broad controller levers:

- Late-touchdown rescue sign/tilt changes regressed `terminal_traj_err_suite`
  from the checkpoint `685 / 720` scored successes to the `650-664 / 720`
  range, even when narrowed to off-pad projected touchdown.
- Extending lateral-hold descent slowing above the touchdown band did not solve
  `traj_overshoot_large / half / a45 / high / seed 6`; it moved the suite to
  `684 / 720` and introduced an additional frontier off-target result.
- Letting urgent latest-safe recovery pick tilt-infeasible short candidates was
  too broad: the suite dropped to `677 / 720` and created new empty/half
  overshoot regressions.
- Shortening terminal gate vertical-brake timing with a shared-control tilt
  assumption was too sensitive globally; both `0.5x` and `0.85x` tilt scales
  broke pinned `pd-control` landing fixtures before suite testing.
- Adding only a midpoint latest-safe candidate kept the suite at `685 / 720` but
  did not fix `traj_overshoot_large / half / a45 / high / seed 6`.
- Extending projected-off-pad lateral hold to `80 m` worsened that seed in a
  single-run probe: it still crashed off target, with higher normal speed and
  attitude error.
- Raising the latest-safe max horizon from `14 s` to `30 s` proved the seed is
  recoverable only when paired with a midpoint candidate, but the broad default
  regressed the smoke suite to `683 / 720`, created new half-payload scored
  crashes/timeouts, and raised mean sim time from about `33.6 s` to `46.8 s`.

Focused diagnosis that led to the accepted fix:

- `traj_overshoot_large / half / a45 / high / seed 6` is the lane's worst
  projected-overshoot smoke seed (`engine_off_impact_x = 105 m`). The passing
  `seed 0` and `seed 1` variants sit at `75 m` and `90 m`.
- At about `40 m` altitude the failed run is still roughly `45 m` off-pad,
  moving laterally at about `7 m/s`, and descending near `19 m/s`; by `24 m`
  it is still roughly `49 m` off-pad. Late touchdown rescue then correctly
  protects touchdown speed, but it has no remaining room to recenter.
- A simple ZEM/ZEV feasibility check from the initial state needs roughly a
  `20 s` or longer capture horizon before the combined acceleration fits the
  tilt limit. The current latest-safe horizon is capped at `14 s`, so the
  controller stays in short-horizon emergency braking until the problem is
  geometrically unrecoverable.

The useful finding is that the sparse half-payload overshoot failures are not
final-touchdown rescue problems. The accepted fix is a conditional
long-capture mode: when an urgent latest-safe candidate is over authority and
the current target offset and projected ballistic miss have opposite signs, the
controller adds `22 s` and `30 s` capture candidates without raising the global
`14 s` latest-safe cap.

The smoke suite improved from `685 / 720` to `686 / 720` scored successes. The
full trajectory-error pack improved from `2732 / 2880` to `2741 / 2880` scored
successes, with no new failures relative to the previous full cache. The patch
fixed six half-payload overshoot-large full-pack failures across `a30/a45`
plus three full-payload frontier failures, leaving only:

- `traj_overshoot_large / half / a60 / high / seed 2`
- `traj_overshoot_large / half / a60 / high / seed 4`
- `traj_overshoot_large / half / a80 / high / seed 2`

### Active implementation focus

1. Keep the next controller pass general and focused on sparse
   trajectory-error scored failures:
   - avoid seed-specific or condition-specific state-machine hacks
   - avoid late touchdown-rescue sign/tilt tweaks as the first lever; the
     latest loop showed those create broad trajectory-error regressions
   - avoid broad latest-safe candidate ordering or vertical timing changes
     unless a pinned fixture and suite run both show no regressions
   - next controller experiment should focus on the remaining `a60/a80`
     overshoot-large high-energy cases without widening long capture into
     same-side projected misses
2. Keep refining feasibility / annotation semantics only where the vehicle is
   authority limited, while keeping frontier failures scored
3. Add the next terminal corpus only after the current clean and
   trajectory-error semantics stay stable:
   - terrain / obstacle conditions
   - later transfer-style conditions
4. Start thresholded regression policy after the current metrics are stable
   enough to distinguish meaningful regressions from frontier churn.

## 2026-04-23

### Status at this checkpoint

- Phase 2 is now primarily a controller-and-corpus phase, not a scaffolding
  phase.
- The terminal bot-lab workflow is in place end to end:
  - executable Earth `half_arc_terminal_v1` matrix
  - payload tiers `empty / half / full`
  - `terminal_pdg_v1` as the serious current lane
  - split trajectory-error condition packs for undershoot/overshoot
  - cache reuse / promotion / current-lane history compare in `pd-eval`
  - impossible-run classification for analytically unrecoverable terminal
    cases
- The maintained Earth terminal baseline now uses:
  - `60s` timeout headroom
  - relaxed `nominal_ttg_by_arc_point`
  - `a80 = 8.00s`
- The internal heuristic baseline lane is still useful as a reference
  controller, but it is no longer the main progress metric.

### Controller checkpoint at this date

- `terminal_bot_lab_suite`
  - `current`: `161 / 180` scored successes, `19` scored failures,
    `9` impossible
  - `baseline`: `33 / 180` scored successes, `147` scored failures,
    `9` impossible
- `terminal_bot_lab_full`
  - `current`: `643 / 720` scored successes, `77` scored failures,
    `36` impossible
  - `baseline`: `135 / 720` scored successes, `585` scored failures,
    `36` impossible

Full-tier current-lane split by payload tier:

- `empty`: `252 / 252`
- `half`: `228 / 252`
- `full`: `163 / 216` scored, `36` impossible

This means the maintained Earth matrix is now doing the intended job:

- `empty` is effectively solved on the current clean corpus
- `half` is mostly solved, with the remaining gap concentrated in `a80`
- `full` still exposes real control-authority limits and frontier cells

### Active implementation focus at this date

1. Close the shallow-tail controller gap on the maintained Earth matrix:
   - `half a80 mid/high`
   - the remaining scored `full` high-band failures
2. Expand the terminal corpus above the clean payload tiers:
   - `traj_undershoot_small`
   - `traj_undershoot_large`
   - `traj_overshoot_small`
   - `traj_overshoot_large`
   - later terrain and obstacle conditions
3. Tighten evaluation semantics now that the workflow is real:
   - broader frontier / infeasible classification beyond the current vertical
     brake bound
   - thresholded regression policy
   - only then more specialized report affordances

### Trajectory-error matrix checkpoint

- Added current-lane-only trajectory-error packs:
  - `fixtures/packs/terminal_traj_err_suite.json`
  - `fixtures/packs/terminal_traj_err_full.json`
- The tree keeps the same selector depth as the clean matrix:
  - `condition_set -> arc_point -> velocity_band -> vehicle_variant -> lane -> seed`
- Condition sets are split by direction and severity instead of aggregating
  undershoot and overshoot:
  - `traj_undershoot_small`
  - `traj_undershoot_large`
  - `traj_overshoot_small`
  - `traj_overshoot_large`
- The condition perturbation is projected engine-off miss distance:
  - `small`: `30m`, `45m`, `60m`
  - `large`: `75m`, `90m`, `105m`
  - undershoot remains short on the approach side
  - overshoot crosses past the target to the far side

Smoke-tier result:

- `terminal_traj_err_suite`
  - `current`: `640 / 720` scored successes, `80` scored failures,
    `36` impossible

Full-tier result:

- `terminal_traj_err_full`
  - `current`: `2557 / 2880` scored successes, `323` scored failures,
    `144` impossible
- By condition:
  - `traj_undershoot_small`: `655 / 720` scored, `36` impossible
  - `traj_undershoot_large`: `684 / 720` scored, `36` impossible
  - `traj_overshoot_small`: `624 / 720` scored, `36` impossible
  - `traj_overshoot_large`: `594 / 720` scored, `36` impossible
- By vehicle tier:
  - `empty`: `1008 / 1008`
  - `half`: `877 / 1008`
  - `full`: `672 / 864` scored, `144` impossible

The immediate read is that empty-payload trajectory error is solved across the
new corpus, half-payload failures cluster in the shallow/high-energy frontier,
and full-payload trajectory error mostly amplifies the already-known
control-authority limits.

## 2026-04-22

### Current status

- The terminal suite is no longer only a maintained design target.
- `terminal_pdg_v1` now exists as the first serious native Rust terminal
  controller lane:
  - terminal-only PDG-shaped guidance
  - latest-safe and nominal gate evaluation
  - braking-envelope vertical schedule
  - touchdown-clearance-aware rescue logic
- `pd-eval` now expands a real terminal matrix entry type:
  - selector hierarchy:
    - `mission`
    - `arrival_family`
    - `condition_set`
    - `vehicle_variant`
  - matrix axes:
    - `arc_point`
    - `velocity_band`
  - controller lane separated from the physical case
- `half_arc_terminal_v1` is now executable as the Earth baseline family.
- The main controller workbench packs are now:
  - `fixtures/packs/terminal_bot_lab_suite.json`
    - smoke tier
  - `fixtures/packs/terminal_bot_lab_full.json`
    - full tier
- The maintained Earth terminal hardware baseline now matches the core
  `pylander` vehicle/engine envelope closely enough for direct reasoning:
  - `8m x 10m` hull
  - `7200kg` dry mass
  - `6300kg` max fuel
  - `240000N` max thrust
  - `25%` ignited minimum throttle
  - `90 deg/s` max rotation rate
- The main vehicle variants are now payload tiers, not fuel-margin tiers:
  - `empty`
    - empty payload
  - `half`
    - half payload
  - `full`
    - full payload
- `pd-lab` still intentionally simplifies one engine detail:
  - no `pylander` overdrive path yet
  - fuel burn scales linearly between minimum and maximum thrust
- The bot-lab `current` lane now points at `terminal_pdg_v1`, not the older
  staged heuristic.
- Batch reports now surface the terminal matrix directly in the review tree:
  - `mission -> arrival_family -> condition_set`
  - `arc_point -> velocity_band -> vehicle_variant -> lane -> seed`

### First matrix results

- The first real Earth matrix run already showed that the old `current` lane
  was not viable on the real matrix and directly forced the `terminal_pdg_v1`
  pass.
- After aligning the maintained Earth suite to the heavier `pylander`
  vehicle/engine baseline and payload tiers:
  - `terminal_bot_lab_suite`
    - `baseline`: `0 / 126`
    - `current`: `0 / 126`
  - `terminal_bot_lab_full`
    - `baseline`: `0 / 504`
    - `current`: `0 / 504`
- That regression is useful signal, not a framework failure:
  - the suite is now discriminating against controllers that were implicitly
    tuned for an easier vehicle model
  - controller work needs to restart from this more faithful baseline instead
    of extrapolating from the earlier lighter craft
- The next controller task is no longer only "fix the shallow tail":
  - first restore viability on the steeper core
  - then reopen the shallow tail once the heavier baseline is under control

### Active implementation focus

1. Restore controller viability on the pylander-aligned Earth terminal matrix:
   - nominal first
   - then heavy-cargo
   - only then re-focus on the shallow tail
2. Expand the now-real matrix corpus carefully:
   - undershoot / overshoot trajectory-error conditions
   - small / large projected-miss severities
   - later terrain and obstacle conditions
3. Only after controller signal is meaningful on the matrix:
   - add thresholded regression policy
   - add compare cache / promotion / invalidation semantics
   - consider matrix-specific batch-report affordances

## 2026-04-21

### Current status

- The repo is now past the first “does the scaffold work?” stage.
- A real bot-lab workflow exists end to end:
  - deterministic sim and replay
  - shared controller kit
  - multiple built-in controller lanes
  - seeded packs and native multithreaded eval
  - single-run and batch report generation
  - report-site navigation under `outputs/reports/`
- `terminal_bot_lab_suite` is now the main controller workbench.
- External pack-vs-pack compare still exists, but only as explicit fixtures:
  - `terminal_compare_baseline_fixture`
  - `terminal_compare_regression_fixture`
- Batch reports now explicitly state their provenance and comparison mode:
  - standalone
  - lane compare
  - external compare
- Reporting is now good enough to support real iteration; it is no longer the
  primary blocker.
- Terminal-suite design now has a dedicated maintained doc:
  - `docs/terminal_suite.md`
- The biggest remaining structural gap is no longer “make the terminal matrix
  real.”
- That work is now in place. The next gap is controller performance and corpus
  expansion on top of the real matrix.

### Active implementation focus

1. Improve controller behavior on the real terminal matrix:
   - clean nominal
   - heavy-cargo / stress
2. Expand the real terminal corpus with:
   - trajectory-error conditions
   - later terrain and obstacle conditions
3. Once the corpus and controller signal are stable, tighten eval semantics
   around:
   - representative runs
   - thresholds and regression policy
   - compare cache / promotion / invalidation
4. After terminal guidance is structurally solid, start the smallest true
   Phase 3 transfer slice.

### Current open gaps

- The terminal matrix is now real, and the current lane is finally
  competitive on it, but the shallow-tail cells still need more work.
- The current bot-lab corpus only covers:
  - `clean`
  - `nominal`
  - `heavy_cargo`
  and still needs trajectory-error and later terrain conditions.
- Compare provenance is explicit in the report, but compare cache / promote /
  invalidate semantics are not implemented yet.
- Reporting is now being driven by real matrix scenarios, but should continue
  to follow corpus/controller needs rather than generic layout tuning.
- Terminal/eval suite design should now be treated as a maintained design
  surface, not only as ad hoc planning notes attached to one implementation
  pass.

#### Checkpoint 23: `terminal_pdg_v1` replaces the staged current lane

- Added a new native Rust controller in `pd-control`:
  - `terminal_pdg_v1`
  - terminal-only PDG-shaped guidance
  - latest-safe and nominal gate evaluation
  - braking-envelope vertical schedule
  - touchdown-clearance-aware rescue and touchdown cut logic
- Added focused `pd-control` tests for the new controller:
  - flat-fixture success
  - guidance metrics and gate-marker emission
- Switched both bot-lab packs so `current` now means `terminal_pdg_v1`:
  - `terminal_bot_lab_suite`
  - `terminal_bot_lab_full`
- The new controller is not only “alive”; it materially outperforms the old
  baseline on the real Earth matrix:
  - smoke: `100 / 126` vs `12 / 126`
  - full: `403 / 504` vs `42 / 504`
- The shallow tail is now the real remaining problem:
  - `a70`
  - `a80`
  - and especially the `high` velocity band

#### Checkpoint 22: real terminal matrix execution and full bot-lab suite

- Added a first-class terminal-matrix entry type to `pd-eval`:
  - explicit selector hierarchy
  - explicit `arc_point x velocity_band` matrix expansion
  - deterministic seed policy with documented side resolution
  - lane-aware execution over the same resolved physical cases
- Implemented `half_arc_terminal_v1` directly in the evaluator as the
  maintained Earth baseline family:
  - `radius_nominal = 800m`
  - `7` arc points
  - `3` velocity bands
  - `smoke` and `full` seed tiers
- Replaced the old provisional bot-lab pack with a real matrix smoke suite:
  - `fixtures/packs/terminal_bot_lab_suite.json`
- Added the matching full-tier pack:
  - `fixtures/packs/terminal_bot_lab_full.json`
- Extended the batch report tree so terminal review now exposes:
  - `arc_point`
  - `velocity_band`
  between the vehicle bucket and lane/seed detail
- The first real Earth runs are already informative:
  - baseline only succeeds on a narrow slice of the matrix
  - current has zero successes on both smoke and full tiers
- That is a good outcome for the framework even though it is bad for the
  current controller. The suite is now doing real discriminating work instead
  of only approximating the intended corpus.

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
- The single-run HTML report has now been reworked around a denser Plotly-based
  layout with a large spatial plot, compact metric panels, hover inspection,
  and summarized events/markers instead of verbose raw tables.

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

#### Checkpoint 11: report density pass

- Reworked the report layout after comparing it with the more mature
  `pylander` viewer shape.
- Borrowed the useful parts conceptually:
  - Plotly-based interactive plots
  - one primary spatial panel
  - compact metric panels instead of long stacked sections
  - hover-driven inspection
  - sparse event markers and summaries instead of long event tables
- Kept the new report simpler than the old `pylander` viewer:
  - no direct transliteration of the old HTML structure
  - still static-output-first and bundle-driven

#### Checkpoint 12: report layout cleanup

- Tightened the report layout after direct visual inspection of generated HTML
  pages in a browser.
- Removed redundant inner Plotly titles so the panel headings remain the only
  titles on the page.
- Reduced chart clutter by:
  - disabling the modebar on compact metric plots
  - simplifying chart margins
  - moving compact-plot legends out of the title/modebar collision zone
- Tightened spacing across the dashboard so the page reads more like a dense
  inspection surface than a stack of roomy cards.
- Moved compact metric-plot legends into the top chart margin so they no longer
  collide with the x-axis title.

## 2026-04-18

### Current status

- Reports can now be served from a repo-owned script instead of ad hoc shell
  commands.
- The intended output-path stance is now captured explicitly:
  - keep single-run report paths stable and semantic
  - reserve digests and commit/workspace keys for batch caches and compare
    workflows
- `pd-eval` now expands scenario families into resolved seeded runs, records the
  resolved perturbation parameters per run, and can execute those runs on a
  native thread pool while preserving stable output ordering.
- Batch outputs now include:
  - `pack.json`
  - `resolved_runs.json`
  - `summary.json`
  - per-run bundles under `outputs/eval/<pack>/runs/`
- Batch summaries are now rich enough to surface:
  - outcome counts by entry and family
  - failed seeds
  - slowest runs
  - stable digests for the pack spec and resolved run set
- `pd-eval` now emits a first static batch `report.html`, and batch comparison
  against a baseline directory is now a first-class workflow rather than only a
  future design note.
- The report server now binds on all interfaces by default and prints a
  detected LAN URL so remote devices on the local network can open generated
  reports directly.
- A separate `outputs/reports/` tree now exists as the primary navigation
  surface for HTML pages, so report browsing no longer starts in raw artifact
  directories.
- `outputs/index.html` now acts as a generated root landing page above the
  report tree, so the server root can point at one stable navigation page.

#### Checkpoint 13: report server script and artifact-key review

- Added `scripts/serve-reports` as the canonical local report-serving entrypoint.
- The script:
  - serves `outputs/` over `python3 -m http.server`
  - runs inside a named detached `tmux` session by default
  - supports `start`, `stop`, `restart`, `status`, `attach`, and `url`
- Documented the workflow in the repo README so remote-over-SSH usage has one
  obvious entrypoint.
- Reviewed `pylander`'s artifact identity approach and kept the useful split:
  - semantic stable keys for per-run inspection paths
  - short digests and commit/workspace identity only for cache-heavy benchmark
    and compare outputs
- Deliberately did not pull over the heavier tracepack/cache naming machinery
  yet, because `pd-lab` does not have the equivalent compare workflow in-tree
  yet.
- Added `latest` symlinks for report-producing `pd-cli` workflows so the most
  recently written single-run or replay report is reachable through a stable
  convenience path.

#### Checkpoint 14: seeded sweeps and multithreaded batch eval

- Extended `pd-eval` pack entries from only pinned scenarios to two concrete
  forms:
  - `scenario`
  - `family`
- Added scenario-family expansion with:
  - explicit seed lists
  - seed ranges
  - deterministic numeric perturbations with optional quantization
- Kept the ownership boundary intact:
  - `pd-core` still runs one resolved concrete scenario
  - `pd-eval` owns family expansion, seed policy, and multirun orchestration
- Added stable resolved-run identity:
  - semantic run IDs such as `terminal_baseline_sweep_seed_0003`
  - recorded `family_id`, resolved seed, and resolved perturbation values
- Added deterministic batch identity with separate digests for:
  - the pack spec
  - the resolved run set
- Added native multithreaded execution in `pd-eval` via a bounded worker pool.
- Preserved deterministic output order and verified that sequential and
  multithreaded execution produce the same resolved-run digest.
- Added richer batch summary output:
  - mission and physical outcome counts
  - end-reason counts
  - grouped summaries by entry and by family
  - failed-run pointers with seed and bundle location
  - slowest-run pointers for quick inspection
- Added a first seeded sweep fixture.
  It has since been demoted and renamed into the explicit compare smoke
  fixtures:
  - `fixtures/packs/terminal_compare_baseline_fixture.json`
  - `fixtures/packs/terminal_compare_regression_fixture.json`
- Added `latest` symlink maintenance for `pd-eval` output directories so the
  most recent batch result is reachable through:
  - `outputs/eval/latest/`
  - `outputs/eval/<pack>/runs/latest/`

#### Checkpoint 15: first batch report and baseline compare

- Added a first static batch-report page at:
  - `outputs/eval/<pack>/report.html`
- Added `pd-eval report <batch-dir>` so batch pages can be regenerated from an
  existing output directory.
- Added optional baseline comparison through `--baseline-dir` on:
  - `pd-eval run-pack`
  - `pd-eval report`
- Added `compare.json` as a first-class batch artifact when a baseline is
  supplied.
- The compare model currently uses shared `run_id` matching and records:
  - compare basis (`shared`, `candidate only`, `baseline only`)
  - global candidate-vs-baseline deltas
  - grouped deltas by entry and family
  - regression, recovery, and outcome-change run lists
- Borrowed the useful part of the `pylander` batch-report approach:
  - comparative reports matter more than standalone summaries
  - grouped scenario views and representative run links are more useful than a
    giant flat table
- Kept the first `pd-lab` batch report simpler than the late `pylander` bundle
  pages:
  - dense HTML tables and cards first
  - no heavy chart layer yet

#### Checkpoint 16: batch cache reuse and promotion

- `pd-eval run-pack` now has a real cache layer under:
  - `outputs/eval/cache/<workspace-or-commit-key>/<batch-stem>/`
- Candidate cache identity is now derived from:
  - clean commit key or dirty workspace key
  - pack id
  - pack spec digest
  - resolved run digest
- `run-pack` now reuses a complete cache automatically by default instead of
  always rerunning the batch.
- Added explicit cache metadata in `meta.json` and threaded cache provenance
  into `summary.json` and the batch `Context` section.
- Added `--compare-ref auto|<ref>|none` to `pd-eval run-pack`:
  - dirty workspaces default to comparing against the clean `HEAD` cache
  - clean workspaces default to comparing against the previous clean commit
    cache
- Added `pd-eval promote-cache <pack>` so a validated dirty cache can be copied
  onto the clean commit key after checkpointing, instead of rerunning the old
  code just to recover a baseline.
- The batch `Context` section now explicitly reports:
  - current cache status (`fresh`, `reused`, `promoted`)
  - baseline cache source / compare ref resolution
  - when a compare was requested but no baseline cache was available
  - drill-down links currently target bundle artifacts unless a per-run report
    already exists

### Active implementation focus

1. Keep growing the controller kit rather than letting built-ins drift apart:
   - richer helper queries over terrain and target frame
   - shared metric/marker conventions across controller styles
   - small reusable guidance utilities instead of copy-pasted heuristics
2. Use the new bot-lab suite to tighten controller iteration:
   - compare baseline vs staged behavior on the same curated families
   - rank which failures are controller issues vs corpus issues
   - decide which scenario families deserve first-class promotion later
3. Tie batch and run reports together more tightly:
   - stronger drill-down from compare rows into run pages
   - more stable labeling of current vs baseline review links
4. Keep improving report meaning before report polish:
   - better representative-run selection
   - clearer "open this next" guidance

### Known limitations

- `pd-eval` now has a real batch report page, but the current version is still
  table-first and intentionally minimal compared with the older `pylander`
  report stack.
- Scenario-family perturbations currently cover only a small set of numeric
  fields; terrain mutation and richer family grammars are still future work.
- Batch identity digests exist, but there is not yet a cache or compare layer
  that actively uses them.
- Replay remains authoritative on scenario plus action/event logs; controller
  telemetry and sampled traces are still carried-through inspection data.
- Batch compare currently matches runs by shared `run_id`. That is the right
  first compare basis, but it is not yet a richer “same resolved scenario under
  different candidate/controller configs” matcher.
- `pd-eval` still does not generate polished per-run drill-down pages itself, so
  batch links currently fall back to bundle artifacts when no `report.html`
  exists for that run.
- The controller kit now exists, but it is still centered on the current
  terminal-descent problem. It has not yet been pressure-tested against richer
  transfer guidance or terrain-reactive scenarios.

#### Checkpoint 16: report-site structure and indexes

- Split stable HTML navigation away from raw artifacts by introducing:
  - `outputs/reports/index.html`
  - `outputs/reports/runs/`
  - `outputs/reports/replays/`
  - `outputs/reports/eval/`
- `pd-cli report` now defaults to the report-site path for bundles rather than
  writing back into the bundle directory.
- `pd-cli` single-run and replay bundle writes now also emit report-site copies
  automatically when the bundle lives under `outputs/`.
- `pd-eval` batch report generation now also emits report-site copies under:
  - `outputs/reports/eval/<pack>/index.html`
- Added simple generated index pages for the report-site home and each top-level
  scope, sorted newest-first.
- The server workflow now points to the root landing page first, with the
  report-only subtree still available under `/reports/`.
- Added a generated root landing page at `outputs/index.html` with links to:
  - the report site
  - raw artifact directories
  - latest run and latest batch pages
- Follow-up fixes tightened the navigation contract:
  - newest-first ordering now keys off the generated report file timestamp
    instead of the directory timestamp, so regenerating a report actually moves
    it to the top
  - nested report collections such as `outputs/reports/eval/<pack>/runs/` now
    get their own `index.html` page instead of dropping back to a raw directory
    listing
  - batch report drill-down links now prefer report-site detail pages when they
    exist, keeping navigation inside `/reports/`

#### Checkpoint 17: run summaries and compare-first batch triage

- Added structured per-run summary metrics to `RunManifest.summary` and bumped
  the run artifact schema to `2`.
- The run summary now carries:
  - fuel used and fuel remaining
  - minimum touchdown and hull clearance
  - max speed, attitude magnitude, and angular-rate magnitude
  - mission-envelope margin ratios
  - landing-specific terminal metrics:
    - pad offset
    - normal and tangential touchdown speed
    - attitude and angular-rate touchdown margins
  - checkpoint-specific terminal metrics:
    - position, velocity, and attitude error
    - checkpoint-envelope margins
- Reworked batch summaries around review value instead of only flat inventories:
  - closest current failures
  - worst failures
  - weakest successes
  - lowest-fuel successes
  - mean remaining fuel for successful groups
- Reworked compare ordering to rank shared runs by envelope-margin regression
  first, then by fuel and timing deltas.
- Borrowed the useful lesson from the late `pylander` benchmarking flow:
  the first question is usually not "show me every run", it is:
  - what regressed
  - what is closest to the edge
  - which run should I open next
- Updated the batch HTML page to surface that directly:
  - `Current Edge` cards on the overview
  - compare-time `Priority Review`
  - standalone `Closest Current Failures`
  - `Representative Successes`
  - failure inventories and slowest runs pushed into secondary `details`
    sections
- Kept the new run summaries and compare surfaces aligned with the report-site
  tree, so report navigation and batch triage now reinforce each other.

#### Checkpoint 18: controller kit and curated bot-lab corpus

- Split `pd-control` into a clearer bot-framework shape:
  - shared controller view/helpers in `pd-control/src/kit.rs`
  - concrete built-in controllers in `pd-control/src/controllers.rs`
- Added a reusable `ControllerView` over `RunContext + Observation` with:
  - target-relative position helpers
  - pad-margin and fuel-fraction helpers
  - target-surface normal/tangent kinematics
  - hover-throttle and vertical-target helper calculations
  - direct terrain sampling/profile helpers for controller code
- Added a small shared diagnostics convention layer:
  - common phase labels
  - common metric keys
  - standard phase/gate marker helpers
  - a `ControllerFrameBuilder` so built-ins stop hand-assembling frames
- Refactored `baseline_v1` onto the shared controller kit.
- Added a second built-in controller, `staged_descent_v1`, with:
  - explicit `translate -> descent -> flare -> touchdown` staging
  - standardized phase and gate markers
  - distinct behavior from the original baseline instead of a renamed clone
- Added a curated pack for controller iteration:
  - `fixtures/packs/terminal_bot_lab_suite.json`
- The new pack intentionally exercises both built-in controllers over:
  - nominal terminal runs
  - crossrange / attitude-biased terminal runs
  - low-margin / low-fuel terminal runs
  - one checkpoint reference
- This is the first point where `pd-lab` feels closer to a real bot framework
  rather than just a deterministic sim with batch tooling:
  - controllers now have a reusable helper layer
  - multiple controller styles share the same lab contract
  - the corpus is shaped for controller work, not only for smoke testing

#### Checkpoint 19: terminal-guidance selector realignment

- Reviewed the current terminal bot-lab pack against the old `pylander`
  selector model and confirmed the deeper issue was not only report layout:
  the corpus was still too flat:
  - one coarse `family`
  - one overloaded `entry_id`
  - one leaf `seed`
- Locked the intended terminal-guidance selector model into the architecture:
  - hierarchy axes:
    - `mission`
    - `arrival_family`
    - `condition_set`
    - `vehicle_variant`
  - matrix axes:
    - `arc_point`
    - `velocity_band`
  - leaf variation:
    - `seed`
  - controller lane:
    - separate from the physical case
- Documented the current implementation stance clearly:
  - current packs still approximate that model with `family + entry + seed`
  - pack metadata should carry explicit selector fields even before the report
    layer grows first-class matrix support
- Simplified the active terminal bot-lab corpus to reduce noise while that
  selector work is still in progress:
  - removed the dedicated `crossrange` family
  - kept clean nominal and low-margin terminal cases
  - recorded expectation tiers so not every case is implicitly treated as
    "must be solved"
- The next structural step is no longer "add more families". It is:
  - teach `pd-eval` about explicit terminal selector coordinates
  - replace ad hoc seeded arrival perturbations with a denser
    `arc_point x velocity_band` matrix

#### Checkpoint 20: suite simplification around terminal bot lab

- Recentered the day-to-day evaluation workflow on:
  - `fixtures/packs/terminal_bot_lab_suite.json`
- Demoted the old sweep/regression packs from primary examples to explicit
  compare smoke fixtures:
  - `fixtures/packs/terminal_compare_baseline_fixture.json`
  - `fixtures/packs/terminal_compare_regression_fixture.json`
- Updated the README batch-eval examples so the main example points at the
  bot-lab suite, while external pack-vs-pack compare is shown only through the
  fixture pair.
- Cleaned up generated outputs so the report index no longer surfaces the old
  sweep suite names as if they were first-class review surfaces.

#### Checkpoint 21: batch report provenance and compare context

- Added an explicit `Context` section near the top of batch reports so the page
  says what kind of report it is before the overview/tree:
  - `standalone`
  - `lane compare`
  - `external compare`
- The context table now surfaces:
  - current source
  - baseline source
  - baseline resolution
  - compare basis
  - scope resolution
  - compare status
- External compare pages now distinguish:
  - exact compare
  - shared-intersection compare
  - unavailable compare
- Internal bot-lab pages now say clearly that the comparison is lane-based
  within one pack, rather than an external baseline compare.
- The page also states a current limitation explicitly:
  - cached result reuse / promotion / invalidation is not modeled in `pd-lab`
    yet, so the report does not pretend to show it.
