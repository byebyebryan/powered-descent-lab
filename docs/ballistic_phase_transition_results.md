# Ballistic phase-transition results

[Documentation home](README.md) · [Frozen plan](ballistic_phase_transition_plan.md) ·
[786/1000 reference](ballistic_finite_correction_results.md)

The combined opt-in candidate lands **797/1000 (79.7%)**, versus 786: twelve
paired gains and one loss, a net improvement of 1.1 percentage points. All four
terrain recipes improve. This is a useful mechanical correction, not a major
reliability breakthrough or default replacement. Recovery remains the largest
failure group; the new native diagnostics establish a common earlier-response
window worth testing next.

## What was tested

All stages use the same frozen final native/source/renderer/reader and original
terrain inputs. Initial terrain-blind construction, waypoint ranking, profile
and correction budgets, fuel, clocks, contact/reserve guards and policy 3 remain
unchanged. Each focused stage has 29 primary worlds and three exact repeats.

| Focused mode | Landed / 29 | Gains versus reference | Losses |
| --- | --- | --- | --- |
| Behavior-identical probe | 14 | 0 | 0 |
| Actual-rotation coast checks | 20 | 6 | 0 |
| Configured terminal takeover checks | 19 | 5 | 0 |
| Pad-reserve command adapter | 15 | 1 | 0 |
| Combined transitions | 23 | 9 | 0 |

The coast ablation recovers 080, 091, 173, 231, 438 and 473. The takeover
ablation recovers 080, 438, 473, 516 and 908. Their effects overlap and are not
additive: together they recover all eight early terminal subjects. The pad-only
ablation recovers 026, but not 119, 538, 796 or 862.

The probe reproduces every complete ordinary flight, not merely its verdict.
Its native queries reproduce the eight subjects' configured takeover failures;
actual coast rotation also exposes insufficient reserve in several cutoff
queries. The new checks use the native plant without changing the live state.
Takeover clones the exact configured controller and checks two seconds of
feedback plus ordinary guards. A decline retains the checked coast and rechecks
on the existing 24-tick cadence; it is not itself an obstacle or mission stop.

Review after the first behavior-identical V1 probe found that the pad adapter
could leak into the separately previewed standalone coast-to-terminal path.
That boundary was fenced before active ablations. V1 remains intact; all final
stages are create-only V2 captures. No outcome-driven trajectory tuning followed.

## Full paired 1k

All 1000 primary worlds and five explicit repeats recorded successfully.
Collection took 548.85 seconds with four workers; the slowest attempt took
4.61 seconds, within the unchanged 60-second case bound.

| Terrain recipe | Reference / 250 | Combined / 250 |
| --- | --- | --- |
| Broad massifs, 8x | 202 | 204 |
| Mountains, 4x | 190 | 195 |
| Mountains, 8x | 204 | 206 |
| Successive ridges, 8x | 190 | 192 |

The twelve gains are 026, 080, 091, 173, 231, 360, 438, 473, 516, 583, 763
and 908. All were previously terminal short-command stops. Three gains are
outside the focused panel. The sole paired loss is 407. Complete ordinary
flights remain exact in 976/1000 worlds, despite additional diagnostics.

Of the 797 verified landings, 364 have no actual handoff and 433 follow waypoint
handoffs. Original baseline cohorts remain separate: 313/387 originally clear
worlds and 484/613 originally blocked worlds land. The older policy-3 cap-24
reference still lands 817/1000; these ballistic candidates are not a universal
replacement for it. This reused development population is not held-out evidence.

There are no actual crashes or off-target landings. The 203 failures retain
flying physical prefixes with protected planning/controller stops:

| Remaining stop | Count |
| --- | --- |
| No safe recovery command | 96 |
| No ballistic aim | 41 |
| Waypoint aim construction miss | 26 |
| Terminal short-command rejection | 22 |
| Original budget | 13 |
| Local proposal budget | 3 |
| Prediction terrain domain | 2 |

There are 834 actual terminal entries: 797 land and 37 stop before landing.
All 95 old recovery-failure flights remain exact; 407 adds the 96th. Thus this
pass does not secretly change recovery selection or solve that workstream.

## What did not work fully

**407 is a routing regression, not a crash.** At tick 2478 the new realized
cutoff rotation check detects reserve loss near x=631 m, earlier than the old
cutoff-coast obstruction near x=685 m. The same waypoint machinery now selects
an earlier waypoint near x=638 m instead of the reference waypoint near x=809 m.
After two handoffs the candidate exhausts recovery at tick 3782. The reference
lands. More accurate rejection evidence can change routing adversely; this
trade-off must stay visible rather than being hidden by the positive total.

**The pad adapter is limited.** Across the full run it changes 80 issued terminal
frames in eleven worlds, two of which land. It activates only on a native
outside-pad reserve conflict, preserves nominal requested lateral acceleration,
adds lift, and issues a replacement only when the same guard passes. In 119,
538, 796 and 862 it buys time but still reaches another reserve rejection.
Late reactive lift does not establish a durable clearance margin while the
rotating body recenters. No landing corridor or physical guard was relaxed.

**715 remains unchanged.** It still stops at the distinct coast-acceptance /
aim-admission boundary. Coast/terminal transition checks do not repair that
heuristic eligibility discontinuity, and no correction was forcibly completed.
741 also remains a nonlanding, moving from a terminal short-command stop to
`no_ballistic_aim`; that classification change is not counted as progress.

## Recovery diagnostic and next decision

At actual planning states 48–84 ticks (0.4–0.7 seconds) before the 96 recovery
stops, native comparisons find a safe existing command over the common response
horizon in **90/96** worlds. The queued turn/burn/coast forecast warns in
**84/96**; warning and a safe alternative coincide at the same recorded state
in **80/96**. The six without a safe option in that window are 016, 218, 330,
433, 446 and 739. These are finite native-query facts, not landing predictions.

For 020, the queued program already warns at tick 350; upright lift and braking
lift still pass the common horizon at ticks 374 and 398. By tick 422 neither
passes. Waiting for the ordinary immediate command rejection loses that room.
This generalizes beyond one selected mission without adding a command family.

The recommended next implementation experiment is therefore bounded early
recovery: use the actual queued program for warning, compare the same existing
choices over one common response window, execute a selected safe response and
resume planning toward the existing goal. Verify complete flights on the full
paired 1k again; do not claim the 90 query passes as ninety recoverable landings.
Review 407 alongside it. Reserve-consistent terminal recentering and 715's
eligibility boundary remain separate, smaller questions. No new campaign starts
automatically from this recommendation.

## Evidence, reports and validation

Final captures use
`outputs/eval/planner_v2_random_terrain/capture-ballistic-phase-transition-{stage}-20261009-v2`,
with stages `probe`, `coast`, `entry`, `pad`, `focus` and `full`. Each has a local
common batch `index.html`, grouped failures and rich detail pages under `runs/`.
Existing terminal metrics, statuses and markers now come from bound issued
controller frames rather than synthetic command-only terminal frames. Rich
spatial/telemetry plots and waypoint annotations remain intact.

Independent read-only verification authenticated all six complete inventories,
original inputs, physical/mission/integrity/source-replay tuples, source snapshots,
exact repeats, focused/full overlap, unchanged reference/protected files,
batch detail links and actual H annotations. The full inventory has 13,281 files;
1,605,287 issued terminal frames and 268,360 rich metric samples bind exactly.
This is generated-data validation, not manual browser/visual acceptance.

The maintained 12-check gate, including exact opt-in retained 44-case parity,
passes. Scoped ballistic Rust tests pass 59/59; terrain-tool tests pass 191/191.
Clippy, formatting, documentation links and whitespace checks pass.

Frozen identities:

```text
native:       fe6525ddad102431d941aa45709348afa0893c7bb6a84996eeda6b75572b1d42
Rust tree:    1e262572a8c6ebc9dff3f76fa4dfb4993ecfe07d93e53cc9198e55825583dfe3
renderer:     838f24810cafc3438e4ebb229c3f3cd13b8313248e5400182f9ed9c81e2a8cf8
full receipt: d3c4571f9e0b6ba54e83c7339b51560c532aaac8a2176e1a532e03bca9766833
full results: e2f3f8b67b150d897a1728926219f063feb05ee6817adf345cc6711e739ffd9d
```

Saved verification is read-only:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_phase_transition_panel.py verify full
```

No commits, pushes, default promotion, accepted report refresh, navigation
publication or server changes were made. Historical captures remain intact.
