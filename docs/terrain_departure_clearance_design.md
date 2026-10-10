# Departure clearance design and review

Status update: **parked**, not the selected next implementation. The
[planning-cycle review](terrain_planning_cycle_review.md) separates later
approach/acquisition and missing-nominal stops from the 46-world departure
cohort. Preserve this analytical hypothesis and its negative predecessor;
diagnose the actual give-up mechanisms before another departure experiment.

Date: 2026-10-08. Verdict: one terrain-screened lift and advance maneuver is
worth a bounded physical probe. Another fixed lift/forward ratio is not. The
screen chooses when to advance; actual plant queries and source replay still
decide whether H is safe. This design pass changes no planner behavior. The
[implementation plan](terrain_departure_clearance_plan.md) needs separate
approval before new flights.

## Saved evidence and common failure pattern

The [previous experiment](terrain_departure_probe_results.md) adds no H on
`327/791`: all 42 added query traces lose the 5 m body reserve before a supported
progress/certificate boundary. Both actual flights remain unchanged zero-step
`NoClearing` stops, not takeoff crashes.

The broader design cohort is every primary zero-correction `NoClearing` record
whose first nominal obstruction is in `source_bridge`, in the receipt-pinned
early-exit/cap-24 capture: **46 worlds**, selected before computing new geometry
or screen metrics. These are inspected development failures, not held-out data.
All 46 have at least one saved full eight-second upright-powered query that
maintains actual reserve through its entire finite trace. None of those searches
admits H. In 23 worlds no row reaches continuation checks; the other 23 have
continuation rejections in the broader search. A clear upright stem is not an
accepted H: progress is checked first.

Terrain between the source and the existing progress coordinate, including
body radius, rises 129–488 m above the source surface. These are steep local
hills. Raising the direct nominal in response would violate terrain-blind
direct-flight selection.

Rejected rows save their endpoint, not full E or intermediate traces. For a
complete upright row, its powered-end position/velocity can be derived by
undoing the known eight-second idle tail. With tail T and physics interval dt,
`vy_power = vy_end + g*T` and
`y_power = y_end - vy_power*T + g*T*(T+dt)/2`.
This follows the current semi-implicit gravity update. These are derived phase
boundaries, not persisted E snapshots or fresh replay evidence.

## Analytical screen results

The read-only [study helper](../studies/terrain_profiles/departure_design.py)
uses those saved stems, unchanged terrain and the existing conservative thrust
acceleration, approximately 13.32 m/s². It checks a short powered advance to
existing progress, idle handoff and two-second continuation, not a landing suffix.

| Analytical observation | Worlds |
| --- | ---: |
| At least one reserve-clear saved upright stem | 46/46 |
| Forward screen passes immediately after eight-second lift | 30/46 |
| Screen passes with optional further lift, within 16 s total power | 46/46 |

The last two rows are **heuristic estimates, not native admissions or landings**.
The shortest estimated total power among each world's saved eligible stems
ranges from 9.92 to 15.75 s, median 11.10 s. The sixteen seconds includes lift
and forward power together, not a separate allowance for each stage.

The earliest full upright stem in `327` suggests eight seconds of lift followed
by approximately 3.15 s of powered turn/advance. In `791`, that stem needs about
2.67 s more lift before roughly 3.20 s of turn/advance: 13.87 s total power.
Later entries can need less lift but have worse drift risk. The last admitted E
on both subjects already loses actual reserve while requesting upright thrust;
retain that rejection rather than reset velocity.

Sixteen seconds is a doubled powered-work ceiling for one row, not a solvability
threshold or a value tuned by candidate flights. `847` is close to this ceiling
in the model. Freeze it before measurement; retain a miss if the plant cannot
meet it. Model success in every world is not evidence that every world will land.
These examples start at saved eight-second powered boundaries. They do not
reproduce the proposed earliest-switch feedback policy, which may switch sooner.
They establish a plausible available maneuver shape, not the policy's actual
selection, exact turning trace or final landing outcome.

## Candidate behavior

Complete the unchanged primary and timing-fallback searches first. Only on
cycle-zero `source_bridge` exhaustion with no accepted old row, try **one new
maneuver per already admitted E**, at most eight rows. Add no clocks, duration
grid, thrust factors or attitudes to search over.

1. From genuine E, request upright thrust at the existing conservative cap.
   Keep live velocity, attitude, turning, mass, fuel and clocks. Check actual
   body reserve every physics step.
2. Once upright settles and vy is nonnegative, evaluate the cheap forward
   screen at each command boundary. Switch once, at the first screen predicting
   at least 5 m reserve within the remaining total powered budget. A failed
   screen means continue lifting, not that the mission is impossible.
3. Request +30-degree thrust at the same cap until **actual** x reaches existing
   progress. Reject the row if the combined sixteen-second power limit or
   original deadline expires.
4. Request zero throttle/upright. Select the earliest aligned, settled ordinary
   H with useful progress, supported state and the existing actual two-second
   continuation certificate. Keep the six-second coast ceiling. Execute only
   a fully proved proposal.
5. Replan the terrain-blind nominal from actual H. Another terrain correction
   may follow; local admission never requires an eventual landing suffix.

Stages never alternate. No braking, retreat, obstacle segmentation or terrain
feature's far-edge finder is added. Keep this initial-departure-only. Old ranking
and old early exits remain unchanged; the new family uses ordinary H.

## Local screen geometry

Use current x/vx/vy/y to estimate powered turn/advance to P and idle continuation.
For forward acceleration `ax = A*sin(30°)`, the positive duration is
`t = 2*(P-x)/(sqrt(vx²+2*ax*(P-x))+vx)` when remaining distance is positive.
Use `ay = A*cos(30°)-g`; round command endpoints upward, with at least one
forward interval. Include an idle upright turn before the two-second certificate.

Turning is not instantaneous. For upright-to-theta at angular speed omega,
continuous constant-thrust integrals include
`delta_vx = A*(1-cos(theta))/omega` and
`delta_x = vx*theta/omega + A*(theta-sin(theta))/omega²`.
Vertical increments include integrated thrust and gravity. During that rising
powered turn, check its swept horizontal span against starting height.

Forward/coast pieces are quadratics. Use a square envelope of half extent R,
the existing conservative body radius. On each intersected terrain line,
clearance is quadratic in time: endpoints and any interior minimum suffice.
Include terrain vertices crossing either horizontal envelope extreme, so narrow
peaks cannot be skipped. Reject an out-of-domain screen explicitly.

This screen is not a rigorous bound on the discrete plant. Held-command mass
conversion, semi-implicit integration and attitude stepping differ. Wider body
geometry does not certify the whole estimate. False positives must fail the
actual query; false negatives may waste lift or miss a row. Keep those distinct
instead of adding per-seed margins. Synthetic dense-sample checks validate
segment arithmetic, not saved mission feasibility.

## Review decisions and simplifications

- Keep correction outside nominal generation; no terrain-dependent apex fitting.
- Use a short projected envelope, not the whole route's maximum height. Requiring
  all hilltop altitude before advance ignores height gained during forward flight.
- Explicitly extend one row's **total** power. Replacing the half-way switch
  inside the old eight-second total leaves too little time for lift plus advance.
- Keep progress and the two-second certificate. Progress relaxation and a
  landing-suffix requirement do not fix the independently observed reserve loss.
- Prefer already available early E clocks. A source-rest exception or braking
  stage expands support unnecessarily; reject unsafe drift in this first probe.
- Freeze one screen and budget. No independent duration ratios, new solver,
  adaptive tuning or candidate-by-candidate landing fits. This remains offline
  game-oriented planning; real-time performance is unmeasured.

## Value and limits

This targets 46 of 183 remaining development misses. Solving every one would
add at most 4.6 percentage points to the existing 1k result, not repair the 83
after-H clearing stops, 35 nominal stops or other 19 unlaunched stops. That is
an upper scope bound, not an expected gain or a new sweep measurement.

One source-sealed implementation/probe is justified. Further departure tuning
is not automatic if it fails. No new H means review the predictor/realization
discrepancy and retire this candidate. H followed by missing nominal means a
separate airborne-acquisition diagnosis. The [staged plan](terrain_departure_clearance_plan.md)
keeps development checks, maintained acceptance and later untouched validation
separate.

## Reproduce the study

```sh
rtk proxy python3 -B studies/terrain_profiles/departure_design.py
rtk proxy python3 -B -m unittest discover -s studies/terrain_profiles
```

The helper authenticates the original receipt, native executable and each saved
scenario/flight/report it reads, then prints JSON to stdout. It writes no capture,
invokes no native process and regenerates no terrain. Baseline:
`capture-early-exit-sweep-20261008-v2`; receipt SHA-256
`e12c5145626f572a1d6df35d1dbb949db3bb52e35515a09ebbedbf043aaca0f0`.

The design-pass checks pass: 121 terrain-study tests (including nine new helper
tests), eight documentation-link Node tests, 835 local links and `git diff --check`.
Two full analytical reads match exactly with `subprocess.run` disabled. All 48
accepted selector/report files match the prior protected inventory. The six
planner/evaluator runtime/Cargo files remain clean; no flight campaign, report
publication, server operation, commit or push occurred in this pass.
