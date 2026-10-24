# Study reporting execution

This pass follows the [reviewed scope](report_study_qol_plan.md). Initial head is
`9033afd`; all foreign Git changes and source report inventories are bound by
`outputs/report-study-qol/loop-20261024T212129Z/workspace-baseline.json`.

The source timing edition contains 1,285 rich flights on 441 physical inputs,
currently represented as separate rows. The source coast/ignite edition has
600 paired original arcs and four variants. This pass changes presentation only.
Original evidence, frozen claims, controller behavior and services stay fixed.

## Shared rich graphics

The colour layer now groups every original segment into at most 32 colour traces
with explicit separators and simplification disabled. Full hover data is retained.
Throttle uses 0–1, and a presentation-only speed-domain slot permits shared
comparison bounds. Plain green trajectories are hidden in scalar modes.
Command/attitude vector glyphs use at most 24 px length, 42 px minimum anchor
spacing and 18 glyphs, recomputed for viewport/zoom changes.

The original failing dense flight now renders 2,628 positive-length coloured
paths instead of none (stationary/subpixel-rounded paths can remain zero), with
29 visible colour bins and 73 total traces instead of 5,292. Its vector view has
seven arrows with maximum length 19.44 px, rather than 28 arrows up to 116 px.
Desktop screenshots confirm both repairs. Three display-model tests and all
60 report-crate tests pass; formatting and whitespace checks pass.

An initial pilot passed a relative output path and stopped at the existing
absolute-prefix guard. Its reserved directory remains untouched; the successful
create-only v2 pilot uses an absolute path. No evidence execution occurred.

## Neutral study projection and paired navigation

The optional version-one study projection binds variants, contrasts, roles,
metric meanings, frozen claims and source observations to the existing paired
mission inventory. It rejects mismatched originals, duplicate attempt bindings,
unbound source digests and incompatible effect/performance denominators.
The renderer derives observed values from saved native records, including
incoming contact rather than the final clamped state.

The shared tree keeps bounded mission browsing. Study filters intersect its
predicate; A/B preview choices follow the declared contrast. Sparse schedules
show `Not scheduled` rather than claiming a miss. Context and pair links are
retained in rich details and matched previous/next navigation.

The saved RL pilot includes six originals spanning all four outcome transitions.
Selecting lost landings produces exactly 355/436/514; its detail links preserve
the heuristic/RL comparison and next moves from 355 to 436. Frozen claim content
does not change. The timing pilot includes three originals and all three timing
variants. Both pilots are labelled software subsets, without new qualification.
All 62 report tests, 22 relevant Node tests and targeted strict Clippy pass.

## Complete study editions

The [visual entry page](http://starship:8787/reports/84f3068e6580bfb059053a19bdab1284e5ee5232f8943a3e3f56b71402e1efdb/)
links the complete integrated views:

- [Timing study and mission tree](http://starship:8000/timing-study-review-v2-20261024T212129Z/report.html):
  1,285 rich flights, 445 scheduled original occurrences and 441 physical inputs.
  Its default 300-original confirmation cohort is separate from 60 boundary
  inputs, 21 retained handoffs, 60 development originals and four repeated pilot
  originals. All three variants land on the 300 confirmation inputs. The default
  63-to-132 comparison retains 21.976 kg lower mean fuel, 72 exact ties and the
  original paired interval. The cost chart uses 12 exposed originals with seven
  measured repeats per variant, preserving every source measurement.
- [Learned ignition study and mission tree](http://starship:8000/ignition-study-review-v2-20261024T212129Z/report.html):
  2,400 rich flights on 600 originals with four variants. Heuristic-to-RL remains
  443 preserved, 146 gained, three lost and eight unsuccessful on both sides.
  Lost originals are exactly 355, 436 and 514. Fuel requires the 443 pairs with
  two verified landings. Isolated latency is explicitly unavailable.

The first complete editions exposed mobile min-content overflow in the outcome
matrix. A shared CSS correction constrains grid children and wraps matrix labels.
Both original editions remain intact; v2 is the delivery edition. Source epoch B
at `0227991` binds the native library and render adapter; the later acceptance
helpers and delivery tools do not alter rendered production source.

## Acceptance and delivery

`release-b/timing-audit.json` and `release-b/rl-audit.json` independently check
every source evaluation and decision against the original report-bound hashes,
physical and complete replay status, reconstruction proofs, all displayed sample
clocks/state/commands, derived metrics and shared colour domains. They recompute
the transition counts and paired means; timing intervals and benchmark arrays
must equal the original analysis. No evaluation or replay is executed.

Four complete viewport checks pass at 1440 × 1000 and 390 × 844, recorded under
`release-b/browser-accepted/`. Each traverses all 445/600 scheduled originals,
keeps frozen conclusions unchanged during selections and follows paired detail,
previous/next and back links with the comparison preserved. The timing checks
also verify 72 exact-zero pairs, 21 unscheduled regression comparisons and the
132-cell recorded forecast grid. Visible lazy previews are decoded and painted
before screenshots. Representative desktop/mobile screenshots were visually inspected.

The rich views retain positive-length scalar geometry with bounded trace counts.
An additional real Plotly zoom and viewport resize keeps vector glyphs at most
24 px and 18 per view, with anchors recomputed. Zero-powered RL failure flights
correctly show no arrows. An explicitly missing chart-library test resource
exercises the static spatial/metric fallbacks and disabled mode controls. Legacy
reports without a study still browse and open rich details without study context.

The maintained 19-step development gate passes after the mobile correction.
Its ordinary synthetic software tests are separate from this presentation pass,
which executes zero campaign flights, native replays or training updates.
`release-b/http-delivery.json` verifies 18,441 complete-edition and linked source
files, 17,942,082,183 bytes, against the running endpoint. The shared publisher
reuses server PID 37689 and publishes seven entry-page files; every published
asset also matches its HTTP bytes. Both published-page viewport checks pass and
its links reach the 300 timing originals and exactly three lost RL originals.

All retained evidence remains in its original place. The final preservation
receipt is `release-b/closure.json`; it checks the initial Git-visible files,
14,751 prior report files, original foreign Git status, artifact/time/free-space
boundaries and unchanged serving PIDs. The full rich editions remain the primary
inspection surface; the small published page is their visual entry point.
