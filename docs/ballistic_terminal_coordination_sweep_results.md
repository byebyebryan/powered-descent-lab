# Frozen terminal coordination: full diagnostic 1k

[Documentation home](README.md) · [Frozen sweep plan](ballistic_terminal_coordination_sweep_plan.md) · [Selected panel](ballistic_terminal_coordination_results.md)

Verdict: **639/1000 verified target landings (63.9%)**, versus V12's 592/1000.
The unchanged V13 candidate gains 49 and loses two: **net +47, or +4.7 percentage
points**. This is a broad paired improvement, not clean preservation or planner
replacement acceptance. The earlier panel admission remains failed on 048/755.

## Complete accounting

All 1,000 original worlds and five external repeats completed: **1,005 complete
records, zero retries, zero native exceptions**. Every record has physical/mission,
integrity, exact source command replay and decision-reproduction evidence.
There are 639 target landings and 361 verified airborne stops, with no executed
crash, off-target landing or actual terrain-domain violation. Repeats of
142/150/288/757/974 reproduce complete feedback bytes exactly and are outside
the 1,000-world denominator. All 121 overlapping random panel flights also
reproduce their complete feedback bytes exactly.

No native rebuild or flight-logic change occurred. Terrain, input bytes, clocks,
fuel, deadline, physical guards and cap-24 opt-in remain unchanged. The collector
adds a separately authorized diagnostic stage; it does not pass the old conditional
stage. Three direct controls already landed in the panel and were not flown again.

| Paired comparison, same 1,000 worlds | Prior | Gains | Losses | Current |
| --- | ---: | ---: | ---: | ---: |
| V12 terminal centering | 592 | 49 | 2 | 639 |
| V10 coast-to-terminal | 562 | 81 | 4 | 639 |
| Older policy-3/cap-24 reference | 817 | 82 | 260 | 639 |

The policy-3 reference remains stronger by 178 landings. These experiments do
not replace the ordinary cap-6 acceptance pack or establish held-out generalization.
The 49 gains against V12 comprise 23 former short-command stops, 12 former
deadline stops and 14 formerly unverified domain exceptions. Only the already
known 048/755 successful V12 flights regress. All four recipes improve.

| Recipe, 250 worlds each | V10 | V12 | V13 |
| --- | ---: | ---: | ---: |
| Mountains 4x | 156 | 166 | 169 |
| Mountains 8x | 146 | 148 | 162 |
| Broad massifs 8x | 147 | 161 | 179 |
| Successive ridges 8x | 113 | 117 | 129 |

Using the retained policy-3 initial classification, clear routes land **295/387**
versus 282; blocked routes land **344/613** versus 310. These original labels
are not V13's own arc classification. There are 300 zero-H landings and 339
landings after one or more actual waypoint handoffs.

## Remaining bottleneck

All 984 complete V12 primary comparisons preserve preterminal commands, updates,
entry clocks and planning/H evidence. Independent checks confirm 649 terminal
entry prefixes and **335 unchanged complete nonterminal flights**. The sixteen
V12 exceptions cannot supply complete old-flight comparisons and remain
historically unverified, even where V13 now supplies a new verified result.

| Remaining stop | Count | Scope |
| --- | ---: | --- |
| `no_ballistic_aim` | 131 | Before terminal entry, unchanged |
| `terrain_recovery_no_safe_command` | 103 | Before terminal entry, unchanged |
| `waypoint_continuation_rejected` | 70 | Before terminal entry, unchanged |
| `waypoint_aim_construction_miss` | 30 | Before terminal entry, unchanged |
| `no_local_waypoint` | 1 | Before terminal entry, unchanged |
| `short_command_rejected` | 20 | After terminal entry |
| `original_budget` | 5 | After terminal entry |
| `prediction_terrain_domain` | 1 | After terminal entry; rejected query |
| Total airborne stops | 361 | Complete physical/mission/replay evidence |

Thus **335/361 failures (92.8%) occur before terminal entry**. Of 665 worlds
that enter terminal control, 639 land (96.1%). Holding these preterminal flights
fixed, perfect terminal completion would cap this population at 665/1000.
This is a measured phase boundary, not proof of physical impossibility.

The five unchanged-budget cases are 611/781/810/816/986. They remain flying at
79.8 s; eventual landing with extra time is not established. The two known
lateral-capture regressions retain the panel's diagnosis and correct body reserve.

**696 exercises the new typed prediction-domain evidence path.** Its actual
state stops at tick 2900, x=1353.9102 m, still flying inside the retained domain.
An upright full-thrust query reaches tick 2924 and x=1356.0137; its 4 m body
extent queries x=1360.0137 beyond the fixed 1360 m edge. That command is not
executed. Actual prefix, query origin/state and requested command pass replay
and decision reproduction. Of V12's sixteen incomplete exceptions, fourteen now
land, 948 stops under reserve and 696 stops at this prediction boundary. Old
exception evidence is neither rewritten nor clamped.

Next keep 048/755 as a small terminal preservation task, but inspect representative
saved states from the unchanged 131 missing-aim and 103 no-safe-command worlds
for meaningful overall progress. Separate arc construction, realization and
immediate command protection in a bounded diagnostic before another fix/sweep.
Another terminal-only tuning batch cannot resolve the main gap. No further
flights, tuning, weaker guards, longer clocks or promotion follow automatically.

## Reports and validation

Create-only capture:
`outputs/eval/planner_v2_random_terrain/capture-terminal-coordination-diagnostic-1k-20261009-v1`.
Its **13,313 sealed files** retain the frozen protocol, original inputs,
candidate source/binaries, prior receipts/results, all native artifacts, the
common failure-first batch and 1,005 rich details.

Independent static report checks bind all 1,005 details, **69,745 planning-cycle
origins/decisions and inline payloads, 415,670 saved samples and 779 actual-H
annotations**. All **7,039 capture-local links** across the batch and details
resolve. Batch HTTP returns 200 on the existing server; no browser visual
acceptance is claimed. The generic previous-candidate comparison is **V10**,
not V12. Its frozen prose mentions two repeats; the authoritative manifest,
results and repeat branch correctly contain **five**. No sealed report is rewritten.

Five pure collector tests, 168 terrain-tool tests, all 65 maintained Node tests,
documentation links and whitespace checks pass. The earlier native development
and retained 44-case parity gate applies to this identical flight binary/Rust
tree; no new native build or parity flight campaign ran here.

Defaults, accepted-site/selector, previous captures, root navigation and server
state remain unchanged. Root still links the previous 592-landings batch; this
new diagnostic is reachable directly at its capture path. No commit, push,
report-root publication or default promotion was performed.

Read-only verification:

```sh
rtk proxy python3 -B scripts/run-terminal-coordination.py verify-diagnostic
```

Identities:

- Native: `89de0cffcb61adae04631c7ff25144201c949c7117c7e26e1ae72c17d1f1fc77`.
- Rust tree: `2c42d3691ee8ca675a3c3332871c47e20969b2c3036c32cf0eed0469191fc899`.
- Renderer: `fe0e9720226321d6596f9f4f93068fd68b4d5ffc6ccefed8b8a71b4ad213178d`.
- Receipt: `49a9dd87d8df9725dd594c9979470d8984446aac31e3aecfec483db64910ccfe`.
- Results: `7f66120ba2851da54c490937591c4fdc3e9ff370738cb6216dc3d78f87fc6121`.
