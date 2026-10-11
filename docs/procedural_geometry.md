# Neutral procedural geometry contract

[Mission workflow](procedural_missions.md) · [Challenge roadmap](procedural_challenge_plan.md) ·
[Implementation loop](procedural_challenge_execution.md)

`pd-missions/transfer-geometry-v1` measures resolved 2D heightfields without
generation, strategies, flights, rewards or outcomes. These descriptors support
coverage inspection; they are not admission rules, solvability certificates or
an exhaustive description of terrain demands.

## Physical projection and coordinates

`measure_mission_geometry` requires a two-pad target-landing mission whose
initial x is the source center and whose prepared terrain agrees with pad-center
elevations. A receipt binds the full native physical identity, the serialized
`GeometrySpecV1` identity, declared pad-edit intervals and measured descriptors.
`MissionGeometryReceiptV1::verify` recomputes that projection. It does not replay
commands or authenticate flight success. Source-rest physics and recipe/collection
admission remain independent validation responsibilities.

`measure_transfer_geometry` also accepts raw terrain. Chord anchors are actual
terrain heights at both pad centers, allowing raw/prepared comparisons without
pretending the raw surface contains flat shelves. Pad widths are provided
physical mission settings. A raw descriptor needs its own raw-input binding in
the V2 resolved record; it cannot replace a prepared physical receipt.

All reported obstacle/crest positions are horizontal distances from source
toward target, regardless of physical left/right direction. Fractions divide by
transfer distance. Pad-edit input intervals use physical x, are ordered and
nonoverlapping, and must lie within the terrain domain. Reflect/translate both
terrain and edits together. Equal geometric projections can have different
physical identities; display normalization does not transform a flight.

Clip exactly at pad centers and interpolate polyline endpoints. Relief is
max-minus-min height in that interval. Signed rise preserves uphill/downhill.
Chord residual subtracts the straight endpoint line; its maximum is a geometric
barrier, not obstruction of a particular ballistic or powered path.

## Barrier, slope and endpoint descriptors

For each declared positive chord-relative height, retain every connected
superlevel interval with positive horizontal length, interpolating crossings.
Equality is included; threshold plateaus have their actual width, isolated
threshold points have zero width and are omitted. Report longest and total
widths, first departure distance and last approach distance. No obstacle gives
`null` proximity, not zero. Preserve all intervals so scalar summaries do not
erase ordering or gaps.

Absolute slope quantiles weight horizontal segment length. Pad-edit exclusion
splits segments at exact edit boundaries before measuring eligible portions;
it does not merely discard vertices or adjacent edges. Report both overall and
outside-edit slope summaries. If edits cover the entire interval, the latter is
`null`. Callers must declare the actual edit intervals; an empty declaration
means no exclusion, not inferred pad preparation.

Measure a declared neighborhood on both sides of both pads, preserving physical
rise/fall relative to each pad, slopes and actual sampled extent. Domain edges
truncate neighborhoods visibly. They do not silently extend/clamp terrain to
the requested radius. Neighborhoods can overlap for short transfers.

## Significant crests

Identify interior local maxima on the chord-residual polyline. Endpoint maxima
are excluded. A flat maximum is one crest at its horizontal midpoint. For each
crest, find the nearest strictly higher vertex on each side, or the clipped
boundary if none exists. Find the lowest residual between the crest and each
such bound; prominence is crest height minus the higher of these two minima.
Equal-height neighboring peaks do not terminate this search.

Measure the connected width around the crest at half prominence, with exact
crossings. Filter by declared minimum prominence and width. Reject crest support
that overlaps a pad-edit interval, so edited shelf/transition geometry cannot
qualify a crest through this descriptor. That conservative exclusion can omit a
real broad feature near a pad; endpoint and barrier measurements remain visible.

Apply minimum separation greedily in descending prominence, then descending
height, then ascending source-relative position. Return accepted crests in
source-to-target order, retaining prominence, support and positions; report
minimum/maximum adjacent spacing or `null` for fewer than two crests. These
definitions recognize selected shapes, not all terrain complexity. Geometry
labels and challenge membership remain separate consumers.

## Numerical and work limits

The version uses 1e-8 absolute geometry tolerance for plateau/higher comparisons,
interval adjacency, positive interval lengths and threshold/separation checks.
Slope cumulative-weight comparisons use the same absolute length tolerance.
The specification identity includes thresholds, crest limits and radius; changing
the algorithm or tolerance requires a new metric version.

Validate 2–100,001 finite ordered terrain points, coordinates within +/-
10,000,000 m and input vertex spacing at least 1e-6 m. Centers must be distinct
and inside the domain; do not extrapolate the transfer. Accept at most sixteen
barrier levels and sixteen nonoverlapping edit intervals. Positive configured
scales are bounded to 1e-6–100,000. These are computational limits, not a newly
supported flight envelope. Crest queries use range minimum/maximum trees;
overall work is O(n log n) and memory O(n), including equal peaks/long plateaus.

Analytical tests cover triangles, zero-barrier elevation changes, separated and
subsidiary ridges, threshold plateaus, proximity counterexamples, weighted
slopes, exact edit clipping, symmetry/subdivision, receipt drift and invalid
inputs. A 20,001-point alternating terrain preserves all 10,000 qualifying peaks.
Ordinary tests need no retained outputs or controller captures.
