# Saved mission geometry review

[Plan](../../docs/procedural_challenge_plan.md) ·
[Review, measurements and research](../../docs/procedural_challenge_review.md)

The committed [V2 pilot declaration](pilot_v2/README.md) owns the next input-only
calibration. Its native recipes and metric/admission/projection specs are distinct
from this historical saved-input audit. No new flights or training are scheduled.

`run_pilot_v2.py --plan` inspects that declaration without generation.
The [native audit workflow](../../docs/procedural_input_audit.md) owns the
create-only collection/QA run; the script orchestrates native tools without
implementing another terrain generator or controller.

`audit.py` reads saved manifest-bound missions, authenticates selected file
bytes and emits geometry summaries to stdout. It runs no generator, strategy,
flight, native replay or external game code, and writes no files. The default
paths select the observed October 10 learning collections and the original/fresh
1k challenge inputs. They are ignored local evidence, not ordinary-test fixtures.

```sh
rtk proxy python3 -B studies/mission_coverage/audit.py
rtk proxy python3 -B studies/mission_coverage/audit.py --collection sample=outputs/my-collection
rtk proxy python3 -B -m unittest discover -s studies/mission_coverage -p 'test_*.py' -v
```

Use repeated `--collection LABEL=PATH` for current collections or repeated
`--legacy LABEL=PATH` for old manifests with random-cohort scenario rows. Explicit
arguments replace the default population list. Missing evidence is an error;
the tool does not regenerate or substitute inputs.

Metrics clip prepared heightfields exactly between source/target pad centers.
The chord-relative obstacle widths are connected horizontal intervals; slope
quantiles are weighted by horizontal length. An edge-peak flag includes any
pad-adjacent segment within 1e-8 relative/1e-9 absolute tolerance of a nonzero
maximum slope, avoiding dependence on tie order or reflected coordinates.
Excluding edge-adjacent segments is a diagnostic, not removal of every pad edit.

File-byte authentication and scalar geometry agreement do not establish native
physical-identity serialization, a valid replay, solvability or controller
success. Implemented [native descriptors](../../docs/procedural_geometry.md) and
[V2 challenge admission](../../docs/procedural_collections_v2.md) belong to
`pd-missions`. This utility supplies reproducible historical input-review evidence.

