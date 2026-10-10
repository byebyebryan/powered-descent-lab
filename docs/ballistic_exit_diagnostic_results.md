# Frozen exit check: full 1k diagnostic

[Documentation home](README.md) · [Frozen diagnostic plan](ballistic_exit_diagnostic_plan.md) · [Next-pass design](ballistic_correction_ownership_design.md)

Verdict: **706/1000 verified target landings (70.6%)**, versus V13's 639.
The unchanged exit-check candidate gains **78** and loses **11**: net **+67**,
or **+6.7 percentage points**. All four terrain recipes improve. This is a
worthwhile broad development improvement, not clean preservation, held-out
generalization or replacement acceptance. The older policy-3/cap-24 result
remains stronger at 817/1000; ordinary cap-6 defaults are unchanged.

The failed 45-world panel verdict remains failed. This separately authorized
diagnostic demonstrates why local zero-regression admission is not a suitable
universal prerequisite for learning on the full population. It does not open
the old combined stage, relabel that panel or promote this candidate.

## Complete accounting

All **1,005 native attempts** complete: 1,000 original worlds plus five external
repeats (047/142/715/044/020), no retries or exceptions. Every record has the
physical/mission/integrity/source-command-replay/decision-reproduction tuple.
There are 706 target landings and 294 protected flying stops, no executed
crashes, off-target landings or actual terrain-domain violations.

All five external repeats and all 45 overlapping exit-panel feedback records
reproduce complete bytes exactly. Four zero-H control flights remain exact
against V13. All 1,000 initial ideal arcs are unchanged. Native, Rust, renderer,
report script, terrain, clocks, fuel, guards, deadlines and cap-24 opt-in stay
fixed. Collector work creates a separate diagnostic contract without changing
flight logic or rebuilding the native binary. Collection takes 241 seconds,
excluding final saved verification/report accounting.

| Recipe, 250 original worlds each | V13 | Exit check | Net |
| --- | ---: | ---: | ---: |
| Mountains 4x | 169 | 181 | +12 |
| Mountains 8x | 162 | 184 | +22 |
| Broad massifs 8x | 179 | 191 | +12 |
| Successive ridges 8x | 129 | 150 | +21 |
| Total | 639 | 706 | +67 |

Using the older policy-3 initial classification, clear worlds land 308/387
(versus 295) and blocked worlds 398/613 (versus 344). These labels are not the
ballistic candidate's own route classification. There are 317 zero-H landings
and 389 landings after actual H; zero-H does not mean obstacle-free flight.

The change affects 329 complete ordinary flights: 149 old winners and 180 old
stops. Of the 149 changed old winners, 138 still land and 11 regress. Thus the
selected panel's 22/45 changed-flight footprint was not a population estimate.

## What worked

Of V13's 70 waypoint-continuation stops, **46 now land**. Only 352 still ends
under that label. The remaining 23 become 12 missing-aim stops, four waypoint
construction misses, four deadline stops, two terminal short-command stops and
one recovery stop. A removed failure label alone is not counted as success.

The 78 gained landings come from:

| Original V13 stop | Gained landings |
| --- | ---: |
| Waypoint continuation | 46 |
| Missing ballistic aim | 15 |
| No safe recovery command | 9 |
| Terminal short-command rejection | 5 |
| Waypoint construction miss | 2 |
| Original deadline | 1 |

The check therefore improves more than its original stop cohort. It changes
waypoint construction/height repair and the resulting realized state, not the
terrain-blind initial destination arc. These downstream effects are why both
gains and losses require the wider paired run.

## Losses and remaining bottleneck

The eleven former V13 landings now stop as follows:

| New stop | Cases |
| --- | --- |
| Missing aim immediately at H | 236, 262, 862, 971 |
| Later waypoint construction miss | 565, 814 |
| Initial four-proposal exhaustion, no takeoff | 570 |
| Terminal body-reserve rejection | 142, 392, 715 |
| Rejected terminal prediction-domain query | 357 |

All eleven contain waypoint height repairs. That is an association with this
changed construction path, not proof that a specific lift or speed alone caused
every loss. For example, 392's repairs are 84.94 m then 27.98 m; it later reaches
terminal near the pad edge at vx 52.62 m/s. 570 exhausts the four proposals at
tick zero after three repairs; it is not a takeoff crash. Keep these mechanisms
separate rather than adding another global waypoint-height knob.

357's actual state is still in-domain at x=1347.97 m. A held-command clone
reaches x=1353.78 m and its body query extends past the fixed domain; the command
is not executed. 696 retains V13's known prediction-domain stop. No terrain
extension, clamping or weaker body reserve was used to remove either label.

| Remaining stop | Count |
| --- | ---: |
| Missing ballistic aim | 135 |
| No safe recovery command | 93 |
| Waypoint construction miss | 32 |
| Terminal short-command rejection | 19 |
| Original deadline | 9 |
| Rejected prediction-domain query | 2 |
| Local proposal exhaustion | 2 |
| Waypoint continuation rejection | 1 |
| No local waypoint | 1 |
| Total | 294 |

There are **264 preterminal stops** and 30 terminal stops. Of 736 terminal
entries, 706 land (95.9%). Of the 135 missing-aim stops, **134 occur exactly at
actual H**; their median remaining distance/vx is 88.96 m / 71.36 m/s. The
exit mismatch is largely resolved, but fast late handoffs remain the largest
cohort. Another terminal-only change cannot resolve that upstream limitation.

## Viable next options, not new fixes in this pass

The [ownership/energy design](ballistic_correction_ownership_design.md) separates
optional replan transactions, correction retention/cadence and a soft preference
for braking room among existing waypoint fits. Its read-only screen reproduces
the recorded constructor choices for 044/050/062/081/715 and finds slower fits
within the existing 32 profiles. Those alternatives require much more thrust
and time; they are neither terrain-audited nor flown. No new landing claim
follows from their nonnegative heuristic room.

Recommended next reference: this source-frozen exit candidate, with its eleven
known losses retained. First test ownership, then ownership plus the soft room
preference with objective-aware completion. Keep recovery-horizon selection as
a separate diagnostic. Use focused missions to explain mechanisms, followed by
paired 1k diagnostics after basic correctness/proof checks—not a requirement
that every small panel has zero landing losses. Default promotion and fresh
held-out validation remain separate decisions.

## Reports and verification

[Common failure-first batch and all 1,005 rich details](../outputs/eval/planner_v2_random_terrain/capture-ballistic-exit-diagnostic-1k-20261009-v1/index.html)
are retained in a create-only capture with **13,275 sealed files**. Start with
gain 047 and loss 142, then 236/570/392 to distinguish handoff, proposal and
terminal failures. Recorded desired arcs, cutoff estimates and actual H remain
separate in the rich planning-cycle views.

Independent saved verification checks every record, frozen source/input/older
comparison bindings, external repeats, complete panel overlap and controls.
Protected current/site/navigation/history hashes match their preflight values.
Static checks cover all 1,005 details, 72,107 inline/sidecar planning cycles and
their recorded origins, 442,368 saved samples, 861 actual handoffs and 180
recovery-query summaries. All 7,039 relative batch/detail links resolve. Batch
HTTP returns 200 on the existing server; no browser pixel acceptance is claimed.

The frozen batch renderer's prose says two repeats and a full candidate change.
Those inherited descriptions are inaccurate here: the plan, manifest and results
have five repeats and one isolated exit-check change. The previous-candidate
headline is correctly V13's 639, while its separate older baseline is 817. No
sealed page was rewritten to hide these presentation limits.

All 182 terrain-tool Python tests pass, including five diagnostic authorization/
scheduling tests and three output-free mathematical-screen tests. The maintained
eleven-step development gate passes, including strict Clippy, formatting,
workspace/CLI checks and 65 Node tests. No new optional retained-parity campaign
was needed: native/Rust flight identities are unchanged from the V14 pass.

Read-only verification:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_exit_diagnostic.py verify
```

Receipt: `48f84223cc6e9212276588130168d323d5885f9aa9acdba4cdf323e8787d2ac9`.
Results: `76e9c8088cdb53938ad714260ba5d1aa833241a15ea9a4e66ac56ea046c3d782`.
The [frozen plan](ballistic_exit_diagnostic_plan.md) records the exact reused
native/Rust/renderer/script identities. The later read-only mathematical screen
is separate from the captured flight source; it does not relabel provenance.

Ordinary defaults, accepted selector/site, root navigation, previous captures
and server state are unchanged. The new batch is reachable at its direct capture
path, not newly published from root. No commit, push, default promotion, extra
flight campaign or new flight-mode implementation occurred.
