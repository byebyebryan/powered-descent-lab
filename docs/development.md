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

For changes to the optional procedural-terrain tooling, also run its pure and
synthetic tests. These do not collect measured flights or require saved captures:

```sh
rtk proxy python3 -B -m unittest discover -s studies/terrain_profiles -p 'test_*.py'
```

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

The default-off normal dependency tree must not include `pd-eval`. The accepted
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
