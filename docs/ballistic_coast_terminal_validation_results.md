# Coast-to-terminal preservation and 1k validation results

[Documentation home](README.md) · [Frozen allowance](ballistic_coast_terminal_validation_plan.md) · [Focused mechanism result](ballistic_coast_terminal_results.md)

Verdict: **the current ballistic candidate makes substantial broader progress,
but is not ready to replace policy 3**. The unchanged candidate lands 562/1000
original worlds versus the last complete ballistic sweep's 401: 200 gains,
39 losses, net +161 / +16.1 percentage points. Every terrain recipe improves.
The retained policy-3/cap-24 experiment still leads at 817/1000.

The new coast-to-terminal branch is selected in nine worlds, eight of which
land. This extends beyond 084, but one later terrain failure demonstrates the
limits of its bounded preview. The overall gain combines multiple revisions
since the 401 sweep; it is not the isolated effect of this branch.

## Scope and preservation gate

The candidate's native binary, Rust tree, renderer and planning script remain
exactly those of the successful focused pass. Only collector support and frozen
validation contracts were added. No flight tuning, source rebuild, terrain
regeneration, retry, restored airborne state or guard/default change occurred.

The full preservation panel executes 16 frozen random worlds, three direct
controls and external repeats of 084/715: **21 verified records**. It lands 8/16
versus V9's 7/16, gaining 084 without losses. Every other ordinary flight stays
exact, including all three flat/uphill/downhill direct controls. 084 still lands
with H1 only; external repeats match complete feedback. The panel has no actual
crash/off-target touchdown and passes the predeclared conditional sweep gate.

The subsequent sweep executes all 1000 original worlds in original order plus
external repeats of 084/715: **1002 verified records**. Each native invocation
also independently reproduces decisions and replays commands from the original
scenario. The 16 overlapping panel/sweep feedback records match completely.
The entire new allowance is 1023 records, with no interruptions or extra attempts.

This is paired development on an already observed population. The panel is
inside that population, and there is no full V9 1k baseline. Neither these worlds
nor the focused successes become a fresh held-out test through rerunning them.
Normal cap-6 policy 3, the accepted 44-case site, controller defaults and the
experimental `ridge` default remain unchanged.

## Broad outcome

| Terrain recipe, 250 worlds each | Last complete ballistic candidate | Current candidate |
| --- | ---: | ---: |
| Mountains 4x | 128 | 156 |
| Mountains 8x | 96 | 146 |
| Broad massifs 8x | 107 | 147 |
| Successive ridges 8x | 70 | 113 |
| Total | 401 | 562 |

Of the 562 landings, 274 have zero actual H and 288 follow waypoint handoffs.
All 438 remaining physical states are flying with in-progress missions and
finite planning stops. There are **no actual crashes or off-target landings**.
Maximum actual H is five; there are no correction-cap or recovery-episode-budget
stops. One flight reaches its unchanged original mission budget.

Using the retained policy-3 nominal classification—not the candidate's own
initial arc labels—clear-world landings improve from 260/387 to 285/387, and
blocked-world landings from 141/613 to 277/613. The new candidate's own initial
arcs classify 536 clear and 464 blocked; these are different predicates and
must not be mixed. Policy 3's retained experiment lands 387/387 clear and 430/613
blocked worlds. Versus that 817 checkpoint, the current candidate gains 72
landings but loses 327; a higher count than 401 is not replacement acceptance.

## Coast branch: useful, not a landing certificate

The branch selects and enters terminal control in
`084/250/259/399/421/428/440/501/900`. The first eight land; **900 stops later**.
There are no coast cancellations. The 287 rejection records are:

- 174 current-coast obstruction rejections;
- 104 bounded coast-preview budget rejections;
- eight entry/terminal-preview rejections;
- one feature-too-close-to-destination rejection.

These are proposal counts, not distinct-world or landing counts. A rejected
query leaves the existing loop in control. None of the 39 regressions versus
401 selects this branch; that observation does not isolate the other revisions
or establish what V9 would do on all 1000 worlds.

In 900, selection occurs at 21.450 s and automatic terminal entry at 21.817 s,
position (677.435, 501.971) m. The two-second terminal preview passes. Actual
terminal flight stops at 24.233 s, still flying at (816.673, 347.659) m; the
24-tick held-command query predicts a reserve violation near x=827.196 m.
That is 2.417 s after entry, beyond the declared preview. It demonstrates later
terrain obstruction under terminal ownership, not an actual crash, a weakened
guard or an incorrect replay. Do not fix it by outcome-driven preview extension
or an end-to-end suffix search in this closed validation pass.

## Remaining stops and regressions

| Finite planning stop | All remaining worlds | Losses versus 401 |
| --- | ---: | ---: |
| No ballistic aim | 131 | 3 |
| No admitted terrain-recovery command | 103 | 5 |
| Terminal command prediction rejected | 102 | 20 |
| Waypoint continuation rejected | 70 | 5 |
| Waypoint aim construction miss | 30 | 5 |
| No local waypoint | 1 | 0 |
| Original mission budget | 1 | 1 |
| Total | 438 | 39 |

Twenty regressions stop under terminal command prediction; another stops at the
mission budget while terminal control owns flight. The other eighteen are
pre-terminal. Those are useful shared-stage groups, not yet a common root-cause
diagnosis. More missions reaching terminal also changes its exposure denominator;
raw stop-count growth alone is not controller-regression evidence.

The 39 paired losses are
`137/139/142/257/271/324/340/366/426/427/432/444/500/506/530/531/564/576/586/678/680/706/734/751/758/767/772/781/827/839/850/856/894/914/917/926/934/960/974`.
The complete gain/loss inventories versus both baselines remain in the results
JSON. No failed world or historical result is removed or relabelled to improve
the count.

## Evidence and reports

Create-only roots under `outputs/eval/planner_v2_random_terrain/`:

- `capture-coast-terminal-preservation-20261009-v1`: 477 sealed files; receipt
  `971d65ca7440405935363db8c56562275a61fd9ffd88c2f835b0014142f98248`.
- `capture-coast-terminal-sweep-20261009-v1`: 12291 sealed files; receipt
  `3b49d6b57410db740780b408fbb6f43a65adb4b7223f6ce68ccbc17a7f4eb648`.

Sweep results SHA-256:
`47fd9c9f2b1a078335b1e5d79b482df89f172975fb2ee94462f7e4a14b1425cc`.
The [frozen sweep contract](../studies/terrain_profiles/ballistic_coast_terminal_sweep_plan.json)
has SHA-256 `d7b29cd8c18a82be4eb0d1303e2082785b6439ea4a73d85267b4dab9ab55c186`.
It pins both baselines, the successful gate panel, the native/Rust/report
identities, source paths and fixed allowance. The capture carries portable
gate receipt/results/manifest copies and the original validation protocol.

Native binary remains
`24684011dc502fed49b1aa314d00f52fa2dc4ed9fd1ff09dbf2be01420081d82`; Rust tree
`8225fe78691aedc1b797171d997097c90107c4c222509945a79e1f636a6b2231`.
Neither measurement follows a native rebuild. Both saved verifiers pass, binding
input identities, complete decision/command/source proofs, repeats, source seals,
protected old evidence/site seals and exact outcome projections.

Collector tests pass 159/159 before the panel and 161/161 before the sweep;
21 relevant Node tests pass. This pass changes no Rust or report implementation;
the focused pass's maintained gate and 44-case normal-policy numerical parity
remain prior preservation evidence, not new candidate acceptance flights.

Static presentation checks cover 21 panel details, 1475 cycles and 16 actual H;
plus all 1002 sweep details, 69591 cycles and 774 actual H. All cycle origins,
goals, executed entry/H bindings and rich sections agree with source data.
There are 1071 panel and 7018 sweep local link checks. The existing diagnostic
batch shell/tree and rich templates are preserved; no browser visual acceptance
is claimed. The existing LAN server returns HTTP 200 for both batch pages and
the sweep's 084/900 details. No accepted navigation/publication or server change.

Review the full batch at
`/eval/planner_v2_random_terrain/capture-coast-terminal-sweep-20261009-v1/`:

- **084**: the automatic one-H success; refresh 67 selects coast, 68 enters terminal.
- **250 or 259**: successful coast selection outside the original focused subject.
- **900**: refresh 64 selects coast, 65 enters terminal; later terrain reserve stop.
- **139**: a paired loss before H, distinct from terminal ownership.
- **142 or 974**: paired losses during terminal control; red predicted contacts
  are not actual flown crashes.

Read-only saved verification, without another flight or current native binary:

```sh
rtk proxy python3 -B studies/terrain_profiles/coast_terminal_validation.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-preservation-20261009-v1
rtk proxy python3 -B studies/terrain_profiles/ballistic_feedback_sweep.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-sweep-20261009-v1
```

## Next decision

Keep the useful coast branch, but do not promote the combined candidate from
this result. Next inspect the 39 previously successful worlds as a focused
regression cohort, separating pre-terminal route/acquisition losses from the
20 terminal-prediction losses. Use 900 to review when terminal ownership should
yield back to planning if later terrain is encountered. Diagnose shared state
and ownership patterns before another small mechanical change; do not add
case-specific heights, longer previews or more retries. The 131 missing-aim
stops are another large cohort, not automatically the same defect.

The pass is closed. No new flights, native tuning, default adoption, commits,
pushes or accepted-site publication followed validation.
