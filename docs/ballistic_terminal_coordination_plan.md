# Opt-in terminal command coordination

[Documentation home](README.md) · [Previous 1k result](ballistic_terminal_centering_sweep_results.md)

Status: closed after 129 admission attempts. The [results](ballistic_terminal_coordination_results.md)
record 115/121 selected random landings, but lost landings in 048/755 fail admission.
The conditional full sweep did not run. The original premeasurement protocol
remains receipt-bound in the capture; the stages below are not new authorization.
The subsequent [full diagnostic](ballistic_terminal_coordination_sweep_results.md)
ran under a [separate contract](ballistic_terminal_coordination_sweep_plan.md)
without changing this failed admission verdict or executing its conditional stage.

User-authorized implementation and conditional validation, 2026-10-09. This is
not planner replacement acceptance, a fresh held-out estimate or permission to
promote cap 24. Keep ordinary constructors, policy 3, standalone coast-terminal
execution, waypoint selection, terrain, fuel, clocks and physical guards unchanged.

## One bounded candidate

`terminal-coordination` inherits V12 acquisition and fallback/countdown control.
When only the early vertical guard activates, preserve the nominal command's
lateral acceleration, supply the existing distance-based required lift, and clip
lateral demand to remaining thrust/tilt authority. Final-height and explicit
lateral rescue retain their existing priorities. At low-energy, body-contained
touchdown, use the existing upright settle before inherited small trim causes
one-foot contact. No new solver, terrain threshold or relaxed contact criterion.

On a strict short-prediction domain overrun, retain the actual executed prefix,
query origin/state, requested command and typed error. Reject the query; do not
apply its command or clamp/extrapolate terrain. Distinguish actual-state overruns.
Replay and reproduce the stopped prefix independently; predicted contact is not
an executed crash, and prefix verification is not a landing certificate.

## Premeasurement stages

Run controller allocation/settlement and typed-domain synthetic tests, tooling
tests, the maintained gate and opt-in retained 44-case numerical parity. Freeze
one isolated binary, renderer and complete source/protocol inventory before the
first measured attempt. No flight-driven tuning, rebuild, retry or second candidate.

1. **129 admission attempts:** the exact 75 V12 gains and 45 lost V10 successes,
   plus 559's existing one-foot prediction, three existing direct controls, and
   exact external repeats of 142/150/288/757/974. Random denominator: 121;
   controls/repeats separate. Authenticate V10, V12 and control source receipts.
   Require complete prefix proofs, unchanged pre-terminal actions/entry clocks
   wherever V12 has a complete record, all 75 gained landings retained, three
   direct landings, upright cases 559/757/888 landed, and at least one recovered
   lost-success case. Outcomes, not exact terminal-command equality, are the
   preservation criterion. A failed admission closes measured work.
2. **1005 attempts, conditional on admission passing:** all original 1000 worlds
   in order plus the same five repeats, with the identical frozen candidate.
   Report all gains/losses against V12's 592 and V10's 562 separately, retaining
   the older policy-3 result as a separate reference. Verify exact unchanged
   flights in the 335 pre-terminal-stop worlds. No default promotion follows.

Total maximum **1134 native invocations**, cap 24, four workers, 60 seconds per
attempt and 7200 seconds per stage. Zero retries. Finite flying stops and typed
prediction-domain stops are outcomes. Stop collection on evidence/runner errors,
source/protected drift, actual invalid-domain states, executed crashes or off-target
touchdowns. Preserve partial results and all declared unattempted rows.

Use create-only captures and the common rich detail/batch templates. Bind inputs,
old outcomes, source/binaries, protocol and admission receipt; do not rewrite old
captures. Finish by verifying receipts, every proof tuple, exact repeats and rich
planning payloads/links, then document the complete result and remaining mechanisms.
No commit, push, server operation, accepted-site/navigation publication or further
automatic campaign is included.
