# Full random-terrain sweep with repair and retry

Date: 2026-10-07. This is the active continuation of the
[original frozen survey](random_terrain_survey_plan.md), after the
[comparison-aware stopped attempt](random_terrain_survey_comparison_results.md).

The owner explicitly authorized completing the full sweep, fixing issues and
retrying rather than stopping the goal at another routine harness defect.
This supersedes the earlier no-retry/stop-on-first-error execution policy. It
does not change the frozen terrain population or rewrite the original plan.

## Scope and evidence

- Keep all 100 seeds, raw profiles, 103 scenarios, vehicle, Earth gravity,
  120/60 Hz clocks, local pad patches and policy-3 strategy unchanged.
- Run three original preservation controls, all 100 random flights and the five
  predeclared repeats. Controls/repeats remain separate from the random denominator.
- Every attempt uses a new create-only capture with its source and executable
  frozen. Retain all previous attempts and reports; never overwrite a stopped run.
- A capture may pause at a repairable defect. Diagnose, fix, validate, build and
  prepare a new capture, then retry. A stopped capture is not completion of this goal.
- Ordinary physical crashes, finite planning stops and nonlandings are measured
  outcomes, not catastrophic failures or opportunities to tune the population.
- Stop finally only for an unrecoverable safety/nonfinite/determinism/evidence
  failure or an impasse requiring new authority. Investigate proof failures to
  distinguish a repairable implementation defect from untrustworthy physics.
- Keep four-process concurrency, bounded per-process/capture time, strict runtime
  collision/proof checks and the complete landing/integrity/replay tuple.
- No commits, pushes, server changes, accepted-benchmark promotion or tuning to
  improve observed outcomes is authorized by this pass.

## Known comparison dependency

The local-body extent numerical repair changes tiny derived clearance values.
Selected local proposals include these diagnostics in their content hashes;
their executed segments reference those hashes. Comparison V2 therefore:

1. Authenticates each source proposal with the existing typed native serializer
   and verifies its executed segment binding, without constructing a simulation.
2. Permits only the two previously named audit diagnostics and selected trajectory
   body-clearance samples within 1e-12 m, with unchanged required reserves and
   unchanged zero/sign/safety dispositions.
3. Compares all maneuver states, schedules, selections, policies, metadata,
   contacts, outcomes and proofs exactly; logs each diagnostic and derived-identity
   exception. Same-source repeats remain exact apart from three wall timings.

The native read-only comparator is shared by collection and report verification,
avoiding a second implementation of Rust floating-point hash serialization.
Absent/V1 contracts retain their historical meaning; the new contract is copied,
source-bound and explicitly versioned in each new input manifest.

## Execution and acceptance

1. Validate focused negative/positive comparison cases, Python study tests and
   the maintained development gate; build the release evaluator.
2. Snapshot and verify protected prior captures/report pages and accepted selector.
3. Prepare a fresh capture; byte-compare all raw/scenario inputs against the
   original before any flight.
4. Run controls, the complete random population and repeats. Preserve, diagnose
   and repair any routine failure, then repeat under a new frozen source as needed.
5. Require a complete final-source-consistent sample and repeat agreement. Account
   for failures and retries explicitly, never count an unverified record as a landing.
6. Verify inventories/projections/proofs, publish rich common detailed/batch
   reports and refresh navigation only. Check actual handoff annotations and links.
7. Report the full denominator, direct/corrected outcomes, nonlandings, retry history
   and practical limits; keep the accepted 44-case benchmark separate and untouched.
