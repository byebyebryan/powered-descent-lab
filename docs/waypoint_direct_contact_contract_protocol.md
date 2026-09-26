# Direct-leg first-contact contract audit protocol

This is an opt-in, outcome-known diagnostic of the frozen launch-feasibility
profiles, not a new V2 certificate or a planner/controller selection rule. It
does not change the plant, contact thresholds, V2 screens, F6, or defaults.
The simulator's ordinary step owns the physical outcome; the audit observes
the state immediately after physics and before terminal landing effects erase
impact velocity and angular rate.

## Frozen inputs and order of gates

Rebuild and verify the existing analytical baseline, topology sweep, nominal
plant, and launch-feasibility artifacts before new contact analysis. Their
expected semantic identities are respectively `fnv1a64:d3fa6b24336f7c05`,
`fnv1a64:1bcd5a3bd6c6da01`, `fnv1a64:9e8cbc902ca11fbc`, and
`fnv1a64:2c7b965ffdc809d6`; the launch no-physics input gate is
`fnv1a64:a4654d06c9166b14`. Any mismatch stops the audit before new
physics. Preserve both selected roles per case, sharing profiles when their
candidate identities match: six cases, nine distinct profiles, and separate
120 Hz per-tick and 60 Hz two-tick-held command lanes.

First quantify the current orientation convention without changing either
model. At attitude `a`, core thrust points `(sin(a), cos(a))`, while rotating
the core body's local up vector by `a` points `(-sin(a), cos(a))`. V2 obtains
its footprint angle from a thrust direction `d` as
`atan2(-d.x, d.y)`. Record the signed direction/footprint discrepancy for
positive and negative tilts, plus the resulting two-foot coordinates,
clearances, and pad membership on flat and asymmetric terrain or a pad edge.
Distinguish a convention discrepancy from an observed change to any selected
profile's first-contact predicate. A material unresolved disagreement blocks
cross-model certificate claims; it does not authorize changing either model
or erase the value of a core-only contact diagnostic.

Use a symmetric `+0.1`/`-0.1 rad` diagnostic with the frozen 4 m foot
half-span and 5 m base offset. In addition to centered flat terrain, probe
the plane `height(x) = 0.1 x` at vehicle center `(0, 5)` and a flat pad of
half-width 6 m with vehicle center `x = 1.75 m`. These are synthetic
convention probes, not a new mission or evidence that the selected profiles
cross these boundaries.

Then audit the flat native landing and flat research-shortest crash in both
cadences. Capture the first contact tick's pre-terminal position, velocity,
attitude, angular rate, two signed foot clearances, minimum hull clearance,
pad membership, normal and tangential closing speeds, attitude error, the
dynamic hull-penetration allowance, and each stable/safe predicate. An
evaluator-local mirror of the unchanged contact rule must reproduce the
authoritative classification on both flat profiles and cadences. Also check
the ordinary terminal event/outcome against that neutral classification.
If the mirror or replay disagrees, stop before expanding to other cases and
record explicit skipped rows. Do not relax a tolerance to make parity pass.

Only after flat parity passes, audit the remaining frozen uncut and obstacle
profiles at both cadences, retaining every selected role and an explicit row
for any unavailable/terminated run. Keep first contact separate from profile
end and post-terminal state. Record 60 Hz handoff error and analytical reserve
as existing distinct evidence, not as an explanation silently substituted
for a contact failure. These nine outcome-known profiles characterize the
contract; they cannot validate a general certificate without held-out cases.

## Artifact and acceptance

Write a separate deterministic research artifact binding the frozen input
identities, case/role/candidate, cadence, orientation probes, pre-terminal
contact predicates, mirror result, authoritative classification, and any
skipped-gate reason. Exclude file paths and wall-clock values from the
semantic identity. Two fresh-path runs must produce byte-identical summaries.
Check focused parity and identity tests, then workspace format, tests, strict
lint, and diff integrity on the integrated tree. A positive local result only
permits a separately designed certificate/held-out validation pass; a
negative result is a specific stopping fact, not permission for a lowered
floor, threshold tuning, or default integration.

## Recorded result (2026-09-24)

The regenerated baseline, sweep, nominal, and launch summaries matched all
four pinned semantic identities. The launch summary also reproduced the
recorded SHA-256
`deab83d3af4cb96b0c608a1266912fc83c1d96bdba750b7610470e74b0627174`.
The new no-physics gate was `fnv1a64:86861aa3e4f8b35e` and accepted exactly
six cases, nine distinct selected profiles, twelve role bindings, and eighteen
materialized cadence rows. It did not create a physics result.

The flat native/shortest pair passed the predeclared four-row gate. All
eighteen paired replays then matched the sealed per-tick command/state fields,
the core neutral classification, the evaluator-local predicate mirror, and
the ordinary terminal event/outcome. Twelve rows landed on target and six
crashed, as in the sealed launch result. At the first contact of *every*
crash row, only the maximum-foot-clearance predicate failed: the higher foot
was `0.263` to `0.341 m` above its terrain while the stable-contact limit is
`0.15 m`. Minimum-foot, hull, normal/tangential speed, attitude, and angular
rate predicates passed. The flat native landing retained downward impact
velocity of about `1.63 m/s` in the pre-terminal state; its ordinary terminal
state reports zero velocity after the landing effect. This confirms why the
sealed terminal snapshot alone could not establish the contact contract.

At synthetic `+0.1` and `-0.1 rad` tilts, core and V2 footprint angles differ
by `0.2 rad` in magnitude. On the centered `height(x) = 0.1 x` probe they
give different foot clearances; at the flat pad edge they disagree on complete
pad membership, with the direction reversed for negative tilt. At the
*selected* frozen first contacts, the geometry-only V2-sign counterfactual
changed no no-contact, stable-geometry, or pad-membership predicate. The
orientation mismatch is therefore a real cross-model contract ambiguity,
but this audit does not identify it as the cause of these six crashes. The
60 Hz source-handoff misses and the flat/native analytical robustness-reserve
shortfall remain separate recorded findings, not contact-rule failures.

Two fresh-path summaries at
`outputs/research/waypoint_direct_contact_contract_20260924/audit_a/summary.json`
and `audit_b/summary.json` are byte-identical: SHA-256
`35021d0a7e4f13ee4cb3c1ba8c31be95592df494dd1688a5bcad58b3b16bdfaa`,
semantic identity `fnv1a64:f5a6600e99cd283b`. Five focused audit tests and
the CLI parser test passed; the integrated workspace gate passed 637 tests,
format, and strict Clippy after a behavior-preserving lint refactor. These
are outcome-known diagnostic replays, not held-out validation.

The next decision is the orientation contract: reconcile what positive
attitude means for thrust, body geometry, and V2 footprints under an explicit
physical convention, with new tests, before using one model to certify the
other. Then design a separate terminal selection screen around the actual
*first-contact* foot/hull/safety predicates, coupled to launch state and 60 Hz
held-command execution, and validate it on held-out flat and sloped cases.
Do not promote this audit to planner selection, controller policy, F6, or a
default without that separate work.
