# Documentation home

Start with the current contracts and workflows. The many protocols and result
files are retained development records, not a list of unfinished prerequisites.
The accepted flight evidence remains the October 5 session/CLI replacement;
later code and documentation housekeeping does not create a new flight capture.

## Daily workflows

| Task | Entry point |
| --- | --- |
| Project overview and quick start | [Root README](../README.md) |
| Build, validate and make safe changes | [Development workflow](development.md) |
| Planner flights, batch evaluation and controller caches | [Evaluation workflow](evaluation.md) |
| Browse, serve, refresh and check saved reports | [Report workflow](reports.md) |
| Identify maintained scripts and their side effects | [Tooling inventory](../scripts/README.md) |
| Understand frozen inputs and archived pack metadata | [Fixture ownership](../fixtures/README.md) |

## Current contracts and ownership

- [Architecture](architecture.md): plant, scenario, controller and persisted
  artifact boundaries.
- [Guidance](guidance.md): terminal, direct-transfer, authored-waypoint and V2
  planning/execution ownership; supported envelope and source layout.
- [Waypoint planning](waypoint_planning.md): current V2 status first, followed
  by explicitly historical V1 and research checkpoints.
- [Terminal suite](terminal_suite.md) and [transfer suite](transfer_suite.md):
  maintained corpus shape, selectors and controller evidence interpretation.
- [Roadmap current status](roadmap.md#11-current-status) and
  [next product decision](roadmap.md#7-recommended-immediate-next-step): closed
  work versus procedural-terrain or host-integration choices.
- [Project direction and scope](project_direction.md): lab/game split and reboot
  rationale.

## Accepted checkpoints and completed housekeeping

- [Session/CLI replacement results](waypoint_v2_session_repair_results.md): the
  accepted native, CLI and repeat checkpoint; 36 core landings, eight separate
  diagnostics and 42 saved-source replays.
- [Common report templates](planner_v2_common_report_templates_results.md):
  shared batch/detail templates and actual handoff visualization.
- [Core-loop cleanup](waypoint_v2_core_cleanup_results.md): completed structural
  work and its preservation gates.
- [Retirement and housekeeping](planner_retirement_cleanup_results.md): sole
  current policy, removed experiments, historical decoding boundaries and final
  cleanup validation. Its [plan](planner_retirement_cleanup_plan.md) is closed.
- [Documentation/repository hygiene review](docs_repo_hygiene_results.md): current
  workflow reconciliation, output-free validation and intentionally preserved
  worktrees/history; reviewed docs/tooling checkpoint, not a new flight capture.

These are measured or completed records, not commands to generate new evidence.
For current developer validation, use the development workflow above.

## Research and history

[Development history and archive](history.md) groups the earlier trajectory,
V1, V2, alternative-planner and presentation records. It preserves their
original filenames, dated results and local evidence links. Restored links for
retired sources point to archival revisions, not live implementations. [Progress](progress.md)
is append-only checkpoint history; [early design](early_design.md) is exploratory.

## Reading evidence correctly

- A planning stop or successful collection exit is not a landing. Keep planning,
  physical outcome, mission outcome, integrity and replay evidence separate.
- Do not combine the 36 core planner cases with the eight diagnostics, or mix
  planner denominators with terminal/transfer controller matrices.
- `outputs/` contains local generated artifacts, not files supplied by Git.
  Missing local captures do not justify inventing results or deleting history.
- A cleanup check can preserve an accepted capture without making that capture
  evidence from the newer source. Per-tick host integration and arbitrary-terrain
  reliability are not established by the current lab checkpoint.
