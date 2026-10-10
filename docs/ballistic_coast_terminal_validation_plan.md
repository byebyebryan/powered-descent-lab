# Coast-to-terminal preservation and conditional 1k validation

[Documentation home](README.md) · [Focused result](ballistic_coast_terminal_results.md)

Status: **closed, all 1023 declared records verified**. The
[results](ballistic_coast_terminal_validation_results.md) record 8/16 preservation
landings and 562/1000 sweep landings, including broad gains and regressions.
No planner, controller, terrain, threshold, budget or report-template tuning
occurred in this pass. No additional flight or adoption is authorized.

## Candidate and allowance

Use the exact focused `coast-terminal` native binary
`24684011dc502fed49b1aa314d00f52fa2dc4ed9fd1ff09dbf2be01420081d82`, Rust tree
`8225fe78691aedc1b797171d997097c90107c4c222509945a79e1f636a6b2231` and renderer
`f9be324b8e4029f45bd3fde1400c868ebc44e3801740ca45ae0a60d3285b83fa`.
Do not rebuild or alter the native candidate between panel and sweep.

First run **21 records**: all 16 previously frozen random preservation worlds,
three direct controls, and external repeats of 084 and 715. Inputs and complete
V9 comparisons come from `capture-landing-countdown-preservation-20261009-v1`.
The old six-world protocol remains closed; this is a separate create-only pass.

Proceed to the conditional **1002-record** sweep only if the complete panel and
all seals/proofs/repeats pass, all seven V9 random successes remain landings,
084 still lands with one H, the three direct ordinary flights stay exact, and
no actual crash/off-target touchdown appears. Any loss closes the flight pass
at the panel; report it rather than tuning or retrying.

The sweep reuses all 1000 original receipt-bound worlds, in original order, plus
external repeats of 084 and 715. Compare with the retained policy-3 817/1000
capture and the last complete ballistic-candidate sweep, 401/1000. There is no
full V9 1k baseline: this measures the combined newer candidate, not the isolated
increment from coast-to-terminal. The observed panel is included in that known
population; this is paired development evidence, not a fresh/held-out test.

Maximum new native allowance: **1023 records**. Each invocation separately
repeats decisions and replays commands from original source. Use cap 24, four
workers, the original per-mission deadlines, 60 s per invocation and 7200 s
campaign bound. Do not extend hop/time/fuel limits or add extra cases/retries.

## Validation and execution boundaries

Pin the gate panel, input receipts, native/Rust/report identities and collection
contract before sweep measurement. Reuse common proof readers and create-only
output/report writers; retain every finite stop and every partial infrastructure
failure. New collector support is not a trajectory change.

Run ordinary collector tests and saved focused verification before panel flights.
Before sweep flights, check the conditional gate, frozen contract and collector
tests. Stop the sweep on source/protected-state drift, runner/verification errors
or an actual crash/off-target terminal outcome; ordinary verified flying stops
do not stop the campaign. Never silently repair and rerun a measured flight.

After collection, authenticate receipts, all original-source/decision/command
proofs, complete repeats, separate denominators, paired gains/losses and typed
outcomes. Inspect coast-selection/entry counts, whether admission leads to later
stops, and common report payload/H bindings. Retain rich detail plots and the
existing diagnostic batch tree. Generated pages stay in isolated capture roots.

No accepted-site publication, navigation/selector mutation, default promotion,
server restart, commit or push is part of this pass. A good panel authorizes the
declared sweep only, not adoption; a good sweep still needs separate maintained
candidate acceptance and untouched-world coverage before a general reliability
claim. Finish the declared inventory, document the verdict and stop.
