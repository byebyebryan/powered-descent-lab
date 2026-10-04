# Planner V2 common report templates

## Contract

Terminal, transfer and native Planner V2 batches consume the same full batch
template and Overview, Coverage, Context, Review Tree, row/table, style and
interaction components in `pd-report::batch`. Domain aggregation and native
schemas stay in their respective adapters. This is template reuse, not a
separately styled V2 summary or a conversion into controller `BatchReport`.

The normal V2 entrypoint is `/reports/eval/planner_v2_lab_suite/`. Its 44 case
pages live beneath that same subtree. Batch, previous/next, report-home and
topic navigation stay in the published site; source JSON links point at the
unchanged accepted capture. There is no raw-capture `<base>` on the canonical
page and no research preview in the normal navigation flow.

The accepted capture remains
`outputs/eval/planner_v2_lab_suite/capture-1791074995928276719`. Results are
36/36 core target landings (11 direct, 25 corrected), plus eight separately
reported diagnostics (two landings, four zero-step `NoClearing` stops, two
unsupported inputs). There are 36 executed handoffs, 44/44 integrity passes
and 42/42 supported source-replay passes. This report change reruns no mission
pack and changes no planning/controller behavior or fixture.

## Presentation and preservation

The batch retains the familiar seven review-tree columns and adds an optional
Executed handoffs column. Aggregate fuel, flight time and landing offsets use
the existing population mean and standard-deviation convention. Missing
measurements and reference metrics remain explicitly unavailable. H markers
show actual correction-segment endpoints, not authored waypoint targets.

Supported details retain the common rich report: full telemetry, spatial and
metric plots, inspection controls and optional executed-handoff annotations.
The standalone supported V2 writer now uses that same annotation-capable
renderer. Unsupported details use the common status shell without invented
trajectories, charts or replay claims.

Canonical publication validates and renders the complete capture before any
site write, checks every target path for traversal/symlinks, publishes details
before the index, and records `render.json` with capture/source/page hashes.
Refreshing the selected native report no longer rewrites the original capture's
HTML. Current selection bytes are retained when the selected capture is
unchanged. The previous published V2 subtree is retained as a rollback copy.

## Validation evidence

Checkpoint evidence is under
`outputs/research/waypoint_v2_common_templates_20261003/`. This directory holds
internal validation artifacts, not a second user-facing report edition.

The baseline protects the complete accepted capture, current selection, native
fixture, report home/topic pages, historical evidence, and raw/published
terminal/transfer reports. Only the canonical V2 published subtree is permitted
to change. The common-template checker independently compares all 44 projected
records and 36 exact H coordinates against raw JSON, compares all 42 rich data
payloads with the accepted detail reports, checks source and previous/next
navigation, and verifies the common section/column contract.

The canonical batch and all 44 case pages are published. Final independent
acceptance is `passed` in `check_final/acceptance.json`, with 18 screenshots and
no browser errors. At 1440 px and 390 px it exercises both report-home URLs,
topic and library discovery, batch-to-case-and-back round trips, mission/depth
expansion, mouse/Enter/Space, exact coverage-to-family navigation, rich plot/H
selection and toggle interactions, and expanded context/provenance wrapping.
All 44 detail pages and their scenario/flight/summary source links resolve.
Coverage labels stay inside their cells; wide tables scroll locally on mobile.
Saved terminal/transfer batches rendered from the final shared source also
pass seed expansion/collapse, depth restoration and coverage navigation.

The complete 42 rich payloads match the accepted detail reports after removing
only the optional annotation/navigation object; exact H annotations are checked
separately against raw segment endpoints. All native projected and displayed
mission metrics, group/family counts, separate summary denominators, 36 preview
H coordinates and the 46 rendered HTML page hashes are checked. Preservation
matches all 22 protected scopes and 1,230 inventoried entries before and after
the audit. The previous published V2 page is backed up in
`baseline/previous-published-v2/`.

`refresh_final/acceptance.json` records successful final-binary
`pd-eval report CAPTURE` and `pd-eval refresh-reports --home-only` checks. The
complete published V2 subtree and protected inventory remain byte-identical
after both commands. The native branch of global report refresh uses the same
validated site publisher; the global command itself was not run on the live
site, to avoid rewriting unrelated legacy reports.

Validation includes a passing full `cargo test --workspace` (574 evaluator
library tests, nine explicitly ignored; 41 evaluator CLI tests; 57 report
tests), 37 script checks, final release build, formatting and diff checks.
After the final stylesheet/row-spacing-only correction, focused legacy report
tests pass 32 with one ignored, native pack tests pass eight with one ignored,
shared batch tests pass five, and the saved-batch presentation test passes
explicitly in `legacy_final/`. Final normal all-target workspace Clippy passes
with only the pre-existing `single_element_loop` warning at
`pd-eval/src/route_capability.rs:274`; strict `-D warnings` fails on that unchanged
warning. No unrelated lint fix was made.

The first checker attempt (`check_a`) failed because its new display-format
assertion incorrectly expected spaces before units; the established format has
none. `check_b` passed the data/browser checks, but screenshot inspection found
long Coverage-label overlaps and run-together row notes. Those were corrected
using common wrapping/stack components, a browser text-containment assertion
was added, and `check_final` passes against the final source and publication.
Earlier receipts/screenshots are retained and are not final visual acceptance.

The unchanged `pdlab-reports` tmux server (PID 483461) still serves outputs on
`0.0.0.0:8000`. No evaluation pack was rerun, and no commit or push was made
during implementation/validation.

## Scope limits

This is a report presentation/publication checkpoint, not new terrain coverage
or a new planner acceptance claim. A full global `refresh-reports` would rewrite
unrelated legacy reports and is deliberately not part of this live-site pass.
Legacy saved batches are rendered into separate internal validation artifacts
to exercise the shared code without modifying their published reports.

Implementation excluded commits, pushes and report-server restarts. Following
visual acceptance, the user authorized review, commit and push. Validation
receipts above retain their original pre-commit renderer/source provenance;
generated captures, published reports and browser evidence remain outside Git.
