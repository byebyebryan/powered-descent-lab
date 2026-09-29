# Strict saved-coverage nominal direct execution results

The opt-in operational executor passes its declared gate: all 24 exposed
controls retain exact nominal proof, and all four presealed fresh missions are
`Direct / CompletedSafeTarget / Match`. Two independent final-source release
runs agree. This closes finite execution/completion accounting, not production
planner wiring, perturbation robustness or waypoint composition.

The [completion contract](nominal_direct_execution_completion_contract.md) and
pre-implementation [protocol](nominal_direct_operational_execution_protocol.md)
own the scope. The [strict nominal executor](nominal_direct_flight_integration_results.md)
and [contact-phase diagnostic](nominal_direct_contact_phase_results.md) retain
their original meanings. This pass does not recover the diagnostic's 48 late,
uncovered observations or authorize its last-command hold.

## Implemented boundary

- [Core](../pd-core/src/sim.rs) adds bounded execution/replay and an incoming
  contact snapshot before ordinary stable-contact normalization. It advances
  the same ordinary plant/mission transition once. A separate versioned envelope
  retains actual final state, stop cause, boundary flags and the ordinary prefix.
- [Control](../pd-control/src/flight_program.rs) adds
  `run_flight_program_operational` and `replay_flight_program_operational` with
  controller identity `flight_program_operational_v1`. Both derive coverage
  from the complete validated program, not the observed log. Existing strict
  `run_flight_program`, `FlightProgramV1` and default controllers are unchanged.
- [The evaluator](../pd-eval/src/nominal_direct_operational.rs) independently
  admits the unchanged full nominal witness/program before motion. Its fixed
  `body_aware_operational_validity_v1` guard audits initial source support,
  every transition's actual held-command fuel budget, airborne phase-aware
  clearance, and incoming authoritative first contact. Generic neutral playback
  is not admitted safety proof. Admission, observed outcome and exact nominal
  comparison are separate records; an independent bounded replay must agree.
- Opt-in CLI modes provide input-only preflight and create-only evidence roots.
  Invalid, Unsupported and finite Unknown decisions execute no flight. Bundles
  retain full generation/proposals, program/witness/admission, raw action/event/
  sample prefix, validity/replay, separate results and compute evidence. A scoped
  summary links to a timing-free ordinary prefix trace; default reports are
  unchanged. New evidence DTOs reject unknown fields.

For expected contact `C`, saved coverage `K = updates.len * global interval`
and original planned end `H`, execute only through `min(K,H)`. Ordinary contact
and horizon handling precede driver stops; no callback at K, transition beyond
the bound, idle fallback, appended command or repeated final command is supplied.
An airborne stop remains raw `Flying / InProgress / Running`, not a fabricated
crash, timeout or completed mission. Driver stopping is a laboratory authority
boundary, not a physical recovery maneuver.

Safety failures remain sticky. Numeric/clock/replay disagreement is
`ExecutionInvalid / NotComparable`; the last valid finite prefix and first bad
boundary are retained. No NaN is disguised as a finite state. Zero-fuel idle
coast remains legal; underfunded powered commands are guard rejections rather
than invented core crashes. Contact uses incoming velocity/rate and unchanged
contact predicates, not normalized zero velocity or airborne clearance rules.

## Acceptance population

The unchanged 24-control nominal regression passes independently on final
source with identity `fnv1a64:73429341077b7008`. The operational gate regenerates
those same physical inputs without stored-program seeding, and all 24 retain
exact full generation, selected program, safe contact, clock/fuel and replay
parity. They are exposed regression controls, not new held-out coverage.

The four fresh inputs were resolved and hashed before runtime edits or flight
evaluation. All use the unchanged vehicle/policies, 120 Hz physics/global held
60 Hz commands, source rest, continuous uncut terrain and flat 36 m endpoint
shelves. No obstacle, waypoint or floor cutaway is added.

| Sealed fresh input | Height change | Contact tick / K | Contact time | H | Fuel remaining |
| --- | ---: | ---: | ---: | ---: | ---: |
| Flat, 685 m | 0 m | 3402 | 28.35 s | 3406 | 5469.827 kg |
| Flat, 845 m | 0 m | 3792 | 31.60 s | 3796 | 5377.933 kg |
| Uphill, 845 m | +75 m | 3792 | 31.60 s | 3796 | 5377.827 kg |
| Downhill, 845 m | -75 m | 3792 | 31.60 s | 3796 | 5377.550 kg |

Every row is `Direct / CompletedSafeTarget / Match` in both final runs, with all
nominal checks and fixed-guard replay passed. All selected contacts are exactly
nominal and at K, before H. The fresh ledgers retain 80 predeclared source rows,
53 available complete source schedules and 53 accepted terminal witnesses;
the 27 source-stage skips remain recorded. There is no fresh coverage gap,
replacement, policy tuning or first-step crash. These four inputs are now
exposed; future reuse is regression evidence.

Incoming normal speed is about 1.5 m/s. Fresh selected hull-penetration margins
are only 6.46–6.78 mm; upper-foot margins are about 0.154 m. These are margins
under the unchanged discrete contact predicates, not obstacle clearance,
swept-path safety or an operating tolerance.

## Verification and provenance

Final gate roots:

- [Run A summary](../outputs/research/nominal_direct_operational_execution_20260928/run_a_schema_v1/summary.json)
  and [HTML report](../outputs/research/nominal_direct_operational_execution_20260928/run_a_schema_v1/report.html).
- [Run B summary](../outputs/research/nominal_direct_operational_execution_20260928/run_b_schema_v1/summary.json)
  and [HTML report](../outputs/research/nominal_direct_operational_execution_20260928/run_b_schema_v1/report.html).
- [Legacy regression](../outputs/research/nominal_direct_operational_execution_20260928/legacy_gate_schema_v1/summary.json)
  and [complete validation evidence](../outputs/research/nominal_direct_operational_execution_20260928/final_validation.json).

Both operational roots have identity `fnv1a64:ac75e5bbc3d2a16d`. Their complete
619-file inventories agree: 530 pairs are byte-identical and 89 differ only in
observational compute timings. Normalization replaces only numeric values of
the nine explicitly listed timing fields; all other bytes, counters, commands,
contact evidence and identities remain exact. HTML/prefix traces are included,
not excluded from the comparison. The normalized inventory SHA-256 is
`9e9ea33304ed72a9868388c3176ad08b1311feddb564bc64c4f2db9306bc462b`.
The retained [read-only verification script](../outputs/research/nominal_direct_operational_execution_20260928/verify_final_repeat.cjs)
can be rerun from the repository root while the local validation checkout exists.

All 128 runtime/input/protocol/contract closure files match before, during and
after every case, and exactly match the artifact-free tested checkout. Binding:
`dfe4bee566ececb3ca35caddcdf4111d388d806c72ca900b3226eeea82c0956f`.
All 1,395 historical files across nominal integration, body-aware terminal and
contact-phase roots retain their exact before-work inventory/bytes. No old
archive or protocol was modified. Results/README/progress/current-status prose
is authored after measurement and is outside the runtime source binding.

| Pin | SHA-256 |
| --- | --- |
| Fresh physical manifest | `6df19fbd3ebb85d5a7f4a783398aaaa48e50c74a9de49454161b78c8e47658f8` |
| Sealed implementation protocol | `397da050a60d6a97fe0a0067904846e672f380ae458fd49550401f8667b2d202` |
| Final release binary | `baf217fdaa47195ca4d32e08a5058d30b8fef96c2e49224308c86d6d443bd1e7` |
| Before-work archive/readiness capture | `c20d3a030fcaa30a33d3d3ce4ce416aa80f54df11249b4bdbedb864d1403df79` |

The final workspace gate passes 809 tests in twelve suites, with one explicit
historical-artifact ignore, in `/tmp/pd-lab-operational-schema-validation-L9oyuo`
using a separate Cargo target and Git metadata. It contains no historical
research outputs before or after testing. All fourteen focused operational/
CLI tests, strict all-target workspace Clippy, formatting and diff checks pass.
Negative fixtures cover contact/bound/horizon precedence, truthful zero/partial
prefixes, early and within-coverage late contact, strict expected-step rejection,
guard stages/fuel, tampered admission, truncated/extra/off-clock/nonfinite replay
and finite artifact projection. Changed-contact fixtures are neutral executor
tests, not admitted disturbed mission flights.

Luna implemented the bounded core/control slice; the primary owned seals,
architecture, guard/admission, adapter/CLI/gates and final review. Final review
caught missing unknown-field rejection on new evaluator DTOs. Only that
deserialization/test contract was tightened: all 28 case/program identities and
all 588 case-bundle file pairs remain unchanged apart from compute timings.
The first successful `legacy_gate`, `run_a`, `run_b`, `validation.json` and first
test checkout remain intact. All final gates were rebuilt/rerun in the separate
`*_schema_v1` roots; those final roots, not the earlier source closure, own
current acceptance. No trajectory or physical policy was retuned after exposure.

## Local measured cost and use

Across the two final runs' 56 observations, local release medians are 963.624 ms
for unchanged research generation, 671.418 ms for independent selected-witness
verification, 3.477 ms for the entire guarded offline flight, 3.354 ms for bounded
replay and 13.025 ms for writing. Whole gates take about 56.47 s each. Compilation
and tests finished before measurement. These are observational, non-isolated
wall times, not an optimized backend or production setup budget. Generation
work still includes original-policy comparison and source-family regeneration;
the recorded verifier tick subtotal is not total generation work.

```sh
rtk cargo run --release -p pd-eval -- nominal-direct-operational-gate --preflight-only
rtk cargo run --release -p pd-eval -- nominal-direct-operational-gate --output-dir outputs/research/my_operational_gate
```

`nominal-direct-operational-flight` accepts `--scenario`, `--source-pad-id`,
`--target-pad-id` and either `--preflight-only` or a new `--output-dir`. Existing
output roots are rejected, not overwritten. The integrated gate requires the
pinned local archive controls; standalone mission preflight does not run flight
generation. Successful evidence capture is not an alias for completed flight:
inspect decision, operational outcome and nominal comparison separately.

## Verdict and stop rule

Close strict saved-coverage execution here. Uncut nominal flat/uphill/downhill
Direct flights work without cutaways; this pass makes their runtime completion
and bounded failure accounting explicit while preserving exact nominal proof.
`pd_plan::plan()` still uses V1's chord model, and production/default selection
is unchanged. No waypoint-demand inference follows from a finite Unknown.

Commands/recovery beyond saved coverage, general robust contact, arbitrary
incoming waypoint states, composition and useful production setup cost remain
separate design/acceptance gates. Do not reopen source fitting, cadence-only
experiments or obstacle sweeps to extend this checkpoint. The implementation
pass included no commit, push or deployment; subsequent review/commit closure
is recorded in [progress](progress.md).
