# One-update terminal-completion reserve implementation protocol V1

## Design-only status and subsequent authorization

This protocol accompanies the reviewed
[reserve contract](nominal_direct_terminal_completion_reserve_contract.md) at
`b6a235a73501a5ea20e168af44e66c9e685fcb16` on 2026-09-28. The current goal
resolves design, seals input-only readiness and verifies preservation. No
runtime implementation, generation, flight measurement, continuation acceptance,
commit, push, deployment, default change or roadmap expansion occurs here.

The implementation sequence below requires separate authorization. Primary
owns policy, replay lineage, guards, integration and acceptance. A settled
bounded core/control slice may be delegated; do not start a writer until its
shared-loop and segment-artifact behavior is settled. Stop for a specific
design gap if a broad checkpoint/disturbance framework would be required.

## Physical input and policy seal

Manifest: `fixtures/research/nominal_direct_terminal_completion_fresh_inputs_v1.json`.
SHA-256:
`57287ac683363b1e76fdfbea0486460a0df5b95f1af88e9ee31ee8ee8fdf716a`.
The schema is `nominal_direct_terminal_completion_fresh_inputs_v1` / 1,
with unknown fields rejected by the future typed loader.

Ordered cases are 735 m flat, 915 m flat, 915 m uphill +75 m and 915 m downhill
-75 m. Preserve 36 m flat shelves, 160 m domain padding, six-point continuous
uncut terrain, source at upright rest, no obstacles/authored waypoints, seed 7,
the existing supported vehicle, unchanged generation/terminal policies,
9.81 m/s2 gravity, 120 Hz physics, 60 Hz global control and 90 s horizon.
The only physical changes from the established nominal input construction are
the prescribed spans and endpoint height changes. No expected flight outcome,
selected schedule or fitted parameter is a manifest input.

Before runtime edits or flight generation, the existing release binary passed
`nominal-direct-operational-flight --preflight-only` on all four resolved
scenarios: `supported=true`, `rejection=null`, `simulation_created=false`.
Binary SHA-256:
`baf217fdaa47195ca4d32e08a5058d30b8fef96c2e49224308c86d6d443bd1e7`.
This checks existing nominal input readiness only, not the unimplemented reserve,
generator coverage, flight success or physical safety of shifted entries.

The manifest also fixes:

- Completion identity `one_update_terminal_completion_v1`: at most one extra
  global update, exact final saved command, original planned-end hard cap.
- Counterfactual identity `terminal_entry_vertical_offset_counterfactual_v1`:
  at actual terminal entry before its due update, change only position.y by
  ordered offsets `0, -0.001, +0.001, -0.005, +0.005, -0.010, +0.010,
  -0.020, +0.020 m`, preserving the full original global clock/state otherwise.

Do not edit the sealed manifest, contract or protocol during implementation or
measurement. Pin their final SHA-256 values in preflight; the protocol's own
hash and final contract hash are recorded in the design closure in
[progress](progress.md). Include all three files in the runtime source closure.

## Regression references and immutable archives

The first 28 cases are the ordered cases in the existing final operational
Run A summary, schema `nominal_direct_operational_gate_v1` / 1:

- `outputs/research/nominal_direct_operational_execution_20260928/run_a_schema_v1/summary.json`;
  SHA-256 `8a294f74dfb18c3ba9448b2f438bee9fdf72ca8b56edaeba09f1e256d4f0683f`;
  passed identity `fnv1a64:ac75e5bbc3d2a16d`.
- The independent 24-control strict nominal reference is
  `outputs/research/nominal_direct_operational_execution_20260928/legacy_gate_schema_v1/summary.json`;
  SHA-256 `4fc857520048719f43e19b967864b90f1154215527ac32e5d72b4ce17a9c70a1`;
  passed identity `fnv1a64:73429341077b7008`.
- The historical 216-row contact-phase reference is
  `outputs/research/nominal_direct_contact_phase_20260928/run_a/summary.json`;
  SHA-256 `b785e3f2ca12729ac6c9d7de08ae79eaf695f7dd7a0d896437639788c1a9c898`;
  passed identity `fnv1a64:7195a7ab25d24515`.

Validate all used request/scenario/policy/witness/program/comparison files and
ordered case bindings before flight. No stored program seeds generation.
Regenerate under current source from physical inputs and require exact full
generation/program/nominal-contact/fuel/action-replay parity for all 28 exposed
controls. Preserve the independent strict nominal 24-case gate. Compare physical
evidence rather than demanding old wrapper/controller/source metadata equals
new versioned evidence. Relocation is allowed only for byte-identical used inputs.

Design readiness captured 128 existing bound files with source identity
`dfe4bee566ececb3ca35caddcdf4111d388d806c72ca900b3226eeea82c0956f`
and every file's bytes matched the committed runtime closure. It also captured
all 4,697 existing regular files across these roots, not just the earlier
1,395-file inventory that excluded the then-new operational archive:

| Historical root under `outputs/research/` | Files | Ordered inventory SHA-256 |
| --- | ---: | --- |
| `nominal_direct_flight_integration_20260928` | 843 | `83126648cc7399578f2c5b2287c2896596271d12e3b9debdbd1b821b55a0e0d0` |
| `waypoint_direct_body_aware_terminal_20260928` | 68 | `c021e2afab4aadf8531df825522116a03771b5701cd8c072f6e4eaca68860da1` |
| `nominal_direct_contact_phase_20260928` | 484 | `e3ae9a5a0fc151c40931eff919eb1e5801feb6429215d41958719e91e3a0e01e` |
| `nominal_direct_operational_execution_20260928` | 3,302 | `78faa5789e88a8946cca0bcc7202553c7c36a258c3b9a87ed6006b1ed38e47d4` |

Inventory digests hash compact JSON arrays of `{path,sha256}`, sorted by
root-relative path. Reject nonregular files rather than silently omit them.
During implementation, source hashes necessarily change; bind the final new
runtime closure instead of claiming it equals the old 128-file closure. Existing
archives and used physical inputs must retain exact inventories/bytes. Extend
the new closure over workspace Rust/Cargo files and every behavior-bearing
input/policy/protocol, including the three new sealed files.

## Ordered implementation and measurement gates

### Gate 1: artifact-free mechanics and admission

Share the ordinary bounded loop with a narrow aligned segment start; preserve
all old APIs/artifacts. Add separate completion-wrapper playback/replay. Derive
complete `C/K/I/H/A/R` and expected actions before motion, never from the log.
Bind a fixed completion guard identity `body_aware_terminal_completion_validity_v1`.
Full source-rest initial support and shifted segment-entry lineage/clearance
are different stages; permissive or caller-chosen guards cannot earn proof.

Capture the actual ephemeral `SimulationState` at E through a fixed evaluator
guard's read-only post-transition clone after successful prefix validation.
Stop the prefix before callback E. The shared segment API takes this in-memory
state, not a deserialized snapshot; snapshot conversion remains query-only.
Derive global ordinal E/I and validate absolute tick/time, never reset local
time/index. Shifted-entry clearance explicitly uses `terminal_bridge` while the
nominal prefix's E post-step scan retains existing `ballistic_coast` accounting.
Replay's ordinal-N reserve action checks absolute tick/time and both command
fields bitwise against the derived final-saved-command update.
Persist original saved K separately from the lane's supplied authorized A,
hard H and effective R. Independently derive saved/authorized/hard/effective
reached flags from the retained final valid tick, not the stop enum or action
count; core coverage flags continue to mean A. Separate issued reserve actions
from physics steps advanced under them. The capture-purpose prefix pause uses
coverage E and original hard H; its temporary pause is not exhaustion of K.

Artifact-free fixtures cover:

- Global ordinal/clock/held-command/fuel preservation; first segment update at E;
  exact recombined zero-offset actions/events/samples and inherited state metrics;
  both on-sample and off-sample E without duplicate or fabricated seam samples.
- Evidence-only snapshot rejection at the admitting boundary; fresh ephemeral
  prefix capture/lineage; shifted-entry terminal phase versus nominal E coast
  phase; pre-intervention sample labeling and no ordinary-trace claim for a jump.
- Contact before/on K, at K+1 and K+I, odd C with rounded K, odd H clipping,
  A/H/horizon ties and authoritative contact before another callback/driver stop.
- Saved-versus-authorized boundary flags for contact/post-transition rejection
  at K, numeric rollback to K-1, odd H between K/A, and a reserve action at K
  followed by pre-transition rejection with zero reserve physics steps.
- No reserve when K>=H; exactly one update when allowed; no transition past R,
  second reserve update, clock reset, appended V1 command or nominal-Match claim
  after nonzero intervention/reserve use.
- Initial/zero-action and command/pre-/post-transition failures; sticky guard
  rejection, actual incoming contact before normalization, airborne/contact
  clearance separation, finite prefix and first bad numeric boundary.
- Legal zero-fuel idle coast, underfunded powered transition rejection and valid
  exact-last-fuel contact. No invented core fuel-exhaustion termination.
- Rehashed program/policy/tail/start/lineage tampering, different delta/clock,
  truncated/extra/off-clock/altered/nonfinite actions, wrong global ordinal,
  forged smaller limits, unused recorded suffix and independent replay failure.
- Unknown-field rejection, typed full-mission versus counterfactual/segment
  evidence, input-only preflight, create-only roots and preserved failure bundles.

### Gate 2: all nominal baselines before offsets

Run the independent 24-control nominal gate. In each new integration run,
regenerate all 28 exposed requests and the four new requests in frozen order.
Independently admit selected witnesses/programs and run full nominal strict
saved-coverage and reserve modes; prove original nominal checks, ordinary replay,
entry/prefix capture and deterministic physical parity. Reserve use must be zero.

All 28 exposed baselines must retain their complete old physical/generation
results. Fresh coverage targets 4/4 Direct/completed-safe/Match. Record all four
decisions, including Unknown/Unsupported/Invalid, without replacement or tuning.
Any baseline mismatch stops the offset stage; a fresh non-Direct decision is
explicitly an unmet generator-coverage/readiness gate, not a contact/control
failure or waypoint-necessity proof. Retain not-executed statuses for gated rows;
do not claim a complete counterfactual matrix if it was not run.

### Gate 3: complete fixed two-lane counterfactual matrix

Only after Gate 2 passes for all 32 missions, reconstruct their actual nominal
source/coast prefixes and compare the two lanes at every fixed offset:

1. Strict saved coverage: original K/H, no update at K.
2. One-update completion: complete admitted wrapper A/H, at most one reserve
   update at K and none at K+I.

Each lane independently validates shifted entry and every authorized transition
through the shared engine and fresh fixed guard, then replays from freshly
reconstructed lineage and intervention. No independent physics integrator,
hidden fault, reference fit, phase-clock reset or ordinary action-only replay
substitute. Branch evidence always retains intervention and execution kind.

There are 252 exposed and 36 fresh case/offset combinations per lane: 288 per
lane, 576 lane observations per run, 1,152 across two independent complete runs.
The 28+4 full nominal baselines are separately counted, not invented additional
counterfactuals. No new velocity/attitude/fuel/combined/random/obstacle axes.

Require all zero-offset branches to reproduce the nominal physical evidence.
Compare the original 216 exposed case/offsets against both historical lanes;
the original 48 late rows must reproduce the diagnostic safe contact/tick/fuel
under at most one newly authorized update. For all 32 missions, the reserve
coverage target is valid safe target contact at every prescribed offset before
or at R, with no guard failure and independent counterfactual replay parity.
Nonzero offsets remain nominal Deviation. Record the strict lane's actual safe/
coverage/deadline results separately; do not predict or force its new-case counts.

### Gate 4: provenance, repeats and final validation

Use new create-only Run A/Run B roots. Persist physical requests/policies,
complete generation/selection/admission, original program/completion contract,
proved prefix, intervention/start, lane actions/events/samples, incoming contact,
final snapshots/bounds/stops/guard/replay, classification and failures. Reports
must label source-rest nominal flights versus counterfactual segments and cannot
reuse an ordinary whole-flight trace label for omitted/intervened prefixes.

Retain all used hashes and verify source/input/protocol closure before/during/
after both measurements and complete historical archive inventory afterward.
Compare every deterministic file/field across repeats, including HTML and
segment traces, with only explicitly enumerated observational compute timings
excluded. Counters, actions, contact/fuel evidence and identities are not timings.
Record local generation/admission/execution/replay/writing cost without claiming
an optimized backend or production budget.

Run focused tests, one artifact-free full workspace gate with Git metadata,
strict all-target workspace Clippy, formatting and diff checks on final source.
Repeat a full suite only after a relevant correction or unresolved risk. Keep
failed and earlier successful roots intact; final acceptance binds final source,
not a subsequently edited tree or earlier closure.

## First decisive exit

Close design when the contract/seam is reviewed, four inputs are sealed and
input-only supported, documentation/fixture checks pass and all old runtime/
archive pins remain unchanged. No full runtime suite or measured gate is repeated
for documentation-only design changes.

The separately authorized implementation closes only if all declared nominal,
finite counterfactual, mechanics/replay and provenance gates pass. Fix ordinary
bounded implementation defects without erasing failure evidence. A physical,
coverage or architectural miss stops with its exact earliest cause: no second
update, changed deadline/contact threshold, source fitting, replacement input,
feedback controller, obstacle sweep or general framework to rescue the gate.
Success supports one opt-in finite completion policy, not continuous or
full-flight robustness, swept-path safety, arbitrary waypoint-state composition,
default/direct-first planner integration, real-time budget or physical deployment.
