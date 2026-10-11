# Native procedural input audit

[Mission workflow](procedural_missions.md) · [Collections](procedural_collections_v2.md) ·
[Pilot declaration](../studies/mission_coverage/pilot_v2/README.md) ·
[Execution record](procedural_challenge_execution.md)

`pd-missions/input-coverage-v1` inspects sealed V2 inputs without an agent,
flight, reward or outcome. It authenticates every resolved proposal, including
exclusions, and produces separate proposed/admitted distributions. Descriptors,
bins, preview selection and QA findings never feed back into admission.

## Population statistics

`InputAuditSpecV1` binds the coverage design, reference-QA schedule and preview
budget. Axes have ordered finite boundaries with explicit underflow, overflow and
missing cells. Bins are left-closed/right-open; equality enters the bin starting
at the boundary. Every resolved input contributes to each marginal/joint count,
including a missing-obstacle cell. Correlations use only observed pairs and report
that denominator; constant or empty populations have no correlation coefficient.

Mission-level percentiles interpolate at `q * (n - 1)` over sorted observations.
These are equal-mission summaries, separate from the geometry contract's
horizontal-length-weighted slope quantiles. Prepared/raw relief and slope summaries,
slopes outside pad edits, edited transfer fraction, crest changes, normalized
obstacle placement and all four endpoint surroundings remain visible. Recorded
neighborhood lengths expose clipped context. Marginal bins retain conditional
left/right counts; morphology-group directions are also recorded.

Morphology groups are mutually exclusive display groups, including mixed and
unclassified. Individual labels and qualifying challenge paths overlap; their
counts cannot be summed into a population denominator. Exact physical duplicate
selection remains the collection's responsibility.

`pd-missions/shape-bucket-v1` provides a coarse concentration diagnostic. Its
signature uses distance and rise rounded to 100 m, both widths to 8 m, significant
crest count, and thirteen equally spaced source-relative chord residuals rounded
to 50 m. It is invariant to the tested coordinate reflection/translation. Shared
buckets indicate descriptor overlap, not equal physical inputs or guaranteed
near neighbors. Quantization boundaries, narrow unsampled features and demands
outside these descriptors remain blind spots. The largest twenty shared groups
retain up to twelve example indices each.

## Resolution and preview sampling

The declared regular QA schedule selects paired offsets 0/1 in each 80-index
block. Add the closest boundary candidates not already selected. Boundary score
is the minimum `abs(measured / threshold - 1)` across raw/prepared sustained width,
absolute rise, retained crest height/prominence/half-width and adjacent spacing.
Break ties by index. This proximity score is not a difficulty or feasibility score.

Reference QA explicitly regenerates the recorded noise/controls at 1 m, remeasures
and rounds both reference pad elevations, applies the same bounded local pad
preparation, then remeasures raw/prepared geometry. Receipts bind original physical
identity, coarse/reference terrain identities, reference pad heights, maximum
sampled raw interpolation error, barrier/width differences and admission changes.
The reference is a counterfactual geometry comparison; the plant retains its
sealed 4 m polyline. Sample agreement does not prove a continuous-terrain bound.
Diagnostic flags remain separate from qualification. Tiny interpolation arithmetic
differences are retained; identical-resolution tests use the metric's 1e-8 tolerance.

Preview candidates use marginal extremes/10th/50th/90th percentiles, occupied joint
cells, morphology groups and admission boundaries. Use one outer queue per axis,
joint, group and boundary ordering; cells take turns within their joint queue.
Each outer round takes one unseen input per nonempty queue. Numeric rank/ties and
index order are deterministic; the declared budget caps the union. Selection
changes neither mission membership nor full population counts.

## Integrity, bounded work and frontdoors

The audit loads every resolved proposal through the strict native collection
reader and recomputes its geometry/admission/projection receipts. Initial manifest
and request bytes plus every declared proposal digest are authenticated again
at completion. An excluded-input mutation fails the audit. The function itself
is read-only; callers own exposure receipts and create-only artifacts.

Limits include 20,000 scheduled indices, ten diagnostic axes, twenty joint pairs,
1,024 reference inputs, 192 preview selections, 64 MiB compact row data and
32 MiB compact reference receipts. CLI audit artifacts have a 128 MiB limit.
Excess work/size returns an error; inputs are never silently dropped. The CLI
reserves its output before inspection and leaves an incomplete artifact on failure.

```sh
rtk proxy cargo run --release -p pd-missions --example missions -- check-v2 --request /path/to/request.json --audit-spec /path/to/audit-spec.json
rtk proxy cargo run --release -p pd-missions --example missions -- audit-v2 --collection outputs/my-v2-collection --spec /path/to/audit-spec.json --output outputs/my-input-audit.json --usage-id input-audit-001
rtk proxy python3 -B studies/mission_coverage/run_pilot_v2.py --plan
rtk proxy python3 -B studies/mission_coverage/run_pilot_v2.py --output outputs/procedural-challenge/my-new-pilot
```

`check-v2` validates without generation. `audit-v2` records input-inspection exposure
and writes a new artifact. The pilot runner verifies committed declaration bytes,
builds the native tool, preflights both complete requests, then collects/audits each
declared stream. It binds source and executable identity, records separate counts
and cross-stream exact shared identities, and refuses existing/dangling destinations.
It cannot resume, create held-out inputs, train, fly, publish or manage a server.
`--plan` is a read-only declaration/file check; native preflight remains pending.

## Maintained local input report

The native composition example reads a completed pilot and passes neutral,
bounded presentation DTOs to `pd-report::input_coverage`. The report crate imports
no generator, collection selector or strategy. Its common full batch/status
templates retain the existing shell and palette; no accepted report edition or
home/library navigation is rewritten.

```sh
rtk proxy cargo run --release -p pd-missions --example input-report -- \
  --pilot outputs/procedural-challenge/COMPLETED-PILOT \
  --output outputs/procedural-challenge/NEW-LOCAL-REPORT
```

The destination must be fresh. The renderer authenticates completed-run/audit
bytes, recomputes aggregate projections from the saved compact rows, reconciles
them with the native selection ledger, and authenticates all immutable inputs
before and after rendering. It loads selected saved physical inputs through the
native reader; it does not regenerate noise or reference samples. `render.json`
binds the executable, pilot sources, collection files, generated pages/snapshots,
shared plot bounds and zero flight/training/publication allocation. A failure
leaves a partial new directory without a completed receipt.

The overview leads with measured coverage and full scheduled/admitted counts.
Marginal charts compare proposed/admitted counts on shared scales, retain missing
and overflow bins, and expose directions within bins. Joint matrices show exact
counts and observed-pair correlations. Regular and targeted reference QA retain
separate denominators and flagged input indices.

Review follows **collection → morphology → measured chord-barrier band → mission**.
Display bands are `<100`, `100–<200`, `200–<400`, `≥400` m above the pad chord,
plus missing. They are presentation thresholds, introduced after selection;
they never become V1 strata, admission rules or difficulty/solvability claims.
Mixed/unclassified groups remain visible. Physical direction is a filter.
Only one band materializes previews, twelve per page, with no initial image
requests. Full band counts remain separate from the selected preview inventory.
Search applies only to selected previews, not the entire saved collection.

Raw/prepared plots use shared source-relative metre bounds across the selected
inventory and reflect display toward the right. Horizontal/vertical pixel scales
differ and are disclosed. Pad edits and the metric's pad chord are marked;
no flight trajectory is fabricated. Details retain admission paths, plant/resource
context, full descriptors and exact saved resolved JSON. Return links reopen the
same band. Input views do not substitute for rich flight detail pages.

`scripts/check-input-coverage-browser.mjs` performs explicit desktop/mobile QA
against local files through an already available test browser. It checks initial
image absence, twelve-image pagination, URL reload, direction filtering, detail
and return navigation, visible assets, overflow and browser errors. It starts no
report server. Native projection/plot checks and the maintained Node model tests
cover count reconciliation, unsafe links, coordinate reflection and pagination.

The [calibration record](procedural_challenge_calibration.md) owns measured pilot
findings. Audit/report success establishes saved-input consistency and selected
distributions, not landings, complete coverage, policy generalization or a frozen
benchmark.
