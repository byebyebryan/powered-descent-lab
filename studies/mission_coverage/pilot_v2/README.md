# V2 input-only calibration declaration

[Implementation record](../../../docs/procedural_challenge_execution.md) ·
[Collection contract](../../../docs/procedural_collections_v2.md)

`pilot.json` declares two exposed development streams before generation:
10,000 ascending indices per backend, eight structural attempts per index,
broad admission without category quotas. Native serialized recipes and separate
geometry/challenge/morphology specs are bound by their file SHA-256 values.
Layered uses seed 20261011031; value noise uses 20261011032. The configuration
retains nominal Earth/vehicle/resources/clocks and 4 m physical terrain. These
seeds will never establish an unexposed held-out result.

The pilot records every proposal disposition, all marginal and five selected
joint distributions, missing values, overflow and overlapping labels. Bins are
left-closed/right-open; equality moves to the bin starting at a boundary. Compare
proposed versus admitted populations; assess observed support and blind spots
without interpreting occupied bins as complete coverage. Keep backend summaries
separate. Only later evidence can justify a value-noise qualification share or
its use as a policy distribution shift.

Reference QA uses 1 m samples for offsets 0 and 1 in each 80-index block
(250 per backend, balanced across both physical directions),
plus up to 32 closest admission-boundary inputs not already selected. Boundary
distance is `abs(measured / threshold - 1)` for sustained width, absolute pad
rise, and retained crest height/prominence/half-width or adjacent spacing; take
the minimum across raw/prepared geometry and break ties by index. The total
maximum is 564 QA inputs. Compare raw interpolation error and raw/prepared
geometry, including admission disagreements. The 25 m height/barrier and 40 m
width flags are diagnostic choices, not benchmark acceptance. This sample does
not establish a continuous-terrain error bound or inspect every mission at 1 m.

Review at most 96 previews per backend, with source-to-target display direction
normalized and original direction retained as metadata. Candidate queues are
ordered marginal extremes/10th/50th/90th percentiles, occupied joint cells,
morphology groups (including mixed/unclassified), and admission boundaries.
Each queue breaks ties by index; take one unseen input from each nonempty queue
in that order per round until the union reaches its budget. Mission navigation
is collection → morphology group → measured band → mission, one open band and
twelve rows per page. Full summary counts and bounded preview counts must remain
distinguishable. Labels/bands/display never change collection membership.

The next implementation milestone provides a native bounded audit driver and
maintained report projection. Commit this declaration before the 20,000-input
run, preserve it in the run receipt, and write to a fresh output directory. If
calibration motivates changes, retain this declaration/run and issue a revision
with new identities. No held-out generation, flight, training, report publication
or server lifecycle action is part of this pilot. Freeze and an unchanged-incumbent
comparison need their own separately budgeted plan after review.
