# Procedural-terrain diagnostic missions

Date: 2026-10-07. Scope: turn observed failure patterns from the
[inspected 1k development population](terrain_validation_1k_results.md) into
small, reproducible capability tests. This is not a planner fix or another
coverage sweep.

## Selection and questions

Ten exact saved worlds, selected before new execution:

| Family | Failing worlds | Successful comparison | Question |
| --- | --- | --- | --- |
| Near-pad departure | `327`, `791` | `258` | Can a clearing maneuver gain forward progress while staying above the nearby rising terrain? |
| Late airborne acquisition | `024`, `516` | `271` | Can the next nominal be established from actual H, or is a less demanding H needed upstream? |
| Repeated short corrections | `030`, `280` | `253` | Do corrections gain useful route distance rather than consume the six-correction budget near the start? |
| Clear-direct preservation | none | `000` | Does an initially clear route still land without corrections? |

Numbers retain their original `random-NNN` identity; recipes and seeds remain
unchanged. The ten byte-identical scenarios will be checked into a separate
research fixture directory so using the pack does not require the 5.1 GiB survey.
The default 44-case pack and its acceptance contract remain untouched.

The departure failures have their first nominal conflict within the 42 m source
pad preparation region, admitted intervention entries and propagated queries,
but no supported forward-progress H. The comparison has a similarly near-pad
conflict and lands after three corrections. It is not an identical-geometry
counterfactual.

The two acquisition failures reach H near x=905 m at about 83 m/s forward speed;
all subsequent nominal candidates fail analytical admission, with zero physical
witness attempts. The comparison lands after a last H near x=1001 m at 64 m/s.
The failure does not prove physical impossibility, nor that changing the landing
controller would help. Record altitude above target, velocity, cheap braking
room, analytical reasons and actual witness count before choosing a fix.

The two progress failures consume six corrections by x=386/384 m; the comparison
also uses six but reaches x=763 m and lands. Record every H, successive x gains
and time to the next nominal conflict. Correction count alone is not the defect,
and raising the cap alone is not the proposed solution.

These three focused subfamilies do not exhaust the 267 failures. In particular,
the 106 `NoClearing` cases reaching progress but failing continuation admission
are a mixed, deferred bucket. The pack is intentionally selected development
data, not a held-out test or a population pass-rate estimate. It cannot predict
how many of the 267 failures a later change will solve.

## Build and review gates

1. Authenticate the saved 1k capture; copy exact scenario bytes and retain
   source capture/manifest/receipt identities, scenario hashes, baseline full
  flight hashes and complete-flight non-timing fingerprints.
2. Provide one small opt-in collector using existing native preflight, flight,
   common rich detail output and evidence helpers. No alternate simulator,
   synthetic trajectory, planner/controller edit or custom report template.
3. Extract compact family observations from the full saved records. Unit tests
   check selection, hashes, family predicates, full landing tuple, bounded
   attempts, exact repeat comparison, source binding and create-only outputs.
4. Review selection and invariants before freezing executable/source/inputs.

## Measured validation allowance

- Ten primary missions, then fixed repeats of `327`, `024`, `030`: **13 attempts**.
- Sequential execution; 120 s per case and 900 s collection wall bound.
- No tuning, seed replacement, automatic retry, rescue flight or budget growth.
- Input-only preflights and synthetic unit tests do not spend flight attempts.
- Stop on collector/integrity/replay errors, source/executable/protected report
  drift or unexpected baseline differences. Preserve partial evidence.
- All ten primaries must reproduce their entire baseline flight except the
  existing `planning_s`, `execution_s`, `replay_s` fields. Repeats must match
  their new primary by the same rule. No query diagnostics or proof fields are
  excluded. Current failures must remain honest finite stops, not fake landings.
- Four comparisons must retain the complete physical/mission/planning/integrity/
  original-source-replay landing tuple; the direct control must use zero H.
- Validate each executed scenario against the frozen fixture and native compact
  summary against its full flight. Retain rich `report.html` pages in the new
  capture, not a report-site publication or navigation change.

The closed capture will include frozen inputs/source, input preflights, manifest,
ordered attempt ledger, full native artifacts, compact observations and an exact
file receipt. Saved verification is read-only and runs no flight or fresh replay.
Run the ordinary maintained developer gate and terrain-study tests separately;
do not launch the optional 44-case numerical parity campaign in this allowance.

The default collector mode characterizes the exact baseline executable. An
explicit `--candidate` mode makes the same fixed missions useful for a later,
separately authorized behavior pass: it records baseline differences, permits
improvements to failure cases and requires comparison landings/direct preservation
and exact same-source repeats. It does not require a repaired case to keep its
old failure-family predicate. No candidate collection is part of this allowance.

## Decision after validation

Use the resulting missions to pick one capability-level change, not six per-seed
patches. Prefer a simple maneuver/transition improvement, with explicit limits
and preserved successful controls. Keep terrain-blind nominal selection, local
clearing and actual-H replanning. Do not require a complete landing suffix at
each local candidate. A later fix needs a paired comparison on development data
and a separately frozen, untouched test population; neither is authorized here.
No commit, push, selector update, report publication or server operation is part
of this pass.
