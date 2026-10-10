# Failure-only ballistic landing duration

[Documentation home](README.md) · [Previous local experiment](ballistic_local_waypoint_results.md)

Status: complete; all 44 planned records verify. The
[results](ballistic_landing_duration_results.md) gain 268 without losing an
existing landing, but the three original failures stop later during descent.
The allowance is closed; each capture retains the original preflight protocol.
User approved trying the small diagnostic follow-up.
No goal loop, default adoption, full 1k sweep, commit/push or site publication.

## Mechanism and boundaries

Saved-command diagnostic replays reproduce all 655 refresh origins/final states
across seven subjects. Combined 715/084/142 have an accepted destination coast
but every ordinary latest-safe duration fails the existing coupled PDG fit.
They are already latest-safe, not waiting for nominal hysteresis. One calculated
shorter duration passes the instantaneous fit in each original saved state;
that is not proof of a safe full descent.

Try exactly one fallback after all existing latest-safe candidates, including
long capture when applicable, have no ready fit. Solve the existing initial
vertical request for `ay = gravity`: `T = 6*dy / (4*vy + 2*target_vy)`.
Use existing target-surface/velocity conventions. Reject nonfinite or out-of-range
times; do not clamp or widen the current 3–14 s limits. Apply the unchanged
full-vector thrust, upward-direction, tilt and applicable terrain checks. Do not
replace an existing admissible selection. Entry assessment and live landing
controller share this candidate; nominal sampling and hysteresis stay unchanged.

Ordinary controller constructors and maintained policy 3 remain unchanged.
Explicit V8 `landing-duration` inherits V7 local-height plus early-target;
`early-target-landing-duration` inherits early-target alone as a success control.
Existing V5/V6/V7 identities and experimental CLI default `ridge` stay unchanged.
No new arc construction, coast bypass, waypoint placement, clearance, recovery,
fuel, clock, contact, mission or replay contract is introduced.

## Frozen measurement

1. Unit checks: finite/bounded derived duration, shared adapter/live use,
   opt-in surviving reset, rejected fallback constraints and exact preservation
   when the ordinary family already admits a fit. Full maintained gate, study
   and report tests, plus explicit retained October-7 44-case numerical parity.
2. Freeze one isolated source/binary in `target/ballistic-landing-duration-20261008`.
   Source original worlds from receipt-authenticated V7 captures. Compare the
   complete full-feedback same-source control and exact ordinary direct controls.
3. Same-source V7 combined control: 715/084/142/349/006, existing flat/up/down
   direct controls and complete 715 repeat: nine native invocations.
4. V8 combined fallback on the identical focused inventory: nine invocations.
5. V8 early-only fallback on original 715, the three direct controls and complete
   715 repeat: five invocations, comparing the saved successful V7 early-only flight.
6. V8 combined fallback on the previous 16 random worlds, three direct controls
   and complete 715/349 repeats: 21 invocations, 19 primary records.

Total allowance: **44 native invocations**, no retry or flight-driven tuning.
All outputs create-only; rich common detail and batch reports retained locally.
Verify original inputs, source/binary seals, complete physical/mission/integrity/
source-replay/decision tuple, external repeats and protected accepted-site files.
Stop on evidence failure; do not silently widen the allowance. Close and report
all gains, losses and finite stops even if instantaneous entry improves but actual
descent fails. This selected development panel is not held-out or 1k coverage.
