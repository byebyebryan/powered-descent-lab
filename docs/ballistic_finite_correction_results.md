# Finite destination correction: native probe and paired 1k

[Documentation home](README.md) · [Frozen pass](ballistic_finite_correction_plan.md) · [706/1000 reference](ballistic_exit_diagnostic_results.md)

Verdict: **786/1000 verified target landings (78.6%)**, versus V14's 706.
There are **109 gains and 29 losses**, net **+80 / +8.0 percentage points**.
All four terrain recipes improve. This is a worthwhile, broad development
improvement, not clean preservation or replacement acceptance. The original
policy-3/cap-24 diagnostic remains stronger at 817/1000; ordinary policy-3/cap-6
defaults are unchanged. This reused population is not held-out validation.

## Mechanism confirmed before changing flight behavior

The native probe runs the unchanged exit-consistency flight while recording
optional destination queries at their actual planning states. All 17 ordinary
flights exactly match the sealed reference. Three complete query-bearing
feedback repeats reproduce exactly.

Of 688 recorded queries, nine pass the finite check. Seven passing queries in
five subjects (044, 050, 081, 236, 862) have an obstructed instantaneous desired
arc, despite a clear native powered acquisition and realized-cutoff coast.
The other two passing queries have clear ideal arcs. Rejections are 222
cutoff-coast obstructions, two powered short-guard conflicts, 78 construction
misses and 377 sufficient/unavailable waypoint-room estimates.

The difference matters: the desired departure velocity cannot be applied
instantaneously. Auditing that fictional immediate arc can reject a valid
finite correction. Conversely, a clear ideal arc cannot establish that the
powered acquisition is safe. The probe checks one existing correction with
the actual cloned plant, slew, paired throttle and ordinary short-command guard;
it does not optimize another route or certify a complete terminal suffix.

062 has no passing native proposal at its actual query ticks. 262 and 971 are
outside the unchanged negative-horizontal-room trigger in the reference probe.
Those limitations are retained, not removed by weakening guards or sampling
more trajectory profiles.

## Bounded behavior change

Two explicit modes are added: `finite-correction-probe` and `finite-correction`.
Both inherit exit consistency and its existing parent options. Only the second
changes admission and retention:

- Admit an optional destination proposal only after its native finite
  acquisition and actual-velocity cutoff coast pass the existing checks.
- Keep a checked destination correction while it establishes that coast;
  do not reclassify an obstructed instantaneous ideal arc as a real obstruction.
- Retain valid absolute arrival/turn/cutoff clocks and the existing 24-tick
  optional-query cadence. Declining an optional query leaves the active leg.
- Replan a genuinely obstructed powered acquisition using its recorded
  native conflict. The mandatory actual short-command guard remains unchanged.
- Preserve existing release on an already safe actual coast. Do not force
  every burn to finish or introduce a new waypoint-energy objective.

The initial constructor, 32-profile search, waypoint placement/ranking,
height/exit rules, recovery command family, terminal controller, body reserves,
fuel/state/clocks, domain bounds, cap and deadline stay fixed. The realized
cutoff is prediction evidence, never a restored or assigned live state.

## Stages and integrity-only repair

The focused candidate lands **17/26**, versus ten in the reference: seven gains,
no losses. Gains are 044, 050, 081, 236, 262, 142 and 565. All three external
repeats prove. This selected panel explains mechanics; it is not a population
estimate or a zero-loss requirement for the full diagnostic.

The first focused capture is retained with one reader error in 814. Its native
flight, command replay and decision repeat pass, but the older reader forbids
waypoint replanning from a powered-acquisition conflict. The repair permits that
only for the new candidate with a same-state, exactly bound native acquisition
record. It also keeps optional query records separate from committed active
goals. No native behavior, input, trajectory selection or tuning changes.

The repaired focus is create-only `...focus-20261009-v2`; the failed `v1` is
not rewritten. Probe `v1` remains valid. Native and renderer identities are
unchanged across probe, both focused captures and the full run.

## Full paired accounting

All **1,005 final native records** complete: 1,000 original worlds plus five
external repeats (044, 262, 715, 349, 020). No retries, exceptions or tuning.
Every record proves integrity, original-command source replay and deterministic
decisions. All five repeats and all 26 focused/full overlapping feedback
records are exact. The four direct-control ordinary flights and all 1,000
initial ideal arcs remain exact against the reference.

There are 786 target landings and 214 protected flying stops: no executed
crashes, off-target landings or actual domain violations. Collection takes
318 seconds with four workers, excluding subsequent saved verification. The
reference took 241 seconds; this is aggregate collection cost, not an isolated
planner-runtime benchmark.

| Recipe, 250 worlds each | V14 | Finite correction | Net |
| --- | ---: | ---: | ---: |
| Mountains 4x | 181 | 190 | +9 |
| Mountains 8x | 184 | 204 | +20 |
| Broad massifs 8x | 191 | 202 | +11 |
| Successive ridges 8x | 150 | 190 | +40 |
| Total | 706 | 786 | +80 |

There are 356 zero-H landings and 430 after actual H. Using the older policy-3
initial classification, clear worlds land 313/387 and blocked worlds 473/613;
these are not the candidate's own initial-arc labels. Its own 464 blocked / 536
clear initial arcs stay unchanged. A total of 292 ordinary flights change.

Of the 109 gains, 89 previously stopped for missing aim, nine for a terminal
short-command rejection, five for waypoint construction, four for recovery,
one for waypoint continuation and one for no local waypoint. Four of the prior
eleven V13-success losses recover (142, 236, 262, 565); seven still do not land.

Across primary traces, 223 early destination commits occur. The minimum
same-waypoint optional-query spacing is 24 ticks. There are 2,134 passing
acquisition/retention checks with obstructed ideal arcs across 185 worlds;
these repeated query checks are not 2,134 distinct missions or handoffs.

## Remaining limits and regressions

| Remaining stop | V14 | Finite correction |
| --- | ---: | ---: |
| Missing ballistic aim | 135 | 40 |
| No safe recovery command | 93 | 95 |
| Waypoint construction miss | 32 | 26 |
| Terminal short-command rejection | 19 | 35 |
| Original deadline | 9 | 13 |
| Rejected prediction-domain query | 2 | 2 |
| Local proposal exhaustion | 2 | 3 |
| Waypoint continuation rejection | 1 | 0 |
| No local waypoint | 1 | 0 |
| Total | 294 | 214 |

There are 836 terminal entries, 786 landings (94.0%), 50 terminal stops and
164 preterminal stops. Of the 40 missing-aim stops, 38 are exactly at actual H.
Recovery is now the largest remaining cohort; this pass does not change it.

The 29 new losses are 19 terminal short-command stops, four missing-aim stops,
two recovery stops, one waypoint construction miss and three deadlines. All
case IDs remain in the paired results JSON. The 19 terminal regressions are not
all almost-landed cases: eight stop more than 100 m before the target, six are
within 25 m laterally and 15 m foot-height, and five are other terminal states.
A safe incoming ballistic coast is not a certificate for the controller's
subsequent powered braking trajectory.

715 exposes a separate release/handoff limit: at tick 3488 its current coast
passes the existing approach heuristic with just +0.204 m horizontal room.
Correction releases. At tick 3490 the estimate is -0.509 m, coast acceptance
fails and no replacement fits. It has not crashed. Do not fix this by globally
forcing burn completion or relaxing the braking-room guard without a focused
terminal-entry diagnostic. 862 now reaches terminal near the pad, but still
stops under its unchanged body-reserve guard.

Recommended next investigation: preserve this candidate as the development
reference, review the 29 regressions, and diagnose terminal-entry timing and
terrain clearance on the 19 terminal losses. Start with early loss 080 versus
near-pad loss 538, plus 862 and the distinct 715 release issue. Keep the 95
recovery stops as a separate workstream. Do not resume waypoint-energy ranking
or enlarge search until a common mechanism and bounded fix are demonstrated.
Fresh held-out acceptance and default promotion remain separate decisions.

## Reports, checks and provenance

[Failure-first common batch and 1,005 rich details](../outputs/eval/planner_v2_random_terrain/capture-ballistic-finite-correction-full-20261009-v1/index.html)
retain desired arcs, cutoff coasts, current ballistic motion, actual traces and
handoffs. New native audit details live in each `feedback.json`. The batch
renderer now reports the actual repeat count instead of the inherited hardcoded
two; no historical sealed page is rewritten.

Independent saved verification authenticates **13,278 sealed files**, full
record/input/source bindings, repeats, focused overlap and controls. Protected
accepted/site/navigation/history hashes remain exact. Maintained development
validation passes all twelve checks, including optional exact 44-case numerical
parity, strict Clippy, workspace/CLI checks and 65 Node tests. All 187 terrain-tool
Python tests and 55 scoped ballistic Rust tests pass. Static report verification
checks all 1,005 detail pages, 103,435 recorded planning cycles and 7,039 relative
links with no issues; the capture contains 471,399 samples, 819 actual handoffs
and 38,898 finite-query records including repeats. Batch HTTP returns 200 on the
existing server. These checks are not browser pixel acceptance.

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_finite_correction_panel.py verify full
```

- Native: `f48798836ae6d4d986b4750a0d44dcecd9ff0c4d4c749cb1c4c43ae3fecf9bd8`
- Rust tree: `23aae0d59ef0ce41ecb3afabbd06ba1c8a788ea11cecff03066b5f5a46534187`
- Renderer: `04842592af94f4f7bf997a78d099e0e4a9b890e3d55d28959f4dc57cd35fa22b`
- Full receipt: `d3cc770abbb792e8ce7b298c4a8ab0494e587b1cd383a0b6ec0c44a85f2dcd09`
- Full results: `e1c834ac11f646304dacc5cd664b69497417ee00e6d3ab95d807c0c1a6fdf682`

No commits, pushes, default promotion, root navigation publication, accepted-site
refresh or server changes occur in this pass.
