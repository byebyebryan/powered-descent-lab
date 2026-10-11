# Travel direction as mission metadata

[Presentation workflow](report_presentations.md) · [Bounded browsing](report_bounded_browsing_results.md) · [Documentation home](README.md)

The owner recalled that travel direction varies with the seed and that previews
normalize left/right. The terminal suite explicitly resolves side from the seed
rather than treating it as a primary mission axis. Saved preview plots transform
terrain and flight together around the target. This supports a simpler mission
hierarchy: terrain family → terrain stratum → mission.

Both travel directions now share each stratum in the procedural comparison and
campaign companions. Mission rows show **Travel left** or **Travel right**;
**More filters** optionally restricts direction and defaults to all directions.
The full retained comparison has two families and eight merged strata, each with
256 scheduled occurrences: 128 leftward and 128 rightward. Within each stratum,
missions follow mission-number order. Browsing still opens one stratum and renders
at most twelve mission rows per page, with previews loaded near the viewport.

Coverage retains its original sixteen exact direction-specific groups. A coverage
cell focuses the merged stratum plus its direction and approach. Previously saved
coverage URLs resolve to the same subset. Direction combines with search,
outcome and approach; changing it resets the page and selected mission. Detail
previous/next follows the same matched mission order and retains the filter on
return. Mission IDs, repeat identities and coverage memberships remain stable.
Campaign companions compose direction and outcome filters, show a matched-attempt
count, and label their summary totals as covering the full collection. The home
and library retain mission-first navigation; their procedural description now
names family and terrain stratum.

Direction remains useful for checking coverage and implementation asymmetries.
The raw planner-backed approach still rejects leftward source-to-target geometry;
the reflection adapter transforms the whole world and maps commands back. The
PPO environment also normalizes signed horizontal observations and commanded
attitude. These existing behavior boundaries remain unchanged. Different leftward
and rightward generated inputs do not form a paired mirror experiment. See the
[previous direction analysis](report_bounded_browsing_results.md#travel-direction),
[terminal suite](terminal_suite.md#side-handling),
[preview implementation](../pd-report/src/lib.rs), and
[learning results](procedural_learning_results.md).

## Delivered edition and validation

The [direction filter edition](http://192.168.1.110:8000/reports/eval/procedural-learning/direction-facet-20261011T054435Z/report.html)
is a create-only presentation of the same 2,048 physical missions and 10,240
scheduled attempts. It retains 9,216 executed rich details and 1,024 unsupported
statuses. It uses saved evidence and performs no new flight or independent replay.

Validation receipts are retained under
`outputs/report-refinement/direction-facet-20261011T054435Z/`.

- All 18 maintained development checks pass, including strict Clippy, formatting,
  workspace/CLI boundaries, documentation links and all 80 Node tests.
  Report tests pass 47/47; presentation model tests pass 11/11, including direction
  intersections, legacy coverage scope, repeated occurrences and matched detail
  neighbors.
- `browser-final/browser-check.json` passes at 1440 × 900 and 390 × 844, with
  21 screenshots and no ordinary browser errors. Both viewports visit all 2,048
  occurrences through eight strata, with a maximum of twelve rendered mission
  rows. The recorded bulk view loads four nearby preview URLs; its table is under
  2,740 px high. Optional filters, empty vertical selection, old coverage and
  expand-all links, page reloads, cross-page detail returns, campaign companions,
  home/library navigation, rich interactions and blocked Plotly fallback pass.
  Median warm branch/filter timings, including two animation frames, are about
  33 ms. These are local Chrome measurements.
- `preservation-final.json` confirms identical saved campaign projections, stable
  mission/repeat/coverage bindings and all 9,216 executed physical/payload/preview
  bindings. All 10,240 preview SVGs are byte-identical to the preceding edition.
  All 37,948 original files retain their sizes and modification times; all ten
  unrelated navigation/server paths are byte-preserved.

Previous editions remain available as history. The existing report server stays
running; home/library refresh changes only navigation views. Flight outcomes,
physics, controllers, planners and retained proof boundaries remain unchanged.
