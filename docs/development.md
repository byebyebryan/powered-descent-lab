# Development workflow

[Documentation home](README.md) · [Evaluation](evaluation.md) · [Reports](reports.md)

Run commands from the repository root. Start with the root README, current
[guidance](guidance.md#current-v2-design-and-support), and roadmap status before
using a dated protocol as an implementation plan.

## Prerequisites

- Rust and Cargo supporting the workspace's Rust 2024 edition. No minimum Rust
  version is declared or tested separately from the local toolchain.
- Node.js with ES modules, `node:test` and `node:util.parseArgs`; maintained
  scripts use built-in modules and need no npm installation. Browser checks also
  require a runtime with global `WebSocket` and an explicitly configured local
  Chrome debugging endpoint.
- Git and RTK: the maintained gate invokes commands through `rtk proxy`.
- Python 3.12 or newer with a linkable Python library for the workspace's PyO3
  bridge. The closed learning experiment retains its Python 3.12 environment
  and pinned CPU dependencies; see its [archive](../studies/rl_procedural_ppo/README.md).
  It is not a prerequisite or training recipe for a new approach.
- Python 3 and tmux for the optional report server. Regenerating terminal-suite
  illustrations uses uv and Python 3.11 or newer, and rewrites tracked SVG assets
  as well as a local HTML preview; it is not needed for Rust tests.

This documentation pass was checked with Rust 1.99.0 and Node 26.10.0. These are
observed local versions, not newly declared minimums or portable-support claims.

## Build and validate

```sh
rtk proxy cargo build --workspace
rtk proxy node scripts/check-planner-development.mjs
```

The ordinary gate runs workspace all-feature tests, feature-enabled and
default-off CLI checks, help/dependency boundary checks, formatting, diff
whitespace, strict all-target Clippy, local documentation links and all maintained
JavaScript tests. It creates no accepted capture or publication and does not
require local historical outputs.

For common report presentation work, see the
[saved campaign presentation workflow](report_presentations.md). Source tests are
part of the ordinary gate; real browser acceptance uses a supplied edition and
disposable browser separately.

For focused documentation work:

```sh
rtk proxy node scripts/check-docs.mjs
rtk proxy node --test scripts/test-check-docs.mjs scripts/test-check-planner-development.mjs
rtk proxy git diff --check
```

The link check validates inline repository-local Markdown links and images,
ATX-heading anchors and explicit HTML IDs. It skips fenced/inline code, HTML
comments, external URLs and generated `outputs/` evidence, which is intentionally
absent from fresh checkouts. It is a small repository check, not a full Markdown
parser or external-link checker; reference-style links are outside its coverage.
It does not run historical commands or publish pages.

The retired PPO package has an output-free source/entrypoint check:

```sh
rtk proxy node scripts/check-rl-retirement.mjs
```

Its tests run through the maintained Node-test discovery. The optional
`--research-worktree PATH` also checks the separately retained source inventory;
ordinary validation does not require that worktree or an installed learner.

For changes to the optional procedural-terrain tooling, also run its pure and
synthetic tests. These do not collect measured flights or require saved captures:

```sh
rtk proxy python3 -B -m unittest discover -s studies/terrain_profiles -p 'test_*.py'
rtk proxy python3 -B -m unittest discover -s studies/mission_coverage -p 'test_*.py'
```

Native procedural geometry and generation checks are part of the workspace gate;
V2 [admission/collection checks](procedural_collections_v2.md) cover analytical
challenge paths, accounting, label-independent membership, quotas, exposure,
Python reads and evaluator binding of excluded proposals.
`rtk proxy cargo test -p pd-missions` is the focused check. See the
[neutral geometry contract](procedural_geometry.md) for measurement definitions
and the [implementation loop](procedural_challenge_execution.md) for input-pilot scope.
`rtk proxy cargo test -p pd-python` includes actual Python-call generation and
native/Python episode parity for the [V2 proposal frontdoors](procedural_generation_v2.md).
It does not install a bridge into the RL environment or allocate training.

The [native input audit](procedural_input_audit.md) has ten statistical,
selection, coordinate and saved-input checks. The coverage study's Python checks
also validate the bounded runner's declaration, scope and create-only boundaries.
The [input report](procedural_input_audit.md#maintained-local-input-report) adds
three native projection/plot checks and three Node pagination/filter checks.
Its explicit browser checker reads local files without starting a report server.

## Optional retained flight regression

Only when local accepted evidence is available and flight-affecting changes
justify the check:

```sh
rtk proxy node scripts/check-planner-development.mjs \
  --parity-capture outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native
```

This executes all 44 frozen inputs locally without writing/publishing captures.
Complete flight records must match the validated saved baseline, excluding only
`planning_s`, `execution_s` and `replay_s`. It is regression evidence, not a new
accepted flight campaign. Keep diagnostic outcomes separate from landings.

This explicitly identifies the later local pre-candidate checkpoint; it does
not relabel or publish it as the accepted site. The published October-5 capture
predates a [documented numerical geometry repair](ballistic_feedback_results.md)
and differs by 8.88e-16 m in its first clearance scalar. Do not weaken the exact
comparator or silently substitute that source when claiming preservation.

## Optional CLI boundary

Ordinary `pd-cli` keeps its existing controllers and defaults. V2 commands are
available only with `--features planner-v2`:

```sh
rtk proxy cargo run -p pd-cli --no-default-features -- --help
rtk proxy cargo run -p pd-cli --features planner-v2 -- --help
```

The default-off normal dependency tree must not include evaluation or planning
packages. The independent platform crates (`pd-run`, `pd-missions`, `pd-eval`,
`pd-report` and `pd-python`) must not import concrete strategies, including with all features.
The gate enforces both boundaries and prevents `pd-planner` from importing
evaluation, rendering or collection management. The accepted
V2 adapter is synchronous session execution, not externally driven per-tick
control or arbitrary airborne snapshot restart.

## Repository hygiene and change boundaries

- [Fixture ownership](../fixtures/README.md) distinguishes current inputs from
  deliberately retained archive metadata; [maintained tooling](../scripts/README.md)
  distinguishes checks from capture, publication and browser operations.
- `outputs/`, `target/` and the old local `tmp_flat_debug/` scratch directory are
  ignored. Do not commit generated captures, caches, executables or report pages.
- Ignored does not mean disposable: accepted captures, selectors, receipt files
  and published pages under `outputs/` may be irreplaceable local evidence. Do
  not use broad cleanup commands over them.
- Keep frozen pack/scenario/research inputs and historical identities intact.
  Retired packs can remain readable archive metadata while execution rejects them.
- Use [current ownership](architecture.md) for source changes. Preserve command
  ordering, state/fuel/clocks, proof boundaries, serialized models and report
  templates during structural cleanup. New tuning or coverage needs a separate
  stated goal and evidence gate.
- Keep workflow instructions here and in evaluation/report pages; add dated
  evidence to result records without rewriting previous measurements. The root
  README is an overview, not another full checkpoint ledger.
- Review the actual diff and run proportionate checks before a requested commit.
  Pushes, report refresh/publication and server lifecycle changes are separate
  actions, not side effects of documentation validation.

Linked worktrees can contain independent uncommitted experiments. Inspect their
status and ownership before removal; stale registrations for missing temporary
directories are separate from existing worktrees. Do not turn repository hygiene
into blanket `git clean`, worktree pruning or deletion of ignored evidence.
Licensing and external CI are owner decisions, not inferred housekeeping changes.

## Adding an approach

Use the open `pd-run::Agent` interface or own a `pd-run::Episode` reset/step
loop. Keep observation/action mapping, reward, training and checkpoint handling
in the approach package. Declare procedural recipes and input collections with
`pd-missions`, then pass an agent factory and explicit evidence scope to neutral
`pd-eval`. Its saved-command assessment and `pd-report` render the captured
records. No edit to the built-in controller registry or maintained planner is
needed. The runnable [checkpoint example](../examples/checkpoint-policy/README.md)
and [independent-agent workflow](independent_agents.md) cover this path.

Concrete solver maintenance belongs in [pd-planner](../pd-planner/README.md),
feedback in `pd-control`, pure calculations in `pd-plan`, and CLI/capture/site
composition in `pd-eval-cli`. Existing application exports are compatibility
adapters; add behavior to its implementation owner. Current Cargo selectors use
`-p pd-eval-cli` for the installed `pd-eval` binary. Frozen copied-source builds
select the package present in their own source tree, and saved receipts keep the
original build command and identity.

The experimental [maneuver example](../examples/maneuver-policy/README.md) owns
a native Episode and deterministic activation program above `pd-control`'s
landing facade. It is a composition package; neutral platform crates and the
checkpoint example remain independent of concrete strategies. Its build-time
source ledger must match execution/reconstruction snapshots, so rebuild after
editing participating source. It requires no archived PPO environment or runs.
