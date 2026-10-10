# Local waypoint height and early destination reacquisition

[Documentation home](README.md) · [Previous entry experiment](ballistic_waypoint_entry_results.md)

Status: complete; all 65 planned native records verify. The
[results](ballistic_local_waypoint_results.md) are mixed: early-target alone
lands 715, while the combined preservation panel loses two landings. The flight
allowance is closed. Each capture retains the original preflight protocol.
This is not a goal loop, default promotion, a 1k campaign, publication, or
permission to commit/push.

## Two small changes

1. Local height: keep the existing first-blocking-ridge x placement. Seed height
   from its resolved crest and goal-local terrain, not the entire incoming
   corridor. For an unresolved local climb, use the scanned local interval.
   Keep the same body/reserve/two-second drop allowance. Full incoming arc,
   height repair, actual body/command and H guards still check all terrain.
   Record local height, the former corridor maximum and the unchanged margin.
2. Early target: once an active waypoint's actual engine-off coast is established,
   project its waypoint entry and estimate lateral braking room to the pad.
   Negative room triggers a destination preview at the existing 24-tick cadence.
   Use the unchanged terrain-blind destination constructor (or preserve an
   already accepted destination coast). Switch only if its ideal arc, estimated
   cutoff coast and immediate command pass the existing guards. A failed preview
   retains the waypoint. No higher destination arc search, landing suffix,
   brake-and-return maneuver or new search/budget is introduced.

An early destination reacquisition is a goal change, **not an actual H**. The
existing waypoint crossing/engine-off/two-second physical H predicate is not
relaxed. Record previous goal and braking-room estimate in the common rich
report. Keep all old panels/overlays and accepted-site navigation unchanged.

All new modes inherit the previous V6 combined effort/recovery behavior.
`combined` is a same-source V6 control; `local-height`, `early-target` and
`local-height-early-target` have distinct explicit V7 identities. Experimental
CLI still defaults to `ridge`; maintained planner policy 3 is unchanged.

## Frozen measurement and stopping point

- Unit tests: local height ignores an earlier shoulder, all incoming terrain
  remains audited, ample-room coast is preserved, a negative-room clear preview
  can reacquire destination, blocked previews retain the waypoint, query states
  stay immutable, destination construction and all previous modes stay unchanged.
- Freeze one isolated source/binary, then run four modes on original
  715, 349, 006, 807, 928, 139 and 268, the existing direct flat/uphill/downhill
  controls and a complete 715 repeat: eleven records per mode, 44 total.
- Run unchanged `local-height-early-target` on the previous 16 random worlds,
  the three controls and complete 715/349 repeats: 21 records, 19 primary.
  Total allowance: **65 native invocations**, no retries or flight-driven tuning.
- Source all inputs/comparisons from the receipt-bound V6 combined preservation
  capture. Same-source control must reproduce its complete saved feedback;
  every direct control must preserve the complete ordinary flight in every mode.
- Authenticate all receipts, original inputs, copied source/binaries, full command
  replay, internal decisions and external repeats. Run the maintained gate,
  study/report tests and explicit retained October-7 44-case numerical parity.
- Report all gains/losses and unresolved finite stops. This selected development
  sample is not held-out coverage. Close the pass even if results are mixed.
  Broader overshoot-and-return recovery and a full sweep remain separate decisions.
