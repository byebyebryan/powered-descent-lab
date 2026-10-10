# Automatic coast-to-terminal alternative

[Documentation home](README.md) · [Frozen plan](ballistic_coast_terminal_plan.md) · [Scheduled capability witness](ballistic_terminal_coast_results.md)

Verdict: **084 now selects the coast-to-terminal continuation automatically and
lands with H1 only, without W2 or a saved takeover clock**. All 22 declared
records verify. The selected six-world comparison improves from 3/6 to 4/6
landings, with no lost landings. This is a focused mechanism result, not a new
1k pass rate or default adoption.

The subsequent [full-panel and 1k validation](ballistic_coast_terminal_validation_results.md)
now measures that wider comparison without native changes. The focused results
and allowance below retain their original scope.

## What changed

The explicit `coast-terminal` experiment inherits combined V9 behavior until a
new destination ideal arc is blocked after an actual waypoint H, with the held
command engine-off. Before adding another waypoint, it asks whether the current
motion can already clear that first blocking crest and enter ordinary terminal
control:

1. Use the existing ridge classifier to locate the blocking crest. Query actual
   engine-off physics to crest x plus half the conservative body diameter,
   bounded to 960 ticks / 8 s and the original mission deadline.
2. Require full-body/contact clearance throughout the coast, descending forward
   motion above/before the target at entry, the existing 45-degree arrival
   safeguard and dynamics-only terminal readiness.
3. Preview two seconds of the standalone terminal controller's actual feedback,
   including attitude slew and the existing 24-tick held-command guard. Keep
   configured controller terrain handling enabled.
4. If admitted, execute the coast with ordinary command guards. Recheck at the
   actual body-clear crossing, then give terminal control ownership. Do not add
   a waypoint, goal revision or H. Decline/cancellation resumes the old loop.

This branch's admission uses bounded actual feedback instead of treating the
configured constant-initial-acceleration terrain approximation as a complete
trajectory certificate. That approximation's verdict remains recorded; it is
false in 084. No controller terrain handling, body reserve, contact check or
live landing command guard is disabled. The two-second preview is not proof of
the entire landing suffix; the complete executed landing supplies that evidence
for this case.

New entry uses ordinary standalone defaults, without V9 duration/countdown or
waypoint retention. V9 acquisition/fallback behavior remains available when the
branch is not selected. Old experimental modes, the `ridge` experimental default,
maintained policy 3, frozen inputs and accepted reports are unchanged.

## Paired outcomes

Eleven same-source V9 controls and eleven V10 candidate records were fixed
before measurement: six selected random worlds, three direct controls, and two
external repeats. No tuning, retry or additional native attempt followed.

| Case | V9 control | New candidate | Actual-flight preservation |
| --- | --- | --- | --- |
| 084 | Flying; command contact prediction stop; two H | Target landed at 61.808 s; one H | Changed; exact successful capability-witness ordinary flight |
| 715 | Flying; command reserve prediction stop | Same stop at 33.667 s; one H | Exact ordinary flight |
| 142 | Flying; command contact prediction stop | Same stop at 30.767 s; two H | Exact ordinary flight |
| 268 | Target landed | Target landed at 33.825 s; two H | Exact ordinary flight; unsafe coast declined |
| 349 | Target landed | Target landed at 55.200 s; one H | Exact ordinary flight |
| 006 | Target landed | Target landed at 59.267 s; one H | Exact ordinary flight |
| Flat/uphill/downhill direct controls | All target landed, zero H | All target landed, zero H | All three exact ordinary flights |

All eleven unchanged-mode controls match their complete saved V9 feedback,
not just their outcomes. All candidate ordinary flights except 084 stay exact.
268 adds a `current_coast_blocked` rejection at 17.850 s, then follows its previous
successful commands. 715/142 do not select this branch: they reach existing
terminal ownership and stop later, under unchanged prediction guards. These
finite stopped flying states are not physical crashes or proof of impossibility.

External repeats of 715 and 084 match complete candidate feedback exactly;
084's repeat also lands. Every invocation additionally reproduces decisions
and replays actual commands from the original source. Three direct controls
and two repeats remain outside the six-world denominator. No previously
successful selected landing is lost. The earlier full 16-world V9 panel remains
7/16 historical evidence; this pass does not measure a replacement 16-world rate.

## 084 walkthrough

At H1, step 2518 / 20.983 s, actual motion is at (879.247, 463.094) m with
velocity (58.213, -24.605) m/s. The new destination ideal arc asks for a slower
42.672 m/s lateral velocity and intersects the crest; current motion clears it.

The classifier finds crest (1000.000, 351.067) m. Vehicle diameter 12.806 m sets
body-clear x=1006.403 m. Actual engine-off physics crosses that threshold at
step 2782 / 23.183 s, after 264 ticks / 2.200 s, at (1007.316, 385.133) m with
velocity (58.213, -46.187) m/s. Fuel remains 5774.236 kg. The queried complete
entry snapshot and the later actual entry snapshot match exactly.

That runtime-derived clock happens to equal the earlier diagnostic's scheduled
checkpoint. The selector does not read that diagnostic, its clock, case ID or
successful suffix. The automatic run starts from the original scenario, rather
than restoring H1 or a synthetic airborne state. Its complete ordinary flight
equals the successful standalone capability witness, despite different planning
decisions and admission provenance.

Touchdown is a stable target contact at x=1202.421 m versus pad center 1200 m,
with incoming velocity (-0.563, -0.758) m/s, upright attitude and 4652.652 kg
fuel. Physical outcome, mission success, integrity, source-command replay and
decision reproduction all pass. Terminal control overshoots laterally and
returns before landing; this is recovery evidence, not an efficient-path claim.

## Evidence, reports and validation

Create-only roots under `outputs/eval/planner_v2_random_terrain/`:

- `capture-coast-terminal-control-20261009-v1`: 345 sealed files; receipt
  `2de2102e79a22522fd5a35d344f34c951f193acf6c26f036f1d8134a42cdee55`.
- `capture-coast-terminal-focus-20261009-v1`: 345 sealed files; receipt
  `fc0d010dd300013e48eceb1f8fa3059bad56f02c52cecc7ef36494c667e2ab53`.

Both packs bind the same source, native binary and renderer:

- Rust tree: `8225fe78691aedc1b797171d997097c90107c4c222509945a79e1f636a6b2231`.
- Native binary: `24684011dc502fed49b1aa314d00f52fa2dc4ed9fd1ff09dbf2be01420081d82`.
- Renderer: `f9be324b8e4029f45bd3fde1400c868ebc44e3801740ca45ae0a60d3285b83fa`.

Saved-only verifiers authenticate source/input/binary seals, protected earlier
evidence/site seals, original-source command replay, exact repeat decisions,
actual coast command intervals and unchanged goal/H ownership. No replay proof
is replaced by a query snapshot.

The maintained development gate passes all 11 checks, including strict Clippy,
formatting, workspace/CLI tests, 65 Node tests and local documentation links.
Focused ballistic Rust tests pass 46/46; terrain-study Python tests pass 156/156.
Explicit retained October-7 normal-policy numerical parity passes all 44 cases.
Static report checks authenticate all 22 common rich details, 1687 exact cycle
origins/goals, actual H/entry bindings, the existing diagnostic batch shell/tree
and 682 local link checks. This is not browser visual acceptance. The existing
LAN server returns HTTP 200 for the candidate batch and 084 detail; no restart,
accepted-site publication or navigation update occurred.

Review the candidate batch at
`/eval/planner_v2_random_terrain/capture-coast-terminal-focus-20261009-v1/`.
Open `random-084`, then use **Jump to decision** or **Inspect cycle**:

- Refresh **67 / coast_through_selected**: actual H1 state, blocked desired arc,
  current ballistic motion and the automatic crest-clear/terminal preview.
- Refresh **68 / coast_terminal_entry**: actual entry after the coast, first
  terminal command prediction and the complete executed landing interval.

Only H1 is an actual waypoint handoff. The crest marker is a terrain feature;
the coast checkpoint is not W2/H2. Existing trajectory, velocity/thrust, sample,
event, quality, controller, performance and mission views remain available.

Read-only saved verification, without another flight or current binary:

```sh
rtk proxy python3 -B studies/terrain_profiles/coast_terminal_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-control-20261009-v1
rtk proxy python3 -B studies/terrain_profiles/coast_terminal_panel.py verify \
  outputs/eval/planner_v2_random_terrain/capture-coast-terminal-focus-20261009-v1
```

## Next boundary

The narrow ownership mechanism is supported; do not broaden its trigger or tune
terrain thresholds to fix 715/142 in this pass. Next, review the successful rich
084 trace, then explicitly scope the unchanged candidate on the full frozen
16-world preservation inventory before any 1k campaign. Keep early-only 715
preservation and later terminal failures separate from this coast-selection
result. General terrain reliability remains unmeasured. No defaults, commit,
push or publication were changed by this closed pass.
