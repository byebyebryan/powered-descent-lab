# Bounded queued-flight recovery results

[Documentation home](README.md) · [Frozen plan](ballistic_recovery_consistency_plan.md) ·
[797/1000 reference](ballistic_phase_transition_results.md)

The opt-in recovery candidate lands **829/1000 (82.9%)**, versus 797: **33 paired
gains and one loss**, a net gain of 3.2 percentage points. All four terrain recipes
improve. This establishes a general recovery-timing mechanism without changing
arc construction, waypoint placement, landing control or physical reserves.
It is development evidence on a reused population, not default promotion.

## What changed

The diagnostic compares the actual paired queued turn/burn/coast program with
the same four existing recovery commands over one common response horizon.
It follows known command switches instead of projecting a stale powered command
past cutoff or borrowing extra lookahead. The ordinary live 24-tick guard remains
unchanged. Early intervention requires both a warning and a passing existing
response; warning alone cannot stop an otherwise clear live prefix.

Selection preserves the existing command order but uses the common horizon,
including during recovery. Recovery keeps the active goal and resumes only when
a correction or acceptable coast exists and its queued prefix passes. There is
no new command family, terrain-aware direct-arc search, angle/height tuning or
change to the original episode/deadline/correction budgets.

## Probe and focused controls

The bounded-comparison probe reproduces all 48 complete ordinary reference
flights, and its three complete repeats match exactly. It records genuinely
bounded warning/alternative evidence in eight of the ten recovery subjects and
nine previously landed controls, not in 407 or 739. These finite query passes
are not complete recovery certificates.

The active panel lands **36/48**, versus 33: 016, 020, 139 and 218 gain landings;
138 loses its previous landing. All three active repeats match exactly. The
four direct controls remain exact ordinary flights. No outcome-driven tuning
followed, and the same final source ran the original 1k despite this honest loss.

For 020, recovery now starts at tick 374 while upright lift still passes,
rather than exhausting recovery at tick 452. It resumes the unchanged waypoint
goal from the actual state, later recovers once more, and lands. Earlier response
room translates into an executed landing here, not merely a better query score.

## Full paired 1k

All 1000 primary worlds and five separate repeats recorded and verified.
Collection took 448.01 seconds with four workers; the slowest attempt took
3.96 seconds within the unchanged 60-second bound. All 48 focused/full feedback
records match exactly. Complete ordinary flights remain exact in 895/1000 worlds.

| Terrain recipe | Reference / 250 | Recovery candidate / 250 |
| --- | --- | --- |
| Broad massifs, 8x | 204 | 213 |
| Mountains, 4x | 195 | 210 |
| Mountains, 8x | 206 | 213 |
| Successive ridges, 8x | 192 | 193 |

The 33 gains are 016, 020, 032, 074, 095, 111, 115, 136, 139, 182, 193, 203,
218, 222, 229, 230, 301, 353, 384, 409, 437, 464, 490, 502, 523, 535, 541,
653, 663, 665, 702, 723 and 884. Thirty-two were recovery failures; 437 was a
waypoint construction miss. The sole loss is 138. Twenty-nine gains are outside
the focused panel. Terminal entries increase from 834 to 867; no landing-controller
change is responsible for this increase.

There are 389 landings without actual H and 440 after waypoint handoffs.
Zero-H does not imply an uninterrupted direct flight: local recovery can occur
without adding a waypoint. Original baseline cohorts remain separate: 340/387
originally clear and 489/613 originally blocked worlds land. The older policy-3
cap-24 reference lands 817; this candidate exceeds its total by twelve, but gains
141 different worlds and loses 129 of its landings. It is not strict dominance.

There are no actual crashes or off-target landings. All 171 failures retain
flying/in-progress physical prefixes with protected stops:

| Remaining stop | Reference | Recovery candidate |
| --- | --- | --- |
| No safe recovery command | 96 | 69 |
| No ballistic aim | 41 | 41 |
| Waypoint aim construction miss | 26 | 21 |
| Terminal short-command rejection | 22 | 23 |
| Original budget | 13 | 13 |
| Local proposal budget | 3 | 2 |
| Prediction terrain domain | 2 | 2 |

Stop-group reductions are not all landings: six old waypoint misses and one
proposal-budget stop now exhaust recovery; two old recovery stops become waypoint
misses. There are 133 preterminal stops and 38 terminal stops. Of 103 worlds
using recovery, 41 land. Passing one response window does not guarantee eventual
clearance or a later feasible correction.

## Remaining limits and next decision

**138 is a terminal-arrival trade-off.** An earlier support response changes the
arrival state; the unchanged terminal controller eventually reaches a pad-edge
reserve rejection near x=1213.7 m. The old flight lands. Do not hide this loss,
add a case-specific exception or relax clearance to preserve the headline.

**407 remains exact and stopped.** The prior routing regression is not repaired
by this recovery change. Its earlier apparent warning borrowed time beyond the
alternatives' horizon; the bounded probe has no qualifying early warning.
715 also remains an exact `no_ballistic_aim` stop; 349 remains an exact landing.

**Resume churn remains an edge case.** 139 lands despite eight same-tick
resume/reject transitions, and 663 lands with two. A queued correction query
does not model every subsequent coast-release/replanning decision. The mandatory
live guard catches those differences. Record this limit rather than claiming
that all phase ownership is solved or tuning this frozen pass around two worlds.

Use this as the next opt-in development reference. The destination-acquisition
gate remains an isolated next diagnostic: test native acquisition at the existing
query states where horizontal room prevented the check. Do not mistake earlier
mathematical fits for safe terrain prefixes, widen the arc search reflexively,
or bundle landing-controller and waypoint-placement changes into that probe.

## Evidence and validation

Captures are `capture-ballistic-recovery-consistency-{probe,focus,full}-20261009-v1`
under `outputs/eval/planner_v2_random_terrain/`. Each retains common rich batch
and detail pages, original inputs, frozen source/binaries, earlier feedback,
native command replay, deterministic decision reproduction and exact repeats.
All **1,107 declared records** verify. Independent checks authenticated the full
13,283-file inventory, all 1,005 full records/repeats, 1,674,361 issued terminal
frames, 489,425 rich samples, bounded warning clocks, common selection horizons,
report detail links and all 63 protected file hashes. This is generated-data
validation, not manual browser acceptance. Both focus and full URLs return HTTP 200.

Scoped ballistic Rust tests pass 64/64; terrain-tool tests pass 194/194. The
ordinary maintained 11-check gate passes, including Clippy, formatting, CLI
boundaries, documentation and 66 Node tests. Whitespace checks pass.

The separate retained 44-case exact-parity test **fails**, both on this candidate
and an isolated untouched HEAD (`138223a`), at the same `v2_clear_685` audit value:
`0.0005533556252954597` versus `0.0005533556252945715`. Debug and release agree.
This pre-existing approximately 9e-16 discrepancy was not weakened, repaired,
or rewritten into the reference. It is not evidence of a new recovery/default
behavior change, and this pass must not be described as passing that opt-in gate.

```text
native:       d9d4eaa7a617f1250a8a788e7ab04595cdf3ea84c839125fdeee2c21d18bff8a
Rust tree:    89bcede9e07893009ede8e8f57cafdd2ff90d7e9bf6de1ff37a3e6e1213714ff
renderer:     ff1d90429020b000db224c8921f06efc338058d439a75b2ddcd26300c120c48e
full receipt: bc4f4ebffd1699a167cc3fdd58f8901b9cb42e9e3a8654b18d53200630c6728b
full results: 932693f34d1660c925674a52eb42040cde6caf1b5dc7678c8e2ae62948ae3678
```

Saved verification is read-only:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_recovery_consistency_panel.py verify full
```

No commits, pushes, default promotion, accepted-site/root publication or server
changes were made. The three pre-existing navigation edits and historical
captures remain unchanged; root navigation continues to feature the 797 reference.
