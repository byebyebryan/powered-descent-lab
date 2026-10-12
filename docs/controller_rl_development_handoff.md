# Independent controller and RL development handoff

[Preparation results](controller_rl_preparation_results.md) ·
[Architecture](architecture.md) · [Development workflow](development.md)

October 12, 2026. Shared preparation is complete. Both worktrees start at
`51c143c`, with separate installed environments/builds and verified input copies.
The [readiness index](../studies/controller_rl_preparation/readiness.json) binds
their actual paths, commits, native/source/ABI receipts and verification.

## Common contract

Use complete source-pad-rest to target-pad landing missions on the sealed 4 m
physical terrain, nominal Earth plant, 6,300 kg initial fuel and original 90 s
horizon. Native physics runs at 120 Hz and commands at 60 Hz. Approach-owned
decision cadence, features, actions, launch primitives and rewards must be
declared. Authored waypoint phases are approach diagnostics, not task acceptance.

Count a landing only with successful physical target contact, mission success,
valid execution, stable input/source integrity, complete native replay and a
mission-terminal stop. Keep every scheduled unsupported/failure/finite-stop
disposition in its denominator. Keep backend rates separate; broad/stress and
development/validation/final-test are different comparison views.

## Ownership and isolation

| Owner | Responsibility |
| --- | --- |
| Shared platform | `pd-core`, `pd-run`, `pd-missions`, `pd-eval`, `pd-python`, `pd-report`, shared schemas/workflows |
| Controller branch | `pd-control`, `pd-plan`, `pd-planner`, `pd-benchmark`, existing CLI composition and controller experiments |
| RL branch | `experiments/rl`, its independent agent, sampling/curriculum, features/actions/rewards, optimizer/checkpoints |

Shared changes require a reviewed platform commit consumed by both branches.
Keep each worktree's Cargo target directory, Python environment/native module and
new outputs separate. Use `experiments/rl/build_native.py` after `uv sync --locked`
in that environment to create a fresh path-bound native/source/ABI receipt.
Historical environments, captures and receipts are immutable evidence.

Prepared worktree paths are `../powered-descent-lab-controller-v2` and
`../powered-descent-lab-rl-v2`; branch names are `dev/controller-v2-20261012` and
`dev/rl-v2-20261012`. Neither starts optimization or controller tuning merely by
being created.

From either worktree, its environment is `experiments/rl/.venv`, Cargo artifacts
are in its own `target/`, and local assets/receipts are in
`outputs/shared-preparation/`. The controller worktree also has its release
`pd-benchmark` executable. Both installed bridges fully verified their local
validation/stress collections and passed all 14 Python tests. No branch-specific
algorithm allocation has started.

## Input assets

The [cohort freeze](../studies/controller_rl_preparation/cohort-freeze.json)
binds new validation/stress manifests and all proposal/exclusion accounting.
Each worktree receives exact immutable validation/stress copies with checked
hashes and a new append-only usage journal. Every usage/output ID needs its
worktree identity; reconcile exposure before any final evaluation.

The 3,311 fixed development regression inputs retain the original shared pilot
paths and original manifests. Use the exact prefix schedules from the retained
[baseline](procedural_challenge_baseline.md); the remaining inspected pilot
inputs are additional development candidates without inherited flight scores.
These physical assets are shared and immutable. Append-only usage receipts may
be added with distinct branch IDs; do not edit requests, manifests or proposals.

Final-test collections remain reserved centrally and are **not copied into the
worktrees**. No input preview, policy execution or checkpoint-selection access
has occurred. Their unseen-input claim is limited to the declared exposure
audit; both terrain backends are already known. Final release needs a separate
predeclared candidate-selection and evaluation contract.

Initial stress has 50 unique inputs and sparse quotas. It supports explicit
diagnostics, not complete interaction coverage. Do not refill it, relax its
rules or silently pool it into broad rates after inspecting outcomes.

## First branch decisions

Before launching an algorithm allocation, each branch must record its hypothesis,
budget, sampling/selection rule, failure accounting and final-test release rule.

- Controller: state a general mechanism to improve, retain the unchanged
  reflected cap-24 reference, use development for diagnosis and frozen validation
  for candidate comparisons. Individual seed repairs do not establish progress.
- RL: declare V2 training/curriculum streams outside reserved namespaces and
  decide features/actions/rewards/control representation under its experiment
  contract. Historical V1 aliases and training defaults remain intentionally
  unchanged; do not launch the old `train.py` defaults as a new V2 experiment.

Preparation supplies an untrained constant-action integration policy, not a new
trained RL controller. The measured incumbent baseline remains 77.0% layered /
77.4% value on 3,311 exposed development inputs. The 16-input smoke has no
qualification meaning and no training transitions were allocated.
