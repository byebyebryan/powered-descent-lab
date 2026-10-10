# Waypoint entry effort and recovery results

[Documentation home](README.md) · [Pinned plan](ballistic_waypoint_entry_plan.md) · [V5 comparison](ballistic_ridge_waypoint_results.md)

## Verdict

The two proposed mechanisms work on original 349: preferring lower added thrust
effort produces a gentler waypoint entry, while retaining a safe ongoing
correction also recovers the original higher-energy entry. Both modes land,
without an apex-at-waypoint constraint or relaxed safety guards.

The combined preservation panel is **mixed, not an adopted replacement**:
8/16 selected random worlds land, the same total as V5, with three gains and
three losses. All three direct controls preserve their complete ordinary flights.
This is a closed 53-invocation experiment, not a new 1k coverage estimate.
No tuning, retries, default promotion, accepted-site publication, server operation,
commit or push occurred in this pass.

## What changed

- `effort` compares the same bounded waypoint profile family by estimated added
  thrust impulse, then predicted entry speed and stable arrival time. The speed
  is a tie-break, not a general low-energy guarantee. Ranking is terrain-blind.
- `recovery` adds an upcoming forward x-crossing seed within the existing
  32-profile budget and removes the descending-height timing floor for waypoints.
  When coast-now is blocked but the refreshed correction's predicted cutoff
  coast and two-second continuation are clear, acquisition can continue rather
  than prematurely replacing the goal.
- `combined` enables both. `ridge` is the same-source V5 control and remains
  the experimental command's default. Maintained policy 3 is unchanged.

Destination construction, approach/braking safeguards, four local proposals,
ordinary short-command/body/contact checks, terrain protection and actual
engine-off handoff guards remain unchanged. The pending coast estimate is a
geometric query, not proof that the powered acquisition will be safe.

The common rich report gains a teal **Predicted cutoff coast · not flown** trace
and predicted goal-time velocity, speed and remaining impulse. All existing
plots, current-state ballistic projections, illustrative arcs, crest/goal markers
and actual flight/handoff data remain. Predictions are not actual arrivals.

## Four focused ablations

Each mode ran the same four original random worlds, three direct controls and
one full-feedback 349 repeat: eight records per mode, 32 records total.

| Mode | 349 | 006 | 807 | 928 |
| --- | --- | --- | --- | --- |
| Ridge control | Aim miss, 0 H | No safe command, 0 H | Land, 1 H | No next aim, 2 H |
| Effort | Land, 1 H | Land, 1 H | Terminal command stop, 1 H | No next aim, 2 H |
| Recovery | Land, 1 H | Land, 1 H | Continuation stop, 0 H | No next aim, 2 H |
| Combined | Land, 1 H | Land, 1 H | Terminal command stop, 1 H | No next aim, 2 H |

All direct controls land with exact V5 ordinary-flight parity in every mode.
The same-source ridge mode reproduces all eight complete saved V5 feedback
records exactly. Every external repeat reproduces its mode's complete feedback.
Every nonlanding in this experiment is a finite `flying/in_progress` stop, not
a physical crash or proof that the mission is impossible.

### 349: the requested behavior is demonstrated

The selected waypoint is unchanged at `(676.81, 871.02)` m, beyond the crest at
`(664.00, 839.99)` m. Its first fit changes as follows:

| Predicted finite-burn realization | Ridge / recovery | Effort / combined |
| --- | --- | --- |
| Arrival horizon | 18.25 s | 23.75 s |
| Powered burn | 15.52 s | 13.35 s |
| Coast after cutoff | 2.68 s | 10.30 s |
| Added thrust-impulse estimate | 256.31 m/s | 220.41 m/s |
| Goal-time velocity `(vx, vy)` | `(64.07, +77.27)` m/s | `(39.47, -8.08)` m/s |

Actual combined flight cuts thrust at 14.93 s and stays engine-off through its
25.13 s handoff, entering at `(39.38, -7.32)` m/s, speed **40.05 m/s**. It then
corrects toward the destination and lands at 55.20 s. This is a ballistic
waypoint transfer, not continuous powered ascent to the waypoint.

Recovery-only retains the original higher-energy realization. At 16.92 s its
`waypoint_acquisition_continues` decision distinguishes an unsafe immediate
coast from the safe coast being established. Thrust cuts at 17.07 s; actual H
occurs at 19.70 s with `(64.00, +77.66)` m/s, speed **100.63 m/s**. Actual-state
correction afterward succeeds and the mission lands at 63.58 s. Safe rising
entry was retained rather than forced toward a waypoint apex.

## Combined preservation panel

The unchanged source then ran the prior 16 random worlds, three direct controls
and full-feedback repeats of 349/807: 21 records, 19 primary missions.

| Compared with V5 | Random worlds |
| --- | --- |
| Newly landed | 006, 142, 349 |
| Lost landings | 807, 139, 268 |
| Still landed | 001, 084, 114, 308, 484 |
| Still finite stops | 000, 034, 055, 715, 928 |

The random result is 8/16 versus 8/16; adding the separate three direct controls
gives 11/19 versus 11/19 primary missions. Six random landings have actual H;
two have zero H. Eight random stops split into three `no_ballistic_aim`, two
`waypoint_aim_construction_miss`, two `terrain_recovery_no_safe_command` and
one terminal `short_command_rejected`. Repeats are not additional coverage.

The losses do not have a single demonstrated cause:

- **807:** W1 clears, with actual H at 18.67 s. Terminal guidance starts farther
  along and lower: `(534.76, 341.40)` m at `(107.02, -61.30)` m/s, versus V5's
  `(433.98, 453.46)` m at `(90.26, -38.89)` m/s. At 33.60 s the vehicle is beyond
  target x, still moving forward. Its proposed terminal command violates body
  reserve about 0.19 s ahead. This is a later approach failure, not a failed
  first waypoint or an executed collision.
- **139:** the selected waypoint correction brakes lateral velocity. During
  acquisition the ideal coast requires height repairs/replacements; the flight
  stops at 10.75 s when no bounded safe command is found. There is no actual H.
- **268:** actual H is later and faster: 20.12 s at `(69.00, -24.96)` m/s versus
  V5's first H at 18.28 s at `(48.85, -4.97)` m/s. The next destination aim cannot
  be constructed; V5 had completed two H and landed.
- **928:** two actual H occur, followed by a destination construction miss at
  34.97 s. The unchanged destination constructor remains a separate limitation.

Therefore least impulse is useful but is **not synonymous with easiest
downstream state**. Do not force a zero-speed/apex handoff, weaken guards, or add
case-ID rules. Before a wider campaign, use these saved states to distinguish
selection tradeoffs, powered acquisition clearance and destination/terminal
recovery. A small read-only comparison of their remaining correction effort
and room is the next decision point; another flight-driven tuning loop is not
authorized by this completed protocol.

## Validation and retained evidence

- Focused Rust tests: 33/33; experimental CLI tests: 18/18; terrain-study tests:
  141/141; planning-cycle JavaScript tests: 8/8.
- All eleven maintained development checks pass, including strict Clippy,
  formatting, workspace/CLI boundaries, docs links and 63 Node tests.
- Explicit retained October-7 normal-policy numerical parity: **44/44 exact**.
  This validates unchanged maintained behavior, not acceptance of the new modes.
- All 53 records authenticate their original inputs, source/binaries, command
  replay, deterministic decisions, complete repeats and create-only inventory.
  The source is identical across all five captures. Accepted selectors/site,
  old captures and the ordinary release executable retain their protected hashes.
- Static report checks cover 53 rich pages, 4,481 cycles, 4,104 cutoff-coast
  overlays, 36 actual H across all records and all local batch links. The 349
  rich page returns HTTP 200 on the existing LAN server. Browser-pixel acceptance
  and human visual approval are not claimed.

All captures live under `outputs/eval/planner_v2_random_terrain/`:

- `capture-waypoint-entry-ridge-20261008-v1`
- `capture-waypoint-entry-effort-20261008-v1`
- `capture-waypoint-entry-recovery-20261008-v1`
- `capture-waypoint-entry-combined-20261008-v1`
- `capture-waypoint-entry-preservation-20261008-v1`

| Identity | SHA-256 |
| --- | --- |
| Native Rust source tree | `5832508883dfb9b6e5f21bb12430b6841eeddacce8d0a49a8a9ae38f242c1dc2` |
| Candidate executable | `93b0a4e5a3ae8ac5a1cb40c447b830abbd3ce7d010b1ded4fe0c007b521ea560` |
| Preservation receipt | `43037375f5defdfa05839a849b1a0d4fe74d24326834872fb3eadfb147bfb556` |

Read-only saved verification, needing neither current binary nor old capture:

```sh
rtk proxy python3 -B studies/terrain_profiles/waypoint_entry_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-waypoint-entry-preservation-20261008-v1
```

On the existing server, open
`/eval/planner_v2_random_terrain/capture-waypoint-entry-combined-20261008-v1/runs/random-349/report.html`.
Use **Jump to decision → waypoint_selected** to compare the idealized arc,
predicted cutoff coast and actual flight. Compare recovery-only 349 at
**waypoint_acquisition_continues**, then inspect preservation-panel 807/139/268.
These are local common-template captures, not newly published accepted navigation.
