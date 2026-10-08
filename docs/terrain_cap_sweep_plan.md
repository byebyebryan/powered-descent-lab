# Paired 1,000-world relaxed-cap sweep

Date: 2026-10-07. User requested the original 1000 worlds again without a binding
hop limit, or with a very relaxed one, after the
[two-world cap diagnostic](terrain_cap_probe_results.md).
Completed by the [paired results](terrain_cap_sweep_results.md): 748/1000
verified landings, no cap-24 stops, all 1010 attempts verified. Allowance closed.

Use **24 corrections**, four times the ordinary cap, as a finite diagnostic
ceiling. Keep the original fuel, absolute planner deadline, terrain, pads,
vehicle, clocks, nominal construction, maneuvers, ranking and safety guards.
If 24 still binds, report that explicitly rather than calling it unlimited or
automatically extending the run. Production remains cap 6.

## Fixed execution and comparisons

- Reuse all 1000 exact saved scenario bytes and original IDs/seeds/order. Do not
  generate a new population or filter finite misses. Authenticate the complete
  original 1k capture before collection.
- Three original successful controls precede the population. Repeat the five
  original cases plus `030/280`, whose continuation was just inspected:
  **1010 measured attempts**, four workers, no retry or tuning.
- Case bound 300 s, campaign bound three hours; offline build bound 900 s.
  Native input-only preflight and pure tests are not measured flights.
- Reuse the isolated cap-probe build helper. Its generated source differs only
  in revision-3 policy identity and cap. Preserve the ordinary binary, tracked
  Rust, accepted selector/report hashes and all historical captures.
- Every non-cap-stopped baseline must reproduce the complete flight except
  three wall timings and the explicitly different root policy/input identities.
  Every old cap stop must reproduce the complete executed prefix through H6,
  actual H6 state, original deadline and next nominal construction/audit.
- Verify complete physical/mission/integrity/replay tuples; repeats must be
  exact except three wall timings. Stop and retain partial evidence on source,
  collector, comparison or proof error, control regression or unexpected
  physical stop. Ordinary finite planning misses continue the sweep.
- Retain all native rich detail reports. No report-site publication, navigation
  changes, server operation, commit or push is authorized.

The [contract](../studies/terrain_profiles/cap_sweep_plan.json) and
[collector](../studies/terrain_profiles/cap_sweep.py) bind preparation, source,
isolated binary, ordered attempts, exact input bytes and comparisons. Saved
verification is read-only. Original flights remain in their authenticated
baseline capture, a declared dependency of the paired verifier; they are not
rewritten, relabeled or unnecessarily copied again.

## Closure

Report paired overall/blocked landings, preserved successes, per-recipe counts,
outcomes of all 23 old cap stops, actual maximum corrections, any remaining
cap-bound cases and observed cost. Keep controls/repeats outside the denominator.
Verify saved evidence independently and check rich waypoint annotations. Run
the ordinary developer gate and terrain-study tests before source freeze; check
closure docs separately. Do not relax production acceptance to validate probes.

This isolates cap sensitivity on inspected development data, not fresh held-out
generalization. It cannot fix earlier `NoNominal`/`NoClearing` stops, and does not
authorize promoting cap 24 merely because the sweep completes.
