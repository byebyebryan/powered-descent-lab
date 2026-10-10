# Ballistic terminal braking: focused result

[Documentation home](README.md) · [Closed plan](ballistic_terminal_braking_plan.md)

Verdict: **partial mechanism success, not an accepted fix**. Earlier braking
lands 974, but 142 still stops under body reserve beside the pad. Both external
repeats match complete feedback exactly. Four records were measured and verified;
the conditional preservation panel and a new 1k sweep did not run. Policy 3,
controller defaults, standalone coast-terminal handling and physical guards
remain unchanged.

## What changed and what the evidence says

`landing-braking-guard` is an explicit experimental candidate. Above the existing
final rescue height, it reuses touchdown rescue only when descent exceeds the
existing braking envelope and the nominal command's applied vertical acceleration
is insufficient for the existing distance-based braking calculation. It adds no
search, height tuning or relaxed reserve. Defaults keep the new flag disabled.

The previous 1k had 102 short-command stops after terminal takeover: 85 predicted
unsafe contacts and 17 body-reserve violations. These were not actual flown
crashes. Saved-command diagnostic replays of 142, 974, 084, 127 and 900 reproduce
every terminal command and the final state. In 974, a retained fit remains active
while the remaining ideal profile demands excessive final thrust; in 142, the
countdown releases at 23.8 seconds and receding guidance also brakes too weakly.
An initially feasible acceleration does not establish an executable full descent.

The fix uses existing braking physics rather than extending a preview or relaxing
the final guard. Fresh 142/974 runs retain **exactly the same pre-terminal action
sequences** as the previous capture, so the focused change is terminal execution,
not different waypoint placement or entry timing. New saved-command diagnostic
replays also reproduce all terminal commands and final states exactly.

| Case | Previous candidate | Braking guard | Verdict |
| --- | --- | --- | --- |
| 974 | Flying stop at 49.000 s; predicted contact at 15.27 m/s downward | Target landing at 68.925 s; mission success, integrity, source replay and decision repeat pass | Vertical braking fix works here |
| 142 | Flying stop at 30.767 s; 18.54 m/s downward, 13.44 m/s sideways | Flying stop at 33.833 s; 4.58 m/s downward, 0.81 m/s sideways; reserve violation | Contact risk reduced, landing not achieved |

Primary result: **1/2 versus 0/2** in the immediately previous ballistic candidate.
Both repeats are outside that denominator. The older policy-3 comparison is a
different baseline (1/2); 142 remains its loss. This selected two-world result is
not a population rate or a new estimate for the 102-stop cohort.

## Why 142 still stops

At the new stop, the center is 15.100 m right of the target. The pad half-width is
18 m, but each upright foot is 4 m from the center: the body-safe center interval
is only +/-14 m. The right foot is at x=1219.100, beyond the pad's right edge
x=1218, where terrain rises steeply. Its current clearance is 6.051 m.

The rejected command's saved 23-tick prediction moves the right foot to
x=1219.255 and reduces its clearance to **4.995 m**, below the required 5 m body
reserve. The prediction is still flying, not a crash or a safe target touchdown.
The special descending-pad reserve applies only with the whole body inside the
pad, so the rejection is correct.

The existing rescue's `inside_pad` test considers the center, not the full body.
Its inside-pad lateral target gives zero correction weight below the safe-speed
threshold. At 0.81 m/s it requests upright braking even though the body is outside
the flat landing corridor; nominal guidance still requests lateral correction.
Early reuse of this logic therefore exposes a separate centering defect. This is
not evidence that the terrain reserve should be removed or that 142 would land
if the stop were ignored.

## Validation and retained artifacts

Capture: `outputs/eval/planner_v2_random_terrain/capture-terminal-braking-focus-20261009-v1`.
All 259 receipt-listed files verify. Four native records each pass integrity,
original-source replay and decision reproduction; both external repeats are
exact. The maintained 11-step development gate and 164 Python collector tests
pass, including four new controller tests and three new collector contract tests.
Old protected evidence/site seals are unchanged. No retained 44-case numerical
parity campaign was run; default preservation is supported by the unchanged
opt-in boundary and ordinary tests, not a newly measured 44-case result.

Identities:

- Native: `e95c81a067dc7c3d115b365dbf6dbbfd7099a4124cafab23538101cb62211397`.
- Rust tree: `4bcc71248d815f73eaf9c99db8f19e5a23ebf7fc9412c3e6ce1b03850c0bc87b`.
- Renderer: `6acbfe51c146929c0443e367bd8bec8d9b64f874802d93d6da74fe6e1009db8b`.
- Receipt: `a87844e8d7fe8307a825564c29be0500b36f706c85bdbade8985553fc5476c39`.
- Results: `1de36b4f0b55cd234e1f044accdda58ed2a305cd810e2d1d15c72b74499f6b3a`.

The capture retains frozen inputs, previous receipt/results/complete feedback,
candidate source/binaries, command and decision proofs, rich common detail pages
and the common batch tree. No accepted-site navigation/selector publication,
server restart, commit or push occurred. Browser visual acceptance is not claimed.

Read-only verification:

```sh
rtk proxy python3 -B studies/terrain_profiles/terminal_braking_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-terminal-braking-focus-20261009-v1
```

`passed: false` is the failed focus acceptance gate, not corrupt evidence. The
conditional preservation collector refuses to run from this focus result.

## Next bounded change

Retain the opt-in braking mechanism as a promising experiment, not a default.
Make rescue centering use a body-safe pad interval and keep correcting position
even when sideways speed is already safe. Establish that this correction preserves
vertical braking authority, then repeat the two-world focus and unchanged
preservation panel under a separate plan. Do not tune against 142's terrain shape.
The 17 original reserve stops and 900's later terrain ownership remain distinct
questions; there is no claim that this one vertical fix solves them all.
