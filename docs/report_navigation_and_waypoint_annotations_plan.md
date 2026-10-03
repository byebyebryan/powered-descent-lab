# Report navigation and waypoint annotations plan

## Approved topic hierarchy followup

The user clarifies that adding the missing preview link does not solve the
navigation problem. Starting from the server root must make the report subjects
and their relationships understandable without a walkthrough. The subsequent
`worker-goal-loop` authorizes this navigation implementation, independently of
further waypoint annotation or future-capture renderer work.

The root and `/reports/` will use one report-home organization. Primary subjects
are **Waypoint planning**, meaning choosing routes around terrain, and **Flight
and landing control**, meaning executing transfers and landing. Planning contains
the selected V2 opt-in mission collection, the maintained planner baseline and
related research. Control contains terminal landing, direct transfer and following
authored waypoint routes. Research and history, a searchable report library and
raw data are secondary destinations. Report type and review status are metadata,
not competing top-level subjects.

The canonical reading paths are:

```text
Reports
  Waypoint planning
    Selected V2 capture: clear terrain, terrain missions, diagnostics
      Detailed mission report
    Maintained planner baseline
    Related studies
  Flight and landing control
    Terminal landing, direct transfer, authored waypoint following
  Research and history
  Browse all reports
  Raw data
```

Keep archive URLs valid and do not recursively publish research directories.
Use explicit fixture metadata for known entrypoints; unknown discovered stable
reports belong in an Unclassified section. Show missing destinations without
active links. Different presentations of one capture are editions, not unrelated
flight results. Keep opt-in V2 outside maintained scorecard denominators.

Execution is one navigation slice: inventory and settle the map, implement the
home/topic/library/history pages, fix competing batch-index ownership, integrate
return navigation, and run native plus real-browser checks. A new create-only
V2 navigation edition may copy the original rich HTML and add a navigation banner;
it must preserve report payloads and all plot code. Exactly one mission retains
the existing waypoint annotations. Original captures, reports and earlier render
receipts remain untouched. The selected preview fixture is a navigation pin,
not product/default acceptance.

Validate browsing tasks from both root URLs: locate a direct control, one
correction, repeated corrections, the maintained baseline and an analytical
study, then return to the collection/topic/home. Verify narrow-screen layouts,
search/filter behavior, missing and unclassified evidence, selection validation,
and both catalogue/generic refresh orders. Repository tests, format, proportionate
Clippy and retained-file hashes are the final implementation gates; human
findability acceptance remains the user's decision. No simulation, controller or
planner change, all-case annotation, future-writer switch, archive deletion,
commit, push or server restart is authorized by this followup.

The earlier sections below record the first preview's design. Their second-slice
annotation/future-writer scope remains deferred and is not part of this goal.

## Decision and scope

This 2026-10-02 design follows the user's clarification: navigation between
reports is cluttered, while the detailed mission reports are valuable. Improve
how readers find the right evidence and add V2 handoffs to the existing rich
report. Do not replace its plots, views or inspection tools with a simplified
mission page.

This supersedes the [standalone presentation design](waypoint_v2_report_presentation_plan.md).
Its [implementation results](waypoint_v2_report_presentation_results.md) remain
historical validation, not acceptance of that user experience. This pass is
design and planning only. It does not resume the old worker goal, start a new
goal, change Rust or generated reports, run missions, restart the server, commit
or promote a planner default.

The recommended execution has an early stop: build a navigation preview and
one annotated rich late-ridge report, then obtain the user's review before
expanding the suite or switching the future-capture writer.

The approved first-slice implementation is recorded in the
[rich preview results](report_navigation_and_waypoint_annotations_results.md).
The early user-review gate still applies before the second slice.

## Findings from the current tree

- `pd-report/src/lib.rs` already provides the rich report: trajectory modes
  Mission, Guidance, Speed, Throttle and Vectors; velocity/thrust plots; sample
  inspection; events; landing, flight and controller statistics; performance;
  and scenario, vehicle and mission details. Retain these sections and the
  recorded data they can actually expose, including honest unavailable fields.
- Dynamic V2 handoffs exist in saved cycles and segment boundaries. The original
  generic writer supplied ordinary samples/events but no V2 annotations. Its
  waypoint sources are authored transfer routes and controller markers. The gap
  is an annotation adapter, not an inability to render rich V2 flight evidence.
- The current uncommitted V2 writer instead sends future captures to the lean
  renderer. Its new payload contains position/velocity points and correction
  explanations, not the old report's complete telemetry and statistics. Fixing
  navigation does not require this replacement.
- The report home combines curated guidance, a versioned V2 presentation and
  raw run/replay/batch/setup categories. The V2 home-card scan can add an
  indistinguishable card for every completed rendering, even when all represent
  the same flight capture. A render receipt does not mean the UI was accepted.
- There are two writers for `outputs/reports/eval/index.html`:
  `ReportSite::write_scope_index` produces a modification-time directory table;
  `pd-eval::report_catalog::write_report_catalog` produces a friendly,
  searchable, fixture-backed catalogue. Ordinary batch output calls both in
  that order; other report writers can call only the generic one. The served
  index currently has the directory-table format. Refresh order can therefore
  change the navigation experience.
- The existing guidance catalogue already separates terminal, direct transfer,
  preplanned waypoint guidance and planner-generated routes. Keep those evidence
  distinctions; V2 opt-in captures must not silently enter their scorecards.

Source pointers are `pd-report/src/site.rs`, `pd-eval/src/report_catalog.rs`,
`fixtures/reports/guidance_catalog.json`, `pd-eval/src/waypoint_v2_report.rs`,
`pd-eval/src/waypoint_v2_output.rs` and the reporting contract in
`docs/architecture.md` under `pd-report` and the static report viewer.

## Navigation design

### One front door with clear reading choices

Keep `/reports/` as the stable entrypoint. Its first section should answer
"what should I open?", with one link to current opt-in V2 flight inspection and
one to the maintained guidance overview. Each says what question it answers and
what evidence it contains. Do not present every render revision as a peer choice.

Secondary navigation exposes the searchable batch library, analytical setups,
replays and research/history. Empty collections should be unavailable or clearly
marked empty, not promising useful reports behind a directory listing. Raw
artifacts remain reachable as secondary links.

The intended route for the current V2 capture is:

```text
Reports
  Current waypoint planner V2 (opt-in)
    Clear controls / Terrain missions / Diagnostics
      Detailed mission report
  Maintained guidance overview
    Terminal / Direct transfer / Waypoint guidance / Generated routes
      Existing batch, comparison and run reports
  Research and history
    Older captures, analytical studies and presentation prototypes
  All batch reports, setups, replays and raw artifacts
```

This is a reading hierarchy, not a requirement to move files. Existing archive
URLs stay valid. The current V2 branch should take at most two link activations
from the report home to a detailed mission.

### Current V2 collection

Use `/reports/waypoint-v2/` as one generated collection front door pointing to
an explicitly selected capture and its accepted full-report rendering. The
versioned output roots remain the archive/provenance locations. During preview,
label the new view as a preview and leave the existing published selection alone.

Pin the flight source to
`outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a`.
Its 32 entries are eight clear controls, sixteen ordinary terrain missions and
eight diagnostics; thirty have ordinary flight evidence and two are unsupported.
Identify policy 3, the currently tested vehicle, Earth gravity and the 120/60 Hz
physics/controller setup. Show the capture date separately from the new render
date. "Current opt-in" must not imply default policy or general coverage.

Lead with three compact suggested inspections: late ridge for one correction,
flat direct for the uncut control and reference plateau for repeated corrections.
Then show one compact case list with friendly names, group, recorded result,
correction count and an explicit Open report action. Keep all 32 entries, with
simple group filtering and name search. Archive IDs remain searchable and
available in details, not the main reading burden. Do not repeat each group as
multiple large sets of cards.

Unsupported entries retain their reason and have no report action. A NoClearing
case is a useful diagnostic, not a missing report or an ordinary success. Preserve
separate group totals; do not combine diagnostics into a claimed ordinary pass rate.

Every enhanced mission has a breadcrumb back to its capture's collection and
the report home. Provide previous/next supported mission links in stable suite
order, with friendly names; the back link anchors the reader's case in the list.
Navigation must not jump to another capture or policy. Existing batch comparison
and run-index links remain available rather than being replaced by V2 navigation.

### Current versus history

Choose the current source/render binding explicitly, never by directory
modification time, a generic `latest` link, highest success count or lexicographic
render name. Use a small `fixtures/reports/waypoint_v2_inspection.json` selection
file with a schema version, relative source root, relative render root and reading
label. Keep resolved roots within the repository's outputs, including symlink
checks, and validate the binding against the source identity and a completed
all-case rich-render receipt. It selects presentation links, not authoritative
flight outcomes. This proposed file is not created in the design pass.

An absent or mismatched selection shows unavailable evidence; it must not fall
back silently to another capture. Report-only generation must not auto-publish
every new directory. Updating this selection is a separate, explicit action
after review, not a side effect of rendering.

Research/history is a secondary inventory of existing report entrypoints, with
known studies described by purpose and unknown ones labelled unclassified.
Do not infer acceptance from a filename or invent a research taxonomy for every
old experiment. Inventory existing stable report entrypoints and explicitly
linked V2 captures, not every directory recursively under `outputs/research`.
Older V2 repetitions and the rejected lean UI belong here as
distinct capture/view editions. Their preserved flight evidence is not invalidated
because the presentation was rejected.

## Evolve the detailed mission report in place

### Preserve the baseline

The existing rich report is the main report, not a linked advanced alternative.
Use its layout and controls as the baseline. Retain plots, legends, zoom/pan,
five spatial modes, metric hover, sample inspection, events and detailed
statistics. Keep its current Plotly dependency and theme in this pass; do not
replace the plotting system or hide existing sections to make room for the new
information. Small label, legend and interaction refinements are welcome when
reviewed against the same data: preservation does not mean freezing the UI forever.

For the same retained input, all baseline report-data fields must remain equal,
except explicitly added navigation context and optional flight annotations.
In particular, preserve samples, events, manifest, terrain, pad, telemetry,
mission/vehicle details and derived statistics. Missing controller-update or
performance evidence stays unavailable; do not synthesize it from V2 cycles.

### Add a small V2 annotation layer

Add numbered H1/H2/H3 markers at the exact ends of executed local corrections,
with the caption "Waypoint handoff: replan from actual state". These are
pass-through states, not authored route goals, stops or capture envelopes.
Keep them visible across the existing spatial modes, with a small overlay toggle.
Hover or selection reveals the recorded time, step, position and velocity;
further recorded state belongs in the added detail, not a fabricated sample.

Add matching H time guides to the velocity/thrust plot, using the exact physics
clock. They should not replace the metric traces or phase/event guides. Nearby
labels may be offset or selected one at a time; physical positions/times cannot
move to make the diagram tidier.

Add a compact Waypoint corrections section alongside the existing event and
inspection information. Each row explains the recorded obstruction, correction
entry/end and what the next planning cycle did. Selecting a row highlights its
handoff and time guide without changing zoom or removing other data. Support
keyboard selection and readable narrow-screen rows. Keep the existing hovered
sample inspector separate from the exact handoff-state detail.

For late ridge, explain that the rejected direct continuation first had about
4.612 m body clearance against a 5 m reserve. This was not a collision on the
executed branch. The correction ends at H2426, then replanning lands at H4080.
Every later handoff must state its actual next-cycle result, not promise landing.

Correction-colored trajectory pieces can be a later additive refinement if the
first preview shows a concrete need. They are not necessary for the first slice.
If added, use actual samples plus exact retained segment endpoints for drawing,
without changing telemetry arrays or overriding Speed/Throttle coloring.

Do not add fitted rejected arcs or draw continuation certificates as flown
paths. Existing generic reference overlays keep their existing meaning; they
must not be relabelled as the V2 planner's selected or rejected proposal.
NoClearing before departure retains its zero-time state and planning reason;
unsupported cases still have no invented flight, and partial flights stop at
their actual endpoint.

## Implementation boundaries and reuse

- `pd-report` owns an optional neutral flight-annotation payload and additions
  to the existing generic renderer. It must not depend on `pd-eval`, plan a
  flight or fabricate controller updates. Preserve existing writer APIs with
  an empty-annotation path; do not migrate every caller.
- `pd-eval/src/waypoint_v2_report.rs` keeps the strict retained reader, identity,
  ownership, continuity, numeric and path checks. Reuse its proven extraction
  and reason logic, but pass the full original scenario/manifest/samples/events
  to the rich renderer rather than treating the lean display DTO as the flight.
- Extract a small shared HTML-rendering helper if needed so the V2 archive writer
  can persist rich pages create-only. Do not call an overwrite-capable generic
  writer against retained source paths. Keep `waypoint-v2-report` report-only,
  fresh-root and optionally single-case; generic refresh must not scan arbitrary
  research roots or execute missions.
- `pd-eval/src/waypoint_v2_output.rs` switches back to the rich renderer with the
  same annotations only after the preview is accepted. The full future and
  retained paths must share the adapter; summary schemas and flight policy stay
  unchanged.
- `pd-eval::report_catalog` owns the curated batch index and guidance scorecards.
  `ReportSite` owns the home and generic collection indexes, but must not overwrite
  the curated eval index during a generic refresh. Keep experimental report
  entrypoints visible in history without changing fixture-backed denominators.
  Test both refresh orders, including the separate conservative-ballistic writer.
- Replace the per-render V2 home-card scan with one selected collection link.
  Keep the small selection validation and collection generation in the evaluator;
  site navigation consumes the resulting report entrypoint, not V2 flight types.
- Retire the lean renderer as the default V2 output after replacement validation.
  Preserve its generated pages and receipts as history. Remove or repurpose only
  implementation known to belong to this pass, after reviewing the dirty tree;
  do not delete captures, useful checks or unrelated user changes.

No planner/core/controller change, numerical tuning, new mission matrix,
dashboard framework, generalized experiment database, global plot refactor,
new playback system or automatic current-result promotion belongs in this pass.

## Execution plan after approval

1. Freeze the dirty-tree ownership, pinned source and existing report hashes.
   Define the additive report-data parity check and exact handoff adapter before
   changing presentation. Review the catalogue/index ownership boundary.
2. Build a navigation preview using existing full-report destinations and one
   late-ridge rich report with H1, its time guide and recorded explanation, into
   a fresh preview root. Other links must identify original versus annotated
   preview views honestly. Do not advertise a fully enhanced suite yet.
3. Run focused data and real-browser checks on the preview, including all five
   old spatial modes, metric traces, hover, zoom/pan and navigation. Give the user
   one preview entrypoint and stop for review. Do not expand based solely on the
   primary's readability judgment or a screenshot pass.
4. After the user accepts the direction, cover direct/sloped controls, two/three
   corrections and finite/unsupported outcomes. Render all thirty supported
   full reports plus all 32 index entries from the same saved capture; wire the
   future writer and explicit selection mechanism, then repair refresh-order
   ownership and complete the navigation/history pages.
5. Run the integrated preservation, native and browser gates. Publish the chosen
   collection entrypoint only after these checks and the early user review.
   Ask for final navigation/report acceptance; review and commit separately
   when requested. Leave the existing server and archive roots intact.

These are two reviewable slices: navigation plus one rich annotated preview,
then complete integration. A later execution goal must reflect this updated
scope, not continue the old simplified-renderer objective unchanged.

## Validation and acceptance

### Navigation checks

Verify one current V2 home entry, explicit capture/policy identity, working
breadcrumbs and previous/next links, group/search behavior, all 32 entries,
unsupported link honesty and discoverable history. Test missing, incomplete,
unsafe-path and mismatched selection bindings; no silent current fallback is
allowed. Repeated renders must not create repeated recommendation cards.
Generic then catalogue refresh and catalogue then generic refresh must leave
the same curated eval navigation and unchanged maintained-scorecard totals.

The human navigation gate is whether the user can identify the current V2
capture, choose a direct/one-correction/repeated-correction case and return to
the collection without being told which timestamped directory to open.

### Full report and data checks

Compare parsed baseline and annotated report payloads for the same capture,
allowing only the new annotation/navigation fields. Add a non-V2 fixture with
empty annotations to prove unchanged rendering and available old controls.
Verify annotation count, chronology, exact unsampled boundaries, odd contact
clocks, ownership and next-cycle state. Corrupt or missing evidence must fail
explicitly, never render guessed H markers.

Use the retained late-ridge H2426, successive-rising H2006/H2774 and plateau
H2820/H3136/H3226 fixtures. Flat/uphill/downhill direct controls have no H
markers. Near-target NoClearing has zero executed segments/time; unsupported
gravity/vehicle have no flight report. Test synthetic partial/odd states only
as presentation tests, not new physical mission evidence.

Real-browser checks at approximately 1280 and 390 px must wait for Plotly and
both charts to render. Verify the five spatial modes, legend toggles, metric and
trajectory hover, zoom/pan, old detail disclosures, exact H markers/time guides,
keyboard correction selection, overlay toggling and absence of page errors.
The old lean-page assertion of no external assets is not appropriate here:
permit the existing pinned Plotly dependency and check that it actually loads.
Do not replace missing Plotly with a reduced report and call the rich-report
gate passed. Update the browser checker to cover these functions rather than
reusing its SVG-only selectors as proof.

Before and after rendering, verify the pinned 127 source JSON/HTML files and
historical report bodies are byte-identical. Preserve the lean prototype's
pages/receipts too. Generated navigation indexes are intentionally different:
whitelist the outputs/home/eval indexes and new V2/history entrypoints that this
pass is authorized to regenerate, and review their changes rather than claiming
every legacy-index hash must stay equal. No capture, batch summary, comparison
payload or original mission report may be overwritten.

Store new render input/output and shared-renderer provenance separately from
historical flight provenance. Native workspace, format and proportionate Clippy
checks apply after implementation, with the existing baseline lint disclosed.
No full flight matrix is needed.

The human report gate is whether the user recognizes the old detailed report,
can still inspect its rich data and can now understand where/why waypoint
handoffs occurred. Navigation acceptance and report acceptance are separate;
neither is replaced by automated tests.

## Design pass verification

The current source and served HTML were inspected for these findings. No new
prototype or annotated report has been built, and this design has no new
native/browser acceptance claim. Documentation links, fenced blocks and
whitespace checks pass. All 458 monitored source, fixture, script and
retained/generated artifact files match their pre-pass hashes.

The next action is review of this design, then approval of the first preview
slice, not another planner research pass.
