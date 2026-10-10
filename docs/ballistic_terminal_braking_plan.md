# Ballistic terminal braking guard

[Documentation home](README.md) · [Previous validation](ballistic_coast_terminal_validation_results.md)

Status: **closed after four verified focus records; preservation gate failed**.
The [results](ballistic_terminal_braking_results.md) record a landing in 974 and
a remaining pad-edge reserve stop in 142. The conditional 21-record panel did
not run. No additional flight or adoption is authorized. This is an opt-in
experiment, not a change to policy 3, controller defaults or accepted captures.

## Diagnosis and single change

The latest 1k capture has 102 `short_command_rejected` stops after terminal
takeover. Of those, 85 predict unsafe contact and 17 violate body reserve.
Saved-command replays of 142, 974, 084, 127 and 900 reproduce every terminal
command and final state exactly. In 974 the retained countdown stays active
while vertical stopping room is spent; in 142 it releases at 23.8 s and the
receding guidance likewise brakes too weakly. Initial acceleration feasibility
is not a certificate for the remainder of a descent. The final guard is correctly
preventing execution of unsafe commands; it is not the defect to relax.

Test one simple mechanism: when descending faster than the existing conservative
braking-speed envelope, and the nominal command supplies less upward acceleration
than the distance-to-touchdown braking calculation requires, activate the
existing touchdown rescue command earlier. Reuse existing configured margins,
speed limits, lateral rescue, vertical-authority tilt cap and engine conversion.
No new search, profile solver, height threshold, retained clock or tuning grid.
The existing final-height rescue remains unchanged.

Apply this only to the ballistic fallback/countdown terminal path, under a new
`landing-braking-guard` candidate identity. The coast-through branch continues to
preview and execute standalone defaults identically; 900's later terrain conflict
is a separate ownership/replanning problem, not covered by this fix.

## Frozen allowance and acceptance

Before any new measured flight, finish unit checks and build one isolated native
candidate. Freeze that binary, Rust source and renderer for all following runs.
No flight-driven changes, retries or automatic 1k sweep are allowed.

First run 142 and 974, with one external repeat of each: **four records** using
the exact worlds from `capture-coast-terminal-sweep-20261009-v1`. Proceed only
if both land with complete physical/mission/integrity/source-replay/decision
proof and repeats match. Otherwise close the pass with the failure evidence.

If focus passes, run the existing 16 random preservation worlds, the three
unchanged direct controls and repeats of 084 and 715: **21 records**. Compare
with `capture-coast-terminal-preservation-20261009-v1`. All eight previous random
landings must remain landings; all three direct ordinary flights must stay exact.
Neither an unsafe physical terminal outcome nor evidence/source/protected drift
is acceptable. Maximum allowance: **25 new native records**, cap 24, original
mission deadlines, 60 seconds per invocation. Saved-command diagnostic queries
are not new flight measurements.

Retain create-only outputs, rich common reports, receipts, finite stops and all
partial errors. Preserve previous captures and selectors. No accepted-site
publication, server restart, default promotion, commit or push is authorized.
After the declared validation, document gains/losses and limitations and stop.
