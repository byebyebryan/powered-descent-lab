# Terminal coordination: selected admission results

[Documentation home](README.md) · [Frozen plan](ballistic_terminal_coordination_plan.md) · [Previous full 1k](ballistic_terminal_centering_sweep_results.md)

Verdict: a substantial selected-world improvement, **115/121 verified random
landings versus V12's 75/121**, but not clean preservation. The two lost V12
landings, **048/755**, fail the predeclared admission. All 129 allowed admission
attempts completed and authenticate; the conditional 1005-attempt sweep did not
run. **59.2% remains the latest measured full-1k ballistic rate.** Do not extrapolate
this deliberately selected recovery panel into a new population rate or promote
the candidate over the retained policy-3 result.

Subsequently, separate user authority ran the
[unchanged-candidate full diagnostic](ballistic_terminal_coordination_sweep_results.md):
639/1000 with 49 gains and the same two losses against V12. That is now the
latest full-1k ballistic measurement; it does not retroactively pass this panel
gate or rerun its closed conditional stage. The statements above describe the
panel-close checkpoint.

## What changed

The new explicit `terminal-coordination` / `ballistic_feedback_v13_terminal_coordination`
mode inherits V12's planner, waypoint and fallback/countdown behavior. An early
vertical-only warning now supplies required lift while retaining nominal lateral
acceleration, clipping to thrust/tilt authority. It no longer automatically
replaces sideways intent with touchdown-rescue intent. Existing final-height and
explicit lateral rescue still own their previous triggers. Low-energy,
body-contained touchdown activates the existing upright settle earlier rather
than retaining small trim. No ordinary default, standalone coast-terminal branch,
mission deadline, terrain, fuel, contact condition or physical reserve was changed.

Strict short-prediction domain errors now carry typed query origin/state and
requested-command evidence. The new mode stops without executing that command,
retains its actual prefix and independently replays/reproduces it. Actual domain
errors have a different label and remain a collection stop. Terrain is never
clamped or extrapolated. Shared geometry retains the original error message and
underlying error type; successful query numerics are unchanged.

## Complete selected accounting

The 121 random worlds are the exact 75 prior gains, 45 lost V10 successes,
and one additional existing one-foot-contact case, 559. Three direct controls
and five exact repeats are separate from that denominator.

| Selected cohort | V12 | V13 |
| --- | ---: | ---: |
| 75 prior gained landings | 75 | 73 |
| 19 new short-command losses | 0 | 19 |
| 14 new budget losses | 0 | 11 |
| 12 unverified former-success domain errors | 0 verified | 11 |
| Existing near-touchdown case 559 | 0 | 1 |
| Random total | 75/121 | 115/121 |
| Direct controls, separate | 3/3 | 3/3 |

Against V12, **42 gains and two losses**, net +40 on the selected worlds. It
recovers 41/45 former-success losses. Against V10, this same slice improves
45/121 to 115/121, with 74 gains and four losses; the older policy-3 slice has
92/121 landings. These are separate comparisons, not contradictory rates.
The batch's generic previous-candidate comparison is **V10**, because V12's
unverified errors cannot be invented as complete flights. Use the V12 comparison
above and the receipt-bound V12 result copy for the current revision comparison.

All 129 records have complete physical/mission, integrity, exact command replay
and decision-reproduction evidence. Every one of the 109 complete V12 primary random
comparisons has unchanged pre-terminal commands, entry clocks and planning/H
evidence. The twelve V12 exceptions cannot supply complete-prefix comparisons.
The three direct controls preserve their pre-terminal evidence and all land;
their terminal commands need not match. Repeats of 142/150/288/757/974 match the
entire same-candidate feedback exactly. No executed crash, off-target touchdown,
native exception, domain stop or retry occurred in this panel.

## Mechanism results and remaining stops

- **150** now lands instead of overshooting about 71 m beyond the pad; **086**
  also lands. This supports separating vertical braking from lateral intent.
- **288** lands at 73.775 s instead of stopping at 79.8 s; **465** lands at
  76.342 s. Eleven of fourteen lost-success deadline cases recover without
  extending the clock. This is a combined revision, not a one-branch ablation.
- **559/757/888** all land upright with complete proof, closing the selected
  small-trim/two-foot-contact cases without relaxing contact geometry.
- Eleven of the twelve selected V12 native-error cases now land. **948** has
  complete evidence and stops at the terrain reserve, rather than losing its
  evidence to an exception. The new domain-stop recording path has synthetic
  tests but was **not exercised by a measured domain-stop flight**. The four
  other V12 exceptions were outside this panel and have not been rerun.

The six remaining flying stops are **048/755/948** at the short-command reserve
and **611/816/986** at the unchanged 79.8 s budget. The latter are still descending:
foot-reference heights are about 0.50/44.25/1.27 m, respectively. Extra-time
landings are not established.

### Why the preservation gate failed

048 and 755 share a clear lateral-capture deficit, not a new waypoint-placement
problem. Preserving the nominal sideways request removes a harmful rescue
override in many worlds, but in these two it also removes useful stronger braking.

- **048:** at 25.0 s, the old flight moves right at 7.67 m/s; the new flight
  moves at 10.94. At 27.0 s, old x is 1.54 m left of pad center with vx=1.42;
  new x is 9.34 m right with vx=5.65. The rejected prediction rotates to -0.42
  rad, moves farther right and carries part of the body outside the flat-pad
  transition while already below the ordinary 5 m reserve.
- **755:** at 42.0 s, old vx=2.45 and x is 12.20 m left of center; new vx=8.87
  and x is 1.09 m left. At the final prediction, full upright thrust brakes
  descent but leaves sideways motion unchanged: the center crosses the 14 m
  body-safe pad interval at about 2.50 m foot-reference height. The guard rejects
  it correctly; no crash was executed.

These are recorded same-clock sample comparisons, not simulated counterfactual
recoveries. The next bounded question is **how to preserve the lateral capture
envelope alongside required lift**, rather than blindly preserve nominal lateral
acceleration or restore the old rescue target everywhere. Inspect the existing
stopping-room/footprint predicates and earlier warning first. Do not hide this
tradeoff with larger pads, weaker reserves, seed rules or extra mission time.

## Validation and retained reports

The maintained development gate passed all twelve checks, including the explicit
44-case retained numerical parity check. Seven new Rust tests cover opt-in
inheritance/reset, symmetric lateral allocation, lift saturation, upright settle,
prediction-domain metadata and actual-versus-predicted failure labels. The 168
terrain-tool tests, four new pure admission/diagnostic tests and two continuation
tests pass. Previous V12 capture verification remains passing.

The shared batch and 129 rich detail reports retain plots, saved samples,
planning refreshes and actual H. Independent static checks authenticate **129
details, 10570 planning-cycle payloads/origins, 142 H annotations, 65022 samples
and 907 local links**. This is data/presentation validation, not browser visual acceptance.
Reports are create-only in
`outputs/eval/planner_v2_random_terrain/capture-terminal-coordination-panel-20261009-v1`.
Start with 150/288 for recovery and 048/755 for the remaining capture issue.
Root navigation, accepted report bodies/selector, previous captures and server
state were not changed; this panel is not the root's latest full-sweep selection.

Read-only verification:

```sh
rtk proxy python3 -B scripts/run-terminal-coordination.py verify-panel
```

The receipt seals 1882 files. Source/binary identities:

- Native: `89de0cffcb61adae04631c7ff25144201c949c7117c7e26e1ae72c17d1f1fc77`.
- Rust tree: `2c42d3691ee8ca675a3c3332871c47e20969b2c3036c32cf0eed0469191fc899`.
- Renderer: `fe0e9720226321d6596f9f4f93068fd68b4d5ffc6ccefed8b8a71b4ad213178d`.
- Receipt: `12e232a09a929d8725b6feee81dd311f6f1401fd6744f6c7e57b614e43980293`.
- Results: `c55328ce66f624ad5e8228b47f816bc4f128b86236b1e33ca0533517d0895640`.

The flight allowance is closed after failed admission. No second candidate,
tuning, broader campaign, commit, push, report/navigation publication or default
promotion follows automatically. The unchanged earlier planning-stop backlog
remains separate; this panel did not rerun those 335 worlds.
