# Retained landing countdown: result and limits

[Documentation home](README.md) · [Frozen protocol](ballistic_landing_countdown_plan.md) ·
[Previous duration result](ballistic_landing_duration_results.md)

Verdict: **countdown works, but does not solve landing**. Combined 084/715/142
still stop under unchanged physical guards. The selected random preservation
panel stays at 7/16, with no gained or lost landings versus V8. All 48 planned
records verify. Keep V9 explicit opt-in; do not promote defaults or run another
1k campaign from this result.

## What changed

Only after V8's failure-only duration is actually selected by live latest-safe
control, retain its first arrival using existing terminal-plan storage. Each
update re-evaluates the remaining fit against current state, existing thrust,
tilt, upward-direction and configured terrain checks. Release once on expiry
or infeasibility, back to V8, without re-admission or deadline extension.

Initial seed bounds remain 3–14 s; the countdown uses the existing retained-plan
0.5 s residual floor. Target-surface and current desired-vertical-speed
conventions are unchanged. Entry assessment is unchanged. Ordinary constructors,
authored-waypoint retention, maintained policy 3 and experimental default
`ridge` are unchanged. No planner construction, physical guard, state/fuel/clock,
contact rule or replay proof was weakened.

Explicit V9 modes are `landing-countdown` (combined local-height/early-target)
and `early-target-landing-countdown` (early-only preservation). Internal countdown
admission/release is not a waypoint handoff or a planner goal change.

## Original subjects: no new landing

| Combined subject | Terminal entry | Countdown released | V9 actual stop | V8 actual stop |
| --- | --- | --- | --- | --- |
| 715 | 30.033 s | 30.817 s: tilt | 33.667 s: reserve | 34.250 s: reserve |
| 084 | 26.883 s | 30.767 s: thrust | 31.567 s: contact prediction | 31.633 s: contact prediction |
| 142 | 23.700 s | 23.800 s: terrain | 30.767 s: contact prediction | 30.833 s: contact prediction |

Actual endpoints remain **flying/in-progress**, not executed crashes or landings.
Unsafe contacts appear only in rejected 0.2 s held-command predictions.

- 715 reaches x=1216.25 m versus pad center 1200 m, moving right at 17.99 m/s
  and down at 8.95 m/s. Its rejected prediction goes beyond the pad edge.
  This is worse lateral capture than V8, not merely the same stop time.
- 084 reaches the target vicinity still descending at 25.49 m/s; the rejected
  full-thrust prediction contacts at about 23.65 m/s downward.
- 142 stops with 13.44 m/s lateral motion and 18.54 m/s downward motion;
  the rejected prediction contacts at about 16.77 m/s downward.
- 268 still lands at 33.825 s, but its ordinary flight is changed, not exact.
  Early-only 715 still lands at 37.267 s with its ordinary flight exact.

## Why retaining the clock was insufficient

A temporary current-source query harness reproduces five complete final states,
all **2,063 accepted terminal commands**, and the three rejected final command
queries exactly. Physical transitions replay saved V9 commands only. Temporary
source/trace: `/tmp/pd-coast-diagnostic-quCZfb/src/bin/countdown_review.rs` and
`/tmp/pd-coast-diagnostic-quCZfb/countdown-review.json`; these are not portable
capture artifacts or counterfactual flight proof.

| Subject | Accepted retained updates | Fixed arrival | Release observation |
| --- | --- | --- | --- |
| 715 | 47 | 33.869 s | Requested downward terminal speed changes from 4.45 to 1.80 m/s. Retained fit needs too much lateral tilt despite thrust ratio 0.549; terrain is clear. |
| 084 | 233 | 32.347 s | Remaining time 1.581 s; thrust ratio rises to 1.0036. Terrain/tilt pass, but actual descent is still 32.69 m/s with 27.64 m local clearance. |
| 142 | 6 | 31.022 s | Retained fit fails terrain at 23.800 s, while thrust ratio 0.636 and tilt pass. |
| 268 | 13 | 30.983 s | Retained fit fails terrain at 23.933 s; ordinary controller subsequently lands. |
| Early-only 715 | 0 | Not admitted | Successful original controller choices never need the fallback. |

The countdown does not reset or extend its arrival. In 084 it increases requested
vertical braking, but runs out of thrust before meeting that deadline. In 715
the changing terminal-speed policy changes the fit even with a fixed arrival;
after release, the unchanged maintained controller does not capture the pad edge.
Simply ignoring these release conditions is not a justified fix.

There is also a simple analytical gap in the fallback itself. The existing
`required_control_accel` law evaluates the **initial** acceleration of a
fixed-endpoint cubic position fit. With a frozen endpoint and terminal velocity,
its ideal thrust acceleration is affine over time. Its ending request is:

```text
ax_end = -6*dx/T² + 2*vx/T           # terminal vx = 0
ay_end = -6*dy/T² + (2*vy + 4*target_vy)/T + gravity
```

These values are an analytical diagnostic, not additional flight attempts.
Using each first selected V9 fallback and holding its entry mass/endpoint fixed:

| Subject | Initial thrust ratio | Ideal ending thrust ratio |
| --- | --- | --- |
| 715 | 0.689 | 1.223 |
| 084 | 0.555 | 1.294 |
| 142 | 0.631 | 1.218 |
| 268 | 0.637 | 1.195 |

Ratios above one exceed the **entry-state** thrust limit. Fuel/mass changes,
attitude slew, terrain and live terminal-speed changes still matter; this is
not an exact whole-flight feasibility test. It does establish why an initially
admissible zero-net-vertical request need not be an executable retained landing
profile. Countdown alone does not repair the choice of profile.

## Preservation, proof and scope

- Eleven same-source V8 control records reproduce complete saved feedback exactly.
- Eleven focused V9 records retain 268/349/006 landings and three exact direct
  controls. 715/084 repeats reproduce complete feedback exactly.
- Five early-only records preserve the complete successful 715 ordinary flight,
  its exact repeat and three exact direct controls. No countdown is admitted in
  that successful 715 descent.
- The combined 21-record pack has **16 primary random worlds**, three controls
  outside that denominator, and two repeats. It lands 7/16 versus V8's 7/16:
  no gains/losses. Twelve random ordinary flights remain exact. Only
  715/084/142/268 change; all six other successful random ordinary flights remain
  exact. 715/349 external repeats are exact.

Every record authenticates the complete physical/mission/integrity/source-replay/
decision tuple, frozen inputs, copied source/binaries and external repeats.
All four packs share one source/binary; final source/protected seals remain exact.
Accepted selectors, navigation, site pages and ordinary release binary are
unchanged. No retries, flight-driven tuning, extra native attempts or publication.
This selected development panel is not fresh/held-out or arbitrary-terrain coverage.

Validation: maintained development gate passes all 11 checks, including strict
Clippy and 64 Node tests. Five countdown unit tests cover admission, original
choices, retained clock, rejection, expiry, no re-admission and reset; focused
ballistic Rust tests pass 39/39 and study tests 153/153. Planning-view JavaScript
passes 9/9. Retained October-7 normal-policy numerical parity remains 44/44 exact.
Static inspection covers 48 common rich detail pages, 3,594 recorded planning
cycles, 43 actual-H revision bindings and 96 local batch detail links.
Batch and 084 detail URLs return HTTP 200 on the
existing LAN server; no restart, pixel inspection or human visual acceptance.

Captures under `outputs/eval/planner_v2_random_terrain/`:

| Capture | Records | Receipt SHA-256 |
| --- | --- | --- |
| `capture-landing-countdown-control-20261009-v1` | 11 | `53b0e69c1ce430b3c557da902f1fc2d7cf28ba367ddd53d1a8e9973a68af9d37` |
| `capture-landing-countdown-focus-20261009-v1` | 11 | `a5487c03262d55ad99dba72d6ea543bdc66e4b1565dfd3ae1dc6e42e5cbabb4d` |
| `capture-landing-countdown-early-preservation-20261009-v1` | 5 | `0c54474c5e6b7260b7d566b9619933fc0ef5067cd1a78711df4a8a8a7670cf51` |
| `capture-landing-countdown-preservation-20261009-v1` | 21 | `0d279c1d4e7c52a6eb0c9962c1bfa008d7fb49ab778e2469bb76928e39ca19d6` |

Frozen Rust tree:
`74fc55091e9eba00201b9484a596fbd4e269f604427c7b73a39565ed10a8ad0e`.
Native binary:
`5c1c524006e63542860a015436f0be8096194597a697e7bab45045908be483d0`.
Renderer:
`fc741e76663d81d103976810f7229575c11c61a567954f39ce5c7f0b5a571470`.
Native path: `target/ballistic-landing-countdown-20261009/release/pd-eval`.

Review `/eval/planner_v2_random_terrain/capture-landing-countdown-preservation-20261009-v1/`.
Use **Jump to decision → maintained_landing_entry**, then final
**short_command_obstruction**. The early-only preservation pack provides the
successful comparison for 715. These local reports are not accepted-site publication.

## Next bounded question, not another authorized campaign

Keep the sampling-gap finding and the working retained clock, but close the
claim that this fallback plus countdown is a landing solution. Before more
flights, use these saved entry states to ask whether a simple duration rule can
request braking sooner while keeping the complete ideal acceleration profile
within authority. Endpoint checks on the affine request are cheap; that is a
simpler direction than a wider coast/terminal grid or additional terrain rules.
They still need the existing actual-command/terrain guards and are not a landing
certificate. Keep 715's changing terminal-speed/edge-capture issue explicit.
Do not ignore release vetoes, widen thresholds, add recovery families or rerun
1k merely to compensate for this negative result.
