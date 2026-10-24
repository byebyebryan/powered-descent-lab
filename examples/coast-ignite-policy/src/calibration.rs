//! Independent native calibration and witness recipes. No waypoint guidance is
//! called while constructing or executing a witness.
use super::*;
use pd_control::arrival_actuation::*;
use pd_control::{ArrivalRequest, ArrivalStatus, assess_arrival_handoff};
use pd_core::{
    Command, ContactClassification, RunContext, SimulationState, TransferWaypointSpec, Vec2,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessRecipe {
    pub physics_ticks: u64,
    pub prepare_ticks: u64,
    pub thrust_accel_mps2: Vec2,
}

pub fn witness_command(
    ctx: &RunContext,
    o: &Observation,
    r: &WitnessRecipe,
) -> Result<Command, String> {
    let attitude = r.thrust_accel_mps2.x.atan2(r.thrust_accel_mps2.y);
    let setting = if o.physics_step < r.prepare_ticks || o.physics_step >= r.physics_ticks {
        0.0
    } else {
        pd_control::poly_landing::paired_throttle(ctx, 48, o.mass_kg, r.thrust_accel_mps2.length())?
    };
    Ok(Command {
        throttle_frac: setting,
        target_attitude_rad: attitude,
    })
}

pub struct WitnessMonitor {
    pub request: ArrivalRequest,
    pub anchor: Observation,
    pub status: ArrivalStatus,
    pub window: Option<u64>,
    pub resolution: Option<Observation>,
    pub violations: Vec<String>,
}
impl WitnessMonitor {
    pub fn new(request: ArrivalRequest, anchor: Observation) -> Self {
        Self {
            request,
            anchor,
            status: ArrivalStatus::Active,
            window: None,
            resolution: None,
            violations: vec![],
        }
    }
    pub fn inspect(&mut self, o: &Observation) {
        if self.status != ArrivalStatus::Active {
            return;
        }
        let a = assess_arrival_handoff(&self.request, self.anchor.position_m, o);
        if a.capture_window_open && self.window.is_none() {
            self.window = Some(o.physics_step);
        }
        self.violations = a.violations.iter().map(|v| v.as_str().into()).collect();
        self.status = if a.contract_pass_in_window(self.window.is_some()) {
            ArrivalStatus::Reached
        } else if a.deadline_reached {
            ArrivalStatus::PlaneMiss
        } else if o.physics_step >= self.request.deadline_physics_step {
            ArrivalStatus::Expired
        } else {
            ArrivalStatus::Active
        };
        if self.status != ArrivalStatus::Active {
            self.resolution = Some(o.clone());
        }
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({"backend":"independent_native_witness_v1",
        "status":self.status,"request":self.request,"anchor":self.anchor,"resolution":self.resolution,
        "window_entry_physics_step":self.window,"replans":0,"model_steps":0,"command_updates":0,
        "tilt_saturated_updates":0,"thrust_saturated_updates":0,"deadline_remaining_s":null,"violations":self.violations})
    }
}

#[pyfunction]
pub fn generate_witness(scenario_json: &str, recipe_json: &str) -> PyResult<String> {
    let s: ScenarioSpec = decode(scenario_json)?;
    let r: WitnessRecipe = decode(recipe_json)?;
    if r.physics_ticks == 0
        || r.physics_ticks > 2400
        || !r.physics_ticks.is_multiple_of(2)
        || !r.prepare_ticks.is_multiple_of(2)
        || r.prepare_ticks >= r.physics_ticks
    {
        return Err(error("invalid bounded witness recipe"));
    }
    let ctx = RunContext::from_scenario(&s).map_err(error)?;
    let mut state = SimulationState::new(&ctx).map_err(error)?;
    let mut observations = vec![state.build_observation(&ctx)];
    let mut commands = vec![];
    let mut steps = 0;
    let mut contact = ContactClassification::None;
    while state.physics_step < r.physics_ticks && contact == ContactClassification::None {
        let o = state.build_observation(&ctx);
        let command = witness_command(&ctx, &o, &r).map_err(error)?;
        commands.push((o.physics_step, command));
        state.set_command(command);
        for _ in 0..2 {
            contact = state.step_physics_and_classify_contact(&ctx);
            steps += 1;
            if contact != ContactClassification::None {
                break;
            }
        }
        observations.push(state.build_observation(&ctx));
    }
    let end = state.build_observation(&ctx);
    let speed = end.velocity_mps.length();
    if contact != ContactClassification::None
        || speed < 1.0
        || (end.position_m - ctx.initial_state.position_m).length() <= 30.0
    {
        return json(
            &serde_json::json!({"admitted":false,"reason":"contact, low speed or insufficient displacement","native_steps":steps,"recipe":r}),
        );
    }
    let request = ArrivalRequest {
        deadline_physics_step: 3600,
        waypoint: TransferWaypointSpec {
            id: "witness-arrival-once".into(),
            position_m: end.position_m,
            handoff_tangent_unit: Some(end.velocity_mps * (1.0 / speed)),
            capture_radius_m: 30.0,
            max_cross_track_m: 30.0,
            max_outbound_heading_error_rad: 0.25,
            min_outbound_progress_mps: 0.85 * speed,
            max_outbound_cross_speed_mps: Some(0.25 * speed),
            min_speed_mps: 0.85 * speed,
            max_speed_mps: 1.15 * speed,
            min_vertical_speed_mps: Some(end.velocity_mps.y - 0.15 * speed),
            max_vertical_speed_mps: Some(end.velocity_mps.y + 0.15 * speed),
        },
    };
    request.waypoint.validate().map_err(error)?;
    let mut monitor = WitnessMonitor::new(request.clone(), observations[0].clone());
    for o in &observations {
        monitor.inspect(o);
    }
    json(
        &serde_json::json!({"admitted":monitor.status==ArrivalStatus::Reached,
        "request":request,"recipe":r,"native_steps":steps,"witness_resolution":monitor.json(),
        "commands_sha256":pd_missions::sha256(&serde_json::to_vec(&commands).map_err(error)?),"endpoint":end}),
    )
}

#[pyfunction]
pub fn actuation_bench(scenario_json: &str) -> PyResult<String> {
    let base: ScenarioSpec = decode(scenario_json)?;
    let mut cases = vec![];
    let mut steps = 0;
    for minimum in [0.0, 0.25, 1.0] {
        for fuel in [0.0, 0.05, 0.20625, 0.4, 2000.0, 6300.0] {
            for demand in [0.0, 0.1, 2.0, 4.0, 6.0, 12.0, 30.0] {
                for rotating in [false, true] {
                    for profile in [
                        ArrivalActuationProfile::LegacyV1,
                        ArrivalActuationProfile::NativePairV2,
                    ] {
                        let mut s = base.clone();
                        s.vehicle.min_throttle_frac = minimum;
                        s.initial_state.position_m = Vec2::new(0.0, 800.0);
                        s.initial_state.velocity_mps = Vec2::new(0.0, -10.0);
                        let angle: f64 = if rotating { -0.3 } else { 0.0 };
                        s.initial_state.attitude_rad = 0.0;
                        let ctx = RunContext::from_scenario(&s).map_err(error)?;
                        let mut state = SimulationState::new(&ctx).map_err(error)?;
                        state.fuel_kg = fuel;
                        let before = state.build_observation(&ctx);
                        let request = Vec2::new(demand * angle.sin(), demand * angle.cos());
                        let allocation =
                            allocate_arrival_acceleration(&ctx, &before, request, 0.72, profile)
                                .map_err(error)?;
                        state.set_command(allocation.command);
                        for _ in 0..2 {
                            let _ = state.step_physics_and_classify_contact(&ctx);
                            steps += 1;
                        }
                        let after = state.build_observation(&ctx);
                        let mean = Vec2::new(
                            (after.velocity_mps.x - before.velocity_mps.x) * 60.0,
                            (after.velocity_mps.y - before.velocity_mps.y) * 60.0
                                + ctx.world.gravity_mps2,
                        );
                        if !rotating && (mean.y - allocation.aligned_mean_accel_mps2).abs() > 1e-8 {
                            return Err(error("native aligned forward calibration failed"));
                        }
                        if !rotating
                            && profile == ArrivalActuationProfile::NativePairV2
                            && allocation.regime == AllocationRegime::Tracking
                            && (mean.y - allocation.projected_magnitude_mps2).abs() > 1e-8
                        {
                            return Err(error("native realizable inverse calibration failed"));
                        }
                        cases.push(serde_json::json!({"minimum":minimum,"fuel":fuel,"demand":demand,"rotating":rotating,
                            "profile":profile,"allocation":allocation,"native_mean_accel_mps2":mean,"before":before,"after":after}));
                    }
                }
            }
        }
    }
    json(
        &serde_json::json!({"cases":cases,"native_steps":steps,"passed":true,"aligned_tolerance_mps2":1e-8}),
    )
}
