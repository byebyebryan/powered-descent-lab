# Planner V2 batch-tree presentation checkpoint

Historical prototype only. Its presentation checks passed, but user review
rejected the separate preview as the delivered report experience. The follow-up
[common-template integration](planner_v2_common_report_templates_results.md)
reuses the actual batch/detail templates and publishes through normal `/reports/`
navigation. The preview artifacts below remain unchanged for provenance.

## Scope and review gate

This is a presentation-only correction to the native 44-case Planner V2 batch.
It reuses the existing terminal/transfer warm report shell, expandable review
tree, table controls and SVG trajectory projector. Native planner records remain
separate from the legacy controller `BatchReport` schema.

The create-only preview is the exit gate for this checkpoint. User visual review
comes before refreshing the stable batch page or changing site navigation.
No evaluation missions are rerun, planner/controller behavior is unchanged, and
no commit, push or report-server restart is part of this checkpoint.

Source: `outputs/eval/planner_v2_lab_suite/capture-1791074995928276719`.
The batch's accepted outcomes remain 36/36 core target landings (11 direct,
25 corrected), plus eight separate diagnostics: two target landings, four
zero-step `NoClearing` stops and two unsupported inputs. There are 36 executed
handoffs, 44/44 integrity passes and 42/42 supported source-replay passes.

## Presentation contract

The default view exposes four group rows and seven terrain-family rows, with
mission leaves collapsed:

- Clear controls (11) -> missions.
- Ordinary terrain (16) -> ridge, plateau, successive and sloped -> missions.
- Additional terrain (9) -> ridge, plateau and compound -> missions.
- Diagnostics (8) -> missions.

`Expand Missions` reveals all 44 missions. Group/family rows support mouse,
Enter and Space; depth controls and recursive collapse share the legacy tree
implementation. Summary rows retain aggregate metrics. Mission rows provide
saved fuel use as percent of vehicle maximum, flight time, verified landing
offset, planning time, integrity/replay status and a clickable SVG preview.

SVG handoff markers use actual local-correction segment end positions, not
proposed waypoint targets. Nondeparture and unsupported rows show only terrain
context, explicit stop labels and unavailable metrics as em dashes. Diagnostics
are not silently folded into the core landing denominator. Detailed mission
pages retain their rich plots and annotation interactions unchanged. Provenance
is available in a collapsed section rather than competing with the review tree.

## Safe rendering and independent validation

`pd-eval report CAPTURE --preview-dir NEW_DIR` validates the entire native
capture before writing. `NEW_DIR` must be a separate create-only directory
beneath `outputs/research`, with no traversal or symlink components. The CLI
returns before current-batch publication and rejects legacy preview requests
and baseline comparison combinations. A receipt binds the source summary,
rendered HTML, Rust source tree and renderer executable hashes.

`scripts/check-planner-v2-tree.mjs` independently projects every mission's
metrics and status from the saved flight/scenario JSON, checks all exact H
coordinates, mission-to-branch placement, group/family counts, page totals,
detail links and desktop/mobile interaction. It also exercises freshly rendered
terminal/transfer previews from their saved batches. The preservation ledger
includes 1,231 entries covering prior protected evidence, the complete accepted
capture, raw terminal/transfer inventories, current selection, published pages,
home/topic pages and the native pack fixture. Symlink targets are hashed without
following them.

## Validation results

Final preview:
`outputs/research/waypoint_v2_batch_tree_20261003/preview_final/index.html`.
LAN URL:
`http://192.168.1.110:8000/research/waypoint_v2_batch_tree_20261003/preview_final/index.html`.

Final independent acceptance is `passed`, with no browser errors:
`outputs/research/waypoint_v2_batch_tree_20261003/check_final/acceptance.json`.
Its directory retains 18 screenshots: the native summary/expanded tree and five
representative detail pages at both widths, plus terminal/transfer at both
widths. The audit verified all 44 mission identities, exact source metrics,
36 H coordinates, four groups, seven families, exact branch placement and
page-level totals. Every detail link resolved; no document-level overflow was
observed at 1440 px or 390 px. Wide data tables scroll locally on mobile.

Mouse, Enter, Space, mission expansion/collapse and depth/recursive controls
passed. Saved direct, one-H, multi-H, nondeparture and unsupported details were
opened from the tree. Existing rich annotation selection/toggle behavior passed
where applicable. Legacy saved-batch preview checks passed for seed expansion,
depth restoration, keyboard expansion and coverage jumps. Protected inventory
matched before and after the audit. A separate post-workspace check at
`outputs/research/waypoint_v2_batch_tree_20261003/preservation_final/baseline.json`
also confirms all 23 protected scopes and their 1,231 inventoried entries match
the original baseline exactly.

The final script suite has 33 passing tests. `cargo test --workspace` passes,
including 573 evaluator-library tests, 41 evaluator CLI tests and 55 report
tests; nine evaluator-library tests remain explicitly ignored. The saved legacy
presentation-preview test was run explicitly in a separate create-only output.
The final release evaluator build, formatting and diff whitespace checks pass.
Normal workspace
Clippy passes with the pre-existing `single_element_loop` warning at
`pd-eval/src/route_capability.rs:274`; strict `-D warnings` fails on that unchanged
warning. No unrelated lint fix was made.

An earlier full Rust run caught two legacy tests that asserted the old JavaScript
switch syntax. They now assert the equivalent shared depth configuration while
retaining their existing rendered hierarchy checks. The final workspace run
passes both. Earlier exploratory preview/check artifacts are retained and are
not the final acceptance evidence.

Six final-binary negative CLI checks rejected existing preview reuse, published
report destinations, evaluation destinations, capture-child destinations,
legacy batch previews and conflicting baseline options. No rejected destination
was created. The preview-path unit test also checks traversal and dangling
symlinks without writing at the requested destination.

The existing `pdlab-reports` tmux server remains bound to `0.0.0.0:8000` with
the same PID (483461). Published navigation and current selection were not
changed. Only the private headless browser used for validation was stopped.

## Next checkpoint

After user visual acceptance, publish the tree through the existing stable V2
batch URL, verify reachability from report home and the waypoint-planning topic,
and repeat link/navigation/preservation checks. That publication is not performed
by this preview checkpoint.
