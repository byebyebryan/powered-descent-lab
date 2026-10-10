# Frozen exit-consistency 1k diagnostic

[Documentation home](README.md) · [V14 panel result](ballistic_mechanics_results.md)

Status: closed after all 1,005 native records and independent verification.
See the [706/1000 result](ballistic_exit_diagnostic_results.md) and
[next-pass design](ballistic_correction_ownership_design.md). The capture retains
the original pre-measurement protocol bytes; this status note is not a rewrite
of its frozen plan or the earlier failed admission.

This is a separately user-authorized investigation, not reopening the failed
V14 combination/sweep gate. Run only the unchanged `exit-consistency` candidate;
no recovery-lead, early-piecewise or combined flights are included.

## Question and fixed candidate

Does the incoming/exit consistency check improve broad paired coverage, and
where do its changed waypoint states help or hurt? The local panel gains three
landings and loses two (26/45 versus 25); that is neither acceptance nor a
population estimate. Zero local regressions are not an investigation prerequisite.

Preserve the native binary, Rust tree, archived common batch renderer and report
script from the reader-verified exit panel:

- Native: `06b1d487cb6b02be4182b4a97fba8ad769775bf971d796b9717038aea5a7ca90`.
- Rust: `7c4dacf7e87887e54ce9d0f407a0fbcb536330de616054387ff271cb5a84db97`.
- Renderer: `e8ff05d3b61310f2f0cfaf8413263b945d3b629050d3f9a9288b8516d74af0c0`.
- Script: `9c7129d5d9aa5b4efa06e82c6ced7c9a72616be2474d859adc030302d63ec37d`.
- Panel receipt: `f322767d5adf3701badcbae8af4a940d3e8b086c3636f5c26199d3d2c6190f83`.
- Panel results: `2e6a893d438a3d6268270b9dbb19ee657da9d8900fdf919e40c5c78c6f63af94`.

Collector-only registration/provenance work is permitted before measurement.
No Rust edit, rebuild, fixture change or flight-driven tuning is permitted.

## Population and accounting

Use the complete V13 capture
`capture-terminal-coordination-diagnostic-1k-20261009-v1`, with receipt
`49a9dd87d8df9725dd594c9979470d8984446aac31e3aecfec483db64910ccfe`
and results `7f66120ba2851da54c490937591c4fdc3e9ff370738cb6216dc3d78f87fc6121`.
Copy exact scenario bytes, seeds, recipes, geometry and comparison records.
The original policy-3/cap-24 labels and 817/1000 stay a separate older comparison.

Run 1,000 primary worlds, 250 per original recipe, followed by five external
same-world repeats: 047/142/715/044/020. Total ceiling **1,005 native attempts**,
four workers, 60-second case ceiling and two-hour collection ceiling. No retries
or replacements. Repeats are outside the denominator; each native record also
owns source-command replay and deterministic decision reproduction.

Require all 45 overlapping primary feedback records to reproduce the complete
exit-panel bytes. Keep four zero-H controls (001/264/503/753) exact against V13.
Retain missing/unverified attempts honestly; do not call collection errors
crashes or exclude them from the declared denominator. Finite stops and landing
regressions do not stop this diagnostic. Source/protected-state drift or invalid
proofs invalidate claims and require explicit investigation, not silent retries.

## Reading results and design work

Report landings, paired gains/losses, recipe counts, phase/stop migrations and
changed-flight footprint against V13. Inspect whether repaired waypoint entries
leave useful correction room, rather than treating a removed stop label as a win.
Keep ideal arc, cutoff estimate, actual H, coast and terminal ownership distinct.

During the unchanged sweep, review existing traces/code for a simple next pass:
optional replans as transactions; correction retention and planning cadence;
whether least-effort waypoint fits preserve excessive horizontal energy; and
recovery choices checked over unequal horizons. Produce a design and validation
plan, not another flight candidate in this allowance. Do not require a complete
landing suffix or introduce broad optimization/search-budget growth.

The common archived batch renderer preserves rich plots and failure-first trees.
It has inherited prose saying two repeats and a full candidate change; label
these limitations in the results instead of rebuilding or rewriting sealed pages.
The authoritative plan/manifest/results contain five repeats and this isolated
exit check. No root-navigation publication, server operation, default promotion,
accepted-site writes, commit, push or delegation is authorized.

## Verification

Before flights: reader/collector tests, docs/whitespace checks and authentication
of the complete existing panel and frozen source/binaries. No old flight gate is
weakened or reclassified. Afterward: independent saved verification of every
record, source/input bindings, repeats, overlap and controls; static rich-report
payload/link checks; unchanged protected evidence; and current developer checks.

Run or read-only verify:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_exit_diagnostic.py run
rtk proxy python3 -B studies/terrain_profiles/ballistic_exit_diagnostic.py verify
```
