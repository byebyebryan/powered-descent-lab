# Independent procedural PPO experiment

[Execution contract](../../docs/procedural_learning_plan.md) ·
[Mission inputs](../../docs/procedural_missions.md)

This package owns RL features, action mapping, rewards, sampling, normalization,
optimization and checkpoint selection. `pd-python` is a thin binding to native
`pd-run` episodes and `pd-eval` capture/replay. Neither layer imports the current
controller or planner. Core contact, fuel and mission horizons are authoritative.

## Reproduce locally

The checked-in `uv.lock` pins Python dependencies; the project uses Python 3.12
and CPU PyTorch. The Rust bridge pins PyO3 0.29.3. From the repository root:

```sh
rtk proxy uv sync --project experiments/rl --python 3.12 --locked
rtk proxy uv run --project experiments/rl maturin develop --release --manifest-path pd-python/Cargo.toml
rtk proxy uv run --project experiments/rl python experiments/rl/test_environment.py
rtk proxy uv run --project experiments/rl python experiments/rl/train.py --output outputs/my-training-seed --cohorts outputs/my-cohorts --seed 801 --steps 8000000 --max-seconds 7200
rtk proxy uv run --project experiments/rl python -m unittest discover -s experiments/rl -p 'test_*.py'
```

The cohort root contains `value-noise-development` and `layered-development`.
Each is an immutable native collection. Training streams the declared families
using seeds 501101 and 501201, starting at index 256. A worker advances its own
index by worker count; workers alternate families. Proposal receipts, physical
identities, structural rejections and terminal outcomes are logged per worker.
No solver outcome filters the sampler. Run directories and result files are new.

## Physical and learning contracts

The bridge's raw observation order is time, physics step, x, y, vx, vy, attitude,
angular rate, mass, fuel, gravity, target dx, height above target, target surface,
target half-width, touchdown clearance and hull clearance. `physical_json()`
contains physical clocks, vehicle, initial state, mission target and terrain
domain. Terrain queries use the authoritative piecewise-linear heightfield.
Labels, seeds, tags and generator membership do not reach policy features.

The policy sees 64 features: 16 explicit normalized state/target/resource values,
24 local terrain heights from -240 to 480 metres along the mission direction,
and 24 terrain heights between the current position and target. Fixed scales
and clipping are defined in `pd_env.py`; no running normalization state exists.
Direction is computed from physical pad displacement. Mirroring features and
the attitude action is a globally defined transformation in this approach.

Two normalized actions map to throttle [0,1] and target attitude [-0.8,0.8]
radians along the mission direction. Each policy action is held for six native
60 Hz command boundaries: policy decisions are 10 Hz, physical steps remain
120 Hz and captured commands remain 60 Hz. During the first 90 physics steps,
the approach uses upright full thrust. This explicit 0.75-second launch primitive
uses the original fuel and time budget; it is not a platform requirement or an
incumbent fallback. An unconstrained 1,024-action exploration pilot terminated
every trial on the first physics step, motivating this global experiment choice.

Reward is 100 times reduction in normalized Euclidean distance to the target
touchdown center, minus 2 times consumed initial-fuel fraction and 0.01 per
elapsed second. Native mission success adds 100; crash adds -20 and other native
terminal outcomes add -15. Reward components and physical outcomes are recorded
separately. Reward cannot establish a landing or change the native stopping rule.
The original finite mission horizon is an intrinsic task terminal; there is no
collector timeout, snapshot reset, fabricated truncation or hidden autoreset.
Stable-Baselines3's vector wrapper explicitly resets completed episodes.

`--design v2` selects the declared kinetic/attitude potential in `pd_env_v2.py`.
It changes reward only and starts a fresh optimizer. The source and native
module are bound before and after each allocation. Preserve their bytes until
the allocation finishes; commit changes to unrelated components independently.

PPO uses separate 128-by-128 actor/critic MLPs, 512 steps per environment,
512-sample minibatches, five epochs, learning rate 0.0003, gamma 0.999, GAE 0.95,
clip 0.2, entropy 0.003 and initial log standard deviation -0.7. Runs persist
the exact configuration, source/native-module hashes, seed, transition count,
wall allocation, checkpoints and development attempts. The maintained Rust
gate checks that `pd-python` has no normal dependency on incumbent strategies.

## Evidence and selection

Recurring development evaluates exactly 64 inputs from each family. Checkpoint
ordering is native summary success count, median final target distance, then
earlier checkpoint. These summaries are selection evidence without saved-command
proof. Final claims require separate frozen-checkpoint captures and exact native
replay. All scheduled failures remain in denominators. The held-out collection
is not opened by training or checkpoint selection.

The existing `target_distance_m` summary is absolute horizontal target offset;
it does not include vertical separation. This remains the declared secondary
ordering for the current experiment. Native landing proof covers contact speed,
attitude, footprint and mission outcome independently of that selection metric.

After all declared allocations finish, `freeze.py --run RUN --output NEW_FREEZE`
checks the three seeds and checkpoint bytes, compares the selected development
panels and copies the winning checkpoints with training receipts. Use
`confirm.py --freeze FREEZE_JSON --cohorts COHORT_ROOT --output NEW_CONFIRMATION`
for the broad development and validation summary confirmations. It never
re-ranks checkpoints. `analyze.py` audits consumed training identities against
reserved cohorts and keeps repeated exposure separate from total unique inputs.

`NativeEpisode.save_capture()` uses the common evaluation schema, immutable seal
and rich report template. Composition supplies before/after evidence bindings,
including the actual loaded native module, policy source, checkpoint and selected
mission bytes. Replay runs commands through core without loading PPO. Summary
mode retains no action/event/sample/update arrays and cannot later invent proof.

`evaluate.py --collection COLLECTION --checkpoint CHECKPOINT --output NEW_DIRECTORY
--native-receipt BUILD_RECEIPT --design v1` evaluates a frozen policy on original
resolved native inputs. The build receipt binds the actual installed extension
and retained compiled source bytes. Installed learning-library code and binary
files are bound across the campaign, while every episode binds its own input,
policy source and checkpoint. Policy loading is inside that evidence boundary.

Every scheduled input receives a disposition, including invalid input,
initialization, execution and capture failures. A failed bulk input read is
subdivided to isolate bad inputs while preserving the order and denominator.
Captured attempts receive independent native saved-command verification and
rich individual reports. The aggregate uses the common `pd-report` campaign
projection and requires campaign integrity before counting verified landings.
Its report exporter must be built with
`rtk proxy cargo build --release -p pd-report --example campaign`.
These commands create local evidence and never publish it.

`verify.py --campaign CAMPAIGN --output NEW_VERIFICATION_JSON` independently
replays saved native commands and checks their projection against the campaign
schedule and descriptor. It imports no learning library and loads no checkpoint.
Supply `--campaign` repeatedly to verify the full paired inventory in a fresh
process, including the incumbent captures. Unsupported inputs remain accounted
for; any capture or campaign integrity failure rejects the final inventory.

