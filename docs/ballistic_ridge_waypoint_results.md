# Ridge-aware waypoint results — 807 solved, broader replacement rejected

[Documentation home](README.md) · [Pinned plan](ballistic_ridge_waypoint_plan.md) · [V4 comparison](ballistic_waypoint_clearance_results.md)

## Verdict

The opt-in `ballistic_feedback_v5_ridge_waypoint` identifies and targets the main
807 ridge instead of handing off repeatedly on its rising face. **807 now lands
on the target with one actual waypoint handoff, at 44.05 s.** This supports the
user's feature-placement diagnosis; it does not establish general improvement.

The fixed preservation panel loses three V4 landings while gaining 807. Its
16 selected random worlds land **8/16 versus 10/16** in V4. The three direct
controls still land with exactly unchanged complete ordinary flights: including
them gives **11/19 versus 13/19** primary missions. Two repeats are outside those
denominators. There are no physical crashes; the eight other random outcomes
remain finite flying stops. V5 is **not ready to replace V4 or accepted policy 3**.
No 1k run, default promotion or threshold tuning followed the mixed result.

## Implemented rule and unchanged boundaries

From the first ballistic conflict's body-expanded region, a forward scan tracks
the highest terrain point until a drop of one conservative vehicle diameter
resolves the crest. Smaller dips merge into that feature; a substantial drop
separates the next ridge. Equal-height plateaus use their last crest point.
The waypoint centre is one diameter beyond that crest. Its existing half-
diameter H window and conservative body half-diameter can therefore remain
beyond the crest even at an early H. If no crest resolves before the destination
boundary, the old local progress goal remains labelled `local_climb_stage`.

This uses one body-derived scale, not a fitted 807 threshold, a global highest-
peak search, line-of-sight test or whole-route landing solver. Proposed arcs are
still checked against the complete vehicle footprint. V4's full incoming/short-
continuation height repair still has four total proposals and the same numeric
height allowance. The destination constructor remains terrain-blind. Actual-
state powered correction, command protection, source support, H continuation
admission, terminal controller, plant, fuel, clocks and deadline are unchanged.

`ridge_selection` records the conflict, scan range, crest, separating drop and
body scale, retaining the same feature during a height repair. The common rich
report keeps all its plots and adds a distinct **blocking ridge crest** marker.
It is terrain, not a waypoint, proposed collision or executed handoff. The marker
and summary persist through later refreshes of the same goal revision. Stage
goals are explicitly not described as clearing a whole ridge.

## Fixed-panel outcomes

| Case | V4 | V5 | Interpretation |
| --- | --- | --- | --- |
| 807 | Continuation stop; two H before x=180 crest | Target landing; one H | Feature-based placement solves this selected mission |
| 928 | Command-recovery stop before second H | Two H, then `no_ballistic_aim` | Both selected crests passed; later destination construction remains unresolved |
| 484 | Target landing | Target landing | Preserved outcome, changed flight |
| 006 | Target landing | `terrain_recovery_no_safe_command`, zero H | Lost landing during waypoint correction |
| 142 | Target landing | Two H, then `no_ballistic_aim` | Lost landing at later destination construction |
| 349 | Target landing | `waypoint_aim_construction_miss`, zero H | Lost landing during waypoint correction |

Other preserved random landings are 001, 084, 114, 139, 268 and 308. Remaining
misses are 034, 055, 715 and 000. Full counts: eight target landings, three
no-command stops, two no-aim stops, two waypoint-construction stops and one
continuation stop. Earlier successful flights are not claimed to remain exact:
only random 001/114 and the three direct controls retain complete ordinary-flight
parity among successful cases. Random 000's old finite flight also remains exact.

807's first selected goal is `(192.806248, 539.482267)` m for terrain crest
`(180, 508.459143)` m. Powered-phase rechecks raise/reacquire that same feature;
14 accepted goal revisions/replacements are **not fourteen executed waypoints**.
Its sole actual H is at 17.983333 s, `(186.470936, 535.443976)` m with velocity
`(10.369233, 10.989249)` m/s. Its conservative trailing body edge has passed the
crest. All admitted ridge H in the panel satisfy that same spatial relationship.

928 selects crests at x=188 and x=1060 m. H1 occurs at x=194.491256 m, H2 at
x=1066.471313 m. It stops immediately after H2 at 30.783333 s with velocity
`(86.487637, -23.685512)` m/s and only 133.528687 m left to the destination.
Passing a terrain crest is not a certificate of a recoverable destination arc.

## Loss review: placement exposes an acquisition mismatch

The three losses do not establish that their peaks were incorrectly detected.
006 moves the goal from x=145.499460 to x=156.806248 m at the same height;
349 moves x=672.572619 to x=676.806248 m, also at the same height. Small changes
are enough to expose timing/control sensitivity. No threshold changes or extra
flights were used to recover them.

Code inspection finds that `construct` seeds **both destination and waypoint**
times with `natural_arrival_steps`, whose contract is a *descending target-height
crossing*. Waypoint coast admission, by contrast, accepts rising/higher motion.
Read-only reconstruction from recorded real states gives:

| Recorded state | Time to waypoint x at current vx | Constructor base horizon | Current vx | Ideal base vx |
| --- | --- | --- | --- | --- |
| 006 at last recovery start | 1.610315 s | 3.216667 s | 17.991014 m/s | 9.006589 m/s |
| 349 at its construction miss | 1.813997 s | 16.683333 s | 62.896317 m/s | 6.838785 m/s |

006's retained correction asks for near-horizontal braking beside the rising
terrain; the unchanged short-command guard/recovery subsequently cannot admit
a command. 349's fresh bounded constructor returns no correction after trying
to fit a much slower arc. These numbers expose a real constructor/admission
inconsistency, not proof that a shorter replacement arc or safe powered command
exists. The table is an analytical reconstruction, not another flight or arbitrary
airborne restart. 142 and 928 instead fail destination construction after H;
their exact rejection predicates still need a focused explanation.

Recommended next pass: keep ridge identification and test whether pass-through
waypoint construction can use a current-state forward crossing horizon instead
of inheriting the descending-height floor. Start with pure timing/clearance
diagnostics for 006/349 and keep actual correction/turn guards; do not add more
peak thresholds, random candidate grids or relax safety. Treat 142/928's later
destination admission as a separate diagnostic. A paired 1k campaign is premature
until the lost selected landings are understood and preservation is restored.

## Validation and retained provenance

- 29 focused native feedback tests pass, including six new ridge geometry tests
  for shallow/deep valleys, plateaus, monotonic slopes, errors and plant immutability.
  These synthetic geometry states are not executable airborne restarts.
- All eleven maintained developer checks pass: workspace/CLI boundaries, Rust
  formatting, strict Clippy, docs links and 62 Node tests. The two common batch
  example tests and all 137 terrain-study tests pass.
- The separately identified October-7 ordinary capture has **44/44 exact complete
  non-timing parity**, with only the three declared wall timings excluded.
  An initial comparison to the published October-5 capture reproduces the already
  documented 8.88e-16 m geometry-scalar difference; it is not relabelled as a pass
  or a new V5 defect. See the [earlier explanation](ballistic_feedback_results.md).
- The 21 native records pass source/input/binary binding, complete command replay,
  internal decision reproduction and two exact full-feedback external repeats.
  The two development records also match the panel feedback bytes exactly.
- Saved verification authenticates the V4 comparison, direct-control bytes,
  copied source/binaries and complete create-only inventory. Accepted selectors,
  published reports, retained captures and the ordinary release executable stay
  unchanged. No captured flight was retried or reconstructed from an injected state.
- Static report checks cover 21 rich pages, 2,284 cycles, 1,657 crest-bearing cycles,
  all local batch links and 16 actual H across primary/repeat records. Node tests
  check crest-marker updates and preserved original traces; HTTP response is 200.
  This is not browser-pixel acceptance or human approval of the presentation.

The complete allowance is used: two development flights followed by 19 primary
missions and two repeats, **23 native invocations**. No native source/threshold
change occurred between the development pair and panel. All captures are separate
and create-only; the panel freezes its source, binaries, plan and V4 comparison.

Capture: `outputs/eval/planner_v2_random_terrain/capture-ridge-waypoint-panel-20261008-v1`.
Probe: `outputs/eval/planner_v2_random_terrain/dev-ridge-waypoint-20261008-v1`.

| Identity | SHA-256 |
| --- | --- |
| Native Rust source tree | `e5fdb0c6125441ae94a9ce5b67cf67557a324eafc86d0ff6400f90df7b5978a4` |
| Candidate executable | `d85a2d26ff01fc477598cda8a9763e61d793bdfbc16199f80f369bdd95b2206e` |
| Panel receipt | `6f37366b7b3ae78583ca06b94aa11763eda93651a37c16ad64b5f65d32ceb9e1` |
| Panel results | `3489e99d6361f8cd9642146b63721cfb6b1a9bab459376bde66c01005270c0e7` |

Saved verification (read-only):

```sh
rtk proxy python3 -B studies/terrain_profiles/ridge_waypoint_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-ridge-waypoint-panel-20261008-v1
```

The common batch is reachable on the existing server at
`/eval/planner_v2_random_terrain/capture-ridge-waypoint-panel-20261008-v1/`.
Review 807 for the new clearing/landing, 006 and 349 for lost acquisition, and
142/928 for later destination stops. No navigation publication, server operation,
commit, push or accepted-planner change occurred in this pass.
