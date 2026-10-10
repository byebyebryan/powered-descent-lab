use super::*;

pub(super) fn execute_resolved_runs(
    resolved_runs: &[ResolvedBatchRun],
    output_dir: Option<&Path>,
    workers_used: usize,
) -> Result<Vec<BatchRunRecord>> {
    if workers_used <= 1 {
        return resolved_runs
            .iter()
            .map(|run| execute_resolved_run(run, output_dir))
            .collect();
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(workers_used)
        .build()
        .context("failed to build pd-eval thread pool")?;

    let results = pool.install(|| {
        resolved_runs
            .par_iter()
            .map(|run| execute_resolved_run(run, output_dir))
            .collect::<Vec<_>>()
    });

    results.into_iter().collect()
}

pub(super) fn analytic_feasibility_for_run(
    resolved_run: &ResolvedBatchRun,
) -> BatchRunAnalyticFeasibility {
    if near_vertical_transfer_route_frontier(resolved_run) {
        return BatchRunAnalyticFeasibility {
            class: BatchRunAnalyticClass::Frontier,
            reason: Some(BatchRunAnalyticReason::NearVerticalTransferRoute),
            ..Default::default()
        };
    }

    if !matches!(
        resolved_run.descriptor.source_kind,
        ResolvedRunSourceKind::TerminalMatrix
    ) {
        return BatchRunAnalyticFeasibility::default();
    }

    let scenario = &resolved_run.scenario;
    let Some(target_pad) = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == scenario.mission.goal.target_pad_id())
    else {
        return BatchRunAnalyticFeasibility::default();
    };

    let mass_kg = scenario.vehicle.dry_mass_kg + scenario.vehicle.initial_fuel_kg;
    let gravity_mps2 = scenario.world.gravity_mps2.abs().max(1e-6);
    let max_upward_accel_mps2 = (scenario.vehicle.max_thrust_n / mass_kg.max(1.0)) - gravity_mps2;
    let downward_speed_mps = (-scenario.initial_state.velocity_mps.y).max(0.0);
    let safe_touchdown_speed_mps = scenario.vehicle.safe_touchdown_normal_speed_mps;
    let available_stop_height_m = scenario.initial_state.position_m.y
        - target_pad.surface_y_m
        - scenario.vehicle.geometry.touchdown_base_offset_m;

    let required_stop_height_m = if downward_speed_mps <= safe_touchdown_speed_mps {
        0.0
    } else if max_upward_accel_mps2 <= 0.0 {
        f64::INFINITY
    } else {
        ((downward_speed_mps * downward_speed_mps)
            - (safe_touchdown_speed_mps * safe_touchdown_speed_mps))
            / (2.0 * max_upward_accel_mps2)
    };
    let stop_height_margin_m = available_stop_height_m - required_stop_height_m;

    if stop_height_margin_m < 0.0 {
        BatchRunAnalyticFeasibility {
            class: BatchRunAnalyticClass::Impossible,
            reason: Some(BatchRunAnalyticReason::VerticalStopHeight),
            available_stop_height_m: Some(available_stop_height_m),
            required_stop_height_m: Some(required_stop_height_m),
            stop_height_margin_m: Some(stop_height_margin_m),
            ..Default::default()
        }
    } else {
        let coupled = coupled_stop_acceleration_bound(scenario, target_pad);
        if coupled.stop_accel_margin_mps2 < 0.0 {
            return BatchRunAnalyticFeasibility {
                class: BatchRunAnalyticClass::Impossible,
                reason: Some(BatchRunAnalyticReason::CoupledStopAcceleration),
                available_stop_height_m: Some(available_stop_height_m),
                required_stop_height_m: Some(required_stop_height_m),
                stop_height_margin_m: Some(stop_height_margin_m),
                available_stop_accel_mps2: Some(coupled.available_accel_mps2),
                required_stop_accel_mps2: Some(coupled.required_accel_mps2),
                stop_accel_margin_mps2: Some(coupled.stop_accel_margin_mps2),
            };
        }

        let class = if low_thrust_high_energy_frontier(
            resolved_run,
            max_upward_accel_mps2,
            gravity_mps2,
            coupled.stop_accel_margin_mps2,
        ) {
            BatchRunAnalyticClass::Frontier
        } else {
            BatchRunAnalyticClass::Scored
        };
        let reason = if matches!(class, BatchRunAnalyticClass::Frontier) {
            Some(BatchRunAnalyticReason::LowThrustHighEnergy)
        } else {
            None
        };

        BatchRunAnalyticFeasibility {
            class,
            reason,
            available_stop_height_m: Some(available_stop_height_m),
            required_stop_height_m: Some(required_stop_height_m),
            stop_height_margin_m: Some(stop_height_margin_m),
            available_stop_accel_mps2: Some(coupled.available_accel_mps2),
            required_stop_accel_mps2: Some(coupled.required_accel_mps2),
            stop_accel_margin_mps2: Some(coupled.stop_accel_margin_mps2),
        }
    }
}

pub(super) const AUTHORITY_FRONTIER_MARGIN_GRAVITY_RATIO: f64 = 0.45;
pub(super) const NEAR_VERTICAL_TRANSFER_ROUTE_MIN_DEG: f64 = 75.0;

pub(super) fn near_vertical_transfer_route_frontier(resolved_run: &ResolvedBatchRun) -> bool {
    if !matches!(
        resolved_run.descriptor.source_kind,
        ResolvedRunSourceKind::TransferMatrix
    ) {
        return false;
    }

    resolved_run
        .scenario
        .mission
        .transfer_route
        .as_ref()
        .is_some_and(|route| route.route_angle_deg >= NEAR_VERTICAL_TRANSFER_ROUTE_MIN_DEG)
}

pub(super) fn low_thrust_high_energy_frontier(
    resolved_run: &ResolvedBatchRun,
    upright_net_accel_mps2: f64,
    gravity_mps2: f64,
    stop_accel_margin_mps2: f64,
) -> bool {
    if resolved_run.descriptor.selector.velocity_band != "high" {
        return false;
    }

    let low_authority_margin_mps2 =
        AUTHORITY_FRONTIER_MARGIN_GRAVITY_RATIO * gravity_mps2.max(1e-6);
    upright_net_accel_mps2 <= low_authority_margin_mps2
        && stop_accel_margin_mps2 <= low_authority_margin_mps2
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CoupledStopAccelerationBound {
    available_accel_mps2: f64,
    required_accel_mps2: f64,
    stop_accel_margin_mps2: f64,
}

pub(super) const REACHABILITY_TIME_STEP_S: f64 = 0.1;

pub(super) fn coupled_stop_acceleration_bound(
    scenario: &ScenarioSpec,
    target_pad: &LandingPadSpec,
) -> CoupledStopAccelerationBound {
    let gravity_mps2 = scenario.world.gravity_mps2.abs().max(1e-6);
    let mass_kg = scenario.vehicle.dry_mass_kg + scenario.vehicle.initial_fuel_kg;
    let x0_m = scenario.initial_state.position_m.x;
    let y0_m = scenario.initial_state.position_m.y;
    let vx_mps = scenario.initial_state.velocity_mps.x;
    let vy_mps = scenario.initial_state.velocity_mps.y;
    let safe_lateral_speed_mps = scenario.vehicle.safe_touchdown_tangential_speed_mps;
    let touchdown_center_limit_m =
        (target_pad.half_width_m() - scenario.vehicle.geometry.touchdown_half_span_m).max(0.0);
    let safe_vertical_speed_mps = scenario.vehicle.safe_touchdown_normal_speed_mps;
    let target_y_m = target_pad.surface_y_m + scenario.vehicle.geometry.touchdown_base_offset_m;
    let target_min_x_m = target_pad.center_x_m - touchdown_center_limit_m;
    let target_max_x_m = target_pad.center_x_m + touchdown_center_limit_m;
    let max_time_s = scenario
        .metadata
        .get("resolved.reachability_max_time_s")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(scenario.sim.max_time_s)
        .max(REACHABILITY_TIME_STEP_S);
    let steps = (max_time_s / REACHABILITY_TIME_STEP_S).ceil() as u64;
    let mut best: Option<CoupledStopAccelerationBound> = None;

    // Sweep possible touchdown times and use optimistic double-integrator lower bounds.
    // If even this lower bound exceeds full-throttle authority, the run is outside the envelope.
    for step in 1..=steps {
        let time_s = (step as f64 * REACHABILITY_TIME_STEP_S).min(max_time_s);
        let ballistic_x_m = x0_m + vx_mps * time_s;
        let required_lateral_position_accel_mps2 = 2.0
            * distance_outside_interval_m(ballistic_x_m, target_min_x_m, target_max_x_m)
            / (time_s * time_s);
        let required_lateral_velocity_accel_mps2 =
            distance_outside_interval_m(vx_mps, -safe_lateral_speed_mps, safe_lateral_speed_mps)
                / time_s;
        let required_lateral_accel_mps2 =
            required_lateral_position_accel_mps2.max(required_lateral_velocity_accel_mps2);

        let freefall_y_m = y0_m + vy_mps * time_s - 0.5 * gravity_mps2 * time_s * time_s;
        let required_upward_displacement_m = target_y_m - freefall_y_m;
        if required_upward_displacement_m < -1e-6 {
            continue;
        }
        let required_vertical_position_accel_mps2 =
            (2.0 * required_upward_displacement_m.max(0.0)) / (time_s * time_s);
        let freefall_vy_mps = vy_mps - gravity_mps2 * time_s;
        if freefall_vy_mps > 1e-6 {
            continue;
        }
        let required_vertical_velocity_accel_mps2 = if freefall_vy_mps < -safe_vertical_speed_mps {
            (-safe_vertical_speed_mps - freefall_vy_mps) / time_s
        } else {
            0.0
        };
        let required_vertical_accel_mps2 =
            required_vertical_position_accel_mps2.max(required_vertical_velocity_accel_mps2);
        let required_accel_mps2 = (required_lateral_accel_mps2 * required_lateral_accel_mps2
            + required_vertical_accel_mps2 * required_vertical_accel_mps2)
            .sqrt();
        let available_accel_mps2 =
            full_throttle_average_accel_mps2(&scenario.vehicle, mass_kg, time_s);
        let stop_accel_margin_mps2 = available_accel_mps2 - required_accel_mps2;
        let candidate = CoupledStopAccelerationBound {
            available_accel_mps2,
            required_accel_mps2,
            stop_accel_margin_mps2,
        };

        if best
            .map(|best| stop_accel_margin_mps2 > best.stop_accel_margin_mps2)
            .unwrap_or(true)
        {
            best = Some(candidate);
        }
    }

    best.unwrap_or_else(|| {
        let available_accel_mps2 =
            full_throttle_average_accel_mps2(&scenario.vehicle, mass_kg, max_time_s);
        CoupledStopAccelerationBound {
            available_accel_mps2,
            required_accel_mps2: f64::INFINITY,
            stop_accel_margin_mps2: f64::NEG_INFINITY,
        }
    })
}

pub(super) fn distance_outside_interval_m(value: f64, min_value: f64, max_value: f64) -> f64 {
    if value < min_value {
        min_value - value
    } else if value > max_value {
        value - max_value
    } else {
        0.0
    }
}

pub(super) fn full_throttle_average_accel_mps2(
    vehicle: &VehicleSpec,
    initial_mass_kg: f64,
    terminal_window_s: f64,
) -> f64 {
    let mass0 = initial_mass_kg.max(1.0);
    let burn_window_s = terminal_window_s.max(1e-6);
    let fuel_used_kg =
        (vehicle.max_fuel_burn_kgps.max(0.0) * burn_window_s).min(vehicle.initial_fuel_kg.max(0.0));
    let mass1 = (mass0 - fuel_used_kg).max(vehicle.dry_mass_kg.max(1.0));
    if fuel_used_kg <= 1e-9 || vehicle.max_fuel_burn_kgps <= 1e-9 {
        return vehicle.max_thrust_n / mass0;
    }

    vehicle.max_thrust_n / (vehicle.max_fuel_burn_kgps * burn_window_s) * (mass0 / mass1).ln()
}

pub(super) fn execute_resolved_run(
    resolved_run: &ResolvedBatchRun,
    output_dir: Option<&Path>,
) -> Result<BatchRunRecord> {
    let ctx = RunContext::from_scenario(&resolved_run.scenario)
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "failed to build run context for resolved run {}",
                resolved_run.descriptor.run_id
            )
        })?;
    let artifacts = run_controller_spec(&ctx, &resolved_run.descriptor.controller_spec)
        .with_context(|| {
            format!(
                "failed to run controller for resolved run {}",
                resolved_run.descriptor.run_id
            )
        })?;

    let bundle_dir = output_dir.map(|root| root.join("runs").join(&resolved_run.descriptor.run_id));
    if let Some(bundle_dir) = bundle_dir.as_deref() {
        write_artifact_bundle(
            bundle_dir,
            &resolved_run.scenario,
            &resolved_run.descriptor.controller_spec,
            &artifacts,
        )?;
    }

    let review = derive_run_review_metrics(&resolved_run.scenario, &artifacts);
    let analytic = analytic_feasibility_for_run(resolved_run);

    Ok(BatchRunRecord {
        resolved: resolved_run.descriptor.clone(),
        manifest: artifacts.run.manifest,
        review,
        analytic,
        bundle_dir: bundle_dir.map(|path| path.to_string_lossy().into_owned()),
    })
}
