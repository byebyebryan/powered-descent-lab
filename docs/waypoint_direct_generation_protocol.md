# Input-driven nominal ballistic-direct generation

This pass implements one opt-in evaluator-owned direct generator. It does not
change `pd_plan::plan()`, V1 route validation, controllers, core physics/contact
thresholds, V2 classifications, F6, or defaults. No obstacle/waypoint expansion,
commit, push, deployment, or robustness claim belongs to this implementation
pass. Historical commands, schemas, identities, and artifacts remain intact.

The completed [results](waypoint_direct_generation_results.md) pass both gates:
exact known-flat physical parity followed by six accepted fresh nominal
flat/uphill/downhill cases under the frozen policy, without retuning.

## Sealed inputs and ordering

Before implementation or any new candidate/physics evaluation, the primary
sealed the six complete scenarios and common generation policy in
[`waypoint_direct_generation_fresh_inputs_v1.json`](../fixtures/research/waypoint_direct_generation_fresh_inputs_v1.json).
Its raw SHA-256 is
`3dffd6bc3f220629b96e817af8e0be7f6cb2e905c75a1df796e97aa15b066d24`.
Case order is span 600 m then 1,000 m, each flat, uphill, then downhill; height
changes are 0 m, +120 m, and -120 m. The source is at x = -span, y = -height
change; the target is at x = 0, y = 0. Pads are 36 m wide. Terrain connects
their flat shelves monotonically, with domain padding max(0.15 * span, 160 m).
There is no floor cutaway, obstacle, authored transfer route, or operational
waypoint. These are predeclared fresh nominal cases, not population-wide
coverage or perturbation-robustness evidence.

All scenarios use the existing vehicle, 9.81 m/s² gravity, 120 Hz physics,
60 Hz held commands, 90 s mission horizon, and source-pad upright rest.
Generation must not read the historical summary chain, stored command logs,
case-specific parameter overrides, or stored candidate decisions. Probe labels
are identity labels only, never branches controlling generation.

The first gate uses the existing pure continuous-flat scenario constructor
and probe label `continuous_flat_r00`. Historical paired/complete summaries
are comparison references only, opened after generation; they cannot supply
commands or candidate parameters to the generator.

## Finite generation contract

Accept a full scenario, source/target pad IDs, probe identity label, and one
versioned generation policy. Initial support is the existing vehicle/physics
configuration, forward transfers, upright source-pad rest, flat declared pads,
and no authored operational waypoint. Unsupported configurations are distinct
from malformed input, replay/integrity failure, and a valid finite `unknown`.

The effective V2 policy copies the embedded policy with only the existing
maximum mission time set to 90 s; its mission-time reserve remains 10 s.
Generate and retain all four duration bases: 0.75, 1.0, 1.25, and 1.5. Eligible
phase-complete analytical seeds each receive source-duration offsets
[-240, -180, -120, -60, 0] physics ticks, for at most twenty variants. Preserve
every skip/rejection and its stage/reason. Nonpositive or unsupported cadence
durations are explicit skips, not silently rounded or replaced.

For each eligible variant, reuse the existing 60 upright and 12 candidate-
directed full-throttle launch ticks. Re-solve the source bridge from actual
held-cadence launch position/velocity to the basis handoff. Keep existing source
screens and the original coast/terminal reference. Reuse the paired-mean
four-parameter correction with six iterations, eight descending line-search
scales, 1e-4 m/s² finite differences, +/-0.25 m/s² coefficient bounds, 1e-3
initial damping, and strict 1e-6 m / 1e-6 m/s handoff tolerance. No retry with
larger budgets, alternative launch rules, or case-specific tuning is allowed.

Generate actual full-flight commands using current mass/fuel and the existing
mapping. A seed's original V2 classification is not the complete witness's
acceptance label. New identities bind the exact request/scenario, policy and
algorithm versions, basis, launch/source/coast/terminal commands and reference,
cadence, and acceptance convention. Output paths and wall/CPU timing do not
enter semantic identity. Record setup compute/trial counters separately; no
real-time-planning claim is made.

## Shared complete-witness verifier

Extract reusable physical helpers instead of cloning another diagnostic
pipeline or constructing fake frozen artifacts. Historical runners remain
adapters with unchanged semantics. The verifier independently replays ordinary
and neutral core states, recomputes source handoff/phase joins, and does not
trust generator-provided passed flags.

Use the acceptance contract in
[`waypoint_direct_complete_flat_acceptance_protocol.md`](waypoint_direct_complete_flat_acceptance_protocol.md):
contact-free departure, source screens and strict endpoint, source-prefix and
replay parity, actual core-rotated feet/hull envelope with strict terrain
domain, wholly contained flat-pad transition corridors, 5 m reserve outside
those corridors, descending clear terminal entry from above, authoritative
stable safe first target contact, and complete command/fuel/time budgets.
Preserve preterminal impact velocity and the ordinary stable-touchdown zeroing
exception in parity. Capture terminal entry before its first physics tick.
Do not weaken the ordinary route validator or core contact thresholds.

Rank only fully accepted complete witnesses by planned total mission time,
then stable identity. First collision/contact time is never a ranking key.
Finite solver/search exhaustion is `unknown`, not physical impossibility.

## Gate A: behavior-preserving extraction

Generate the known flat request from inputs alone. Compare corresponding
(basis identity, source-duration offset) rows, not historical row positions:
the three eligible bases have nine survivor schedules, four complete accepted
witnesses, and five research-shortest target-contact crashes. Their command
payloads, source handoffs, and first-contact outcomes must reproduce the sealed
references. The winning physical witness remains native offset -180, planned
34.35 s, stable target contact at 34.275 s. The fourth rejected basis may add
five explicit skips to the new twenty-row ledger. New outer schema/identity
metadata need not equal old artifact bytes; the historical lanes must still
reproduce their own identities and outcomes.

If Gate A fails, stop before evaluating any fresh case. Correct extraction or
integrity defects without changing the generation/acceptance policy. Gate B
can be opened only after primary acceptance of Gate A and a production-code
freeze, with the sealed manifest's raw digest rechecked.

## Gate B: six fresh cases, no retuning

After Gate A, run all six sealed cases under the same frozen generator and
policy. Record the complete finite family, accepted/rejected counts, chosen
witness, earliest failed stage/tick, signed contact margins, clearance, fuel,
time, and solver/compute evidence for every case, including failures.

The declared gate passes only if every case has at least one complete accepted
nominal direct witness. A missed case is a precise bounded failure, not proof
of physical impossibility. Publish all six outcomes and stop; do not expand
the corpus, increase budgets, or tune failing cases into a fresh-validation
claim. Once inspected, cases are development data for any later corrected
algorithm, which would need a new fresh set.

## Validation and exit

Provide input-only preflight with no simulation construction/candidate solving
or output writes. Use create-only summaries, serialized identity round trips,
and two fresh-path runs for deterministic artifact checks. Cover malformed
and unsupported inputs, preserved skips/budgets, command tampering and phase
boundaries, geometry/domain/corridors, stable-contact terminal effects, and
accepted-only ranking with focused tests. Finish with workspace tests,
formatting, strict all-target Clippy, and diff integrity checks. Recheck the
sealed manifest and all historical reference digests; leave the worktree
uncommitted. No new reporting framework is needed.

The pass ends with a reproducible implementation plus the Gate A / conditional
Gate B verdict. A passing six-case result permits planning a separate obstacle
and waypoint-composition pass, not implementing it or promoting defaults.

## Command surfaces

`waypoint-direct-generation` accepts either `--scenario SCENARIO_JSON` with
`--probe-id LABEL` and optional source/target pad IDs and policy JSON, the pure
`--known-flat` input factory, or an input-only `--sealed-case CASE_ID` adapter.
`--preflight-only` never solves candidates, constructs `SimulationState`, or
writes outputs. The sealed-case adapter is available for preflight and
inspection; fresh evaluation in this pass uses the full gated six-case command.

The reproducible gate ordering is:

1. Generate known-flat evidence in a fresh directory using
   `waypoint-direct-generation --known-flat --output-dir DIR`.
2. Compare it using `waypoint-direct-generation-gate-a --generated-summary FILE
   --paired-command-summary FILE --acceptance-summary FILE --output-dir DIR`.
   This reads independent generation first, verifies raw historical digests,
   and compares complete paired schedules plus independently verified physical
   acceptance. A failed gate is published, not silently repaired by tuning.
3. After primary acceptance, use `waypoint-direct-generation-freeze
   --gate-a-summary FILE --output-dir DIR`. Its semantic identity binds all
   workspace Rust sources, Cargo inputs, the base vehicle fixture, the V2 seed
   fixture, and the sealed fresh manifest. Documentation and output paths are
   excluded.
4. Use `waypoint-direct-generation-fresh-gate --gate-a-summary FILE
   --code-freeze-summary FILE --output-dir DIR`. It verifies the accepted gate,
   manifest and freeze before evaluation, checks the freeze between cases,
   writes one complete artifact per case, and publishes a six-case summary.
   Repeat only into another fresh path to check determinism, without changes.

All commands are opt-in `pd-eval` subcommands; invoke them through
`rtk cargo run -p pd-eval -- COMMAND ...`. Summaries and their directories are
create-only. Identity checks are reproducibility/integrity checks, not signatures
or authorization against an adversarial caller.
