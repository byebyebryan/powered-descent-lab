# Retained ballistic landing countdown

[Documentation home](README.md) · [Previous duration results](ballistic_landing_duration_results.md)

Status: **CLOSED**. All 48 planned records are complete; see the
[results](ballistic_landing_countdown_results.md). The implementation counts down
as intended but adds no landings: 084/715 still stop. The sections below retain
the original preflight protocol. No goal loop, default adoption, full 1k sweep,
commit/push or accepted-site publication occurred.

## One small change

V8's one failure-only horizon opens landing but can repeatedly request zero
initial net vertical acceleration. Pin the arrival time on the **first actual
selection of that fallback** by live latest-safe landing control. The entry
adapter stays unchanged. Count down the remaining duration instead of resetting
that arrival every tick, using the existing terminal-plan storage/admission
machinery. Ordinary and authored-waypoint controller retention stay unchanged.

Use exactly the same target-surface and current desired-vertical-speed conventions
as V8. Each update re-evaluates the remaining fit with existing thrust, tilt,
upward-direction and configured terrain checks. Keep it only while ready. On
expiry or infeasibility, release once to the unchanged V8 controller; never
re-admit or automatically extend the deadline. The initial seed remains within
3–14 s; its countdown uses the existing retained-plan 0.5 s residual time floor.
No new horizon search, destination arc, waypoint placement, clearance exception,
recovery maneuver or landing threshold. Actual command/body/contact guards and
all replay proofs remain unchanged.

Explicit V9 `landing-countdown` inherits V8 combined local-height/early-target;
`early-target-landing-countdown` checks early-only successful 715. Old identities,
maintained policy 3, controller defaults and experimental CLI `ridge` remain
unchanged. Countdown admission/release is internal terminal state, not a waypoint
handoff or a planner goal change.

## Freeze, compare, close

- Unit checks: fallback-only admission, unchanged successful original choices,
  constant arrival/decreasing remaining time, full-vector and terrain rejection,
  one-time release on invalidity/expiry, no re-admission and reset behavior.
  Run maintained gate, study/report tests and retained October-7 44-case parity.
- Freeze one source/binary in `target/ballistic-landing-countdown-20261009`.
  All worlds/previous feedback come from authenticated V8 captures; no new seeds.
- Same-source V8 control: 715/084/142/268/349/006, three existing flat/up/down
  direct controls, exact 715/084 repeats: **11 records**.
- V9 countdown on the same focused inventory: **11 records**.
- V9 early-only on successful 715, three direct controls and exact 715 repeat:
  **five records**, against the saved V8 early-only successful ordinary flight.
- V9 combined preservation on the previous 16 random worlds, three separate
  direct controls and 715/349 repeats: **21 records**, 19 primary.

Allowance: **48 native invocations**, no retry, tuning or source changes after
measurement begins. Same-source control must match complete saved V8 feedback;
all direct ordinary flights must stay exact. Retain every gain/loss, physical
and mission outcome and finite stop. Authenticate receipts, full source-command
replay, decisions, exact repeats, inputs, binary/source and protected-site seals.
Use create-only outputs and unchanged rich common detail/batch templates.
Stop on evidence failure; don't invent new flight allowance. Close and report
mixed results honestly. This selected panel is not fresh/held-out coverage.
