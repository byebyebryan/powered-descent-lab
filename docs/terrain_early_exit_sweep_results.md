# Paired early-exit 1,000-world results

Status: **completed collection and saved verification**. Closes the user-requested
[paired rerun](terrain_early_exit_sweep_plan.md) under its unchanged
[machine contract](../studies/terrain_profiles/early_exit_sweep_plan.json).
The exact admitted early-exit executable reran the original procedural population
with explicitly isolated cap 24. No planner/controller change, terrain generation,
tuning, default promotion or publication occurred.

## Verdict

**817/1000 verified target landings**, versus **748/1000** in the paired cap-24
baseline: **69 additional landings, +6.9 percentage points, zero lost landings**.
Every terrain recipe improves. The gains are not confined to the five selected
diagnostic subjects: 63 old nominal-exhaustion cases and six old clearing-exhaustion
cases now land after an earlier checked nominal exit.

| Measure | Previous cap 24 | Early-exit cap 24 |
| --- | ---: | ---: |
| Verified target landings | 748/1000 (74.8%) | 817/1000 (81.7%) |
| Initially clear direct landings | 387/387 | 387/387 |
| Initially blocked landings | 361/613 (58.89%) | 430/613 (70.15%) |
| `NoClearing` | 154 | 148 |
| `NoNominal` | 98 | 35 |
| Correction-limit stops | 0 | 0 |
| Actual crashes, fuel stops or timeouts | 0 | 0 |

Every landing has planning `landed`, physical `landed_on_target`, mission
`success`, integrity passed and original-source replay passed. The remaining
183 primaries are physically `flying`, mission `in_progress`: finite planner
stops, not crashes or proof that the worlds are physically impossible. Of these,
65 stop before any correction and 118 after corrections. All 387 initially clear
routes retain their complete direct flights and zero corrections.

## Terrain coverage

Each recipe keeps its original 250 worlds and exact scenario bytes. No seed was
replaced or difficulty adjusted after seeing outcomes.

| Recipe | Previous landings | Early-exit landings | Gain |
| --- | ---: | ---: | ---: |
| Mountains 4x | 231/250 | 242/250 | +11 |
| Mountains 8x | 172/250 | 192/250 | +20 |
| Broad massifs 8x | 210/250 | 229/250 | +19 |
| Successive ridges 8x | 135/250 | 154/250 | +19 |

Successive ridges remain weakest. This is paired development evidence on an
already inspected population, not a fresh held-out reliability estimate.

## What changed in flight

The selected clearing proposal, original H witness, ranking and deadline remain
unchanged. One optional query uses the first eligible earlier settled-coast
boundary. Only a clear, audited nominal with source proofs permits an earlier
actual handoff; that exact nominal is queued and executed next. Other outcomes
retain the original H fallback. A clearing waypoint still does not need a landing
suffix for admission.

Across the 1000 primaries, **288 flights commit an early exit**: 69 newly land and
219 already successful flights take an earlier checked continuation. The other
**712 flights retain their complete non-timing baseline records**, except the
declared root identities and independently checked optional early-query metadata.
All remaining finite failures belong to this unchanged-fallback group.

There are 1132 executed corrections and corresponding optional-exit records:
288 committed, 477 terrain-blocked queries, ten missing nominals, and 357 maneuvers
without an eligible earlier boundary. These are maneuver counts, not additional
mission attempts. Maximum corrections remain 13; no flight binds cap 24.

## Collector correction and attempt accounting

The first capture stopped after seven launched missions because the collector
required byte-identical native reserialization of scenario JSON. All seven native
flights passed integrity/replay, and every complete parsed scenario value matched.
This was a collector error, not a numeric trajectory defect or physical failure.

Only the collector and its tests changed. Original input bytes remain receipt-pinned;
native output scenarios are compared by every parsed value without tolerances or
omitted fields. A new create-only capture imports the seven authenticated raw
flights, preserves their original error ledgers and collector-source snapshots,
and explicitly revalidates them without execution. Only the remaining 1003
attempts launch. Thus the campaign has **1010 unique measured executions**, not
1017: 1000 primaries, three controls and seven exact repeats. No mission retry ran.
The stopped capture and its original receipt remain unchanged.

## Validation and retained evidence

- All 1010 native attempts pass integrity and original-source replay. Three
  controls land; the seven repeats exactly match their primary non-timing flights,
  including the two honest `NoClearing` repeats. Controls/repeats are excluded
  from the 1000-world denominator.
- Final saved verification passes for source/input/binary bindings, full native
  and compact results, paired fallback/prefix checks, exact queued nominals,
  continuation accounting, ordered ledgers and the complete closed receipt.
- Independent Node checks cover all 1010 common rich reports and **1155 exact
  executed-H annotations**: 718 complete fallback comparisons, 292 original
  selected-proposal/ordinary-prefix comparisons, all seven repeats and all 387
  direct clear routes. Established trajectory, velocity/thrust, sample, event
  and configuration views remain present. This is static checking, not browser
  or human visual acceptance.
- All 247 selected source/input files and all 1064 protected existing files,
  including the ordinary evaluator, served reports and selectors, remain
  hash-identical across collection. The ordinary default remains cap 6.
- All 104 terrain-study tests, 52 maintained Node tests, documentation links and
  diff-whitespace checks pass. The previous normal 44-case acceptance capture
  also passes its read-only saved check; no new 44-case flight campaign ran.

Final capture:
`outputs/eval/planner_v2_random_terrain/capture-early-exit-sweep-20261008-v2`.
It retains 8831 receipted files, about 5.7 GiB. Measured collection after import
took 623.206 seconds; final saved verification is additional. These include
collection/comparison overhead and are not controlled planner-speed measurements.
The authenticated prior cap-24 capture remains a saved-verification dependency.

Stopped original capture:
`outputs/eval/planner_v2_random_terrain/capture-early-exit-sweep-20261008-v1`.

- Final manifest SHA-256:
  `6dd0299b0246a866577284acc669081667f8fcc0675b891ce6c470a9715613df`.
- Final receipt SHA-256:
  `e12c5145626f572a1d6df35d1dbb949db3bb52e35515a09ebbedbf043aaca0f0`.
- Retained early-exit cap-24 executable SHA-256:
  `54ffd58ac4b4e4a4e9a3d557725cfd03238f40a2bb226b8edbbdb85141271509`.

## Next decision

Keep the simple early-exit capability: the paired gain is broad and all prior
landings are preserved. Before more tuning, freeze a separately authorized fresh
100-world test under the same recipes and settings; do not alter the candidate
in response to that test. The remaining dominant family is local clearing
exhaustion, including 65 unchanged pre-correction stops. Diagnose common mechanics
there rather than accumulating per-seed thresholds. Default-cap promotion,
report publication and any further campaign remain separate decisions.
