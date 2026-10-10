# Built-in evaluation application

This composition package installs the `pd-eval` binary. Cargo callers use
`-p pd-eval-cli`; native command names, defaults, serialized legacy records and
existing capture/replay/publication rules are unchanged.

Reusable neutral evaluation now lives in the sibling `pd-eval` crate. This
package is a transitional owner for the existing solver modules until the
maintained solver extraction finishes. It is not a dependency of independent
agents, mission generation, generic evaluation or report rendering.

| Current module area | Final owner / treatment |
| --- | --- |
| `waypoint_v2`, `planner_flight`, `local_clearing` | Maintained Policy 3 solver orchestration in `pd-planner`; existing tests move with behavior |
| `ballistic_feedback` solver modules | Combined-safety and required comparator implementations/configuration in `pd-planner`; preserve opt-in admission |
| Solver output/report/frontdoor modules | Application composition and saved evidence readers; no strategy in generic rendering |
| `model`, `resolution`, `execution`, `review`, `comparison`, `runtime` | Legacy authored pack compatibility and application registration; neutral equivalents live in platform crates |
| `waypoint_v2_pack`, acceptance, bundle, site and terrain survey | Legacy evidence workflows/readers; retained captures keep original schemas and provenance |

See [implementation record](../docs/platform_restructure_results.md),
[execution contract](../docs/platform_restructure_execution.md) and
[independent agents](../docs/independent_agents.md). This move alone does not
complete the solver ownership milestone.
