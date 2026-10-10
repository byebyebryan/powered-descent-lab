# Body-aware terminal centering: results and scope correction

[Documentation home](README.md) · [Closed plan](ballistic_terminal_centering_plan.md)

Follow-up: the separately authorized [frozen-candidate full 1k](ballistic_terminal_centering_sweep_results.md)
now records 592/1000. This page retains the earlier 25-record checkpoint and its
failed exact-control admission; the later run uses a separate outcome-based
contract, not a retroactive change to that verdict.

Verdict: **positive mechanical result, not broad replacement acceptance**. The
body-aware rescue lands 142 while retaining the braking fix's 974 landing.
Preservation improves from **8/16 to 10/16**, gaining 142 and 715 with no lost
previous-candidate landings. All three direct controls still land. However,
their terminal commands change, violating the plan's exact-flight requirement.
The conditional 1k did not run; at that checkpoint the last complete ballistic
rate remained 562/1000.

## The revision and its demonstrated effect

`landing-body-centering` / `ballistic_feedback_v12_landing_body_centering` is an
explicit experiment combining the earlier braking guard with one centering fix.
Rescue's pad test now includes the horizontal extent of the current rotated hull
and feet. If the body is outside that corridor, existing outside-pad lateral
correction continues even when sideways speed is already low. It reuses existing
rescue heights, target velocities, engine mapping and vertical-authority tilt cap.
It adds no solver, terrain-specific tuning, new waypoint rule or relaxed reserve.

Policy 3, ordinary controller constructors, accepted reports and standalone
coast-terminal previews/execution remain unchanged. The opt-in ballistic
adapter's countdown/fallback terminal controller has the new flags enabled;
this path also handles the three direct controls. “Direct” does not bypass
terminal descent or imply numerically unchanged descent commands.

| Case or population | Previous coast-terminal candidate | New candidate |
| --- | --- | --- |
| Focus 142 | Flying reserve/contact stop at 30.767 s | Verified target landing at 35.900 s |
| Focus 974 | Flying unsafe-contact stop at 49.000 s | Verified target landing at 68.925 s |
| Preservation random worlds | 8/16 landings | 10/16; gains 142 and 715, no losses |
| Preservation direct controls | 3/3 landings | 3/3 landings; changed descent commands |
| 084 | One-H landing at 61.808 s | Same complete ordinary flight and one-H landing |
| 715 | Flying reserve stop at 33.667 s | One-H target landing at 66.592 s |

142 differs from the braking-only candidate first at 32.050 s: its center is
13.523 m right of the target, with 24.619 m target clearance. Rotation puts the
body outside the safe pad interval. The old rescue commands upright braking;
the new rescue retains inward correction at -0.42 rad with the corresponding
vertical thrust compensation. It lands 9.281 m right of center, with incoming
velocity -1.295 m/s sideways and -0.959 m/s vertically. 974's complete ordinary
flight is exactly unchanged from the braking-only experiment.

Every primary case retains its previous complete pre-terminal action sequence
and terminal-entry clock. These gains therefore demonstrate terminal execution
changes, not different launch arcs, waypoint placement or handoff timing. The
selected older policy-3 comparison is **11/16**, distinct from the immediate
8/16 baseline; this result still loses its 034/000/139 landings. Do not treat
the selected panel's rate as a population estimate or replacement acceptance.

## Why the 1k admission failed

The preserved control requirement demanded complete ordinary-flight equality.
All three controls instead first differ when the opt-in early braking guard
activates, with the whole body already inside the pad interval. Saved-command
controller diagnostics reproduce every command and final state exactly and
identify this trigger at 38.083 s (flat), 37.133 s (uphill), and 36.600 s (downhill).
This is actual flight change, not a harmless metadata mismatch or corrupt proof.

The flat/uphill/downhill flights remain zero-H landings. They take respectively
0.950/0.967/0.950 s longer and use about 23/24/23 kg more fuel. This does not
establish unchanged behavior, but it also is not a lost landing or a physical
failure. The exact-flight criterion was too restrictive for evaluating a
terminal-braking change. Restricting the fix to these observed failures just to
keep control numerics identical would be the wrong generalization target.

The declared criterion was not weakened after measurement. The normal panel
verifier raises `ridge baseline or direct-control flight changed`; this legacy
message denotes the exact-flight acceptance miss here. No full-sweep contract
or capture was created. Separate read-only authentication checked every record,
receipt, frozen input, previous comparison, source/binary and external repeat;
it does not override that failed admission. The outcome-only helper's success
is not permission to ignore the stronger captured contract.

## Retained evidence and validation

Focus: `outputs/eval/planner_v2_random_terrain/capture-terminal-centering-focus-20261009-v1`.
Preservation: `outputs/eval/planner_v2_random_terrain/capture-terminal-centering-preservation-20261009-v1`.

All **25 native records** pass physical/mission projection, integrity,
original-source command replay and decision reproduction. Four external repeats
(142/974 focus and 084/715 preservation) match complete feedback exactly. All
261 focus and 482 preservation receipt-listed files authenticate. Collection
source and protected accepted-site/old-evidence seals remain unchanged. There
are no executed crashes or off-target terminal outcomes; six preservation
random cases remain verified airborne finite stops.

The maintained 11-step development gate and 168 Python collector tests pass,
including four new geometry/centering controller tests and four new collector
contract tests. No retained 44-case numerical parity campaign ran. Eight
additional query-only saved-command diagnostics reproduce existing records;
they are not newly chosen flights or additional measured missions.

Both captures retain frozen inputs, prior receipts/results/complete feedback,
candidate source/binaries, command proofs, rich common details and the shared
batch tree. HTTP checks of the preservation batch and 974 detail return 200 on
the existing LAN server. All 25 batch-to-detail links resolve; the inline planning
data matches saved cycle JSON, all 1814 refresh origins/decisions bind to raw
feedback, and all 22 plotted H snapshots match actual handoffs exactly.
No browser visual acceptance, navigation publication,
server operation, commit, push or default promotion is claimed.

Native SHA: `eeb07070b7378186c50f7deba34a0afab96cbd3b1d5b44a32a0311b056418366`.
Rust tree: `696ae4de5a80abb742a347e610143e9166d9039dc566eebab2cffbdf85cb3dab`.
Renderer: `1f52999f81e4827614346c445af7b7b3b2a331407c62c59230f9c0c92115a591`.
Focus receipt: `0182fc9cb329bfac3ccfc9cdedcc298689f249546a53ae0c33a8f7332c6977a3`.
Preservation receipt: `e66d9a78e7eb1c80fe6d843e4755f53d67369126d1267782bf208c69b3f7783a`.

Read-only verification (run from repository root):

```sh
rtk proxy python3 -B studies/terrain_profiles/terminal_centering_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-terminal-centering-focus-20261009-v1
rtk proxy python3 -B studies/terrain_profiles/terminal_centering_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-terminal-centering-preservation-20261009-v1 --panel
```

The second command is expected to fail the unchanged exact-control criterion.
Keep that capture and contract intact; do not relabel it as accepted preservation.

## Practical next decision

Authorize a separate full-1k contract for this **same frozen native candidate**,
without another fix, rebuild or tuning pass. Require all three control landings,
their complete proofs and exact pre-terminal prefixes, rather than identical
terminal commands. Keep candidate-to-candidate external repeats exact and report
all paired gains/losses against 562 and 817 separately. That is the useful broad
test; these selected gains cannot estimate the effect on the original 102
terminal short-command stops.

The six remaining panel stops are 807 (terminal body reserve), 928 (no ballistic
aim), 034/139 (terrain recovery), and 000/055 (waypoint construction). Five retain
their complete previous ordinary flights. 807 still fails, now earlier: it
enters terminal at 107.024 m/s sideways and later crosses the target with
79.712 m/s sideways, meeting terrain outside the pad under the unchanged guard.
Its entry fit is not proof of a recoverable whole descent. This is a distinct
lateral/terrain ownership question, not justification for removing reserve or
another case-specific fix before the broad test. 900's later coast terrain
ownership and the original 131 missing-aim stops remain separate questions.
