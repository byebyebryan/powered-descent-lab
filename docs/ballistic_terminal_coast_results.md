# 084 coast-through / terminal capability probe

[Documentation home](README.md) · [Previous countdown result](ballistic_landing_countdown_results.md)

Verdict: **084 can preserve its H1 coast, clear the crest and land without W2**.
The ordinary standalone terminal controller lands from the predeclared H2-time
checkpoint. This is a successful continuation witness, not an automatic planner
fix or a new random-terrain pass rate.

## Fixed experiment

Four native invocations were declared before flights: standalone terminal,
transfer terminal configuration, transfer configuration with the existing
final-waypoint retention lifecycle, and an external repeat of that last setup.
Each invocation also reproduces its decisions and replays commands.
No outcome-driven tuning or additional cases followed.

All trials use the exact saved V9 `random-084` input and replay its original
commands to H1 at step 2518 / 20.983 s. They then issue 132 engine-off commands
to step 2782 / 23.183 s, using the saved H2 **clock only**, not selecting W2 or
recording another H. Position, velocity and fuel at that checkpoint equal the
original H2 motion. Near the crest, the recorded full-body clearance is 33.256 m.

The terminal takeover clock is deliberately forced for this capability test.
There is no synthetic airborne restore, neutral terrain, new waypoint, fuel
reset, clock extension, weakened physical reserve or contact exemption. Actual
body reserve and the existing 24-tick held-command prediction remain active.
Terminal candidate terrain handling remains configured and unchanged.

## Outcomes

| Setup | Physical / mission result | End time | Actual waypoint H |
| --- | --- | --- | --- |
| Standalone terminal defaults | Target landed / success | 61.808 s | H1 only |
| Transfer terminal defaults | Flying / in progress; original budget stop | 79.800 s | H1 only |
| Transfer defaults + waypoint retention | Same complete ordinary flight as transfer | 79.800 s | H1 only |
| External repeat of retention setup | Exact feedback and controller trace repeat | 79.800 s | H1 only |

The successful incoming contact is a stable target touchdown at x=1202.421 m
(pad center 1200 m), with velocity (-0.563, -0.758) m/s, upright attitude and
4652.652 kg remaining fuel. The ordinary full terminal sequence, including its
lateral correction and eventual descent, is executed and replayed—not just
inferred from a gate query. It overshoots laterally before returning; this is
usable recovery evidence, not an efficient-landing claim.

The transfer-configured trials choose an initial 22 s horizon versus the
standalone controller's 14 s. They overshoot farther, descend to roughly 118 m
center height, then climb again before descending late. They have not landed
when the unchanged original mission budget ends; we do not infer their outcome
under a longer budget. Waypoint retention releases immediately on the existing
vertical-braking-margin condition and is never active, so that trial does not
demonstrate successful retained-plan use.

## What this establishes, and what it does not

- W2 is not necessary to clear the crest in this particular continuation.
  The current motion already clears it; constructing the slower destination
  arc is what makes that proposed arc intersect terrain.
- Earlier terminal ownership works in the standalone capability trial. The
  previous V9 route waits another 3.7 s after H2 while correcting the destination
  arc, losing about 187 m of height before entering terminal control; it then
  stops under contact prediction. More terminal fallback tuning is not the only
  possible repair.
- Dynamics-only entry readiness is true at the probe checkpoint, but the
  configured terminal terrain approximation reports unsafe. It extrapolates
  initial acceleration over the whole horizon, not the actual feedback flight.
  The successful guarded flight demonstrates that disagreement for this state;
  it does not justify deleting the terrain check or treating every urgent gate
  as feasible.
- We have **not** established that the planner can decide this at H1, that H1
  itself is a safe immediate terminal entry, or that this rule generalizes.
  The probe's takeover clock comes from saved evidence. A runtime rule must not
  consult that clock, case ID or a saved successful suffix.

The next design question is a small coast-to-terminal alternative before
adding another clearing waypoint: preserve actual terrain-clear motion, decide
when terminal ownership is useful, and keep checking real command drift.
Distinguish a blocked newly desired arc from a blocked current coast. Validate
the selector on 084 and a small preservation panel before a wider campaign.
Do not replace this with a complete landing-suffix solver or automatic adoption.

## Evidence and validation

All four records pass source-input binding, complete original-source replay,
exact H1 replay, full command replay and deterministic decision reproduction.
The standalone success is independently re-executed inside its invocation;
the separately invoked external repeat covers the retention setup, not the
successful standalone setup. No stopped trace is labelled a crash or landing.

The maintained development gate passes all 11 checks, including workspace/CLI
tests, formatting, strict all-target Clippy, 64 Node tests and local docs links.
Three new tests cover the retention adapter and diagnostic source/boundary
rejections. Explicit retained October-7 normal-policy parity passes all 44
complete non-timing flight records. Common rich details retain terminal metrics,
original plots and planning-cycle views; static checks verify 68 cycles per report, H1 binding and
the explicit coast/takeover decisions. The existing LAN server returns HTTP 200
for the successful detail; no server or accepted-site/navigation change occurred.

Capture root:
`outputs/eval/planner_v2_random_terrain/capture-terminal-coast-084-20261009-v1/`.
Its create-only `protocol.json` binds the four-case allowance. Each run contains
`attempt.json`, source/command proofs, `terminal-diagnostic.json`, `feedback.json`
and the common rich `report.html`. The capture receipt binds these files.
All 38 sealed files verify. Receipt SHA-256:
`b7e8937b910c7262e154c4dfc54f484ca5564f1b2578a68063f6eb1d13bb4920`.

Frozen Rust tree:
`cec19ee27bbd83eb039bcf4c674e273193e0cbf0252db360f503cc1497988dcb`.
Native example binary:
`97048d6cb7f6fa95681eecc79521148e3f8c55727a1a1659577b635887e330cb`.
Saved baseline feedback:
`08fa478ca454029529637d9a88b7d33dc960c6059739d43ec674a8d6a1481236`.

Review the successful detail at
`/eval/planner_v2_random_terrain/capture-terminal-coast-084-20261009-v1/runs/084-standalone/report.html`.
Use the refresh selector: **66 / diagnostic_preserve_existing_coast** at H1,
then **67 / diagnostic_scheduled_terminal_takeover** (the default final cycle).
There is no W2/H2 marker in this
trial. The old H2 time is only a diagnostic checkpoint.

For an explicitly requested future reproduction into a fresh output directory:

```sh
rtk proxy cargo run --release -p pd-eval --example ballistic_terminal_coast -- \
  --scenario outputs/eval/planner_v2_random_terrain/capture-landing-countdown-preservation-20261009-v1/runs/random-084/scenario.json \
  --feedback outputs/eval/planner_v2_random_terrain/capture-landing-countdown-preservation-20261009-v1/runs/random-084/feedback.json \
  --prefix-handoff 1 --takeover-handoff 2 --terminal-setup standalone \
  --output outputs/eval/084-terminal-coast-new-attempt
```

This example is separate from all maintained planner selectors. It is not a
production entry policy or authorization for another flight campaign. Defaults
and frozen fixtures/captures are unchanged. No commit, push or publication.
