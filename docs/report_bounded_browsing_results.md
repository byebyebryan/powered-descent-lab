# Bounded procedural mission browsing

[Presentation workflow](report_presentations.md) · [Mission hierarchy correction](report_mission_hierarchy_results.md) · [Documentation home](README.md)

The owner reported that **Show missions** expanded the whole procedural
collection, loading many previews and creating an impractically long page.
Native image lazy loading did not bound the number of rendered mission rows.

The collection tree now opens one terrain stratum at a time and presents twelve
scheduled mission occurrences per page. **Browse missions** opens the first
matched stratum while retaining the family/direction/stratum summaries.
Opening another stratum closes the previous one. A stratum selector and
previous/next controls navigate the matched strata; page controls appear above
and below the table. Search reveals the first matching stratum. Summary and
coverage counts retain the full filtered denominator, independent of paging.

The URL retains the active stratum and page. A detail return reveals the exact
selected occurrence, including when previous/next crosses a page boundary.
Old expand-all links materialize only one stratum. Physical identity pairing,
repeat occurrences, controller attempts and rich detail tools remain intact.
The common procedural preview loader uses viewport intersection rather than
assigning image URLs to every row with layout. It also serves campaign companions;
their existing common tree controls remain unchanged. Synthetic trees are unchanged.

## Travel direction

Direction is a declared coverage axis in the generated collections. Steering
must follow its sign, but globally mirrored worlds should admit mirrored control
solutions without separate direction-specific tuning. The raw planner-backed
incumbent currently admits only forward source-to-target geometry. The retained
comparison therefore contains 1,024 unsupported leftward inputs; its rightward
slice has 984/1,024 verified landings. The reflection adapter transforms the whole
leftward scenario and maps commands back to the original world: it lands
989/1,024 leftward and 984/1,024 rightward inputs. PPO normalizes signed horizontal
observations and maps commanded attitude back by travel direction.

These leftward/rightward sets contain different generated physical inputs;
their result difference is not a paired mirror experiment. Direction remains
useful for coverage and implementation asymmetry checks. This browsing repair
retains the existing hierarchy and does not introduce direction-specific control.
Source boundaries are in [the native admission checks](../pd-planner/src/planner_flight/input.rs),
[the reflection adapter](../pd-benchmark/src/lib.rs) and
[the PPO environment](../experiments/rl/pd_env.py). The
[learning run record](procedural_learning_results.md) retains complete outcome
and replay accounting.

## Delivered edition and validation

The [full bounded browsing edition](http://192.168.1.110:8000/reports/eval/procedural-learning/bounded-browsing-20261011T024857Z/report.html)
is a create-only presentation of the same 2,048 physical missions and 10,240
scheduled attempts. It retains 9,216 executed rich details and 1,024 unsupported
statuses. Rendering performs no new agent flight or independent replay.

Evidence is retained under `outputs/report-refinement/bounded-20261011T024128Z/`:

- All 18 maintained development checks passed; report tests pass 47/47 and
  presentation model tests pass 9/9, including complete ordered paging of repeats,
  off-page occurrence selection and invalid/empty page bounds.
- `browser-final-3/browser-check.json` passes at 1440 × 900 and 390 × 844, with
  nineteen screenshots and no ordinary browser errors. At each viewport, all
  2,048 occurrences are visited through sixteen strata in their original order,
  with a maximum of twelve rendered mission rows. Bulk browsing opens one
  stratum and assigns URLs to five nearby mission previews in the recorded view.
  Its table height stays below 3,200 px; these are local Chrome measurements.
- Page reload, old expand-all links, cross-page detail return, exact coverage
  focus, rich interactions, campaign companions, home/library navigation and
  deliberately blocked Plotly fallback pass. Five warm trials per viewport
  include two animation frames; median branch/filter times are about 33 ms.
- `preservation-final.json` confirms identical saved campaign projections,
  mission/repeat identities and all 9,216 physical/payload/preview bindings.
  All 37,948 original source files retain their sizes and modification times.
  All ten unrelated navigation/server paths are byte-preserved. The existing
  report server remains running without a lifecycle change.

Previous presentation editions remain retained as history. This change repairs
collection browsing; it does not change outcomes or establish new flight evidence.
