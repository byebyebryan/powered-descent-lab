# Waypoint V2 ground diagnostic primary review

## Decision

Accept the bounded diagnostic/review checkpoint, not runtime replacement or a
usable planner. The [results](waypoint_v2_ground_diagnostic_results.md) support
one narrow next change: state-derived, lateral-compatible terminal timing with
the same physical reference backend and 0.925 thrust budget. This is a separate
primary contract review, not an independent-agent review.

## Contract findings

The eight original controls reproduce on actual terrain from fresh H0 under
their original 9600-tick deadline. Complete command mapping, source boundaries,
observed S/T, final states, raw incoming contact and independent replay agree.
The adapter captures boundaries before the next held update, handles valid odd
contact endpoints, and excludes full-thrust ground preparation from terminal
headroom measurements. No snapshot restoration or fuel/time reset is used.

Sixteen references isolate duration at actual T without a sweep. Original
references reconstruct original paired throttle exactly. Alternative references
are clearly analytical: their fuel/acceleration estimates are not labeled as
executed, and each records all predicates separately from the first rejection.
Acceleration limits divide force by incoming mass; changing-mass applied demand
is a separate measurement. Completed attitude turns are not charged a second
time. The all-eight lateral-reversal result is supported; explaining every
research ground miss solely by this matched-entry result would not be.

Three shadow selections match the predeclared sealed-ledger oracle. Negative
selection reads no terrain witness result; no eligible entry means no substitute.
Alignment and fuel checks remain unchanged before realization. The realizer
retains rate-limited attitude, paired throttle, changing fuel, deadline, raw safe
contact and replay. Rejected entries remain rejected in the artifact; positive
control admission is unchanged. Independent preparation replay metadata is
correct in the accepted final pair.

All three probes land, but only downhill meets the terminal thrust budget.
Body reserve and engine headroom are separate contracts. The flat/uphill probes
are useful diagnostic flights, not new nominal coverage, reserve-safe proposals
or justification to loosen margins. Tightening only the combined bound cannot
make their approximately 0.993 applied throttle fit 0.925.

Acquisition inspection stays within existing histories and estimates. The
adapter makes no additional seed search or continuation witness. Ground entry
acquisition remains unresolved; a successful legacy entry is not proof that
the research acquisition produces it.

## Integration and failure review

Only the diagnostic module/tests and minimal characterization export/CLI wiring
are added in this pass. Existing helper bodies, protected sources, fixtures and
old evidence retain their byte hashes. There is no runtime planner/local/controller
change or new reporting framework. The existing CLI emits a compact identity
summary and fails on integrity errors while retaining create-only evidence.

Pinned corpus/study/canonical hashes, sorted source bindings and post-run checks
fail closed. Missing inputs or command/replay mismatches are integrity failures;
finite analytical or physical rejection remains a diagnostic result. Unit tests
cover duration provenance, held clocks/odd endpoint coverage, exact versus
combined norms and units, selection independence, malformed limit parsing,
tampering, missing/changed bindings and create-only JSON-object output.

Both accepted complete summaries/bindings match byte for byte, and all 247 bound
files independently verify against the final tree. Workspace tests pass 889
with three ignored; focused tests including the two explicitly requested
archive tests pass all six checks. Formatting, structural/input,
corpus/tamper, fake-suite and preservation checks pass. The results document
records final workspace validation separately from physical diagnostic evidence.

Strict Clippy's remaining `single_element_loop` failure is in an unchanged
baseline file under rustc 1.99.0. The narrow allowance passes with all other
warnings denied. Accept this as a disclosed baseline lint exception, not an
unqualified strict-Clippy pass; do not expand this diagnostic to edit that file.

## Next-pass boundary

Keep the terrain-blind current-state-to-target model and one-sided vertical
preservation. The next design should change terminal time/reference construction
only, with a predeclared small coupled-state candidate set, the same margin and
the same realizer. Demonstrate compatibility on proven entries, then measure
actual new ground acquisition and preserve retained airborne behavior. Do not
use stored known-good durations as the runtime answer, weaken no-reversal to
hide long-time overshoot, or alter acquisition simultaneously without evidence.

If a bounded time change cannot recover the ground starts, retain the working
old constructor and decide on one ground-mode acquisition adapter. Do not
repeat an open-ended timing/seed search. Mathematical unification is not a
reason to discard the usable game-oriented route while this remains research.

Runtime approval and the unchanged full 32-case usability gate remain later
decisions. No commit, push, deployment, default promotion or additional policy
revision is authorized by accepting this diagnostic.
