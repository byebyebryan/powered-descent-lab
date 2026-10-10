# Frozen terminal centering: full paired 1k results

[Documentation home](README.md) · [Initial contract](ballistic_terminal_centering_sweep_plan.md) · [Contained-error continuation](ballistic_terminal_centering_continuation_plan.md)

Verdict: **592/1000 verified target landings (59.2%)**, versus the previous
ballistic candidate's 562/1000. The unchanged braking/body-centering fix
generalizes beyond the focus cases, but introduces substantial terminal
regressions. It is not replacement acceptance or a default promotion.

## Complete population, including missing physical evidence

All 1000 original worlds and two external repeats of 142/974 were attempted.
There were 1002 total native invocations, zero retries and no new terrain,
rebuild, tuning, changed clocks/fuel or relaxed physical guard. Cap 24 remains
experimental; ordinary cap-6 defaults and the accepted report site are unchanged.

| Primary outcome | Count |
| --- | ---: |
| Verified target landing | 592 |
| Verified airborne finite stop | 392 |
| Native terrain-domain error; physical outcome unverified | 16 |
| Original denominator | 1000 |

984 primary records and both external repeats have complete physical/mission,
integrity, original-source command replay and decision-reproduction evidence.
No actual crash or off-target outcome occurred among those verified records.
The 16 exceptions have no complete physical outcome and must not be relabeled
as crashes, airborne stops, unsupported inputs or landings.

The first collector correctly stopped after 280 attempts on native domain
errors in 261/278. Its sealed partial capture stays stopped and unchanged. A
separate create-only continuation copied those 280 attempts unchanged and ran
only the remaining 720 worlds plus two repeats. Only the exact observed
finite-heightfield exception was contained; other runner/proof/safety errors
retained the stop rule. The complete capture is honestly labeled
`completed_with_terrain_domain_errors`, not error-free completion.

## Paired gains and regressions

Against the immediate 562-landings candidate, **75 previous failures now land,
but 45 previous landings are not reproduced**: 19 verified short-command stops,
14 verified original-budget stops, and 12 unverified domain exceptions.
Thus `562 + 75 - 45 = 592`: a net gain of 30, or 3.0 percentage points.

All 75 gains come from the original 102 `short_command_rejected` failures.
Of that cohort, 21 still stop there, two reach the original budget and four have
unverified domain exceptions. The fix is therefore a broad mechanical recovery,
not just a success on 142/715/974; its regressions prevent a clean acceptance.

The older policy-3/cap-24 experiment remains stronger at 817/1000. Compared with
that separate baseline, this candidate gains 67 landings and fails to reproduce
292 (282 verified stops and 10 unverified exceptions): a net gap of 225.
Neither comparison is the ordinary production cap-6 acceptance pack.

| Recipe, 250 original worlds each | Previous ballistic | Frozen centering |
| --- | ---: | ---: |
| Mountains 4x | 156 | 166 |
| Mountains 8x | 146 | 148 |
| Broad massifs 8x | 147 | 161 |
| Successive ridges 8x | 113 | 117 |

Using the retained older nominal classification, clear routes land 282/387
versus 285, and blocked routes land 310/613 versus 277. That classification is
not the new candidate's initial-arc predicate. Of the 592 landings, 298 use no
actual waypoint handoff and 294 follow one or more actual handoffs.

## Remaining stop groups and review targets

| Verified finite-stop group | Previous ballistic | Frozen centering |
| --- | ---: | ---: |
| `no_ballistic_aim` | 131 | 131 |
| `terrain_recovery_no_safe_command` | 103 | 103 |
| `waypoint_continuation_rejected` | 70 | 70 |
| `waypoint_aim_construction_miss` | 30 | 30 |
| `no_local_waypoint` | 1 | 1 |
| `short_command_rejected` | 102 | 40 |
| `original_budget` | 1 | 17 |

The 16 unverified errors are additional to this table. All query coordinates
fall just beyond the retained right domain edge, x=1360 m. The error can arise
in a body-envelope prediction; without complete flight records it does not
prove the executed vehicle left the world. Do not extend terrain or invent a
final state to repair this evidence gap.

For the next bounded diagnostic, inspect terminal regressions before changing
waypoint placement: all 984 complete primary pre-terminal action prefixes match
the previous candidate exactly, as do terminal-entry clocks wherever entry occurs.

- New short-command losses: 086, 150, 220, 244, 339, 385, 396, 511, 549, 669,
  673, 694, 757, 888, 891, 915, 933, 953, 976. Case 086 is a useful lateral
  overshoot/reserve example; do not assume all 19 share its cause.
- New budget losses: 288, 322, 344, 354, 465, 469, 589, 611, 736, 750, 814,
  816, 868, 986. Case 288 is near the target and descending when the original
  79.8 s clock ends; 465 is still about 10.7 m above it. Neither establishes a
  landing with extra time. Keep the deadline unchanged during diagnosis.
- Domain exceptions: 261, 278, 357, 448, 519, 565, 573, 602, 646, 678, 690,
  696, 697, 948, 989, 991. Twelve were former landings; 278/565/678/696 were
  former short-command failures. Improving exception artifact retention would
  improve diagnosis, not itself establish more landings.

No further fix or flight campaign follows this completed allowance automatically.

## Evidence, reports and verification

Complete capture:
`outputs/eval/planner_v2_random_terrain/capture-terminal-centering-sweep-20261009-v1-complete`.
Interrupted capture:
`outputs/eval/planner_v2_random_terrain/capture-terminal-centering-sweep-20261009-v1`.

The complete receipt authenticates **12224 files**. Both external repeats are
exact, and all 18 overlapping focus/preservation comparisons retain complete
feedback exactly. Independent report checks cover all 986 rich detail pages,
68288 raw-bound planning refresh origins/decisions, 757 exact actual-H snapshots,
and the inline/saved planning-cycle payloads. All 6906 capture-local links across
the batch and 986 details resolve. Batch/detail HTTP checks return
200 on the existing server; no browser visual acceptance is claimed.

**Report comparison limitation:** the frozen shared batch summary lists only
the 33 recorded losses against the previous ballistic candidate. Its paired-loss
headline excludes the 12 unverified former successes. Use the complete **45-loss**
comparison above; do not interpret `75 gains / 33 losses` as the whole-population
net change. The receipt-sealed report and raw historical summary are not rewritten.
At collection close this capture was not linked from root navigation. The
subsequent user-requested [navigation update](reports.md#report-serving) exposes
it under **Latest planner experiment**, **Waypoint planning** and the report
library, while retaining the previous comparisons. That navigation-only refresh
does not rewrite this capture or promote it into the accepted planner site.

The initial contract admitted the retained focus/panel under outcome-based direct
control preservation: all three controls land with full proof and exact
pre-terminal prefixes/entry clocks, while terminal commands may change. The
older focus/panel exact-flight admission remains failed, not retroactively waived.

Six new no-flight runner tests and the 168 existing Python collector tests pass.
Documentation links, all 65 maintained Node tests and whitespace checks pass. The prior
11-step development gate applies to the identical frozen Rust tree; no new native
build or retained 44-case numerical-parity campaign ran here. Source, binaries,
protected prior evidence and accepted-site/selector seals remain unchanged.
No server operation, commit, push, default promotion or accepted-site publication
was performed.

Native SHA: `eeb07070b7378186c50f7deba34a0afab96cbd3b1d5b44a32a0311b056418366`.
Rust tree: `696ae4de5a80abb742a347e610143e9166d9039dc566eebab2cffbdf85cb3dab`.
Renderer: `1f52999f81e4827614346c445af7b7b3b2a331407c62c59230f9c0c92115a591`.
Complete receipt: `98e072430d172a833d9b3f702eb46179a66ad4b47cb09747f98ffe74f15107ac`.
Complete results: `4f86018a4d6ad29e50d49a0f424276ffe5c0284d804a2d266b6eed17dceece34`.
Interrupted receipt: `616d6e9471714330be654f308597c98661918dd31c61cf518f1ddb47924c6d87`.

Read-only verification (repository root):

```sh
rtk proxy python3 -B scripts/continue-terminal-centering-sweep.py verify
```

The driver checks the complete attempt inventory, every recorded proof, source
and admission seals, exact repeats, contained exception class and unchanged
copied prefix. Its run mode is closed by the existing create-only destination.
