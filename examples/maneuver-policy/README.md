# Deterministic maneuver composition

[Implementation plan](../../docs/maneuver_controller_implementation_plan.md) · [Controller API](../../pd-control/src/maneuver/mod.rs) · [Evaluation workflow](../../docs/evaluation.md)

`pd-maneuver-example` is the experimental planner/controller composition.
An immediate, never or fixed-tick program chooses when to request landing.
The reusable `TerminalLandingController` owns the request lifecycle and delegates
the entire ordinary landing implementation to the unchanged terminal backend.
Native `Episode` owns every physical transition, clock, fuel budget and end.

## Supported inputs

The initial capability is airborne, route-free `LandingOnPad` on the mission's
fixed target. Physics is 120 Hz, commands 60 Hz and planner opportunities 10 Hz,
beginning at tick zero. Earth gravity is exactly 9.81. The vehicle uses the
tracked flat-terminal fixture's geometry, engine, servo and contact limits:
8 × 10 m hull, 4 m half span, 5 m base offset, 240 kN thrust, 49.5 kg/s fuel burn,
0.25 minimum throttle, pi/2 rad/s rotation, touchdown limits 3/2 m/s,
0.15 rad and 0.35 rad/s. Valid mass/fuel, airborne state, terrain, pad geometry
and deadline may vary. The application caps the declared horizon at 600 seconds.

These are semantic bounds. They do not certify a landing envelope. Positive
near-empty fuel, ascending and below-target airborne starts remain attempts.
The native scenario validator rejects zero initial fuel; fuel exhausted during
execution remains native state and does not revoke admission. Unsupported whole
inputs return an error before creating a flight capture. Cancellation,
retargeting, preemption, arbitrary arrival and durable resume are unsupported.

## Run one explicit software example

Use a new destination whose parent already exists. These commands execute one
native episode and explicit proof passes, and create no campaign or publication.
They are examples to invoke deliberately, not a scheduled characterization.

```sh
rtk proxy cargo run -p pd-maneuver-example -- run \
  --scenario fixtures/scenarios/flat_terminal_descent.json \
  --program examples/maneuver-policy/programs/immediate.json \
  --output /tmp/new-maneuver-bundle
rtk proxy cargo run -p pd-maneuver-example -- verify /tmp/new-maneuver-bundle
rtk proxy cargo run -p pd-maneuver-example -- report /tmp/new-maneuver-bundle \
  --output /tmp/new-maneuver-report.html
```

The supplied [never](programs/never.json) and [fixed-tick](programs/fixed-tick.json)
programs use the same schema. Fixed ticks are absolute multiples of 12 physics
steps. Off-grid ticks fail validation; a tick after native termination never
fires. Before commitment, coast means engine off with the upright attitude servo
still active. Activation resets the backend once and generates its command on
that same boundary. After acceptance the planner is no longer invoked.
The facade preserves raw native engine settings and attitude commands.

## Evidence and verification

A new bundle contains `capture/evaluation.json`, `capture/seal.json`, `trace.json`
and a final `receipt.json`. The neutral capture/seal schemas are unchanged.
The receipt binds exact capture/seal/trace bytes, descriptor, physical identity
and original input-file identities. An interrupted write leaves incomplete new
evidence. Retry requires another destination; it never fills a prior bundle.
Verification and input loading require bounded regular files without symlinks.

Verification reports three checks independently, plus companion byte integrity:

1. Neutral saved-command replay reconstructs the physical episode.
2. A fresh frozen controller consumes recorded requests at native observations;
   admissions, commands, authority, final lifecycle and counters must agree.
3. A separate fresh pass reconstructs planner opportunities and decisions from
   actual native observations and recomputed feedback, including no calls after
   commitment. It does not trust recorded admissions or authority.

Source identity covers participating package manifests/source, the workspace
manifest/lockfile and the application build script. Build-time source hashes
must match runtime snapshots; before/after snapshots also bind the executable.
Archives, unrelated docs and generated destinations are outside the scope.
Reconstruction requires the same executable and compiled source identities at
their bound paths. A changed build can still perform independent physical replay.
This is a local reproducibility contract, not a cryptographic signature or a
portable cross-build controller checkpoint.

The neutral untagged telemetry decoder reads integer-valued metrics as floats.
Saved-frame comparison therefore uses the existing encode/decode projection on
both sides, while checking commands directly and exactly. Values, markers,
phases and statuses still compare; no timing field enters behavior comparison.
The library parity panel compares complete in-memory frames/artifacts exactly.

`preparation_resources` counts assessment and save validation (two neutral
replays), plus both decision reconstructions. The `run` command then performs
composite verification, whose printed resources count one additional neutral
replay and both reconstructions. Each later `verify` or `report` also performs
these three passes. Backend updates, planner calls and physical steps are
counted; an errored neutral API with no exposed step count is explicitly marked
unavailable. Wall timing is recorded; thread CPU time is unavailable here.

The report command uses the maintained rich ordinary-run template. Its landing
claim remains the neutral physical/mission/integrity/replay tuple. Companion
verification results are printed separately; rendering never promotes a selector
or starts a server. A valid complete proof may describe a failed landing.

## Prepare an input-only evaluation

After [exporting native terminal inputs](../../docs/native_scenario_inputs.md),
prepare a new schedule without controller updates, physics steps or forecasts:

```sh
rtk proxy cargo run -p pd-maneuver-example -- prepare-evaluation \
  --recipe examples/maneuver-policy/recipes/retained-input-audit.json \
  --inputs /tmp/new-terminal-input-set --output /tmp/new-controller-plan
rtk proxy cargo run -p pd-maneuver-example -- check-evaluation /tmp/new-controller-plan
```

Repeat `--inputs` to include additional explicit sets. The supplied recipe selects
seed zero from every supplied entry. `recipe.json`, full native scenario/program
files, `plan.json`, `audit.json` and a final `completion.json` retain the schedule
and exact identities. Original expectation labels stay source provenance; new
roles do not select a controller. Existing destinations are never filled.

The audit separates roles, semantic support, physical aliases and parent groups;
records marginal ranges and declared joint-bin counts; bounds original and proof
steps; and checks supplied exposure inventories. Missing historical exposure
coverage stays unknown. These commands establish inputs and scheduling, not a
landing rate. Structured/local recipes use the versioned typed
[recipe model](src/evaluation/recipe.rs), including joint state correlations,
explicit transforms, diagnostic overrides and separate independent cohort streams.
Sampler `local-joint-sampler-v2` places terrain and obstacles relative to the pad
surface. The input audit requires every non-diagnostic local pad to agree with
its supporting terrain within 1e-9 m over the complete pad interval, checking
both endpoints and all interior heightfield vertices. Deliberately inconsistent
diagnostics remain separate model-integrity probes; native input acceptance and
contact classification are unchanged. Retain the exact source epoch when
reproducing older sampler inputs.

## Execute and inspect a frozen evaluation

```sh
rtk proxy cargo run --release -p pd-maneuver-example -- run-evaluation \
  --plan /tmp/new-controller-plan --output /tmp/new-controller-results
rtk proxy cargo run --release -p pd-maneuver-example -- check-evaluation-results \
  --plan /tmp/new-controller-plan --results /tmp/new-controller-results
```

The runner starts one supervised attempt subprocess at a time. Each executed
attempt retains its ordinary bundle, neutral outcomes, companion proof,
activation/cost counters and exact outer-plan/input identities. Direct parity
uses the same native Episode and ordinary backend on both sides, comparing full
artifacts and frames with only compute timing excluded. A verified failed
landing remains a completed scientific attempt. Invalid/unsupported diagnostics
retain their input disposition without a fabricated capture.

Every scheduled slot gets a create-only result row. Wall-time interruption,
storage exhaustion or proof/parity failure keeps incomplete evidence and marks
the remaining slots not started. The complete row inventory and summary are
sealed even when the phase is incomplete. `check-evaluation-results` validates
byte bindings and aggregate accounting without replay; each saved attempt's
original proof remains available through ordinary bundle verification.

For a supervisor interruption, use a new destination and
`run-evaluation --plan PLAN --output NEW --continue-from OLD`. Only exactly
matching plan/source paths/executable/configuration can reuse completed attempts.
Their native outcomes stay unchanged and re-verification is supervised and
counted. Interrupted attempts cannot be replaced by a lucky rerun; partial
bundles are never filled. Preserve the measured source epoch at its bound paths.

Independent recipes require a frozen confirmation criterion. A complete fixed
sample receives an exact one-sided binomial lower bound; an incomplete sample
receives no passing reliability verdict. Internal forecast work and unavailable
CPU timings stay unavailable. Report rendering is a separate explicit command
with its own three proof passes; batch execution changes no report selectors.

## Software checks and next boundary

```sh
rtk proxy cargo test -p pd-control
rtk proxy cargo test -p pd-maneuver-example
rtk proxy node scripts/check-rl-retirement.mjs
rtk proxy node scripts/check-planner-development.mjs
```

The library also offers summary mode without trace/action/event/sample/update
vectors. It retains native summaries and counters, and cannot manufacture a
verified bundle later. The next measurement task is a separately declared cost
pilot and activation/envelope characterization, including full terminal packs.
This software pass performs no training and establishes no general landing rate.
