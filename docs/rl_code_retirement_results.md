# RL code retirement

[Development history](history.md) · [Retired PPO package](../studies/rl_procedural_ppo/README.md) · [Controller implementation plan](maneuver_controller_implementation_plan.md)

October 21, 2026. The closed procedural PPO package has moved out of the current
`experiments/rl` namespace into a historical study. The separately retained RL
worktree remains at its original paths for source-bound readers and verification.
This cleanup changes no controller, planner, plant, mission input or recorded
experiment outcome. E24 evaluation/delivery remains incomplete.

## Source recovery

Before editing, a create-only recovery archive captured all Git-visible tracked
and untracked files from both checkouts: 789 main files and 1,497 research files,
2,286 total. Every archived member passed its recorded SHA256 and length check.
Restoring into a new temporary directory also reproduced every file's bytes,
length and executable mode without overwriting either live worktree.
The archive excludes ignored outputs and installed environments, which remain
in place.

Local recovery directory:
`/home/bryan/code/.pd-lab-rl-retirement-20261022T024553Z`.
Its `snapshot.json` records original paths, modes, hashes, heads and dirty state;
`source-recovery.tar.gz` is 6,454,899 bytes with SHA256
`6ede2faa6b107be145719cc231eea32cec69a39be6ed5409cac4f4af89913a82`.
The UTC directory date is October 22; the workspace date is October 21.
Recovery should extract into a new directory and reconcile explicitly against
the snapshot, rather than overwrite either live worktree.

## Retired and retained code

| Scope | Disposition |
| --- | --- |
| Main `experiments/rl` Python project | All 17 Python modules, dependency manifest/lock, ignore rules and original README preserved as 21 exact source files under `studies/rl_procedural_ppo` |
| Main legacy entrypoint | README redirects to the archive; Python launchers and project manifests are absent. Existing `.gitignore` and installed `.venv` remain |
| Main bytecode cache | 15 old module-cache files, 125,239 bytes, copied and hash-verified under recovery `retired-bytecode/`, then removed from that exact cache directory |
| Historical composition check | Its import path now selects the archived package; campaign logic is unchanged |
| Isolated RL worktree | 747 inventoried files across 28 study families, legacy modules and the terminal-input export stay at original paths and bytes. A root README banner marks the allocation sequence historical |
| Verification and analysis | Original readers, native replay entrypoints, checkpoint formats and exact source remain available; the archived campaign verifier imports no learner |
| Native plant/controllers/planner and frozen fixtures | Unchanged |
| Outputs, models, optimizer state, installed runtimes, receipts, reports and servers | Retained in place; no output retirement, publication, regeneration or server operation |

The research inventory includes 632 Python files, 498 of which were untracked
at the initial review. Older modules are imported by later studies, and run
bindings name absolute paths. Pruning individual frozen files or moving that
worktree would break those dependencies. Its source is therefore retired in
place rather than promoted into the current implementation or deleted by
experiment outcome. The terminal-input exporter remains a separate potential
reuse decision.

Current development/evaluation/architecture guidance points to the archive.
Historical source citations follow the relocated exact bytes. Original dated
command text and the 56 embedded document/retirement snapshots remain unchanged.
The controller design and proposed implementation pass are retained independently.

## Preservation checks

The latest run has 64 source/configuration/document bindings. Five documentation
paths already differed from its recorded closure hashes before cleanup:
`docs/README.md`, `docs/rl_control_architecture_review.md`,
`docs/rl_experiment_history.md`, `docs/rl_lessons.md` and `docs/roadmap.md` in main.
All five are untouched by this pass. Preserve that pre-existing drift rather
than rewriting old bindings or claiming that the closure describes today's docs.

The cleanup compares every binding against its actual pre-cleanup bytes,
separately from the original run's hashes. The 34 release source/configuration
bindings matched at entry. Historical landing and publication receipts retain
their own provenance; this pass does not requalify flights or HTTP availability.

## Validation

The read-only [retirement checker](../scripts/check-rl-retirement.mjs) validates
the 21 archived files and rejects executable/project leftovers in the old
namespace. The optional research-worktree argument verifies the 747 inventoried
files. Ordinary validation works without that checkout or an installed learner.
Its tests are discovered by the maintained developer gate and include changed
or missing source, resurrected entrypoints, symlink/path substitution, duplicate
mappings, unknown versions and a changed research inventory.

| Validation | Result |
| --- | --- |
| Archived main package | All 21 original files match their hashes and lengths |
| Isolated source inventory | All 747 files match at original paths |
| Retirement fault tests | All eight tests pass, including source tampering and resurrected entrypoints |
| Focused Node checks | 21 pass, zero failures or skips |
| Python source/import checks | All 17 archived modules parse; the three historical composition imports resolve from the archive; no model or campaign is loaded |
| Maintained developer gate | All 18 checks pass: workspace tests, feature/default CLI checks, formatting, Clippy, Node/docs and dependency boundaries |
| Latest run bindings | All 64 actual pre-cleanup byte identities preserved; the five pre-existing documentation mismatches remain unchanged |
| Embedded historical snapshots | All 56 SHA256/length checks pass |
| Unrelated source and Git state | Preserved; both heads and indexes unchanged. Research worktree changes only its root README banner |

The recovery directory retains `developer-gate.log`,
`preservation-validation.json`, `recovery-verification.json` and
`bytecode-retirement.json`. The scope audit accounts for 11 existing main-file
edits, 19 exact relocations and 27 new main files; all other existing source
matches the pre-cleanup snapshot. These checks describe the pre-commit cleanup
checkpoint. No new flight or
training campaign, output cleanup or report operation was performed.
