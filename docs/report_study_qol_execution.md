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
