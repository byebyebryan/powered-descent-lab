//! Query-only evidence. These projections never select an action or step a plant.
use pd_control::{maneuver::*, poly_landing::*};
use pd_core::{Observation, RunContext, ScenarioSpec, Vec2};
use pd_run::EpisodeSummaryV1;
use serde_json::{Value, json};

pub fn project(
    scenario: &ScenarioSpec,
    reference_end: u64,
    admission: Option<&AdmissionV2>,
    activation: Option<&Observation>,
    feedback: Option<&ManeuverFeedbackV2>,
    summary: &EpisodeSummaryV1,
) -> Result<Value, String> {
    let ctx = RunContext::from_scenario(scenario)?;
    let contact = summary.incoming_contact.as_ref().map(|contact| {
        json!({"recorded":contact,"geometry":contact.state.to_simulation_state().contact_diagnostics(&ctx)})
    });
    let reference = activation
        .map(|entry| {
            build_reference(
                &ctx,
                LandingKinematicsV1 {
                    position_m: entry.position_m,
                    velocity_mps: entry.velocity_mps,
                },
                LandingKinematicsV1 {
                    position_m: Vec2::new(
                        ctx.target_pad.center_x_m,
                        ctx.target_pad.surface_y_m + ctx.vehicle.geometry.touchdown_base_offset_m
                            - 0.005,
                    ),
                    velocity_mps: Vec2::new(0.0, -1.5),
                },
                reference_end - entry.physics_step,
            )
        })
        .transpose();
    let reference_projection = match reference {
        Ok(Some(reference)) => {
            let entry = activation.unwrap();
            let mut view = summary.final_state.to_simulation_state();
            let mut first_contact = None;
            let mut min_y = entry.position_m.y;
            let mut max_y = entry.position_m.y;
            for tick in 0..=reference.physics_ticks + 4 {
                let k = reference.state_at(&ctx, tick);
                let thrust = paired_mean(&reference, &ctx, tick - tick % 2);
                view.position_m = k.position_m;
                view.velocity_mps = k.velocity_mps;
                view.attitude_rad = thrust.x.atan2(thrust.y);
                view.angular_rate_radps = 0.0;
                view.physics_step = entry.physics_step + tick;
                view.sim_time_s = view.physics_step as f64 / 120.0;
                min_y = min_y.min(k.position_m.y);
                max_y = max_y.max(k.position_m.y);
                if first_contact.is_none() {
                    let geometry = view.contact_diagnostics(&ctx);
                    if geometry.min_touchdown_clearance_m <= 0.0
                        || geometry.min_hull_clearance_m <= 0.0
                    {
                        first_contact = Some(
                            json!({"local_tick":tick,"physics_step":view.physics_step,"sim_time_s":view.sim_time_s,"kinematics":k,"requested_attitude_rad":view.attitude_rad,"geometry":geometry}),
                        );
                    }
                }
            }
            let failed_tick = match admission {
                Some(AdmissionV2::NoPlan { reason })
                    if reason.starts_with("paired throttle")
                        || reason.starts_with("declared burn") =>
                {
                    Some(
                        feedback
                            .map_or(0, |f| f.admission_actuation_checks)
                            .saturating_sub(1)
                            * 2,
                    )
                }
                _ => None,
            };
            json!({"reference":reference,"first_contact":first_contact,"minimum_center_height_m":min_y,"maximum_center_height_m":max_y,"reference_end_physics_step":reference_end,"first_failed_pair_local_tick_from_admission_counter":failed_tick,"assumptions":"reference only; instantaneous requested pointing; zero angular rate; no attitude or plant forecast; native contact remains authoritative"})
        }
        Ok(None) => Value::Null,
        Err(reason) => json!({"construction_error":reason}),
    };
    Ok(
        json!({"schema_version":1,"query_only":true,"native_forecast_steps":0,"admission":admission,"activation":activation,"reference_projection":reference_projection,"incoming_contact":contact,"native_stop":summary.stop,"native_final_state":summary.final_state}),
    )
}
