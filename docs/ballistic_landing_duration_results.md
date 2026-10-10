# Failure-only landing duration: result and limits

[Documentation home](README.md) · [Frozen protocol](ballistic_landing_duration_plan.md) ·
[Previous local experiment](ballistic_local_waypoint_results.md)

Verdict: **entry sampling gap confirmed, complete landing fix not established**.
The one state-derived fallback opens landing control in combined 715/084/142,
but all three stop later under unchanged physical safety guards. The selected
random preservation panel gains 268, moving from 6/16 to 7/16, with no lost
landings. Keep this explicit opt-in; do not promote defaults or launch a new 1k
campaign from these results. All 44 planned native invocations are complete.

## Mechanism and scope

After all existing latest-safe fits fail, query one duration:
`T = 6*dy / (4*vy + 2*target_vy)`. It solves the existing coupled PDG initial
vertical request for `ay = gravity`, or zero initial net vertical acceleration.
Reject nonfinite/out-of-range values; retain the existing 3–14 s limits,
full-vector thrust, upward-direction, tilt and applicable terrain checks.
Never replace an existing admissible selection. Nominal sampling is unchanged.

Entry and live descent share the opt-in, including the terrain-ignore dynamics
copy. It survives controller reset. V8 `landing-duration` inherits V7 local-height
plus early-target; `early-target-landing-duration` inherits early-target alone.
Old identities, ordinary controller constructors, maintained policy 3 and
experimental default `ridge` are unchanged. No waypoint/destination construction,
physical limit, mission clock, contact, clearance or proof contract was weakened.

## Original failures: entry improves, touchdown does not

| Combined subject | Previous stop | New landing entry | New actual stop |
| --- | --- | --- | --- |
| 715 | No ballistic aim, 30.100 s | 30.033 s | Clearance reserve rejected, 34.250 s |
| 084 | No ballistic aim, 26.967 s | 26.883 s | Unsafe contact predicted, 31.633 s |
| 142 | No ballistic aim, 24.200 s | 23.700 s | Unsafe contact predicted, 30.833 s |

All three actual endpoints remain **flying/in-progress**. Predicted crashes
in the final 084/142 refresh belong to rejected 0.2 s held-command queries,
not executed crashes or landings.

- 715 stops at x=1217.33 m versus pad center 1200 m, moving right at 5.19 m/s
  and down at 5.03 m/s. Its rejected prediction goes beyond the pad's right
  edge and fails the existing reserve. Landing entry has not solved edge capture.
- 084 is descending at 22.70 m/s near the target surface. The rejected
  full-thrust prediction contacts terrain at about 20.86 m/s downward.
- 142 is descending at 17.91 m/s with 13.37 m/s lateral motion. The rejected
  full-thrust prediction contacts terrain at about 16.07 m/s downward.

## Why an instantaneous fit was insufficient

A temporary current-source query harness reproduces five complete final states
and all **2,106 accepted terminal commands** exactly, replaying only saved V8
commands. It also reproduces the three rejected final controller-command queries.
Temporary source/trace: `/tmp/pd-coast-diagnostic-quCZfb/src/bin/landing_review.rs`
and `/tmp/pd-coast-diagnostic-quCZfb/landing-review.json`; not portable captures.
No counterfactual flight or weakened guard was used.

Repeatedly recomputing the fallback keeps requesting about 9.81 m/s² upward
while gravity is also 9.81 m/s². The live controller re-solves each tick rather
than committing this short fallback's arrival countdown. Initial admission
therefore does not ensure sustained braking. Applied motion also includes
attitude slew and later maintained-controller choices.

| Subject | Accepted fallback updates | Longest continuous interval | Vertical speed / target-relative height over that interval |
| --- | --- | --- | --- |
| 715 | 23 | 30.033–30.400 s | −32.63→−32.33 m/s; 97.09→85.16 m |
| 084 | 126 | 26.883–28.967 s | −49.97→−46.19 m/s; 202.16→101.35 m |
| 142 | 189 | 25.567–28.050 s | −45.47→−40.93 m/s; 198.92→91.59 m |
| 268 | 5 | 23.717–23.783 s | −50.33→−50.89 m/s; 267.10→263.72 m |
| Early-only 715 | 0 | Not used | Successful ordinary flight remains exact |

The final controller queries in 084/142 have vertical-braking margins of about
−48.04/−29.73 m despite initial acceleration ratios below one. This exposes
insufficient actual braking, not physical impossibility of the original states.
715's lateral/edge issue remains distinct; countdown is not yet proved to fix it.

## Preservation and denominators

- Nine same-source V7 controls reproduce complete saved feedback exactly.
- Nine focused V8 records preserve 349/006 and three exact direct controls;
  715/084/142 stop later. The complete 715 repeat is exact.
- Five early-only V8 records preserve the complete successful 715 ordinary
  flight (37.267 s), its repeat and three exact direct controls.
- The combined 21-record pack has **16 primary random worlds**, three separate
  controls and two repeats. It lands 7/16 versus V7's 6/16, gaining only 268
  and losing none. All six previous successful random ordinary flights remain
  exact; 12/16 random ordinary flights are exact overall. Only 715/084/142/268
  change. All direct controls and external repeats are exact.
- 268 enters landing at 23.717 s and lands at 33.825 s with two actual H.
  Only five fallback updates are needed before the ordinary controller resumes.
  Compared with V6's 8/16, this is still missing 084/142 while gaining 268;
  the earlier local-height combination is not vindicated.

This reused selected development panel is not held-out coverage, a new 1k
pass rate or arbitrary-terrain acceptance. Entry is not a verified landing.

## Validation and retained evidence

All 44 records authenticate full physical/mission/integrity/source-replay/
decision evidence, unchanged inputs, copied source/binaries, external repeats
and source/protected seals. All four packs share one frozen source/binary.
Accepted selectors, navigation, report pages and ordinary release binary stay
unchanged. No retry, flight-driven tuning or additional measured flight.

- Final-source maintained gate: all 11 checks, including strict Clippy,
  workspace/CLI tests, formatting, docs links and 64 Node tests.
- Terminal tests cover bounds/nonfinite inputs, reset, shared adapter/live use,
  original-choice/command preservation, full-vector/tilt rejection and terrain
  veto. Focused ballistic Rust tests: 38/38; study tests: 150/150.
- Planning-view JavaScript: 9/9. Retained October-7 normal-policy parity: 44/44 exact.
- Static common rich reports: 44 detail pages, 3,246 recorded planning cycles,
  actual-H bindings and 88 local batch detail links. Batch and 268 detail URLs
  return HTTP 200 on the existing LAN server. No server restart, pixel inspection
  or human visual acceptance is claimed.

Captures under `outputs/eval/planner_v2_random_terrain/`:

| Capture | Records | Receipt SHA-256 |
| --- | --- | --- |
| `capture-landing-duration-control-20261008-v1` | 9 | `eab587ef651c81320416fb56d12344f5ac153bb23d9d2762f6f32f545879fbba` |
| `capture-landing-duration-focus-20261008-v1` | 9 | `ae4718a203656c4df157a3311ff5db41c14abeeea3604b04bcd358d669a876b3` |
| `capture-landing-duration-early-preservation-20261008-v1` | 5 | `918d4607df899b420d4f08e87f106f9418ee0e998f5042c830540dc5c2f44c4a` |
| `capture-landing-duration-preservation-20261008-v1` | 21 | `fea3542241831d85eb9896f25b5faf16ae64958dd0c584e4d0d64d15db5b0b81` |

Frozen Rust tree:
`648bc83501b9b4a0f5d2453e2a468a8ac88ea0f7b02ed5b41a2be8e73ba26a70`.
Native binary:
`7a7670c93e7d9ed5fc1212adc331709f3ece5aef1950df5adbcebffa19b80102`.
Renderer:
`51b9f2a0e6b5d8d5e0b4d2cc3cf36aff718908fa5731d926548e5b0f3160a959`.
Native path: `target/ballistic-landing-duration-20261008/release/pd-eval`.

Review `/eval/planner_v2_random_terrain/capture-landing-duration-preservation-20261008-v1/`.
Start with gained 268, then 084/142's descent stops and 715's edge stop.
**Jump to decision → maintained_landing_entry** finds the new ownership
transition; the final **short_command_obstruction** distinguishes actual state
from rejected prediction. These local captures are not accepted-site publication.

## Next bounded question, not an authorized campaign

Keep the sampling-gap finding, but do not equate initial acceleration feasibility
with sustained braking. Inspect a small retained arrival countdown using existing
controller machinery, versus repeatedly restarting the zero-net-vertical fit.
Use saved 084/142/268 states before authorizing more implementation/flights.
Keep 715's edge behavior separate. Do not add terrain thresholds, wider duration
grids, guard exceptions or a 1k rerun yet. Early-only still needs a full
preservation panel before a broader coverage claim.
