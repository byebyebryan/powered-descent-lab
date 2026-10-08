# Paired 1,000-world relaxed-cap results

Date: 2026-10-07 (local). Completes the user-requested
[relaxed-cap sweep](terrain_cap_sweep_plan.md). All **1010 measured attempts**
completed: 1000 exact original worlds, three controls and seven fixed repeats.
No retry, tuning, new terrain or production-default change occurred.

## Verdict

Allowing 24 corrections instead of six produces **748/1000 verified landings**,
up from 733/1000 on the same worlds: **15 additional landings, +1.5 percentage
points**, with every previous success preserved. No flight reaches the new cap;
the maximum actually used is **13**, in a successful landing.

This isolates a real budget limitation, not a new maneuver or per-seed fix.
For this population, cap 24 is nonbinding, so remaining incomplete flights
cannot be attributed to the correction-count limit. This is not literally
unlimited execution and does not establish that 24 is nonbinding everywhere.

| Measure | Original cap 6 | Isolated cap 24 |
| --- | ---: | ---: |
| Verified target landings | 733/1000 (73.3%) | 748/1000 (74.8%) |
| Initially clear direct landings | 387/387 | 387/387 |
| Initially blocked landings | 346/613 (56.44%) | 361/613 (58.89%) |
| `CorrectionLimit` | 23 | 0 |
| `NoClearing` | 149 | 154 |
| `NoNominal` | 95 | 98 |
| Actual crashes, fuel stops or timeouts | 0 | 0 |

Every landing has planning `landed`, physical `landed_on_target`, mission
`success`, integrity passed and original-source replay passed. The remaining
252 primaries are physically `flying`, mission `in_progress`, not actual crashes
or proven physically impossible worlds. Exit zero alone is not a landing.
Controls/repeats are separate from the 1000-case denominator.

## What happened to the 23 old cap stops

- **15 land:** `030`, `344`, `426`, `443`, `466`, `481`, `514`, `519`, `551`,
  `565`, `585`, `648`, `675`, `799`, `981`.
- **5 reach `NoClearing`:** `280`, `291`, `423`, `786`, `864`.
- **3 reach `NoNominal`:** `611`, `716`, `761`.

The five clearing stops use respectively 8, 7, 6, 6 and 6 corrections. Thus
three worlds do not execute another hop: removing the cap reveals an exhausted
local search at the same H6 state. Do not claim that every old cap stop makes
further physical progress. The three new nominal stops use 8, 7 and 7 corrections.

The fifteen new landings use 7–13 corrections: seven use 7, four use 8, two use
9, one uses 10 and one uses 13. `565` is the thirteen-waypoint example, landing
at 74.767 s with 4335.781 kg fuel remaining. `675` lands at 79.967 s: the
original 80 s absolute planner deadline was not extended to obtain completion.
`030` and `280` reproduce the complete earlier cap-12 probe results except
their explicitly different root identities and wall timings.

Across all primaries, executed corrections increase from 1118 to 1157. The
other **977 worlds preserve their complete non-timing flights**, excluding only
the two declared root policy/input identities. This includes all 733 previous
landings and all 244 earlier nominal/local-clearing stops.

## Terrain-recipe coverage

Each recipe retains its original 250 worlds, seed order and exact terrain/pad
bytes. Every recipe gains at least one landing; none loses a success.

| Recipe | Cap 6 landings | Cap 24 landings | Gain |
| --- | ---: | ---: | ---: |
| Mountains 4x | 230/250 | 231/250 | +1 |
| Mountains 8x | 167/250 | 172/250 | +5 |
| Broad massifs 8x | 203/250 | 210/250 | +7 |
| Successive ridges 8x | 133/250 | 135/250 | +2 |

Successive ridges remain weakest. Relaxing a count cap is not a replacement for
missing clearing/acquisition capability. The 252 remaining stops comprise
65 before departure and 187 after at least one correction: 154 total clearing
exhaustions and 98 nominal-construction exhaustions. None is cap-bound now.

## Validation and retained evidence

- Authenticate the complete original 1k capture before collection. Reuse all
  1003 primary/control scenario bytes without generating or editing terrain.
  All native input-only preflights pass before the first measured attempt.
- Generate an isolated source/binary with policy identity
  `piecewise_local_clearing_v2_policy_3_cap_probe_24`. Exactly one Rust file
  differs from the frozen workspace, in the copied revision-3 cap and identity.
  Nominal construction, templates, intervention clocks, ranking, safety guards,
  controllers, fuel, clocks and replay are unchanged.
- Every old cap stop retains exact first-six cycles, executed segments,
  actions/events/samples, complete H6 state, original deadline and the next
  nominal construction/audit. Every other primary retains its entire flight
  except three wall timings and the two explicitly different root identities.
  There is no numeric tolerance or proof exclusion.
- All three controls preserve complete flights; all seven repeats match their
  new primaries except the three wall timings. All 1010 attempts pass integrity
  and original-source replay; no collection, comparison or proof error occurs.
- Collector final saved verification passes. Independent read-only Node checks
  verify 985 complete baseline comparisons and 25 exact H6 prefixes, including
  controls/repeats. All **1010 common rich reports** retain **1180 exact H
  annotations**, plus maintained plots/events/config/mission detail. This is
  static evidence checking, not browser or human visual acceptance.
- All **235 selected source/input files**, the ordinary evaluator and **48
  protected accepted report/selector files** remain hash-identical across the
  sweep. All **91 terrain-study tests** and **11 maintained development checks**
  pass; closure docs are checked separately. No optional 44-case flight parity
  campaign, publication, navigation write, server operation, commit or push ran.

Capture:
`outputs/eval/planner_v2_random_terrain/capture-cap-sweep-20261007-v1`.
It retains 9562 receipted files, about 5.14 GiB. The authenticated original 1k
capture remains a declared dependency for paired saved verification; its flights
are not overwritten or copied unnecessarily. Measured collection took 560.390 s
(about 9 min 20 s); the isolated offline release build took 31.59 s. Collection
includes paired comparison overhead; these costs are not a controlled planner
speed comparison or per-tick game-loop guarantee.

- Contract SHA-256:
  `a2c25779806e7cea0d9043416704a06b82c2f9df4e327e624d2c2c81f904af62`.
- Manifest SHA-256:
  `33397ffe3d99187a44326ab921baf981b60d86397585c9983e49ac5edce04315`.
- Receipt SHA-256:
  `51808ed5b564d210e27d4e971172e49a7d7f6d005cd0667adbbf25ae3635551b`.
- Isolated cap-24 evaluator SHA-256:
  `1271673819ad8416d219970ac0d5e8486a773d57ca75bb42984f6c5ce9abac26`.
- Unchanged ordinary evaluator SHA-256:
  `fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af`.

Commands are maintained in the
[evaluation workflow](evaluation.md#paired-1000-world-relaxed-cap-sweep).
The allowance is closed; saved verification launches no flights or fresh replay.

## Alignment and next decision

The user's concern was justified: six was an artificial stopping point for
fifteen otherwise completing missions. A generous finite correction budget is
now supported as a default candidate, with time, fuel and local safety still
governing actual execution. Fold it into the maintained planner only after
reviewing saved-policy identity compatibility and normal 44-case acceptance;
this experimental sweep does not silently change production.

Do not redesign short-hop progress from correction count alone. A long sequence
can land, as `565` demonstrates. Keep useful progress and finite continuation
guards, but focus further capability work on the now-unmasked `NoClearing` and
`NoNominal` families. Departure clearance-then-forward on `327/791`, preserving
`258` and the direct control, remains a simple bounded candidate; later H8 in
`280` supplies an additional clearing boundary, not a solved landing.

This is a paired improvement on an inspected development population. It is not
fresh held-out generalization, arbitrary-terrain reliability or permission for
per-seed tuning. Original cap-6 fixtures, outcomes and accepted reports remain
authentic historical evidence. Any default promotion or next capability change
is a separately scoped implementation decision.
