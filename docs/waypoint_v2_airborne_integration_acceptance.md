# Waypoint V2 airborne integration acceptance review

## Decision

The [bounded implementation](waypoint_v2_airborne_integration_results.md)
passes the [approved plan](waypoint_v2_airborne_integration_plan.md)'s measured
usability gate on the final corrected source:
8/8 direct uncut controls, 16/16 ordinary terrain missions, every family 4/4,
three corrections on the reference plateau, intact safety/replay and latency
within the declared limits in both repeated runs. Accept the first usable
supported opt-in V2 checkpoint; the implementation goal is achieved.

Luna's late read-only review found an unexplained endpoint mismatch could be a
finite rejection instead of an integrity error, plus a defensive witness status
fallthrough. The primary corrected both without numerical/selection changes.
The user explicitly approved three additional corrected-source checks after the
original allowance was used: one policy 2 preservation matrix and two policy 3
repeats, with unchanged inputs and no tuning. They pass, as do nine focused tests,
the fresh 39-row adapter gate and 915 workspace tests. Luna's follow-up review
finds no remaining correctness defect. Earlier roots are retained as preliminary
evidence, not substituted for this corrected-build validation.

This is the primary's separate acceptance review, not default promotion or an
independent-agent certification. Luna also performs a bounded read-only source
review; architectural decisions and acceptance remain with the primary.

## Contract review

The live facade accepts actual state and original deadline, not a saved answer,
restored snapshot or research runner. Shared seed, acquisition, timing and rank
semantics are unchanged. First executable selection precedes terrain audit;
only finite execution rejection enables another of the three shortlisted trials.
Nonfinite, binding, command-coverage and replay failures stop with integrity
failure. No old 56-trial fallback or terrain-driven higher-arc retry is added.

The complete typed program binds dynamics, origin, deadline, raw endpoint,
commands and acquisition/coast/terminal boundaries. Held-pair and phase checks
reject gaps, unsupported source phases, boundary mismatches, invalid commands
and endpoint dynamics before audit propagation. Post-step phase ownership uses
the consumed command; odd contact endpoints keep their original clock.
Acquisition/coast retain full reserve, and only existing descending target-pad
terminal behavior can use its corridor. Airborne source exemption is impossible.

The outer loop uses unchanged local ranking, progress and continuation checks
without a landing suffix. It executes only accepted prefixes and clearing to
actual H; certificates and rejected unsafe suffixes are not executed. Complete
and partial supported flights retain full source/official replay. No velocity,
time, fuel, attitude or outcome reset is used.

Policy 3 explicitly opts into airborne replacement while inheriting policy 2
launch/intervention behavior. Policies 1/2 and default 1 remain intact. All thirty
supported initial cycles equal historical policy 2, and its entire final-source
matrix preserves all sixty-four per-case payloads under existing exclusions.
The research wrapper likewise preserves all fifty-five rows and control reports.

## Evidence assessment

The bound 39-row adapter gate is an exact retained-program comparison, not a
complete mission proof. It also verifies four designated recoveries and the one
finite fallthrough. The subsequent complete frozen matrix supplies the missing
loop evidence: four former ordinary misses now land, including new actual
handoffs and repeated clearing. No ordinary failure or diagnostic is excluded.

One development matrix, the original final pair and the user-approved corrected
final pair pass the unchanged complete gate. Corrected-source roots are
`final_hardened_policy_3_a` and `final_hardened_policy_3_b` beneath the results'
evidence base; preservation is `preservation_hardened_policy_2`.
Normal final-root comparison matches all 64 summary/flight payloads, with equal
source/binary/runner provenance. Median planning is 0.382719/0.383171 s and p95
0.608003/0.614668 s over all 24 clear/ordinary attempts. Timing observations are
not deterministic identities or a comparative performance guarantee.

Corrected-source native validation passes 915 workspace tests with four ignored,
nine focused runtime tests and explicitly rerun retained-prefix adapter validation.
The fresh 254-bound-file research reproduction preserves all 55 rows after
excluding only refreshed bindings and artifact identity. Policy/parser,
corpus/tamper, fake-suite, structural, format and whitespace gates pass.
The sole unchanged strict-Clippy baseline exception remains disclosed; the
narrow allowance passes with all other warnings denied. Historical policy 2's
suite exit 1 is its preserved coverage failure, not a failed preservation check.

## Limits and closure

Four difficult diagnostics still stop NoClearing while flying, and two unsupported
setups reject without simulation. These finite misses remain honest recorded
limits. They do not block the predeclared game-oriented checkpoint and do not
prove physical impossibility. Selected local certificates retain at least the
5 m reserve; their observed minimum of 6.163038 m does not establish robustness
to arbitrary perturbations or swept geometry.

Ground unification, reverse/overshoot/arbitrary powered recovery and wider
vehicle/gravity support remain deferred. No policy 4, numerical tuning,
GUI/controller/default promotion, commit, push or deployment is approved here.
The explicitly approved final guard revalidation is complete; no additional
matrix or diagnostic research is required to close this checkpoint. The realistic
next step is a requested review/commit, followed by a separate adoption decision
if GUI/controller integration is wanted. Any later adoption work needs its own
scope and validation; do not infer permission to promote the default.
