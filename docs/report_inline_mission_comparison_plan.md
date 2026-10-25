# Inline mission comparison

This is the implementation contract for the separately requested Luna goal loop.
It follows the completed [comparison refinement](report_comparison_refinement_execution.md)
and the review of the established tree. The user wants comparison visible in
that tree, with the same design supporting reports without a comparison.

## Intended behavior

Each visible mission entry leads with its identity and the declared comparison,
then the selected runs' previews and aligned data. A comparison shows A/B previews
side by side, two labelled run rows, the outcome transition and eligible selected
metric difference. These require no mission expansion or separate inspection
page. A single-run report has one preview and one run row, without A/B labels,
invented reference, delta or empty second column.

Comparison is an explicit presentation capability, not inferred from the number
of attempts. A missing side of a declared comparison stays visible as missing or
unscheduled; it must not silently become a single-run report. Unsupported or
unverified inputs retain their separate evidence status. A failed landing with
less fuel use cannot become an eligible efficiency improvement.

Mission 011 is a useful overlap check: both runs follow a vertical line, but the
63-pair reference coasts/burns for 3/16 s and uses 588.8738237969719 kg; the
132-pair reference uses 5/12 s and 536.5208704774468 kg. Aligned values must explain
differences even when geometry overlaps. Recorded coast/burn timing may have a
small aligned timeline when those source quantities are declared. Missing timing
does not become zero, and a requested or rejected burn is not actual ignition.

## Presentation and compatibility

Side-by-side previews are the first default. Overlay is deferred; it is not a
prerequisite for this pass. Both paired previews use the same physical camera,
aspect ratio, direction normalization and scale. Preserve full paths and targets;
do not independently fit the images or add visual offsets to separate overlap.
Single views retain a suitable focused camera. Run identity uses labels as well
as consistent color; outcome color remains distinct from A/B identity.

Keep the family/group/mission hierarchy, ordering, twelve-occurrence paging,
lazy previews, filter membership and provenance. Group summaries identify which
run or comparison they describe. A comparison's preview, values, labels and delta
all resolve the same pinned runs, even after changing selectors or restoring a
URL. Many attempts without a declared comparison remain a single-view capability.

Expanding a mission exposes other attempts, provenance and on-demand recorded
time inspection. Reuse the existing exact-payload flight inspection component.
Do not initialize full flight packs or heavy history charts for every list row.
Keep complete rich-detail links and existing pair URLs as compatible optional
deep links. Routine review and inline inspection use the existing 8787 transport;
explicit raw archives keep their disclosed source endpoint.

Use maintained neutral report components. Optional presentation metadata and
structured display values can be additive; do not derive scientific metrics by
parsing HTML cells or rewrite native state/command/proof payloads. Legacy inputs
with missing metadata remain readable with honest unavailable values.

## Work and validation

Luna Max investigates and implements a bounded shared-renderer slice, with
explicit ownership and focused tests. The primary settles contracts, handles
portable edition integration and documentation, reviews actual diffs and checks
the browser and overall exit gate. Do not change worker-owned files without a
handoff. Preserve all unrelated dirty and untracked work.

Create new presentation exports from retained evidence. Reuse the nine existing
immutable timing/RL data publications where possible, rather than duplicating
multi-gigabyte canonical editions. Any template display patch binds its exact
source and stays outside the scientific payload. No previous edition or receipt
is replaced. Publish only bounded dedicated UI exports through the existing
shared service; no service/configuration change, deletion or push is authorized.

Acceptance includes:

- Both complete saved inventories remain available: timing 1,285 flights / 445
  scheduled occurrences and RL 2,400 flights / 600 originals. Cohort roles,
  transition matrices, eligible pairs, exact ties and frozen conclusions match.
- Inline A/B previews and values are visible without another page. Test overlaps,
  divergent paths, gained/lost/missed cases, missing/unsupported sides, exact ties,
  and the explicitly same-capture earlier-edition reference.
- Genuine single-run, multiple-attempt-without-comparison and legacy reports show
  one selected run clearly. New comparison capability does not require study data.
- Inline time inspection uses native sample clocks and stop times, ends commands
  correctly and has explicit load errors. Full rich tools, navigation context and
  old pair deep links continue working.
- Desktop 1440 x 1000 and mobile 390 x 844 layouts are readable, aligned and
  contained. Preserve focus and native semantic controls. Verify visible previews
  are decoded before screenshots and no full flight pack loads before inspection.
- Keep one bounded active group and twelve mission occurrences per page. Record
  painted filter/page timings and request bytes. Verify the served final UI with
  the 8000 origin blocked, plus explicit decoder/pack failures.
- Run focused semantic/rendering checks, independent source/payload preservation,
  the maintained 19-step developer gate after source freezes, docs links and diff
  whitespace. Keep original workspace/report hashes and service PIDs unchanged.

## Operational allowance

Allocation: `outputs/report-inline-tree/loop-20261025T031425Z`, initial head
`8fbdb8d`. The baseline binds 985 Git-visible files and 18,441 retained report
files. Use three hours, at most 2 GiB of new local artifacts including publication
snapshots, at most 256 MiB / eight new publications, and a 40 GiB free-space floor.
Existing data bundles may be referenced without republishing them. Preserve failed
or reserved roots. Campaign flight executions, source replays and training are
zero; ordinary developer tests retain their software-only scope. Commit completed
slices locally within the user's continuing authorization.
