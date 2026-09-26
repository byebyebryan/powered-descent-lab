# Input-driven nominal ballistic-direct generation results

This is an opt-in evaluator-owned setup-time generator, not a replacement for
`pd_plan::plan()`. It generates launch/source/coast/terminal command witnesses
from a full scenario and one fixed policy without opening the historical
summary chain. The contract, sealed inputs, and stopping rule are in
[the generation protocol](waypoint_direct_generation_protocol.md).

Gate A and all six predeclared fresh cases passed without retuning. This
closes the scoped input-driven nominal flat/uphill/downhill question; it does
not close robustness, obstacle demand, in-flight waypoint composition, or
production planner/controller integration.

## Gate A: extraction accepted

The input-only known-flat result has four bases, twenty predeclared rows,
eleven analytical skips, nine fitted/replayed schedules, and four complete
accepted witnesses. Five skipped rows retain the rejected fourth V2 base;
the other fifteen rows correspond to the historical three-base family.
All nine complete paired schedules and all fifteen corresponding physical
acceptance payloads reproduce the sealed references exactly. Five shorter
schedules still crash at target contact and remain ineligible for selection.

The winner is native V2 basis `fnv1a64:dee613017622ca16`, source offset
`-180` ticks, new wrapper `fnv1a64:6705d52b9ff4d37a`, and source schedule
`fnv1a64:2940049eabe499e3`. Planned time is `34.35 s`; authoritative stable
target contact is at `34.275 s`, with `950.075866 kg` fuel used. Strict source
endpoint errors remain about `5.84e-8 m` and `1.13e-9 m/s`. The signed
maximum-foot-clearance margin is about `+0.0167 m`; the smaller hull-penetration
margin is about `+0.0083 m`. These are narrow nominal contact margins, not a
robustness certificate.

Final known-flat runs `known_flat/run_b` and `known_flat/run_c` are
byte-identical: semantic identity `fnv1a64:445f88ba2c594369`, raw SHA-256
`3e479a9e46c1753c0950076f2434c607db7588003830762c45298fc3e3953e35`.
The independently produced Gate A summaries are also byte-identical, with
identity `fnv1a64:e0f682dcc2904ac8`. The earlier `run_a` is retained exploratory
metadata evidence, not the final artifact; the final conservative compute
accounting changed its outer identity without changing physical commands.

Both historical adapters reproduce their original raw bytes on the final
tree. Complete acceptance keeps identity `fnv1a64:ba51ace2c0bf1d08` and SHA-256
`85dd1276eb3e1bcbd958cf1d50dd94fc6f82d81d1faecf20c870de61f6c06cd6`;
paired feasibility keeps identity `fnv1a64:080e8e9867b02d97` and SHA-256
`6477a947537208872056d46e82c7b6fc0c7aa6801f14ca9ccb8b340f4da86247`.
No historical artifact was overwritten.

## Sealed fresh evaluation

Before fresh candidate solving or physics evaluation, Gate A was accepted
and the implementation frozen. Both fresh-path freeze summaries are
byte-identical: freeze `fnv1a64:14dc733c348b1625`, production inputs
`fnv1a64:390da4addcd93cac`. The manifest's original raw SHA-256 remains
`3dffd6bc3f220629b96e817af8e0be7f6cb2e905c75a1df796e97aa15b066d24`.

Every case retained all twenty rows with zero omissions. The six-case gate
passed: 120 rows, 44 analytical skips, 76 complete fitted/replayed schedules,
28 accepted witnesses, and 48 rejected target-contact crashes. Both runs
used the same frozen code, vehicle, launch, source-duration offsets, fitter
budget, geometry/contact verifier, and accepted-only ranking. No cutaway,
obstacle, authored waypoint, case-specific override, or retuning was used.

| Span / height change | Schedules / accepted | Planned s | Stable contact s | Fuel used kg | Hull-penetration margin mm |
| --- | ---: | ---: | ---: | ---: | ---: |
| 600 m / flat | 13 / 4 | 36.600 | 36.500 | 981.731 | +13.891 |
| 600 m / uphill +120 m | 13 / 4 | 36.600 | 36.500 | 981.760 | +12.417 |
| 600 m / downhill -120 m | 9 / 2 | 34.350 | 34.317 | 927.927 | +9.060 |
| 1,000 m / flat | 15 / 5 | 46.850 | 46.792 | 1,247.329 | +14.761 |
| 1,000 m / uphill +120 m | 14 / 4 | 45.100 | 45.033 | 1,204.334 | +2.245 |
| 1,000 m / downhill -120 m | 12 / 9 | 39.350 | 39.250 | 1,080.944 | +11.067 |

Times, fuel, and margins describe each selected witness. All selected source
position errors are below `7.62e-8 m` and velocity errors below
`1.68e-9 m/s`, against the strict `1e-6` tolerances. The slowest original
V2 base (multiplier 1.5) supplies five selected witnesses; the 1,000 m downhill
winner comes from multiplier 1.25. Selection is not hard-coded to a historical
role, row, or source offset.

The smallest selected hull-penetration margin is only `+2.244808 mm` on the
1,000 m uphill case. This is the remaining allowed penetration under the
current core contact rule, not a claim that the hull stays above ground at
contact. The global minimum airborne clearance occurs at the first upright
launch tick, about `0.000553 m`, inside the source-pad transition corridor
where the required reserve is zero. Outside whole-body flat-pad transition
corridors, the verifier still requires the unchanged `5 m` reserve. Neither
number is a perturbation margin or a continuous swept-path guarantee.

The 48 rejected full schedules all first contact in the terminal bridge with
both feet inside the target pad, but fail `first_contact_stable_safe_on_target`.
Their earliest rejection ticks per case are retained below. They are not
first-step source crashes or terrain-impossibility findings. Thirty of the
44 skips come from the rejected fourth seed; fourteen come from source
analytical screens and were not sent to the fitter. Full ledgers retain each
individual reason and stage.

| Case | Generation identity | Earliest rejected full-contact tick |
| --- | --- | ---: |
| 600 m flat | `fnv1a64:4db2b92441499be0` | 3322 |
| 600 m uphill | `fnv1a64:7876ecf4a764796c` | 3199 |
| 600 m downhill | `fnv1a64:05f10272d2b159a5` | 3325 |
| 1,000 m flat | `fnv1a64:dcaff2a856dcdb96` | 4074 |
| 1,000 m uphill | `fnv1a64:d1f97b86b5ea83bb` | 4343 |
| 1,000 m downhill | `fnv1a64:7e93653f301604a0` | 3957 |

Fresh runs `gate_b/run_a` and `gate_b/run_b` have byte-identical gate summaries
and byte-identical full artifacts for all six cases. Gate identity:
`fnv1a64:5826978a52f72e3f`; gate-summary SHA-256:
`494039d2800e8333fc15c09444f1db853841ddec1ad22be6c9261461fac2aa50`.

## Validation and next boundary

The integrated final tree passed `691` workspace tests in twelve suites,
strict all-target workspace Clippy, formatting, and diff checks. New focused
coverage includes input-only support preflight, malformed/unsupported/tuned
input refusal, cadence/nonpositive skips, create-only paths, independent
endpoint and command-provenance checks, accepted-only selection/tie-breaking,
CLI option conflicts, failed/tampered gate refusal, and deterministic freezes.
Existing geometry/domain/corridor and stable-touchdown replay tests still pass.
The sealed manifest, base vehicle scenario, V2 fixture, and all three direct
historical reference digests were rechecked unchanged. The implementation
pass left changes ready for a separate review/commit; no push or deployment
occurred.

Keep this as a finite nominal direct primitive, with an explicit `unknown`
outcome when its family is exhausted. It supports only the declared upright
source-pad rest configuration; it does not yet support arbitrary incoming
waypoint states or operational leg composition. A separately scoped obstacle
boundary pass may now be planned to distinguish blocked ballistic/recovery
paths from family limitations. It must retain the uncut baselines and must
not turn direct `unknown` into a proof that waypoints are physically needed.
Robust-contact validation remains a prerequisite for any production/default
promotion, especially given the millimetre-scale selected margins. No such
expansion or promotion was implemented here.

## Reproduction and evidence boundaries

Artifacts are local and create-only under
`outputs/research/waypoint_direct_generation_20260925/`. The full generated
JSON includes the input snapshot, every seed/offset skip or rejection, source
and full-flight evidence, held commands, independent complete verification,
signed first-contact margins, selection, and compute counters. Gate summaries
are compact indexes; inspect the per-case ledger for failure stages and ticks.

Use the four opt-in commands documented in the protocol, in this order:
input-only known-flat generation, sealed-reference Gate A comparison,
production-input freeze, then gated six-case evaluation. Repeat into different
fresh directories only; do not overwrite or tune the fresh set.

The fixed policy caps the family at four V2 seeds and twenty source-duration
variants, with six fitter iterations and eight line-search trials per
iteration. Observed fit/finite-difference/trial counts are separate from the
conservative policy-worst-case physics-work bound. Actual cumulative physics
ticks and setup timing are not instrumented; no real-time performance claim
is made.

Across the six cases, the observed counts are 24 analytical seed evaluations,
90 source bridge/analysis attempts, 76 fitter iterations, 304 reported
finite-difference columns, and 76 line-search trials. Every fitted schedule
converged in one recorded iteration. Conservative per-case physics-tick
upper bounds range from 21,319,200 to 35,316,000; they are budget bounds, not
measured simulator work or elapsed-time measurements.

V1/default planning, controllers, core physics/contact thresholds, V2 seed
classifications, F6, and historical research inputs remain unchanged. Accepted
results are discrete pointwise nominal simulator witnesses, not swept-path
proofs, perturbation guarantees, or controller integration. Finite direct
exhaustion means `unknown`, never physical impossibility or automatic waypoint
demand. Probe labels are identity labels, not generation branches.
