# W4 physical/executor development comparison

## Status and claim

W4 is implemented and its already-seen development comparison is complete.
It compares two orthogonal questions for the exact selected route in each
existing D0 development row:

- did the frozen W3 input-only finite proposal find an exact physical witness;
- did the separately sealed R1 frozen executor complete that same rank-zero
  route/input pair?

This is a descriptive development result. It is not held-out evidence, a
robustness result, a negative physical certificate, or authorization to change
planner ranking, controller behavior, maintained fixtures, or D2 status.

## Outcome-isolated procedure

`pd-eval bounded-trajectory-physical` has no executor-artifact argument. It
resolves all `36` baseline and `24` diagnostic inputs, fixes the W3 search
budget to the complete `189`-attempt catalog, and seals rank-zero physical
predictions before any R1 root can enter the process. Search order within each
row remains serial and deterministic; independent rows may execute in
parallel.

The physical corpus is compact. Each row retains the validated input-only R1
preparation, W3 configuration, physical prediction, exact W3 result digest,
and all route/input joins, rather than copying every rejected command trace.
Loading it recomputes `RouteCapabilityInputV1`, reruns the complete W3 search
and every exact verification, and requires the prediction and result digest to
match.

Only after that physical root was sealed did the review run a fresh
`candidate-replay --all` corpus. `pd-eval physical-executor-comparison` first
exactly reloads the entire physical root, then opens and fully validates the R1
root and joins candidate rank zero on both axes. A selected physical result
cannot be joined to an alternative executor result. Source D0, row/corpus,
preparation, exposure, base resolved input, route plan, physical input,
witness/proposal configuration, R1 pairing, both source artifacts, unchanged
axis decisions/reasons, R1 diagnosis, status, and interpretation are all
digest-covered.

The three R1 selection-gap diagnoses remain descriptive fields in the W4
summary. Their supported alternatives are not substituted for the selected
executor axis.

## Canonical development result

The sealed physical lane used:

- source D0 input digest `fnv1a64:4f3b0eb97bce8209`;
- witness-configuration digest `e0e1de0892943ad6`;
- full-catalog W3 proposal-configuration digest `9ad52069df8609c3`;
- `60` complete rows, `0` supported and `60` unknown;
- all `60` reasons were `unknown/coverage/bounded_search_exhausted`; and
- physical summary digest `e25aea24b7a578c6`.

The fresh post-seal R1 run reproduced its authoritative summary digest
`3ea79b442c6caf2d` and pairing-configuration digest `321c0b7dbcc8cf82`.
The joined W4 result contains `60` complete and zero invalid comparisons:

| Physical prediction | Selected frozen executor | Count | Interpretation |
| --- | --- | ---: | --- |
| `unknown` | `supported` | 47 | W3 proposal incomplete; selected executor pair completes. |
| `unknown` | `unsupported` | 13 | Physical existence remains unresolved; executor failure is not infeasibility. |
| every other valid cell | — | 0 | Not observed in this development run. |

The comparison summary digest is `a007dc927399126c`. It retains the three
already-known `r+60`, seed-2 selection-gap rows for single-ridge empty/full
handoff and double-ridge full sequence. In each row the W4 executor axis is the
unsupported selected route, not the supported alternative.

## Interpretation and next gate

W4 validates the artifact boundary and shows that the deliberately small W3
backend has zero useful coverage on this development corpus. The result does
not make any of the 60 routes physically unsupported. It also does not
retroactively turn R1 executor success into a pre-run W3 proposal success.

W1 through W4 are now complete, but no integration-eligible physical proposal
exists and D2 remains blocked. The next action is a fresh design review for a
separately versioned proposal escalation—such as the previously deferred
sequential-convex direction—before adding a solver, changing reconstruction,
defining a new exposure registry, or touching held-out inputs.
