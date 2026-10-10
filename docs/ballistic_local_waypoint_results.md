# Local waypoint height and early destination reacquisition results

[Documentation home](README.md) · [Pinned plan](ballistic_local_waypoint_plan.md) · [V6 comparison](ballistic_waypoint_entry_results.md)

## Verdict

**Early destination reacquisition alone lands original 715.** It corrects toward
the destination before reaching the late second waypoint, while lateral braking
room remains. The waypoint is superseded, not falsely recorded as H2.

Lower local waypoint height removes 715's unrelated incoming-shoulder height
from W2, but does not produce a landing. Combining the changes also fails later.
The combined preservation panel falls from **8/16 to 6/16** random landings,
losing 084/142 with no gains; the three direct controls remain exact. Do not
adopt the combination or infer general coverage from the early-only success.
Early-only has seven focused random comparisons, not a full 16-world panel.

All **65 planned native records** authenticate inputs, source/binaries, command
replay, deterministic decisions and repeats. No flight-driven tuning, retries,
1k sweep, default promotion, accepted-site publication, server operations,
commit or push occurred. Every nonlanding remains `flying/in_progress`, not a
physical crash or a proof of impossibility.

## What changed and what did not

- `local-height` retains the existing first-blocking-ridge x selection, but seeds
  height from the resolved local crest/goal terrain rather than the entire
  incoming corridor. An unresolved climb uses the scanned local interval. The
  body radius, 5 m reserve and two-second gravity-drop allowance stay unchanged.
  Full incoming trajectory audits, height repair and command/body/H guards still
  check all terrain; earlier peaks are not exempted from clearance checks.
- `early-target` previews the destination at the existing 24-tick cadence during
  actual engine-off waypoint coast, only when projected waypoint-entry lateral
  braking room is negative. It uses the unchanged destination constructor and
  switches only after ideal-arc, estimated cutoff-coast and immediate-command
  audits pass. Failed previews keep the waypoint. This is a goal revision, not H.
- `local-height-early-target` enables both. All three inherit V6 combined
  effort/recovery. `combined` is the same-source V6 control. Maintained policy 3
  and the experimental CLI's `ridge` default are unchanged.

There is no waypoint-apex constraint, terrain-selected higher destination arc,
complete landing suffix, new overshoot-and-return maneuver or larger search
budget. Powered correction remains guarded from actual state.

## Four frozen focused comparisons

Each mode runs original 715/349/006/807/928/139/268, three direct controls and a
complete 715 repeat: eleven records per mode, 44 total.

| Mode | 715 | 349 / 006 | Other four random subjects |
| --- | --- | --- | --- |
| V6 combined control | No next aim after 2 H, 29.72 s | Both land | All retain finite stops |
| Local height only | No next aim after 2 H, 29.70 s | Both land | All retain finite stops |
| Early target only | **Land, 1 H, 37.27 s** | Both land | All retain exact V6 ordinary flights |
| Both changes | No next aim after 1 H, 30.10 s | Both land | All retain finite stops |

All eleven same-source control records reproduce complete V6 feedback exactly.
All direct controls preserve complete ordinary flights in every mode; every
external repeat reproduces complete feedback. 349/006 also preserve their exact
V6 ordinary flights in all new modes.

### 715: lower geometry is not automatically an easier state

H1 is unchanged at 16.38 s, `(334.70, 209.23)` m with velocity
`(25.01, +1.42)` m/s. W2 targets the crest at `(1158, 66.84)` m. Local height
reduces W2 from **161.05 to 97.87 m**, retaining its x at 1170.81 m. The former
height basis was a 130.03 m incoming shoulder; the local basis is 66.84 m plus
the unchanged 31.02 m allowance. No height repair is needed in this case.

But local-only still reaches late H2 at `(1164.70, 99.88)` m with velocity
`(72.81, -57.83)` m/s. Lowering W2 does not remove lateral energy or create
braking room, and leaves less vertical recovery space than the original H2.

Early-target alone keeps the old W2 geometry. It reacquires the destination at
**25.80 s**, `(876.90, 283.89)` m with `(73.53, -12.39)` m/s. A finite lateral
braking burn establishes a destination coast at 28.37 s; maintained terminal
guidance takes ownership at 28.38 s and lands at 37.27 s. There are two waypoint
selections, but only one actual H: W2 is canceled before its crossing.

With both changes, the first admitted destination preview occurs later, at
26.63 s and `(941.40, 231.23)` m with `(72.81, -27.75)` m/s. Its correction
establishes a destination coast at 30.02 s, but terminal ownership does not open
before that coast loses approach admission at 30.10 s. The finite constructor
then finds no replacement. No additional terrain obstruction is recorded here.

### A common coast-to-terminal ownership gap

Read-only recomputation of the unchanged `approach_ok` estimate on saved states
identifies the expired guard. These are conservative admission heuristics using
the original full-fuel mass, not exact terminal-feasibility certificates.

| Combined case | Destination coast first accepted | Later finite stop | Expired estimate at stop |
| --- | --- | --- | --- |
| 715 | 30.02 s | 30.10 s | Lateral room: -0.52 m (was +2.31 m at acceptance) |
| 084 | 26.87 s | 26.97 s | Vertical stopping-height room: -1.47 m |
| 142 | 23.68 s | 24.20 s | Vertical stopping-height room: -0.059 m |

All three accept a near-target ballistic coast, cut thrust, then lose approach
admission before maintained terminal guidance owns the flight. Their ballistic
lateral misses remain inside the same 7 m tolerance. This establishes a shared
boundary to diagnose; it does not establish that keeping coasting or bypassing
terminal admission would be safe. The successful early-only 715 has roughly
46 m lateral and 85 m vertical estimated room at destination-coast acceptance.

## Combined preservation panel

The same frozen source runs the previous 16 random worlds, three direct controls
and complete 715/349 repeats: 21 records, 19 primary missions. Total allowance
across all five captures is 65; repeats are not additional coverage.

- Six random landings remain: 001, 006, 114, 308, 349, 484. Their complete ordinary
  flights are unchanged from V6.
- 084/142 lose their V6 landings. Neither performs early destination reacquisition:
  the changed local-height path is sufficient to expose the later ownership gap.
- 715/268 change flight but still stop; the other six nonlandings keep exact
  ordinary-flight parity. Thus 12/16 random ordinary flights remain exact.
- Three direct controls land with exact ordinary-flight parity, giving **9/19**
  primary landings versus V6's 11/19. Ten random stops are five `no_ballistic_aim`,
  two `waypoint_aim_construction_miss`, two `terrain_recovery_no_safe_command`
  and one terminal `short_command_rejected`.

084/268 also demonstrate that local height does not ignore upstream terrain:
full trajectory rechecks replace the goal with an earlier, taller blocking
crest before returning to the later crest. This produces two actual H rather
than one; the incoming guard remains active.

## Validation and retained evidence

- Focused Rust tests: 37/37; terrain-study tests: 146/146; planning-cycle
  JavaScript tests: 9/9. All eleven maintained development checks pass,
  including formatting, strict Clippy, workspace/CLI boundaries and 64 Node tests.
- Explicit retained October-7 normal-policy parity: **44/44 exact**. This is
  maintained-behavior preservation, not acceptance of intentional candidate changes.
- All 65 saved records verify again after collection. Current source/protected
  seals still match every capture; old captures, accepted selectors/navigation
  and the ordinary release executable are unchanged.
- Static common-template checks cover 65 rich pages, 4,953 planning cycles,
  4,780 cutoff-coast overlays, 55 actual H, 76 local-height records, six early
  reacquisition records and 150 local batch links. The six goal-change records
  are explicitly not H. Spatial/metrics/event/inspection views remain available.
  Existing LAN detail/batch URLs return HTTP 200. Browser-pixel and human visual
  acceptance are not claimed.

All captures are under `outputs/eval/planner_v2_random_terrain/`:

- `capture-waypoint-local-control-20261008-v1`
- `capture-waypoint-local-height-20261008-v1`
- `capture-waypoint-local-early-target-20261008-v1`
- `capture-waypoint-local-combined-20261008-v1`
- `capture-waypoint-local-preservation-20261008-v1`

| Identity | SHA-256 |
| --- | --- |
| Native Rust source tree | `2785e93f487a5d98d40cd69e8d73a82a20212fa01831117b81a5fa96ba846384` |
| Candidate executable | `eddb5cedda7ca83898bd9157855ff1236ea6ccab07eee7637dfe52cb05262a0a` |
| Early-only receipt | `69626281da84612634e3238f37b61ed130f85db6d13badcbf0befcee751dd15c` |
| Preservation receipt | `9392ce7ce82864e0e690898b7eb1503f8b28e46d73966a9676a1fe94674497b8` |

Read-only verification needs neither current executable nor the older capture:

```sh
rtk proxy python3 -B studies/terrain_profiles/waypoint_local_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-waypoint-local-preservation-20261008-v1
```

## Review and next decision

On the existing server start with
`/eval/planner_v2_random_terrain/capture-waypoint-local-early-target-20261008-v1/runs/random-715/report.html`.
Use **Jump to decision → destination_reacquired_before_waypoint** to compare the
new destination arc, predicted cutoff coast, canceled W2 and actual braking.
Compare the same mission under `capture-waypoint-local-combined-20261008-v1`,
then preservation-panel 084/142. These are local common-template captures, not
new accepted-site navigation.

The next small diagnostic should align destination-coast admission, its retention
and terminal ownership on saved 715/084/142 states, with successful early-only
715 and unchanged 349 as comparisons. Do not tune crest thresholds or relax
physical guards to recover the panel. Early-target alone merits separate full
preservation validation before any wider campaign. This pass does not authorize
either that panel or a new recovery family.
