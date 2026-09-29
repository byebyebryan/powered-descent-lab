# Nominal direct first-contact and command-coverage results

The fixed height-offset study is complete and reproducible. It finds a
command-coverage / completion-contract boundary, not a physical contact failure
in this matrix. Saved programs reach safe target contact in 168 of 216 rows;
48 rows stop before the next missing command. The explicitly diagnostic
final-command hold reaches safe target contact in all 216 rows, within the original
planned end. Nothing here promotes that hold to an accepted flight policy.

The [protocol](nominal_direct_contact_phase_protocol.md) owns this pass. The
[nominal flight integration](nominal_direct_flight_integration_results.md) and
its exact expected-contact proof remain unchanged. These 24 selected programs
are exposed regression controls, not new held-out cases.

## Fixed experiment and baseline gate

Use the committed checkpoint `da0f78993fd65085b69384a52345249b6feb9d56`
and its pinned `gate_a` artifacts. No generation, source fitting, policy tuning,
terrain change or added disturbance axis occurs. At each actual live terminal
entry, clone the full state and change only vertical position by
`0, -1, +1, -5, +5, -10, +10, -20, +20 mm`.

All 24 zero-offset baselines pass before any nonzero row in each run: actual
entry, source/coast contact-free prefix, complete clearance scan, saved command
payload, incoming first-contact audit, contact tick, fuel, ordinary/neutral
parity, archived ordinary flight artifacts and independent action replay.
Global 120 Hz physics / held 60 Hz commands and all core contact rules are
unchanged. Incoming contact evidence is retained before ordinary touchdown
zeroes velocity. The shifted entry pose is checked separately without stepping.

There are 216 case/offset rows per run and 432 observations across two complete
runs. Each row has two evidence lanes; neither a shifted entry state nor its
diagnostic continuation is a new accepted complete-flight witness.

## Physical contact versus finite command coverage

Counts below are per run. Both runs agree exactly.

| Terminal-entry offset | Rows | Saved-program lane | Diagnostic continuation | Contact tick relative to nominal |
| --- | ---: | --- | --- | ---: |
| 0, -1, +1, -5, +5 mm | 120 | 120 safe target contacts | 120 safe target contacts; no continuation | 0 |
| -10 mm | 24 | 24 safe target contacts | Same; no continuation | -1 |
| -20 mm | 24 | 24 safe target contacts | Same; no continuation | -2 |
| +10 mm | 24 | 24 coverage-limited rows; no contact yet | 24 safe target contacts | +1 |
| +20 mm | 24 | 24 coverage-limited rows; no contact yet | 24 safe target contacts | +2 |
| Total | 216 | 168 safe contacts / 48 coverage-limited | 216 safe contacts | 120 exact / 48 earlier / 48 later |

There are zero unsafe/off-target contacts, pointwise clearance or strict
body-domain violations, fuel-exhaustion observations, or harness/parity failures in
either lane. Coverage-limited means simulation stopped before supplying a
missing globally due command; it is not a crash.

Each of the 48 later-contact rows uses exactly one additional global control
update, repeating the exact final saved command value. Actual contact occurs
one or two physics ticks later: 8.33 or 16.67 ms. The original planned end is
four ticks after nominal contact, so no row reaches or exceeds that cap and no
extra reserve is appended. The constant hold does not recompute the reference
or provide feedback recovery.

Incoming normal speed spans about 1.445–1.555 m/s; absolute angular rate spans
0–0.157 rad/s. All unchanged predicate mirrors pass. Hull-penetration predicate
margin spans 1.293–17.380 mm across diagnostic contacts. Its dynamic allowance
depends on incoming closing motion and angular rate, so the baseline 12.50 mm
allowance is not a constant for every shifted contact. These are discrete
contact-predicate margins, not obstacle clearance or an operating tolerance.

For example, flat-600 normally contacts at tick 3342 / 27.85 s. A +20 mm entry
offset is still airborne when saved coverage ends at tick 3342. The diagnostic
hold contacts safely at tick 3344 / 27.8667 s, before planned end 3346, with
incoming normal speed about 1.446 m/s. This is safe changed-timing contact, not
an exact nominal replay or an approved fallback.

## Reproducibility and provenance

Create-only evidence is retained at
`outputs/research/nominal_direct_contact_phase_20260928/run_a/` and `run_b/`.
Both roots have identity `fnv1a64:7195a7ab25d24515` and decision
`operational_completion_contract_next`. All 241 non-summary file pairs
(preflight, 24 baseline checks and 216 row files) are byte-identical. Root
summaries match after removing only the four observational compute fields.

Root summary SHA-256 values differ only because of those timings:

- A: `b785e3f2ca12729ac6c9d7de08ae79eaf695f7dd7a0d896437639788c1a9c898`.
- B: `11d597e7135d414e7f1fb3604cb637adf988a23212c3d43562cc40292846cc78`.

All 337 used input-file pins, all 120 current source/input/integration-protocol
pins and the separate study protocol are unchanged before and after both runs.
The current source binding is
`fcec14f5cbc9e60ab6f6874317ac68fb344a74937c70f80a85e455e93dac8849`;
study protocol SHA-256 is
`4c5944ec5ce9d209468858e9e30a538a14dad3a75ca5682478fecc1d28ea6d0e`.
All 911 files in the preceding nominal-flight and body-aware-terminal archives
retain their original bytes and inventory (843 + 68 files).

Measurement uses the reviewed local release binary, SHA-256
`0f34e28cc8a2e9d70f09854274bee183cbc7f47bc35d7d9609340137133a1400`.
Compilation and workspace tests finish before measurement. Whole-study wall
times are 2.763546 s and 2.764206 s, including parsing, hashing, baseline checks,
two-lane replay and artifact writing. They are observational diagnostic cost,
not generator setup cost or an optimized planner benchmark.

## Validation and unchanged authority

- Thirteen artifact-free study tests and one CLI mode test pass. Coverage
  includes entry preservation, zero-offset contact/tick/fuel repeat, safe
  changed-tick contact, genuine crash, missing coverage, exact bounded hold,
  shifted-entry clearance, global-clock/malformed/truncated input rejection,
  path/pin tampering, timing-independent identity and invalid-study decisions.
- The workspace gate passes 780 tests in twelve suites in a source-matching
  staged checkout with Git metadata, a separate target directory and no
  retained `outputs/`. One historical-artifact integration test remains
  explicitly ignored by default; this 24-case study is run separately.
- Strict all-target workspace Clippy, formatting and diff checks pass.
  All 121 bound source/protocol files match the artifact-free checkout.
- Release CLI preflight is ready for all 24 cases and creates no simulation,
  generator or output. Missing/conflicting output modes are rejected. Reusing
  `run_a` is refused before simulation or overwrite.

Only evaluator diagnostics, opt-in CLI wiring and documentation change.
`pd-core`, `pd-control`, `pd-plan`, generator policies, default behavior and
the strict nominal expected-contact verifier are unchanged. No commit, push,
deployment, new accepted program or robustness promotion is part of this pass.

## Verdict and smallest justified next step

Close this fixed matrix here. It does not justify another obstacle sweep,
source fit, cadence change, contact-threshold relaxation or terminal optimizer.
The small nominal penetration margin did not predict unsafe contact under these
vertical-only probes. The observed differences are expected-tick mismatches and
missing command coverage, not unsafe landings.

Next, design a separately scoped opt-in operational completion and
command-coverage contract. Keep exact nominal proof strict, distinguish actual safe
first contact from expected-tick equality, and define what execution may do when
saved commands end while still airborne, with explicit finite caps and failure
outcomes. The diagnostic hold is one observation to inform that design, not a
preselected production policy. Any implemented policy needs its own acceptance
gate and newly sealed evidence rather than tuning these exposed controls.

General disturbance recovery, attitude/velocity/combined errors, swept-path
safety, arbitrary incoming waypoint states, waypoint composition, useful
generator setup cost and production/default selection remain unproven. Neither
this finite study nor finite direct-family Unknown establishes waypoint demand.
