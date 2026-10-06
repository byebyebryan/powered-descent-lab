# Repository guidance

## Orientation

- Read `README.md`, `docs/README.md`, current `docs/guidance.md` and the roadmap's
  current-status/next-product sections. Check Git state before editing.
- Historical protocols and dated "next" steps are not an active task list.
  Policy 3 is the sole executable planner; older data readers are compatibility.
- Use `rtk` for shell commands (`rtk proxy` for an unfiltered command).
- Keep investigations read-only until a change is requested. Do not create a
  goal, delegate, commit, push or publish merely because an old protocol did so.

## Protected boundaries

- Keep planner/controller behavior, command ordering, state/fuel/clocks,
  create-only outputs and proof/replay boundaries unchanged in housekeeping.
- Do not change frozen fixture inputs or rewrite historical outcomes to make a
  check pass. Preserve captures, report pages, selectors and receipts.
- `outputs/` is ignored generated data, but may contain retained evidence. Do not
  delete it or run broad cleanup commands over the workspace.
- Report refresh/publication and server start/stop are state-changing operations;
  documentation and code review do not authorize them.
- Preserve rich common report templates and navigation. Extend maintained views
  when data is missing; do not substitute a stripped-down report.
- Preserve unrelated worktree changes. Workers, when explicitly requested, need
  owned files/responsibilities and must not revert another writer's work.

## Validation and documentation

- The maintained gate is `rtk proxy node scripts/check-planner-development.mjs`.
  It covers workspace/CLI boundaries, formatting, Clippy, docs links and Node tests.
- Retained 44-case numerical parity is explicit opt-in through `--parity-capture`;
  do not require locally saved evidence for ordinary tests or relabel its source.
- For docs-only changes, run `scripts/check-docs.mjs`, relevant Node tests and
  `git diff --check`; no flight campaign or report regeneration is needed.
- Keep current workflow instructions in `docs/development.md`,
  `docs/evaluation.md` and `docs/reports.md`. Link dated records through the docs
  home/history rather than expanding the root README into a research ledger.
- Claim landings only with the complete physical/mission/integrity/replay tuple.
  Keep core cases, diagnostics, unsupported inputs and controller packs separate.
