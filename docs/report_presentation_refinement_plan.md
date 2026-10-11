# Report presentation refinement plan

[Documentation home](README.md) · [Report workflow](reports.md) ·
[Common-template contract](planner_v2_common_report_templates_results.md)

Status: implementation launched, October 10, 2026, after the owner's explicit
"go for a goal loop, commit as you go." The
[run record](report_presentation_refinement_results.md) tracks validated milestones.
This document scopes the presentation and quality-of-life pass.
Reports are the lab's main inspection interface for individual flights and whole
experiments. Their usability is a product requirement, not optional polish.

## Goal and attention flow

Make it easy to answer, in order: what happened overall, which missions need
attention, what happened on this terrain, and what evidence supports that result.
Refine the maintained report components rather than introduce another report
system. Retain the established palette, tree interactions and rich detail pages.

| Surface | First impression | Main inspection | Secondary detail |
| --- | --- | --- | --- |
| Batch / campaign | Population, landing rate and outcome distribution | Focused mission tree with terrain/flight previews | Coverage, performance, configuration and receipts |
| Paired comparison | Each approach's outcomes on the same population | Same mission, different approaches, aligned previews | Stratified coverage, descriptors and pairing evidence |
| Flight detail | Outcome, actual stop and key endpoint/contact measurements | Prominent trajectory, terrain and linked time plots | Sample inspection, events, optional planner/controller diagnostics and raw artifacts |

Aggregate pages should lead with a compact overview and a visual outcome
breakdown, followed by shortcuts into the review tree. Coverage remains available
but does not occupy several screens before a mission can be inspected. Raw
identifiers and configuration are disclosure content. A compact proof status
stays visible, and failed or missing verification stays prominent.

## Observed regression and reusable work

The October 10 procedural comparison and campaign pages contain no flight
previews. The comparison tree ends at approach/family links; its coverage cells
jump to an approach rather than the selected family/direction/relief subset.
The campaign puts full physical hashes, raw option/enum strings and booleans in a
ten-column table. Configuration appears before the review tree. Individual
captures still use the rich trajectory/metrics template.

The earlier maintained reports already provide the desired foundation:

- [Common batch shell](../pd-report/src/batch.rs), styles and
  [tree engine](../pd-report/src/batch_tree.rs).
- [Saved-flight SVG renderer](../pd-report/src/lib.rs), including terrain, pad,
  actual flight and endpoint/handoff markers.
- [Focused stop review](reports.md#report-serving): previews beside mission links
  in a sticky first cell, direct branch links and scoped previous/next navigation.
- [Rich detail template](../pd-report/src/rich_template.rs), linked spatial/time
  plots, inspection controls, events and optional approach annotations.

The existing procedural screenshot confirms that a long coverage matrix consumes
attention before any mission is visible. The earlier batch/detail screenshots
demonstrate working previews and interactive plots. These retained screenshots
are baseline references, not acceptance of this proposed pass.

## 1. Restore previews and readable mission rows

First repair [campaign presentation](../pd-report/src/campaign.rs), without waiting
for the broader comparison and layout work.

Place a clickable terrain/flight preview with each mission link in the sticky
first cell. Keep outcome, stop, a compact verification status, flight time and
fuel visible. Put full hashes, descriptors, wall-time details and per-check flags
in accessible secondary details. Use explicit units and human-readable labels;
missing values remain unavailable rather than zero. Short identifiers must be
unambiguous and expose the full identity on inspection.

Use the existing marker vocabulary: success, physical crash, saved airborne stop
and no recorded departure. Label a native timeout separately from a finite
solver/guard stop. A rejected prediction is not a flown collision. Unsupported
or failed preparation may show bound terrain context but no invented flight.
Do not derive an unsupported disposition from the travel direction in the UI.

The current row projection contains neither samples nor a preview asset. Add an
optional, versioned presentation projection carrying a relative preview link and
the display metadata actually available from the retained capture. Older
projections must still render, with an explicit preview-unavailable state.
Do not embed unchecked HTML/SVG in evidence records or change historical schemas.

The read-only rendering adapter binds the original input/capture and receipt,
then supplies the existing renderer with recorded samples. Preserve or explicitly
label a saved final endpoint when it is absent from the sample ledger; do not
invent intermediate motion. Preview decimation is display-only and retains start,
end and recorded important markers. Original samples and command ledgers stay
unchanged. Preview generation must not execute agents, planning queries or replay.

Acceptance: an expanded procedural campaign branch shows readable mission links,
real previews and outcomes together, including at 390 px, without horizontal
scrolling to find the plot. Every scheduled failure remains represented.

## 2. Put outcomes and useful review actions first

Add compact, labeled outcome bars to the overview, using the complete scheduled
denominator. Present count and percentage together. For the frozen comparison,
the initial view should make zero learned landings, mostly learned timeouts,
raw-incumbent unsupported inputs and reflected-incumbent landings immediately
distinguishable.

Outcome segments form one mutually exclusive partition of scheduled attempts.
Preparation/capture failures, unsupported inputs and unverified results remain
visible; they cannot disappear from the chart. Integrity/replay status is an
additional dimension, not another segment that double-counts an attempt. Only
the existing complete landing tuple can be shown as a verified landing. Keep
unique missions, repeated attempts and separately labeled supported-slice rates
distinct. Mean/spread metrics retain their existing population definitions.

Use the existing semantic colors with labels, counts and marker shapes, so color
alone never conveys a result. Avoid a separate plot for every numeric column.
Coverage can use restrained rate shading with its denominator and unsupported
state shown explicitly; a zero-success cell must not look like a successful one.

Add count-bearing review shortcuts for recorded outcomes, unsuccessful attempts,
unsupported/execution failures and differing outcomes between selected approaches.
These focus the same tree rather than create separate curated dashboards. Keep
the full population one action away. Any start-here examples use a declared,
deterministic selection rule with visible scope and access to all matching cases;
they do not stand in for the aggregate.

Apply the desired section order through the shared batch renderer. Keep legacy
section IDs and navigation targets; first opt the procedural views into the
order, then reconcile other maintained views after their compatibility review.
Avoid adapter-specific duplicate page layouts.

Acceptance: at 1440 x 900, outcomes and review actions are visible in the first
viewport. At 390 x 844, the landing result and a working review action are visible
before the long tables. The selected branch follows directly; configuration does
not obstruct the main path.

## 3. Make paired mission comparison a real review tree

Extend [comparison presentation](../pd-report/src/comparison.rs) to provide
coverage group -> mission -> approach/attempt inspection. Group by the declared
family, direction and relief stratum. Keep an approach summary above the tree and
the existing campaign links as secondary navigation.

Join approaches by original physical identity and repeat occurrence, not row
position or display labels. Reject mismatched physical inventories/repeat counts
as today. Preserve each repeated attempt as a named child; never select the best
attempt or silently deduplicate it. Core cases, diagnostics, controls/repeats and
exposure roles stay separate where the source distinguishes them.

Each mission branch shows its terrain and each approach's outcome and saved-path
preview. Use the same physical coordinates and common plot bounds across the
approaches on that mission, including recorded endpoints. Individual automatic
rescaling must not make unlike paths appear equivalent. A five-trace overlay is
not the default; allow a bounded selected pair while every approach remains
accessible. Unsupported approaches still occupy their row.

Coverage clicks must focus the exact family/direction/stratum and selected
approach, rather than open an unrelated approach branch. Search and outcome
filters affect visible rows only; full-denominator aggregate counts remain
identified. Keyboard expansion, depth controls and direct URLs retain their
existing semantics. Returning from a detail restores the same branch/filter;
previous/next navigation stays inside the review subset, with a visible option
to return to the full population.

Acceptance: open one mission and compare RL with the incumbent without visiting
separate campaign pages or hunting for matching IDs. Direct links and back/next
round trips preserve the intended mission, approach and subset.

## 4. Refine detail pages without losing inspection tools

Keep the current rich detail template. Prioritize outcome/stop, trajectory and
the handful of measurements explaining the endpoint. For non-landings, display
actual target separation, motion, fuel and elapsed time when recorded; do not
label airborne endpoint values as touchdown quality. Horizontal offset alone
must not imply arrival. Distinguish reference height above upright touchdown
from attitude-dependent hull clearance.

Move long provenance text and serialized descriptors out of the leading banner.
Retain a concise visible verification status and obvious evidence links. Keep
the trajectory prominent on desktop and mobile; inspection panels should not
push it several screens down. Default to actual terrain and flown motion, with
optional reference/prediction layers clearly distinguished and easy to reveal.
Do not remove spatial modes, linked hover, velocity/thrust plots, events, actual
handoffs, planning-cycle selection, decision navigation or full telemetry.

Keep the leading outcome and a saved-flight static preview readable if the
interactive plotting library cannot load. Show an explicit chart-unavailable
message, preserving evidence links; a blank plot must not be the only account of
the flight. Reuse the saved SVG instead of introducing another chart library.
Include text alternatives, visible keyboard focus and labeled controls.

Provide batch/mission/approach breadcrumbs and review-subset previous/next links
through the common annotation/navigation interface. Generic agents need not
invent controller phases or planner decisions. Where training diagnostics are
available, place learning curves and reward components in a separate labeled
experiment section; reward is never presented as verified landing success.

Acceptance: a timeout's elevated endpoint, a physical crash, a landed terminal
case and a finite planner stop are distinguishable from the leading summary and
plot. Existing plot/decision/inspection interactions still work.

## 5. Shared ownership, scale and delivery

`pd-report` owns shared presentation, preview geometry, display labels and
interactions. Read-only application adapters own capture loading and original
navigation/evidence references. Evaluation owns proof and accounting. Learning
packages supply experiment metadata, not a competing report template. Maintain
the all-features dependency boundaries; reporting imports no concrete strategy.

Serve static derived assets. Prefer lazy-loaded compact SVG previews and reuse a
mission's bound terrain rather than inline every full flight into a 2k-mission,
five-approach page. Read each capture once during rendering, avoid per-row
collection hashing, and record rendering time, page/asset sizes and browser
interaction timings. At 2,048 missions, the first view, branch expansion and
filtering must remain responsive; do not load all trajectories at page startup.
Measure before adding pagination or replacing the established tree engine.
Measure at least five warm branch/filter interactions on a recorded browser and
machine, targeting a median below 250 ms; investigate misses before broadening
the UI change. Record cold first-load time separately and confirm that unopened
branches do not fetch their complete flight data or all preview assets.

Produce one linked, create-only report edition in an existing discovery scope
such as `outputs/reports/eval/procedural-learning/EDITION/`. All new aggregates and
detail navigation belong to that edition, while source links point to original
captures. Record source/renderer/input/page hashes in a rendering receipt.
Inspect any nested discovery rules before choosing final paths, and keep
individual detail pages out of the report catalogue.

Do not overwrite the frozen procedural report or accepted historical pages.
Report rendering is distinct from new flights, fresh proof or benchmark promotion.
Preserve fixture inputs, original capture/receipt bytes and accepted selectors.
The ten existing navigation/server worktree paths remain owned by their writer;
this pass does not require rewriting their discovery or server implementation.

## Execution and acceptance sequence

Implement sections 1 through 4 as reviewable milestones, each including focused
tests and a before/after browser review. Section 5 governs all milestones. The
first milestone is a usable campaign repair, not another shell-only integration.
Complete the paired view and shared-detail checks before calling the pass done.

| Check | Required evidence |
| --- | --- |
| Accounting and evidence | Displayed counts match original projections; chart partitions sum to scheduled attempts; unsupported, invalid and missing-proof cases stay visible; repeats remain distinct |
| Preview correctness | Bound terrain/flight and actual endpoints match saved data; unsupported inputs have no invented path; paired previews share bounds; deterministic decimation preserves important markers |
| Attention and readability | First-viewport criteria above; useful branch with visible previews; no raw enum/boolean/hash wall; no page-level horizontal overflow at 390 px |
| Interactions | Mouse, Enter/Space, expansion/collapse, exact coverage focus, outcome filters, search, direct links, batch/detail/back and scoped previous/next |
| Rich detail compatibility | Spatial modes, hover synchronization, events, handoffs and applicable decision/planning-cycle controls work; blocked plotting dependencies retain outcome/static preview; embedded physical payloads match original captures |
| Scale | Timed 2,048-mission/five-approach view; deferred preview loading; bounded page payload; no repeated whole-collection work per row |
| Historical compatibility | Terminal, simple transfer, authored waypoint, native V2, procedural RL and incumbent examples retain their applicable views and honest status-only pages |

Use small maintained synthetic reporting inputs for ordinary tests: verified
landing, crash, native timeout, finite airborne stop, zero-step stop, unsupported,
preparation/capture failure, missing/failed verification and repeated physical
inputs. Include older projections lacking previews. These tests must not require
the retained output archive or a study/game checkout.

Then render representative saved reports into the new edition and exercise the
real 2,048-mission comparison. Extend the existing
[browser helpers](../scripts/planner-v2-browser-helpers.mjs) and
[tree checks](../scripts/test-batch-tree-focus.mjs), with explicit assertions for
preview presence and visibility rather than only common-shell class names.
Inspect screenshots at 1440 x 900 and 390 x 844: overview, focused tree, paired
mission and rich detail. Browser assertions and human visual acceptance remain
separate; retain screenshots for review.

Run focused reporting/Node checks, the maintained development gate and docs/diff
checks. No ordinary check requires retained 44-case parity or a new flight
campaign. Preserve the source worktree and recorded numerical inputs/outcomes.
Update the maintained report/evaluation workflows with implemented behavior.

## Launch and preservation boundaries

The owner has authorized local implementation, create-only derived report editions,
browser validation and validated commits in the shared checkout. Deliver the
rendered edition, rather than stopping after source-only changes. Ordinary
source-only synthetic tests are included. No training, mission campaign, fresh
proof replay, push, accepted-page/selector replacement, server lifecycle change,
delegation, worktree or broad cleanup is included. Existing captures and
other-writer navigation/server paths remain protected.
