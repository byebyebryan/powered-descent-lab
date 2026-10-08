# Correction-cap diagnostic results

Date: 2026-10-07 (local). Completes the
[bounded cap sensitivity plan](terrain_cap_probe_plan.md).
Six measured attempts completed under isolated cap 12; cap 24 was not admitted.
No production planner/controller behavior or default changed.

## Verdict

The original six-correction stops did not establish broken short-hop logic.
With the same maneuvers and exactly the same executed prefixes, `030` lands
after one additional correction. `280` makes two additional corrections and
then reaches a different finite stop, `NoClearing`, before the higher cap binds.
Both repeats reproduce these complete results except the three wall timings.

| Mission | Original cap 6 | Isolated cap 12 | Actual endpoint |
| --- | --- | --- | --- |
| Subject `030` | `CorrectionLimit`, 6 H, flying | Verified target landing, 7 H | 66.633 s; 4528.117 kg fuel |
| Subject `280` | `CorrectionLimit`, 6 H, flying | `NoClearing`, 8 H, flying | 50.150 s; x=837.010 m; 4938.521 kg fuel |
| Direct control `000` | Verified landing, 0 H | Same complete non-timing flight | 41.350 s |
| Corrected control `253` | Verified landing, 6 H | Same complete non-timing flight | 79.967 s |

Every recorded attempt passes integrity and original-source replay. Landings
also have physical `landed_on_target` and mission `success`; both `280` attempts
remain physical `flying`, mission `in_progress`, end reason `running`.
There are no actual crashes, fuel stops, timeouts or collector errors.

These are paired selected-world results, not an improvement to the 1k landing
rate, an untouched test population or a promoted cap-12 policy. The ordinary
six-correction result and all historical captures remain unchanged.

## What changed after H6

### `030`: one more correction was enough

The exact original H6 is at x=385.869 m, t=37.917 s. The unchanged next nominal
is blocked, so the probe can now execute the local correction that the original
cap prohibited. H7 reaches x=811.307 m, t=49.233 s: a 425.438 m increment.
Replanning from that actual H gives a clear direct suffix and verified landing
at t=66.633 s, leaving 13.367 s of the original 80 s deadline.

The earlier 25–56 m increments were not proof of an inability to make useful
progress. They were followed by a much larger correction when the state and
obstruction allowed it. Do not add a universal minimum-hop distance or ban
quick reblocks based on this example.

### `280`: the later limit is local clearing, not correction count

The exact original H6 is at x=384.319 m, t=37.367 s. H7 reaches x=502.526 m,
t=42.783 s; H8 reaches x=837.010 m, t=50.150 s. These add 118.207 m and
334.484 m respectively. At H8, about 363 m of horizontal route, 29.850 s and
4938.521 kg fuel remain; vx=61.641 m/s, vy=-4.056 m/s.

A new nominal is successfully constructed, but its terrain audit predicts a
5 m body-reserve violation at step 6496, near x=1053.645 m, during the terminal
bridge. The audit later predicts contact. This is an **unexecuted query**, not
an actual crash in the retained source flight: execution remains at H8.

The local search admits four distinct entry clocks and propagates 168 rows.
No row is accepted. Its 60,480 classified boundaries comprise 53,505 incomplete
prefixes, 6498 insufficient-progress boundaries and 477 unsafe continuations.
Forty rows record a first continuation rejection, all in the `physical_trace`
family; representative reasons are body reserve falling below 5 m shortly
after a candidate H. No fallback clock remains after deduplicating the already
tested entries. The planner therefore reports honest `NoClearing` rather than
executing the predicted unsafe path.

This exposes a concrete later clearing limitation. It does not prove the world
physically impossible, establish that ranking alone is wrong, or show which
new maneuver would solve it. Positive cheap braking room at H8 is still not a
terrain-clearance or landing certificate. Cap 24 would not change this earlier
search exhaustion, so its six conditional attempts were not run.

## Validation and evidence boundaries

- The probe uses explicit policy identity
  `piecewise_local_clearing_v2_policy_3_cap_probe_12`, not ordinary accepted
  policy 3. Its generated Rust copy differs in one file and exactly two fields:
  revision-3 identity and correction cap. Main Rust source remains untouched.
- Both subjects and repeats match their original first-six planning/query
  cycles, all executed segments, source-prefix actions/events/samples, complete
  H6 state and original deadline. The next nominal construction/audit is also
  exact before the cap decision. There is no synthetic airborne restoration.
- Both successful controls match complete flights except three wall timings
  and the explicitly different root policy/input identities. Subject repeats
  match complete same-policy flights except the three wall timings; there is
  no numeric tolerance or proof exclusion.
- The collector's final verification and a separate saved verification pass.
  An independent read-only Node check also verifies prefixes, controls, repeats
  and all six common rich detail reports, binding **36 actual H annotations**
  to raw segment entry/end states. This is static evidence checking, not browser
  or human visual acceptance.
- All **84 terrain-study tests** pass, including ten pure cap-probe tests.
  All **11 maintained development checks** pass, including 52 Node tests,
  workspace/CLI tests, formatting and strict Clippy. Closure documentation is
  checked separately. No optional 44-case parity campaign ran.
- All **232 selected source/input files**, the ordinary release evaluator and
  **48 protected accepted report/selector files** are unchanged across collection
  and match their saved inventories. No accepted site refresh, navigation write,
  server lifecycle operation, commit or push occurred.

Capture:
`outputs/eval/planner_v2_random_terrain/capture-cap-probe-20261007-v1`.
It retains 533 receipted files, about 151.4 MiB including original baseline
artifacts, source snapshot, isolated build source/binary, preflights, logs,
ledger, native flights and rich detail reports. Measured collection took
21.549 s; the isolated offline release build took 31.94 s. These are observed
costs, not a per-tick game-loop or performance guarantee.

- Contract SHA-256:
  `f661aa7f7fd6b268262078fdada63d614dc34bbbd17d0ebdcce11afdab29b224`.
- Manifest SHA-256:
  `344a3e482a72d1a0232910d3260a3c59fec06b28ddf20b302fd50708b5884425`.
- Receipt SHA-256:
  `e012b6d48e32e75c490c159db11124ddb0db9977bb0ff247dc73d54e440c709d`.
- Isolated cap-12 evaluator SHA-256:
  `3a941dabed32411d9800328a0ec73ce3439abaeb52e2701b91d577db1af47130`.
- Unchanged ordinary evaluator SHA-256:
  `fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af`.

The [evaluation workflow](evaluation.md#isolated-correction-cap-diagnostic)
provides read-only saved verification. The six unused conditional attempts are
not a standing retry or tuning allowance.

## What to keep and what to defer

Keep the cap as an explicit finite budget, but do not treat reaching it as a
crash or a proven progress defect. `030` is now a selected-world budget
sensitivity comparison; `280` also supplies the exact later H8 clearing stop
for inspection. Their frozen cap-6 expectations are not rewritten.

A modest larger default cap is worth a separate paired development check, not
automatic promotion from two selected cases. Only 23 of the 267 finite stops
in the [1k baseline](terrain_validation_1k_results.md) were `CorrectionLimit`;
the other 244 were nominal or local-clearing exhaustion. Cap headroom alone
does not address those earlier stops, so it is not the main coverage mechanism.

For the next capability pass, retain the simple departure clearance-then-forward
direction on `327/791`, with `258` and the direct control preserved. Use the
other diagnostics, including this later `280` boundary, to expose downstream
limits. Diagnose one missing maneuver before changing progress thresholds,
ranking or guards; do not turn this into a larger fixed grid or end-to-end
landing search. Broader generalization still needs paired development evidence
and a separately frozen untouched sample. No next implementation is authorized
by this completed diagnostic.

Subsequent follow-up: the [paired full 1k cap sweep](terrain_cap_sweep_results.md)
has now completed that broader budget check. It preserves all 733 old successes
and adds fifteen for 748/1000 under cap 24, with no count-cap stops. Production
still remains at six pending normal acceptance and compatibility review.
