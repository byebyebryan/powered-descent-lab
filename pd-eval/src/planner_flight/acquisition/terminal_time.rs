//! Current planner acquisition terminal_time; no research orchestration.
use super::*;
use anyhow::{Context, Result, bail};

mod candidates;
use candidates::alternative_durations;
use serde_json::{Value, json};

fn plan_from_seed(seed: &NominalSeedEvidenceV1, live: &SimulationState) -> Result<AcquisitionPlan> {
    let thrust = seed
        .requested_thrust_acceleration_mps2
        .context("missing cached acquisition thrust")?;
    Ok(AcquisitionPlan {
        thrust_acceleration_mps2: thrust,
        turn_ticks: seed.turn_physics_ticks,
        burn_ticks: seed.burn_physics_ticks,
        predicted_end: seed
            .predicted_acquisition_end
            .context("missing cached acquisition endpoint")?,
        virtual_target_error_m: seed
            .virtual_target_error_m
            .context("missing acquisition target error")?,
        estimated_fuel_kg: seed
            .estimated_acquisition_fuel_kg
            .context("missing acquisition fuel")?,
        upward_impulse_mps: seed
            .upward_impulse_mps
            .context("missing acquisition impulse")?,
        turn_updates: seed.turn_updates,
        target_attitude_rad: if seed.turn_physics_ticks > 0 {
            thrust.x.atan2(thrust.y)
        } else {
            live.attitude_rad
        },
    })
}

fn screen_duration(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    plan: &AcquisitionPlan,
    original: &NominalEntryScreenEvidenceV1,
    ticks: u64,
) -> Result<NominalEntryScreenEvidenceV1> {
    let mut e = original.clone();
    e.terminal_physics_ticks = Some(ticks);
    e.terminal_time_s = Some(ticks as f64 * context.sim.physics_dt_s());
    e.coupled_thrust_bound_mps2 = None;
    e.terminal_fuel_kg = None;
    e.total_predicted_fuel_kg = None;
    e.admissible = false;
    e.reason = None;
    e.lateral_velocity_reversal_checked = false;
    let finish = live
        .physics_step
        .checked_add(plan.turn_ticks)
        .and_then(|t| t.checked_add(plan.burn_ticks))
        .and_then(|t| t.checked_add(e.coast_physics_ticks))
        .and_then(|t| t.checked_add(ticks))
        .and_then(|t| t.checked_add(TERMINAL_EXTRA_TICKS))
        .context("finish clock overflow")?;
    e.predicted_finish_physics_step = Some(finish);
    if finish > deadline {
        e.reason = Some("predicted terminal finish exceeds original absolute deadline".into());
        return Ok(e);
    }
    let r = build_reference(
        context,
        &BodyAwareTerminalPolicyV1::default(),
        e.predicted_entry,
        ticks,
    )?;
    if !terminal_forward_velocity_valid(context, &r) {
        e.lateral_velocity_reversal_checked = true;
        e.reason = Some("terminal reference reverses forward horizontal velocity".into());
        return Ok(e);
    }
    e.lateral_velocity_reversal_checked = true;
    let bound = conservative_terminal_thrust_bound(context, &r);
    if !bound.is_finite() {
        bail!("nonfinite terminal bound");
    }
    e.coupled_thrust_bound_mps2 = Some(bound);
    let fuel = live.fuel_kg - plan.estimated_fuel_kg;
    let limit = MAX_THRUST_FRACTION * context.vehicle.max_thrust_n
        / (context.vehicle.dry_mass_kg + fuel.max(0.0)).max(1.0);
    if bound > limit {
        e.reason = Some(format!(
            "coupled terminal thrust bound {bound:.6} exceeds 0.925 incoming-mass limit {limit:.6}"
        ));
        return Ok(e);
    }
    let first = paired_reference_thrust(context, &r, 0);
    let first_angle = first.x.atan2(first.y);
    let (align, _) = match turn_ticks_for(
        context,
        if plan.turn_ticks > 0 {
            plan.target_attitude_rad
        } else {
            live.attitude_rad
        },
        Vec2::new(first_angle.sin(), first_angle.cos()),
    ) {
        Ok(turn) => turn,
        Err(error) => {
            e.reason = Some(format!("terminal first-thrust alignment: {error:#}"));
            return Ok(e);
        }
    };
    if e.coast_physics_ticks < align {
        e.reason = Some(format!(
            "terminal entry has {} coasting ticks for {align} ticks of first-thrust alignment",
            e.coast_physics_ticks
        ));
        return Ok(e);
    }
    match estimate_terminal_fuel(context, &r, fuel) {
        Ok(used) => {
            e.terminal_fuel_kg = Some(used);
            e.total_predicted_fuel_kg = Some(plan.estimated_fuel_kg + used);
            e.admissible = true;
        }
        Err(error) => e.reason = Some(format!("terminal throttle/fuel screen: {error:#}")),
    }
    Ok(e)
}

pub(super) fn evaluate_timed_seed(
    context: &RunContext,
    live: &SimulationState,
    deadline: u64,
    spec: SeedSpec,
) -> Result<(NominalSeedEvidenceV1, Vec<Value>)> {
    // The unchanged evaluator performs acquisition once and supplies the baseline screens.
    let mut seed = evaluate_seed(context, live, deadline, spec)?;
    let mut traces = Vec::new();
    if seed.entry_screens.is_empty() {
        return Ok((seed, traces));
    }
    let plan = plan_from_seed(&seed, live)?;
    for e in &mut seed.entry_screens {
        let baseline = e.clone();
        let mut screens = vec![baseline.clone()];
        let mut metadata = vec![
            json!({"candidate_index":0,"provenance":"unchanged_vertical_baseline","physics_ticks":e.terminal_physics_ticks}),
        ];
        let entry = e.predicted_entry;
        if !entry.position_m.x.is_finite()
            || !entry.position_m.y.is_finite()
            || !entry.velocity_mps.x.is_finite()
            || !entry.velocity_mps.y.is_finite()
            || e.coupled_thrust_bound_mps2.is_some_and(|b| !b.is_finite())
        {
            bail!("nonfinite entry screen");
        }
        let height = entry.position_m.y
            - context.target_pad.surface_y_m
            - context.vehicle.geometry.touchdown_base_offset_m;
        let direction = forward_direction(context, entry.position_m.x, entry.velocity_mps.x);
        let entry_tick = live
            .physics_step
            .checked_add(plan.turn_ticks)
            .and_then(|t| t.checked_add(plan.burn_ticks))
            .and_then(|t| t.checked_add(e.coast_physics_ticks))
            .context("entry clock overflow")?;
        if !baseline.admissible && height > TARGET_ENTRY_RESERVE_M && entry.velocity_mps.y < 0.0 {
            for c in alternative_durations(
                direction * (context.target_pad.center_x_m - entry.position_m.x),
                direction * entry.velocity_mps.x,
                context.sim.physics_dt_s(),
                entry_tick,
                deadline,
                baseline.terminal_physics_ticks.unwrap_or(0),
            )? {
                screens.push(screen_duration(
                    context,
                    live,
                    deadline,
                    &plan,
                    &baseline,
                    c.physics_ticks,
                )?);
                metadata.push(serde_json::to_value(c)?);
            }
        }
        let selected = if baseline.admissible {
            Some(0)
        } else {
            screens
                .iter()
                .enumerate()
                .filter(|(_, s)| s.admissible)
                .min_by(|(ia, a), (ib, b)| {
                    a.total_predicted_fuel_kg
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
                        .then_with(|| {
                            a.predicted_finish_physics_step
                                .cmp(&b.predicted_finish_physics_step)
                        })
                        .then_with(|| ia.cmp(ib))
                })
                .map(|(i, _)| i)
        };
        traces.push(json!({"seed_id":seed.seed_id,"entry_index":e.entry_index,
            "candidates":metadata,"screens":screens,"selected_candidate_index":selected,
            "baseline_retained":baseline.admissible,"maximum_candidates":3}));
        if let Some(i) = selected {
            *e = screens[i].clone();
        }
    }
    seed.selected_entry_index = seed
        .entry_screens
        .iter()
        .enumerate()
        .filter(|(_, s)| s.admissible)
        .min_by(|(ia, a), (ib, b)| {
            a.total_predicted_fuel_kg
                .unwrap_or(f64::INFINITY)
                .total_cmp(&b.total_predicted_fuel_kg.unwrap_or(f64::INFINITY))
                .then_with(|| {
                    a.predicted_finish_physics_step
                        .cmp(&b.predicted_finish_physics_step)
                })
                .then_with(|| ia.cmp(ib))
        })
        .map(|(i, _)| i);
    seed.status = if seed.selected_entry_index.is_some() {
        "shortlisted_estimate"
    } else {
        "finite_miss"
    }
    .into();
    if let Some(i) = seed.selected_entry_index {
        seed.estimated_fuel_kg = seed.entry_screens[i].total_predicted_fuel_kg;
        seed.predicted_finish_physics_step = seed.entry_screens[i].predicted_finish_physics_step;
        seed.reason = None;
    } else {
        seed.reason = Some(
            seed.entry_screens
                .iter()
                .filter_map(|s| s.reason.as_deref())
                .take(4)
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    seed.identity = seed_identity(&seed)?;
    Ok((seed, traces))
}
#[cfg(test)]
mod tests;
