# Report navigation and rich waypoint preview results

The topic-first hierarchy follow-up to the
[navigation and annotation plan](report_navigation_and_waypoint_annotations_plan.md)
is implemented and served for user review on 2026-10-02. The two report homes
now share one organization, rather than merely linking to a timestamped preview.
The existing rich reports are preserved; only Late ridge has the previously
approved waypoint annotation. Automated validation passes. Findability and
presentation acceptance remain the user's decision.

## Review before commit

The user reports that the hierarchy is better and requests review and commit
before returning to planner V2 work. Review found one mismatch left over from
the superseded prototype: new V2 captures still selected the lean renderer.
That pending change is removed. The flight writer keeps its existing rich
report path, while retained preview rendering remains explicitly opt-in.
All-case annotations and future-writer annotation integration are still deferred.

A new supported-capture regression runs an existing fixture in a temporary
test directory and verifies both charts, all five modes, sample count and rich
statistics, with no preview annotation payload. The unchanged unsupported
preflight test still verifies no report and create-only output. These are native
tests, not another mission matrix or new coverage evidence.

Final review validation passes **957 workspace tests with seven ignored**, plus
all three explicitly run local retained-presentation/rich/navigation gates.
Format, both browser-script syntax checks, whitespace and all-target Clippy
with the same narrow baseline allowance pass. The live edition's 127 input and
32 output hashes still verify; the two homes are byte-identical. No served HTML
or capture was rerendered, and the prior browser evidence remains that edition's
90-link, desktop/mobile validation rather than a new browser run. Render and
flight executable provenance remain historical; receipts are not relabelled
after rebuilding or committing source.

The implementation and its design/evidence records are committed separately.
Planner/controller code, policy defaults and authoritative outcomes are unchanged.
No push or server restart is included in this review.

## Where to start

Start at the [LAN server home](http://192.168.1.110:8000/). It offers two
subjects: **Waypoint planning** and **Flight and landing control**. Each explains
what its reports answer. Research and history, the searchable report library and
raw data are secondary routes. The [report home](http://192.168.1.110:8000/reports/)
is the same page. Both use the existing `pdlab-reports` tmux server without a restart.

Review navigation and the detailed report separately:

- Can you find direct flights, terrain corrections, the maintained planner
  baseline and controller reports from the home without a supplied walkthrough?
- Can you distinguish current opt-in evidence, analytical setups and historical
  view editions without comparing timestamped directories?
- Does Late ridge still feel like the original detailed report, with the
  waypoint information added rather than its plots and data removed?

Late ridge's H1 is the exact executed handoff at physics step 2426, time
20.216666666666665 s, position `(-288.84978519456723, 318.2854446064752)` m and
velocity `(76.94860325400595, 10.850162616364521)` m/s. The recorded rejected
continuation had approximately 4.612 m body clearance against a 5 m reserve at
step 2449. That continuation was not flown. Replanning from H1 produced the
continuation that landed at step 4080, time 34 s.

## Topic hierarchy implementation and validation

The selected V2 collection is under **Waypoint planning**, alongside the
maintained planner baseline and two explicitly labelled analytical studies.
**Flight and landing control** separates terminal landing, direct transfers and
following authored waypoint routes. These are controller tasks, not evidence
that the planner chose those authored routes. Report type, lifecycle and
availability are separate labels; V2 remains opt-in and its UI under review.

The library has 59 searchable entries from fixture metadata, direct stable
evaluation/setup entrypoints and explicitly selected view editions. Missing
captures remain unavailable; unknown report topics remain unclassified.
This is a bounded inventory, not a recursive publication of research/cache
artifacts. History identifies the two previous presentations as different views
of the same pinned capture, not new results. Raw data links only to existing
approved collections.

All 30 supported V2 cases now have home/topic/collection return paths and
previous/next links in a fresh, create-only navigation edition. Twenty-nine
original full reports are copied verbatim except for an escaped navigation
banner. Only Late ridge retains the additive exact H1 annotation. The two
unsupported entries remain without simulated flight links. Original reports,
old editions and receipts are not overwritten.

One navigation writer owns the homes, topic pages, library and compatibility
indexes. The guidance catalogue still owns its scorecard/group indexes, but no
longer competes for `eval/index.html`. In configured hierarchy mode,
`refresh-reports --home-only` refreshes navigation indexes, not just two homes;
`refresh-navigation` also refreshes maintained guidance indexes. Neither runs
missions or rewrites detailed report bodies.

- Final-source workspace gate: **956 passed, seven ignored**. Both local
  retained-capture tests are intentionally ignored in ordinary CI. The original
  rich-preview gate and new all-thirty-report navigation/payload gate were
  explicitly run and passed.
- Three navigation tests cover known/unknown/missing entries, escaping,
  malformed manifests, unsafe input paths and input/output symlink escapes
  before navigation writes. The catalogue test proves identical navigation in both
  refresh orders and after an ordinary report-index update. Repeated live
  navigation refreshes also preserve all fourteen published navigation and
  guidance index hashes.
- Real headless Chrome 154 passes six retained rich-preview page/width checks
  plus four root-origin browsing routes: both homes at 1280 and 390 px.
  Tasks cover direct flight, corrections, baseline, controller reports,
  analytical studies, library search/filters, history and return navigation.
  **90 unique links** resolve; **22 screenshots** were captured. Desktop/mobile
  home, topic, library and history layouts were visually inspected. No
  horizontal overflow, required-asset errors or JavaScript errors were recorded.
  An optional same-origin favicon 404 remains a separate diagnostic.
- The rich-report gate retains all five modes, H1 hover/time guides, overlay
  controls, sample inspection, zoom/pan, legends and disclosures. All thirty
  parsed original payloads match after removing only the previously approved
  annotation. The twenty-nine navigation-only copies pass exact HTML comparison
  after removing their banner.
- All 159 pinned-capture HTML/JSON files, historical report bodies/receipts and
  monitored evaluation outcome JSON are byte-identical. No monitored files were
  removed. Only nine existing generated navigation/guidance indexes changed;
  five additional hierarchy indexes and the create-only edition were added.
  The final receipt verifies **127 input hashes, 32 output hashes, six renderer
  source hashes and the actual rendering executable hash**.
- Format, JavaScript syntax, whitespace and all-target Clippy pass. Clippy uses
  only the unchanged `clippy::single_element_loop` baseline allowance; strict
  baseline cleanliness is not claimed.

Current rendering evidence is under
`outputs/reports/waypoint-v2/rich_preview_20261002_hierarchy_v1`; browser evidence
is under `outputs/reports/waypoint-v2/hierarchy_20261002_browser_checked_a`.
Luna built the bounded navigation renderer and fixture integration. The primary
reviewed and corrected it, integrated catalogue ownership, navigation-only rich
copies and CLI/checker behavior, and performed the final gates. No new mission
matrix, planner/controller/policy change, future-writer switch, commit, push or
server restart occurred. User findability review is the next gate.

## Original rich preview changes

The following describes the earlier first slice. Its front-door organization
is superseded by the topic hierarchy above; its rich-report behavior is retained.

The home has one current V2 entry, a separate maintained guidance entry and
secondary links to batch reports, analytical setups and history. Replays are
explicitly unavailable because no report destination exists. The collection
shows all 32 retained cases in their capture groups: eight clear controls,
sixteen ordinary terrain missions and eight diagnostics. Search, group filters,
friendly recommendations and stable case anchors reduce navigation clutter.

Exactly one case opens an **annotated rich preview**. Twenty-nine cases open
their **original full reports**, labelled as not enhanced. Two unsupported
cases show their recorded reasons without a flight-report link. The reference
plateau recommendation therefore has no new H markers yet, despite its three
recorded corrections. The preview does not advertise an enhanced full suite.

The detailed Late ridge report retains Mission, Guidance, Speed, Throttle and
Vectors modes; the velocity/thrust chart; sample hover and telemetry; events;
landing, flight and controller statistics; performance; and mission/vehicle
details. Additions are an exact H1 marker across modes, a matching time guide,
an overlay toggle, a compact correction explanation, exact-state disclosure
and preview navigation. Selecting a correction or toggling guides preserves
plot zoom. The ordinary sample inspector stays separate from the exact handoff.

The existing dashed reference retains its generic-report meaning. It is not a
reconstruction of the selected or rejected V2 proposal. No controller updates
or performance measurements were synthesized from planning cycles. The retained
capture lacks those measurements; existing empty fields and zero-placeholder
summary values remain unchanged rather than becoming measured zero-cost claims.

## Initial preview validation results

These results describe the original preview slice, before the navigation
entrypoint follow-up below. In particular, its published-index preservation and
rendering-executable match describe that checkpoint, not later CLI builds.

- Final-source workspace gate: **946 passed, six ignored**. The new ignored
  test requires the local retained capture, keeping ordinary CI independent of
  untracked outputs. It was explicitly run and passed separately.
- Focused navigation tests: eight passed. Annotation tests: four passed,
  including empty/non-V2 compatibility, an odd exact boundary, invalid clocks,
  nonfinite state, escaping and unsafe-link rejection. Evaluator tests cover
  payload tampering and rejected capture/output targets; existing strict reader
  corruption/continuity/ownership tests also pass in the workspace gate.
- Original versus annotated rich `reportData` is exactly equal after removing
  only the new optional `flightAnnotations` key. All 341 samples, events,
  manifest, terrain, pad, telemetry and derived statistics are preserved.
- Real headless Chrome 154 checks pass for home, collection and rich report at
  1280 and 390 px: six page/width checks, six full-page screenshots and 43 unique
  links. Both Plotly charts load using the unchanged pinned dependency. Checks
  cover all five modes, exact H1 hover and time guide, overlay toggling, native
  sample hover, pointer zoom/pan, legend interaction, keyboard selection and
  old/new disclosures. No horizontal overflow or browser errors were recorded.
  Screenshots were inspected, including an additional mobile correction view.
- Format, JavaScript syntax and whitespace checks pass. All-target Clippy passes
  with only the unchanged `clippy::single_element_loop` baseline lint allowed.
  Strict Clippy still reports that baseline in `pd-eval/src/route_capability.rs:274`;
  strict-baseline cleanliness is not claimed.
- All **235 monitored existing output HTML/JSON files** are byte-identical.
  This includes all 159 JSON/HTML files under the pinned capture, its 30 original
  report bodies, the earlier lean presentations and published navigation. The
  renderer consumes and hashes 127 evidence/report files; the other 32 capture
  files are retained input copies and were also preserved.
- The final rendering has three HTML pages and a separate receipt. Its six
  renderer-source hashes and actual rendering-process hash match the current
  sources/executable. Historical flight provenance remains separate. Reusing
  the completed output root is rejected without writes.

Render evidence is under
`outputs/reports/waypoint-v2/rich_preview_20261002_v2`; browser evidence is under
`outputs/reports/waypoint-v2/rich_preview_20261002_v2_browser_checked`.
The earlier create-only preview and incomplete checker attempts remain local,
unpublished artifacts; none is a user-accepted edition.

<a id="navigation-entrypoint-follow-up"></a>

## Earlier navigation entrypoint follow-up

This was the link-only checkpoint before the user clarified the need for a
whole-site hierarchy. Its home layout and two-index-only command behavior are
historical, not the current implementation.

The user finds the detailed page closer to the intended report, but identifies
that it cannot be reached from the main server index. This is partial report
feedback, not acceptance of the entire navigation design.

The two home pages now expose the preview routes described above.
`fixtures/reports/navigation_preview.json` explicitly pins these preview
entrypoints so an ordinary home refresh preserves them. This is an under-review
navigation selection, not an accepted current-capture or planner-default
selection. Missing preview files produce an unavailable card rather than a
historical fallback. Invalid schemas, unsafe paths, mismatched collections and
symlink escapes fail before index writes. The earlier lean edition no longer
competes as a current home card; it remains reachable through preview history.

`pd-eval refresh-reports --home-only` refreshes just `outputs/index.html` and
`outputs/reports/index.html`, without rewriting report bodies or catalogues.
The live follow-up used this option. All monitored pre-existing capture,
report-body and catalogue files are byte-identical; only the two generated home
indexes changed. The three rich-preview HTML pages and their historical render
receipt remain unchanged.

Follow-up validation passes 47 report tests and 38 CLI tests, plus format,
JavaScript syntax, whitespace and all-target Clippy with the same narrow baseline
allowance. Full workspace tests were not rerun in this follow-up. Headless Chrome
checks four native-keyboard routes from the two live LAN homes at 1280 and 390 px,
in addition to the existing six rich-preview page/width checks and 43 links.
Ten screenshots were captured; inspected home layouts have no horizontal
overflow. The final browser receipt records no browser errors. An exact
same-origin optional favicon 404 is logged separately if encountered; required
assets and JavaScript errors still fail the checker.

Browser evidence is under
`outputs/reports/waypoint-v2/navigation_entrypoints_20261002_browser_checked_b`.
No mission, planner/controller/policy change, full-suite expansion, future-writer
switch, commit, push or server restart occurred in this follow-up.

## Implementation ownership and stop boundary

Luna implemented the navigation-only renderer and its focused tests. The primary
implemented and reviewed the additive generic-report layer, retained-evidence
adapter, explicit `--rich-preview` CLI flag, payload parity gate and browser
checker. Existing dirty work was preserved. The original preview slice did not change core,
controller, planner, policy, fixtures, lockfiles, `ReportSite` or the
future-capture writer. No new mission matrix, tuning, commit, push or server
restart occurred.

The rich-preview checkpoint, bounded home-link follow-up and subsequent
topic-hierarchy implementation are complete, not human UI acceptance. History
inventory and competing index ownership are now addressed within the bounded
hierarchy scope. Rendering all thirty annotated reports, switching the future
writer and promoting an accepted collection/default remain deferred. Stop for
user review; they must not proceed merely because automated checks pass.

## Reproduction

The report-only command requires the pinned capture and a fresh output directory
whose name starts with `rich_preview_` under `outputs/reports/waypoint-v2`:

```sh
rtk cargo run -p pd-eval -- waypoint-v2-report --suite-root outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a --output-dir outputs/reports/waypoint-v2/rich_preview_REVIEW_ID --rich-preview
rtk cargo test -p pd-eval pinned_rich_preview --lib -- --ignored
```

For a navigation edition containing the thirty preserved rich reports, add
`--site-navigation` to the rendering command. Explicitly run its local gate with
`rtk cargo test -p pd-eval pinned_navigation_edition --lib -- --ignored`.

The presentation checker connects to a caller-owned isolated local Chrome
debugging session. It creates a fresh validation directory and runs no missions:

```sh
rtk proxy node scripts/check-rich-waypoint-preview.mjs --url http://127.0.0.1:8000/reports/waypoint-v2/rich_preview_REVIEW_ID/ --output-dir outputs/reports/waypoint-v2/rich_preview_REVIEW_ID_browser_checked --cdp-url http://127.0.0.1:9225
```

To refresh the hierarchy and maintained guidance indexes, use
`rtk cargo run -p pd-eval -- refresh-navigation`. For navigation indexes without
guidance scorecard refresh, use `rtk cargo run -p pd-eval -- refresh-reports
--home-only`. Add `--root-url http://127.0.0.1:8000/` to the browser checker to
exercise both home-origin browsing tasks; the root and preview must share an origin.
