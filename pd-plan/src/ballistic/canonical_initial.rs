//! Endpoint-only nominal bases. No world, terrain, or clearance query is input.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialEndpointsV1 {
    pub source: PadInputV2,
    pub target: PadInputV2,
    pub initial: KinematicStateV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialBasisPairV1 {
    pub source_handoff: HandoffV2,
    pub terminal_handoff: HandoffV2,
    pub source_bridge_ticks: u64,
    pub coast_ticks: u64,
    pub terminal_bridge_ticks: u64,
    pub minimum_nominal_margin: f64,
    pub nominal_fuel_burn_kg: f64,
    pub nominal_time_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalInitialBallisticBasisV1 {
    pub duration_multiplier: f64,
    pub arc: VirtualBallisticArcV2,
    pub source_option_count: usize,
    pub terminal_option_count: usize,
    pub primitive_attempt_count: usize,
    pub selected: Option<CanonicalInitialBasisPairV1>,
    pub rejection_reasons: Vec<String>,
}

/// A finite endpoint/dynamics family; this is not a terrain certificate.
pub fn canonical_initial_ballistic_bases_v1(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    endpoints: &CanonicalInitialEndpointsV1,
) -> Result<Vec<CanonicalInitialBallisticBasisV1>, String> {
    policy.validate()?;
    vehicle.validate()?;
    if endpoints.target.center_x_m <= endpoints.source.center_x_m
        || ![
            endpoints.source.center_x_m,
            endpoints.source.surface_y_m,
            endpoints.source.width_m,
            endpoints.target.center_x_m,
            endpoints.target.surface_y_m,
            endpoints.target.width_m,
            endpoints.initial.position_m.x,
            endpoints.initial.position_m.y,
            endpoints.initial.velocity_mps.x,
            endpoints.initial.velocity_mps.y,
        ]
        .iter()
        .all(|v| v.is_finite())
        || endpoints.source.width_m <= 0.0
        || endpoints.target.width_m <= 0.0
    {
        return Err("unsupported or nonfinite nominal endpoints".into());
    }
    let release = Vec2::new(
        endpoints.source.center_x_m,
        endpoints.source.surface_y_m
            + vehicle.geometry.touchdown_base_offset_m
            + policy.minimum_clearance_m,
    );
    let target = KinematicStateV2 {
        position_m: Vec2::new(
            endpoints.target.center_x_m,
            endpoints.target.surface_y_m + vehicle.geometry.touchdown_base_offset_m,
        ),
        velocity_mps: Vec2::new(
            0.0,
            -policy.terminal_target_downward_speed_fraction
                * vehicle.safe_touchdown_normal_speed_mps,
        ),
    };
    let durations = bridge_duration_steps(policy);
    let mut result = Vec::new();
    for (multiplier, steps) in candidate_steps(policy, target.position_m.x - release.x) {
        let arc = VirtualBallisticArcV2::new(policy, release, target.position_m, steps);
        let mut attempts = 0;
        let mut reasons = std::collections::BTreeSet::new();
        let mut options = |kind, indices: Vec<u64>, fixed: KinematicStateV2| {
            indices
                .into_iter()
                .filter_map(|step| {
                    let state = arc.state_at(step);
                    let (start, end) = if kind == BridgeKindV2::Source {
                        (fixed, state)
                    } else {
                        (state, fixed)
                    };
                    for &ticks in &durations {
                        attempts += 1;
                        if let Some(reason) = primitive_precheck_failure(
                            policy, vehicle, start, end, ticks,
                        )
                        .or_else(|| {
                            local_attitude_precheck_failure(
                                policy, vehicle, kind, start, end, ticks,
                            )
                        }) {
                            reasons.insert(format!("{reason:?}"));
                            continue;
                        }
                        let bridge =
                            exact_discrete_bridge_v2(policy, vehicle, kind, start, end, ticks)
                                .expect("validated finite primitive");
                        if bridge.classification == CertificationV2::Certified
                            && bridge.margins.minimum_normalized() + 1.0e-12
                                >= policy.declared_robustness_margin
                        {
                            return Some((
                                HandoffV2 {
                                    arc_step: step,
                                    state,
                                },
                                bridge,
                            ));
                        }
                        reasons.extend(bridge.reasons.iter().map(|r| format!("{r:?}")));
                    }
                    None
                })
                .collect::<Vec<_>>()
        };
        let source = options(
            BridgeKindV2::Source,
            source_handoff_steps(&arc, policy.handoff_interval_ticks()),
            endpoints.initial,
        );
        let terminal = options(
            BridgeKindV2::Terminal,
            terminal_handoff_steps(&arc, policy.handoff_interval_ticks()),
            target,
        );
        let mut best: Option<CanonicalInitialBasisPairV1> = None;
        for (sh, sb) in &source {
            for (th, tb) in &terminal {
                if sh.arc_step >= th.arc_step {
                    continue;
                }
                let coast = (th.arc_step - sh.arc_step) as f64 * policy.dt_s();
                let source_direction = final_powered_direction(sb).expect("powered source").1;
                let terminal_direction = first_powered_direction(tb).expect("powered terminal").1;
                let fuel = sb.fuel_burn_kg + tb.fuel_burn_kg;
                let time = sb.duration_s + coast + tb.duration_s;
                let margin = sb
                    .margins
                    .minimum_normalized()
                    .min(tb.margins.minimum_normalized())
                    .min(
                        MarginV2::upper(
                            coast,
                            angle_between(source_direction, terminal_direction)
                                / vehicle.max_rotation_rate_radps,
                        )
                        .normalized,
                    )
                    .min(MarginV2::upper(vehicle.initial_fuel_kg, fuel).normalized)
                    .min(MarginV2::upper(policy.mission_budget_s(), time).normalized)
                    .min(
                        MarginV2::upper(
                            vehicle.safe_touchdown_attitude_error_rad,
                            angle_between(
                                Vec2::new(0.0, 1.0),
                                first_powered_direction(sb).unwrap().1,
                            ),
                        )
                        .normalized,
                    )
                    .min(
                        MarginV2::upper(
                            vehicle.safe_touchdown_attitude_error_rad,
                            angle_between(
                                Vec2::new(0.0, 1.0),
                                final_powered_direction(tb).unwrap().1,
                            ),
                        )
                        .normalized,
                    )
                    .min(
                        MarginV2::upper(
                            vehicle.safe_touchdown_angular_rate_radps,
                            terminal_final_rotation_rate(tb).unwrap_or(f64::INFINITY),
                        )
                        .normalized,
                    );
                if !margin.is_finite() || margin + 1.0e-12 < policy.declared_robustness_margin {
                    continue;
                }
                let pair = CanonicalInitialBasisPairV1 {
                    source_handoff: *sh,
                    terminal_handoff: *th,
                    source_bridge_ticks: sb.steps,
                    coast_ticks: th.arc_step - sh.arc_step,
                    terminal_bridge_ticks: tb.steps,
                    minimum_nominal_margin: margin,
                    nominal_fuel_burn_kg: fuel,
                    nominal_time_s: time,
                };
                if best.as_ref().is_none_or(|old| {
                    pair.minimum_nominal_margin
                        .total_cmp(&old.minimum_nominal_margin)
                        .reverse()
                        .then_with(|| {
                            pair.nominal_fuel_burn_kg
                                .total_cmp(&old.nominal_fuel_burn_kg)
                        })
                        .then_with(|| pair.nominal_time_s.total_cmp(&old.nominal_time_s))
                        .then_with(|| tuple(&pair).cmp(&tuple(old)))
                        .is_lt()
                }) {
                    best = Some(pair);
                }
            }
        }
        if best.is_none() {
            reasons.insert("no_nominally_feasible_nonoverlapping_pair".into());
        }
        result.push(CanonicalInitialBallisticBasisV1 {
            duration_multiplier: multiplier,
            arc,
            source_option_count: source.len(),
            terminal_option_count: terminal.len(),
            primitive_attempt_count: attempts,
            selected: best,
            rejection_reasons: reasons.into_iter().collect(),
        });
    }
    Ok(result)
}

fn tuple(pair: &CanonicalInitialBasisPairV1) -> (u64, u64, u64, u64) {
    (
        pair.source_handoff.arc_step,
        pair.terminal_handoff.arc_step,
        pair.source_bridge_ticks,
        pair.terminal_bridge_ticks,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DirectBridgePolicyV2 {
        DirectBridgePolicyV2 {
            physics_hz: 120,
            gravity_mps2: 9.81,
            duration_multipliers: vec![0.75, 1.0, 1.25, 1.5],
            minimum_clearance_m: 5.0,
            maximum_mission_time_s: 180.0,
            mission_time_reserve_s: 10.0,
            thrust_derate: 0.81,
            declared_robustness_margin: 0.075,
            handoff_interval_s: 0.25,
            bridge_duration_interval_s: 0.5,
            terminal_target_downward_speed_fraction: 0.5,
        }
    }

    fn vehicle() -> VehicleInputV2 {
        VehicleInputV2 {
            geometry: VehicleGeometryInputV2 {
                hull_width_m: 8.0,
                hull_height_m: 10.0,
                touchdown_half_span_m: 4.0,
                touchdown_base_offset_m: 5.0,
            },
            dry_mass_kg: 7200.0,
            initial_fuel_kg: 6300.0,
            max_fuel_kg: 6300.0,
            max_fuel_burn_kgps: 49.5,
            max_thrust_n: 240000.0,
            min_throttle_frac: 0.25,
            max_rotation_rate_radps: std::f64::consts::FRAC_PI_2,
            safe_touchdown_normal_speed_mps: 3.0,
            safe_touchdown_tangential_speed_mps: 2.0,
            safe_touchdown_attitude_error_rad: 0.15,
            safe_touchdown_angular_rate_radps: 0.35,
        }
    }

    #[test]
    fn canonical_bases_accept_no_terrain_input_and_are_repeatable() {
        let policy = policy();
        let vehicle = vehicle();
        let endpoints = CanonicalInitialEndpointsV1 {
            source: PadInputV2 {
                center_x_m: 18.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            },
            target: PadInputV2 {
                center_x_m: 2500.0,
                surface_y_m: 0.0,
                width_m: 36.0,
            },
            initial: KinematicStateV2 {
                position_m: Vec2::new(18.0, 5.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
        };
        let a = canonical_initial_ballistic_bases_v1(&policy, &vehicle, &endpoints).unwrap();
        let b = canonical_initial_ballistic_bases_v1(&policy, &vehicle, &endpoints).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4);
        assert!(a.iter().any(|basis| basis.selected.is_some()));
        assert!(
            serde_json::from_value::<CanonicalInitialEndpointsV1>(serde_json::json!({
                "source": endpoints.source,
                "target": endpoints.target,
                "initial": endpoints.initial,
                "terrain": []
            }))
            .is_err()
        );
    }
}
