# Body-aware terminal rescue and conditional 1k comparison

[Documentation home](README.md) · [Braking result](ballistic_terminal_braking_results.md)

Status: closed after 25 records. Focus passed; preservation retained every old
landing but failed the exact direct-control flight requirement. The conditional
1k did not run. See the [results](ballistic_terminal_centering_results.md).
No default promotion or accepted-site publication occurred. The premeasurement
protocol remains captured verbatim; its stages below were not relaxed after flight.

## One mechanical revision

Combine the existing opt-in early braking guard with body-aware rescue centering.
An upright 8 m wide craft cannot land safely with its center 15 m from the center
of an 18 m half-width pad. Existing rescue instead calls that state inside the
pad and can stop lateral correction once speed is low. Use the horizontal extent
of the current rotated hull and feet to classify the rescue's safe pad interval.
Keep the existing outside-pad correction active when the body is outside it,
including at low sideways speed; do not enter settle/glide there. Reuse existing
rescue heights, target speeds, engine mapping and vertical-authority tilt cap.
No solver, terrain-specific constants, changed waypoint rule or relaxed guard.

The new `landing-body-centering` candidate inherits the previous ballistic
candidate plus the braking guard. Apply both only to its fallback/countdown
terminal path. Standalone coast-terminal previews and execution stay unchanged,
as do policy 3, controller constructors, frozen inputs, deadlines and replay.

## Fixed stages and stop rules

Finish query-only geometry/controller tests and the maintained development gate,
then freeze one isolated native binary, Rust source and common renderer. No
flight-driven tuning, rebuild or retry after the first measurement.

1. **Four focus records:** original 142 and 974 plus an exact external repeat of
   each. Compare with the receipt-bound 562/1000 coast-terminal capture. Both must
   land with complete physical/mission/integrity/source-replay/decision proof.
2. **21 preservation records**, conditional on focus passing: the existing 16
   random preservation worlds, three flat/uphill/downhill direct controls and
   repeats of 084/715. Compare with the old coast-terminal preservation capture.
   All eight previous random landings must remain landings; 142 must land; 084
   must retain its one-H landing; three direct ordinary flights must stay exact.
3. **1002 sweep records**, conditional on preservation passing: every original
   world in its original order plus repeats of 142/974. Compare separately with
   the last full ballistic candidate (562) and the retained policy-3/cap-24
   experiment (817). Report every paired gain/loss and all stop groups.

Maximum new native allowance: **1027 records**. Cap 24, original mission deadlines,
60 s per invocation, four workers and a 7200 s sweep bound. These are observed
development worlds, not a new held-out population. A failed focus/preservation
gate closes the flight pass; report the evidence rather than tuning or retrying.
Evidence/runner errors, source/protected drift or an actual crash/off-target
terminal outcome stop collection. Verified airborne finite stops do not interrupt
the full sweep; a broad regression is a result, not permission to tune mid-run.

Use the sealed common collector and rich batch/detail templates. Freeze each
stage's input manifest, source/binary/renderer identity and prior receipts before
flights. Sweep admission also binds the completed panel and focus to the same
candidate. Generated contracts and reports are create-only in isolated outputs.
Retain partial failures and all previous captures, report pages and selectors.

After collection, authenticate receipts, repeats, proof tuples, exact controls,
source preservation and report payloads; document the overall verdict and next
failure groups. No accepted-site navigation mutation, server operation, commit,
push, delegation or automatic follow-on campaign is part of this goal.
