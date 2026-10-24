# Mission comparison and flight inspection refinement

Status: authorized October 24, 2026. The user requested the execution goal loop.
The [execution record](report_comparison_refinement_execution.md) owns its slices
and validation; the scope and acceptance below govern this pass.
The preceding [study reporting pass](report_study_qol_execution.md) supplies the
retained timing and learned-ignition evidence and existing shared components.

## Intended result

Keep mission-first navigation and the established bounded tree. Add persistent
comparison and target context throughout study, group, mission and flight views:
what changed, whether each metric improved or regressed, and which declared
requirements were met. Opening a pair should explain its recorded behavior with
shared spatial scales and a linked time cursor, while retaining full individual
rich inspection and provenance.

The delivered study, tree, paired viewer and individual rich views should be
usable through the existing `8787` publisher. Complete raw native captures may
remain explicit archival downloads on the lab endpoint. Routine interactive
browsing must not depend on `8000`.

This is presentation work on saved evidence. Flight logic, controllers, clocks,
commands, fuel, contact/evaluation rules, training, replays and frozen claims
remain unchanged. No new scientific evaluation, default promotion, push, service
configuration change or server lifecycle operation belongs to this pass.

## Comparison and target contracts

Treat the following as separate references, with stable identities visible in the
UI and preserved in links. A report's modification time never selects a baseline.

| Reference | Meaning | First delivery |
| --- | --- | --- |
| Strategy baseline | Another supplied approach on the same originals | Existing 63 versus 132 timing and heuristic versus RL contrasts |
| Earlier run | Explicitly pinned saved run and approach | Import pinned sources and show compatibility and inventory accounting; no automatic history discovery |
| Declared target | A sourced requirement for a case or cohort | Existing native landing contracts and frozen timing qualification; undeclared fuel/cost budgets remain undeclared |

The comparison declaration binds baseline/candidate source records, approach
descriptors, input/cohort roles, evaluation/plant semantics, metrics and target
references. Intentional controller/checkpoint differences are named changed
factors; compatible physical and scoring semantics remain fixed.

Pair by physical input and scheduled occurrence within an explicitly compatible
cohort. Preserve repeats without choosing the best. Do not join by display name,
mission number or seed alone. Account for baseline-only, candidate-only,
unsupported, unverified and incompatible records. A cross-run join shows both
source totals and its matched denominator. Independent populations support
labelled aggregate summaries, not invented per-original gains or losses.

Reuse a versioned neutral presentation projection in `pd-report`. Preserve V1
readers and legacy pages; introduce a separately versioned extension if the
required fields cannot be added compatibly. Evidence/campaign formats do not
change to accommodate presentation. A missing or invalid requested baseline is
disclosed rather than silently replaced by another comparison.

Keep two independent judgments:

- Change: better, worse, exact tie, descriptive, or unavailable, per declared
  metric direction. Classification uses unrounded values. A material-change
  tolerance applies only if explicitly declared; tiny differences remain visible.
- Target attainment: met, unmet, undeclared, unavailable, or not applicable, with
  its reason. A target declares metric/units, stage, operator/range, scope,
  aggregation and source contract. Cohort confidence requirements retain their
  original method, population and frozen result.

Native incoming-contact values and native margins supply touchdown assessment;
post-contact zeroed velocity cannot stand in for impact state. Fuel comparisons
still require two verified landings. Cost timings retain their separate exposed
12-original, seven-measured-repeat denominator. A plot selection never produces
a new qualification, interval or performance measurement. No single combined
score may trade a lost landing against saved fuel.

## Product behavior

The study overview shows a compact comparison summary: pinned A/B references,
outcome transitions, metric changes and declared target attainment. Existing
question, changed/fixed factors and frozen claims stay available. The mission
hierarchy, its group order and twelve-occurrence pages remain intact.

Each group shows its comparable/eligible counts and outcome transitions. Each
mission shows A to B outcomes, both values, signed difference and applicable
target status. Selecting regressions or unmet targets uses the existing tree
predicate and preserves its context through every detail/back/next link. Define
regression explicitly as lost landings or worsening of the selected metric;
provide separate filters rather than hiding a multi-metric ranking in one label.

Mission 011 is a concrete acceptance example: two verified landings, fuel used
588.8738237969719 to 536.5208704774468 kg, coast 3 to 5 s and burn 16 to 12 s.
Its fuel improvement is separate from landing-contract attainment. An undeclared
fuel budget does not become a new historical target.

Add a comparison mode inside the maintained flight inspection workflow, reached
from a mission and from either individual detail. Desktop has baseline/candidate
trajectory panes with identical world limits and colour domains; mobile stacks
them using the same physical scales. Show outcomes, fuel, contact and recorded
decision differences beside them. Full individual rich views retain their
inspection, phases, forecasts, events, proofs and evidence links.

The pair consumes the original serialized rich sample payload through a shared
presentation accessor/loader. Do not embed two complete interactive documents
with colliding DOM IDs, scrape visible chart geometry as scientific input, or
build a second renderer with different state/command semantics.

Default synchronization uses elapsed mission time on the original clocks. A
shared cursor connects trajectory position, actual/commanded attitude and
commanded throttle. Coasting remains visible. When one flight ends, mark it ended
at its recorded time; do not extend its samples or imply continued thrust.
Burn-relative alignment, playback, path overlays and dense vector ribbons are
deferred until the basic paired inspection is accepted.

## Preview and direction display

For `reference-primary-011`, the path is centered inside its 156-pixel SVG, while
the 188-pixel preview card is offset inside its 300-pixel table cell. Correct card
alignment and balance the ground, scale and labels as well as camera framing.
Single previews fit their complete flight plus target/goal anchors. Paired
previews share the union of the selected A/B flights and anchors, with equal
metre scales. Neither mode clips a genuine excursion to improve its appearance.
Expose the frame/scale choice; preserve left/right normalization and scale bars.

Fix vector attachment so the recorded vehicle position is the arrow tail and
the head extends along the actual attitude. Pixel/data conversion must work for
all directions, equal-axis scaling, zoom and resize. Retain the current limits
of 24 px at full command and at most 18 displayed glyphs.

Replace the chronological spatial-only greedy selection with a bounded sample
over the whole powered interval, prioritizing ignition and marked changes in
direction/command. Arrows supplement complete histories; a vehicle rotating
in place must still have its direction changes visible in the history.

Use two aligned histories: actual versus commanded attitude, and commanded
throttle with optional horizontal/vertical command components. Show angle units
and native sign convention, handle wrap without a fabricated full revolution,
and draw held commands as steps. Both histories share the cursor and recorded
phase/event times. Keep command, actual attitude and net acceleration distinct.
Physical thrust/acceleration reconstruction is deferred; the first display is
explicitly command-based and does not claim measured force.

## Portable delivery feasibility

The current complete editions total about 5.94 GB and 14,752 files. The current
publisher limits each bundle to 100 MiB and 2,000 files. Simply moving links or
gzipping standalone pages is not a demonstrated solution.

Before a complete rerender, measure lossless shared-template/sample packing on
the retained full inventories, with decoded HTML/sample equality checks. Reuse
compatible maintained packing components where available. The existing
untracked `scripts/saved_terminal_batch/` work is separately owned: preserve it
byte-for-byte and avoid an uncommitted runtime dependency. Any adoption needs
an explicitly owned, source-bound integration.

Prefer one published UI bundle per study. If necessary, permit immutable data
bundles on the same `8787` publisher, with receipt-bound URLs in the study
manifest. The visible entrypoint stays the study; users do not navigate shards.
Use small lazy-loaded packs, targeting at most 8 MiB compressed per data pack.
Initial tree load contains index/summary data, not every flight. Opening a pair
loads only its required packs. Transport layout and scientific identity remain
separate; URL rewriting cannot alter source values or comparison membership.

Bundle the decoder for plain HTTP operation. Preserve static spatial/metric
fallbacks when Plotly cannot load and a clear load-error state when a pack is
missing. Do not claim a portable offline raw-capture archive: archive download
links disclose their separate endpoint and availability.

Recheck publisher limits and storage at execution start. Do not raise limits,
delete existing publications or change services to make a bundle fit. If complete
portable UI delivery cannot fit the declared allocation, record the exact size,
compatibility or storage blocker; stop that delivery gate without calling the
whole pass complete or quietly retaining a routine `8000` dependency.

## Implementation sequence

| Slice | Concrete result | Gate before broadening |
| --- | --- | --- |
| 1. Source inventory and contracts | Frozen workspace/source inventories, pinned comparisons/targets, compatibility and cross-run join receipt | Existing matrices and all source roles reproduced; explicit missing/incompatible accounting |
| 2. Packaging feasibility | Measured complete-inventory packing, bundle plan and decoded-payload identity receipt | Fits publisher/file/storage bounds or produces a bounded delivery blocker before complete rerender |
| 3. Shared visual prototype | Centered preview, corrected arrows, linked histories and paired mode on a labelled small saved subset | Real browser geometry, cursor/end behavior and full individual inspection work on desktop/mobile |
| 4. Comparison integration | Sourced target states, A/B values, group counts and regression/unmet-target filters in the existing views | Context survives selection, pair/single switching, reload and matched navigation; frozen claims remain fixed |
| 5. Complete editions and delivery | New full timing/ignition editions, portable UI bundles, independent audits and publication receipts | Complete acceptance below, served-byte checks and source/foreign-work preservation |

Commit completed implementation slices locally during a separately authorized
execution loop. Freeze the final renderer/library and adapter before complete
rendering. Failed/reserved outputs stay create-only; corrections use new roots.
Do not refactor controller or experiment execution modules for presentation.

## Acceptance

- Preserve the complete timing inventory: 1,285 flights, 445 scheduled original
  occurrences, 441 physical inputs; primary 300, boundary 60, retained handoffs
  21, development 60 and pilot four remain distinct. Preserve 72 exact ties and
  the source fuel intervals and cost measurements.
- Preserve the RL 600-original, four-variant inventory: 443 preserved, 146 gained,
  three lost and eight shared misses; loss selection is exactly 355/436/514 and
  conditional fuel has 443 eligible pairs.
- Audit every rich numeric sample, clock/state/command binding and native proof
  against saved sources. Packing round-trips the new canonical rich HTML exactly,
  including numeric token spelling. Presentation changes do not rewrite history.
- Exercise a same-source earlier-edition comparison with zero scientific
  changes, plus changed/missing/incompatible-run fixtures. Prove improvement
  and target attainment can disagree, and missing targets never become passes.
  A real earlier behavioral run is shown only when the inventory finds one with
  a valid comparison contract; its absence is disclosed.
- Check mission 011 card/path alignment, tilted/shallow arcs, a large excursion,
  exact/sub-display differences, sparse schedules, finite stops, zero command,
  angle wrap, rapid rotation at low displacement and unequal end times.
- Check arrow tail and head geometry against the intended vector, not merely
  annotation count/length. Verify histories/cursor values at native sample times
  and actual command boundaries. Static fallbacks remain usable.
- Browse every original through the bounded tree at 1440 by 1000 and 390 by 844.
  Decode/paint visible lazy previews before screenshots; inspect representative
  frames. Preserve keyboard access, context, full rich tools and legacy no-study
  behavior. Compare pages have no viewport overflow.
- Record five warm filter/group/page trials per viewport, including paint.
  Target a median below 250 ms on the checked host; report first/warm detail load
  separately. Measure transferred bytes and verify startup does not fetch the
  full corpus. Report these as host/browser evidence, not a device guarantee.
- Verify every published UI/data asset's served bytes, direct deep links and
  fresh navigation with the `8000` origin blocked. Separately verify explicitly
  archived source links. Keep decoder and blocked-Plotly failures honest.
- Run the maintained developer gate and targeted semantic/geometry/packing
  checks. Preserve initial foreign Git state, prior report/capture hashes and
  service identity. Record final commits, source epochs and delivery receipts.

## Proposed execution bounds

Use a fresh allocated root with workspace/report hashes and a source epoch.
Recommended outer allowance: four hours, 14 GiB new local artifacts and a 40 GiB
free-space floor; zero campaign executions, source replays or training updates.
Publication allowance: at most 1 GiB new snapshots across no more than 16 bundles,
each respecting the existing publisher limits and available store allowance.
Publisher snapshots and export staging count toward the artifact budget.

Prototype before complete rendering. Permit one corrected complete edition only
if remaining time/bytes allow it. Budget, source drift, proof mismatch or failed
delivery gates retain their receipts and stop the affected phase. Ordinary
software tests remain distinct from new measured flight evidence.

## Deferred

Automatic experiment-history discovery, training dashboards, controller changes,
physical thrust reconstruction, new reliability/efficiency targets, newly fitted
statistical tests, autoplay, phase-normalized comparisons and infrastructure
migration are separate passes. Existing dated plans remain historical records;
this proposed pass does not reactivate them.
