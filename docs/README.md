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

## Terrain source study

The [procedural profile study](terrain_profile_study_results.md) compares 18
Pylander-based and FastNoiseLite profiles, with raw data, shared-scale plots and
repeatability/sampling checks. It is a shape-only checkpoint, not a new flight
capture or planner-coverage claim. Its [runner and frozen contract](../studies/terrain_profiles/README.md)
are explicit opt-in; the flight workspace and accepted report site are unchanged.

The [ridge-slice refinement](terrain_ridge_refinement_results.md) adds twelve
fixed-offset before/after profiles and twelve locally pad-prepared inputs. The
shared ridge motif is reduced and 12/12 native input-only preflights pass; no
procedural flight or landing result is claimed. Its offline galleries remain
separate from the accepted common-template flight report site.

The [random-terrain survey plan](random_terrain_survey_plan.md) freezes 100 fresh
cases with separate preservation controls/repeats and common rich flight reports.
It is exploratory coverage, not a replacement for the accepted 44-case benchmark.
The [completed full-sweep results](random_terrain_full_sweep_results.md) record
100/100 verified random direct landings, three preserved controls (including
one- and three-handoff landings), and five exact non-timing repeats. All random
routes were clear: this remains the separate sanity baseline.
The subsequent [harder-terrain coverage plan](terrain_challenge_plan.md) and
[completed challenge results](terrain_challenge_results.md) retain a separate
24-case calibration and independent 100-case population. The latter has 64
blocked routes, 21 corrected landings and 36/36 clear direct landings, with 43
honest finite stops. This exposes planner coverage limits without changing the
accepted benchmark or rewriting the sanity sweep.
The [intervention-timing comparison](intervention_timing_results.md) retains a
16-case logging-only baseline and timing candidate: three versus six landings,
but a successful control and the existing ridge CLI test regress. The replacement
is not adopted; previous timing is restored and compact query diagnostics remain.
The conditional full recheck was not run. This is development reuse, not a new
held-out estimate.
The subsequent [failure-only fallback](intervention_fallback_results.md) preserves
all 57 prior challenge landings and adds eight, for 65/100 on the unchanged worlds.
Its fresh benchmark passes all 36 core landings; the accepted selector/site remain
unchanged. The current source tries extra timings only after finite primary
search exhaustion, with unchanged guards and actual-handoff replanning.
The completed [handoff braking-room preference](handoff_room_results.md) adds
three landings for 68/100, preserving all 65 previous successes. It changes
only a negative-room winner when an already accepted nonnegative-room alternative
exists. The benchmark and accepted site remain unchanged; this is a cheap
development heuristic, not a landing-feasibility certificate.
The [original stopped results and diagnosis](random_terrain_survey_results.md) record three
verified random direct landings and an initial replay-envelope roundoff failure;
96 random cases were unattempted in that retained original capture. No
arbitrary-terrain reliability claim is made.
The [numerical fix and recheck plan](random_terrain_survey_recheck_plan.md)
keep the same 100 inputs and 108-attempt ceiling. The [recheck results](random_terrain_survey_recheck_results.md)
stop at the first preservation control: its flight/proofs pass, but one clearance
scalar differs by 8.88e-16 m. No random flight was launched in the recheck.
The subsequent [comparison-aware results](random_terrain_survey_comparison_results.md)
accept the direct control but stop at the corrected control's diagnostic-derived
proposal hash. Its maneuver/flight/proofs are unchanged; the comparator needs
selected-trajectory/hash-binding coverage, supplied by the subsequent full sweep.
The [full-sweep continuation](random_terrain_full_sweep_plan.md) supersedes the
earlier stop-on-first-error/no-retry execution policy: repair routine defects,
retain every source-frozen attempt and complete the unchanged full population.

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
