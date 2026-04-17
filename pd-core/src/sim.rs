use crate::{
    eval::{
        ContactClassification, apply_contact_classification, apply_max_time,
        apply_progress_evaluation,
    },
    math::Vec2,
    model::{
        ActionLogEntry, Command, EndReason, EventKind, EventRecord, MissionOutcome, Observation,
        PhysicalOutcome, RUN_SCHEMA_VERSION, RunArtifacts, RunContext, RunManifest, SampleRecord,
    },
};

#[derive(Debug)]
pub enum SimulationError {
    InvalidContext(String),
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContext(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SimulationError {}

#[derive(Clone, Debug)]
pub struct SimulationState {
    pub sim_time_s: f64,
    pub physics_step: u64,
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub fuel_kg: f64,
    pub held_command: Command,
    pub physical_outcome: PhysicalOutcome,
    pub mission_outcome: MissionOutcome,
    pub end_reason: EndReason,
}

impl SimulationState {
    pub fn new(ctx: &RunContext) -> Result<Self, SimulationError> {
        ctx.sim
            .validate()
            .map_err(SimulationError::InvalidContext)?;
        ctx.world
            .validate()
            .map_err(SimulationError::InvalidContext)?;
        ctx.vehicle
            .validate()
            .map_err(SimulationError::InvalidContext)?;
        ctx.initial_state
            .validate()
            .map_err(SimulationError::InvalidContext)?;

        Ok(Self {
            sim_time_s: 0.0,
            physics_step: 0,
            position_m: ctx.initial_state.position_m,
            velocity_mps: ctx.initial_state.velocity_mps,
            attitude_rad: ctx.initial_state.attitude_rad,
            angular_rate_radps: ctx.initial_state.angular_rate_radps,
            fuel_kg: ctx.vehicle.initial_fuel_kg,
            held_command: Command::idle(),
            physical_outcome: PhysicalOutcome::Flying,
            mission_outcome: MissionOutcome::InProgress,
            end_reason: EndReason::Running,
        })
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self.end_reason, EndReason::Running)
    }

    pub fn set_command(&mut self, command: Command) {
        self.held_command = command.clamped();
    }

    pub fn mass_kg(&self, ctx: &RunContext) -> f64 {
        ctx.vehicle.dry_mass_kg + self.fuel_kg.max(0.0)
    }

    pub fn build_observation(&self, ctx: &RunContext) -> Observation {
        let touchdown_points = self.touchdown_points_world(ctx);
        let touchdown_clearance_m = touchdown_points
            .iter()
            .map(|point| point.y - ctx.world.terrain.sample_height(point.x))
            .fold(f64::INFINITY, f64::min);

        Observation {
            sim_time_s: self.sim_time_s,
            physics_step: self.physics_step,
            position_m: self.position_m,
            velocity_mps: self.velocity_mps,
            attitude_rad: self.attitude_rad,
            angular_rate_radps: self.angular_rate_radps,
            mass_kg: self.mass_kg(ctx),
            fuel_kg: self.fuel_kg,
            gravity_mps2: ctx.world.gravity_mps2,
            target_dx_m: ctx.target_pad.center_x_m - self.position_m.x,
            height_above_target_m: self.position_m.y - ctx.target_pad.surface_y_m,
            target_surface_y_m: ctx.target_pad.surface_y_m,
            target_pad_half_width_m: ctx.target_pad.half_width_m(),
            touchdown_clearance_m,
            min_hull_clearance_m: self.min_hull_clearance_m(ctx),
        }
    }

    pub fn step(&mut self, ctx: &RunContext) -> Vec<EventRecord> {
        if self.is_terminal() {
            return Vec::new();
        }

        let dt_s = ctx.sim.physics_dt_s();
        self.apply_attitude_command(ctx, dt_s);
        let throttle_frac = self.consume_fuel(ctx, dt_s);
        self.integrate_translation(ctx, dt_s, throttle_frac);

        self.physics_step += 1;
        self.sim_time_s = self.physics_step as f64 / f64::from(ctx.sim.physics_hz);

        let contact_events =
            apply_contact_classification(ctx, self, self.detect_contact_classification(ctx));
        if self.is_terminal() {
            return contact_events;
        }

        let progress_events = apply_progress_evaluation(ctx, self);
        if self.is_terminal() {
            return progress_events;
        }

        if self.sim_time_s >= ctx.sim.max_time_s {
            return apply_max_time(self);
        }

        contact_events.into_iter().chain(progress_events).collect()
    }

    fn apply_attitude_command(&mut self, ctx: &RunContext, dt_s: f64) {
        let max_delta = ctx.vehicle.max_rotation_rate_radps * dt_s;
        let delta = shortest_angle_delta(self.attitude_rad, self.held_command.target_attitude_rad);
        let applied_delta = delta.clamp(-max_delta, max_delta);
        self.attitude_rad += applied_delta;
        self.angular_rate_radps = applied_delta / dt_s;
    }

    fn consume_fuel(&mut self, ctx: &RunContext, dt_s: f64) -> f64 {
        let throttle_frac = if self.fuel_kg > 0.0 {
            self.held_command.throttle_frac.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let fuel_used = (ctx.vehicle.max_fuel_burn_kgps * throttle_frac * dt_s).min(self.fuel_kg);
        self.fuel_kg -= fuel_used;
        throttle_frac
    }

    fn integrate_translation(&mut self, ctx: &RunContext, dt_s: f64, throttle_frac: f64) {
        let thrust_n = ctx.vehicle.max_thrust_n * throttle_frac;
        let mass_kg = self.mass_kg(ctx).max(1.0);
        let (sin_a, cos_a) = self.attitude_rad.sin_cos();
        let thrust_accel_mps2 =
            Vec2::new((thrust_n / mass_kg) * sin_a, (thrust_n / mass_kg) * cos_a);
        let total_accel_mps2 = Vec2::new(
            thrust_accel_mps2.x,
            thrust_accel_mps2.y - ctx.world.gravity_mps2,
        );

        self.velocity_mps += total_accel_mps2 * dt_s;
        self.position_m += self.velocity_mps * dt_s;
    }

    fn detect_contact_classification(&self, ctx: &RunContext) -> ContactClassification {
        let touchdown_points = self.touchdown_points_world(ctx);
        let touchdown_clearances_m =
            touchdown_points.map(|point| point.y - ctx.world.terrain.sample_height(point.x));
        let min_touchdown_clearance_m = touchdown_clearances_m
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_touchdown_clearance_m = touchdown_clearances_m
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let min_hull_clearance_m = self.min_hull_clearance_m(ctx);

        if min_touchdown_clearance_m > 0.0 && min_hull_clearance_m > 0.0 {
            return ContactClassification::None;
        }

        let normal_speed_mps = (-self.velocity_mps.y).max(0.0);
        let tangential_speed_mps = self.velocity_mps.x.abs();
        let attitude_error_rad = self.attitude_rad.abs();
        let angular_rate_radps = self.angular_rate_radps.abs();

        let touchdown_x_min = touchdown_points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let touchdown_x_max = touchdown_points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let pad_x_min = ctx.target_pad.center_x_m - ctx.target_pad.half_width_m();
        let pad_x_max = ctx.target_pad.center_x_m + ctx.target_pad.half_width_m();
        let on_target = touchdown_x_min >= pad_x_min && touchdown_x_max <= pad_x_max;
        let stable_touchdown = min_touchdown_clearance_m <= 0.05
            && max_touchdown_clearance_m <= 0.15
            && min_hull_clearance_m >= 0.0;
        let safe_touchdown = normal_speed_mps <= ctx.vehicle.safe_touchdown_normal_speed_mps
            && tangential_speed_mps <= ctx.vehicle.safe_touchdown_tangential_speed_mps
            && attitude_error_rad <= ctx.vehicle.safe_touchdown_attitude_error_rad
            && angular_rate_radps <= ctx.vehicle.safe_touchdown_angular_rate_radps;

        if stable_touchdown && safe_touchdown {
            return ContactClassification::StableTouchdown { on_target };
        }

        ContactClassification::Crash
    }

    fn touchdown_points_world(&self, ctx: &RunContext) -> [Vec2; 2] {
        let geometry = &ctx.vehicle.geometry;
        let left_local = Vec2::new(
            -geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        );
        let right_local = Vec2::new(
            geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        );

        [
            self.position_m + left_local.rotated(self.attitude_rad),
            self.position_m + right_local.rotated(self.attitude_rad),
        ]
    }

    fn hull_vertices_world(&self, ctx: &RunContext) -> [Vec2; 4] {
        let geometry = &ctx.vehicle.geometry;
        let half_w = geometry.hull_width_m * 0.5;
        let half_h = geometry.hull_height_m * 0.5;
        let local = [
            Vec2::new(-half_w, -half_h),
            Vec2::new(half_w, -half_h),
            Vec2::new(half_w, half_h),
            Vec2::new(-half_w, half_h),
        ];
        local.map(|point| self.position_m + point.rotated(self.attitude_rad))
    }

    fn min_hull_clearance_m(&self, ctx: &RunContext) -> f64 {
        self.hull_vertices_world(ctx)
            .iter()
            .map(|point| point.y - ctx.world.terrain.sample_height(point.x))
            .fold(f64::INFINITY, f64::min)
    }
}

pub fn run_simulation<F>(
    ctx: &RunContext,
    controller_id: &str,
    mut controller: F,
) -> Result<RunArtifacts, SimulationError>
where
    F: FnMut(&RunContext, &Observation) -> Command,
{
    let mut state = SimulationState::new(ctx)?;
    let mut actions = Vec::new();
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let control_interval_steps = ctx.sim.control_interval_steps();
    let sample_interval_steps = ctx.sim.sample_interval_steps();
    let mut controller_update_index = 0_u64;

    maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);
    issue_controller_update(
        &mut state,
        &mut controller,
        &mut controller_update_index,
        &mut actions,
        &mut events,
        ctx,
    );

    while !state.is_terminal() {
        events.extend(state.step(ctx));
        maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);

        if state.is_terminal() {
            break;
        }

        if state.physics_step % control_interval_steps == 0 {
            issue_controller_update(
                &mut state,
                &mut controller,
                &mut controller_update_index,
                &mut actions,
                &mut events,
                ctx,
            );
        }
    }

    Ok(RunArtifacts {
        manifest: build_manifest(ctx, controller_id, &state, controller_update_index),
        actions,
        events,
        samples,
    })
}

pub fn replay_simulation(
    ctx: &RunContext,
    controller_id: &str,
    actions: &[ActionLogEntry],
) -> Result<RunArtifacts, SimulationError> {
    if actions.is_empty() {
        return Err(SimulationError::InvalidContext(
            "action log must contain at least one controller update".to_owned(),
        ));
    }

    let mut state = SimulationState::new(ctx)?;
    let mut replay_actions = Vec::with_capacity(actions.len());
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let control_interval_steps = ctx.sim.control_interval_steps();
    let sample_interval_steps = ctx.sim.sample_interval_steps();
    let mut next_action_index = 0_usize;

    maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);
    consume_replay_action(
        ctx,
        &mut state,
        actions,
        &mut next_action_index,
        &mut replay_actions,
        &mut events,
    )?;

    while !state.is_terminal() {
        events.extend(state.step(ctx));
        maybe_push_sample(&mut samples, &state, ctx, sample_interval_steps);

        if state.is_terminal() {
            break;
        }

        if state.physics_step % control_interval_steps == 0 {
            consume_replay_action(
                ctx,
                &mut state,
                actions,
                &mut next_action_index,
                &mut replay_actions,
                &mut events,
            )?;
        }
    }

    if next_action_index != actions.len() {
        return Err(SimulationError::InvalidContext(format!(
            "action log contains {} extra controller updates after termination",
            actions.len() - next_action_index
        )));
    }

    Ok(RunArtifacts {
        manifest: build_manifest(ctx, controller_id, &state, replay_actions.len() as u64),
        actions: replay_actions,
        events,
        samples,
    })
}

fn issue_controller_update<F>(
    state: &mut SimulationState,
    controller: &mut F,
    controller_update_index: &mut u64,
    actions: &mut Vec<ActionLogEntry>,
    events: &mut Vec<EventRecord>,
    ctx: &RunContext,
) where
    F: FnMut(&RunContext, &Observation) -> Command,
{
    let observation = state.build_observation(ctx);
    let command = controller(ctx, &observation).clamped();
    state.set_command(command);

    actions.push(ActionLogEntry {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        controller_update_index: *controller_update_index,
        command: state.held_command,
    });
    events.push(EventRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        kind: EventKind::ControllerUpdated,
        message: "controller_updated".to_owned(),
    });
    *controller_update_index += 1;
}

fn consume_replay_action(
    ctx: &RunContext,
    state: &mut SimulationState,
    actions: &[ActionLogEntry],
    next_action_index: &mut usize,
    replay_actions: &mut Vec<ActionLogEntry>,
    events: &mut Vec<EventRecord>,
) -> Result<(), SimulationError> {
    let Some(action) = actions.get(*next_action_index) else {
        return Err(SimulationError::InvalidContext(format!(
            "action log ended before controller update {} at physics step {}",
            *next_action_index, state.physics_step
        )));
    };

    if action.physics_step != state.physics_step {
        return Err(SimulationError::InvalidContext(format!(
            "action {} expected physics_step {}, got {}",
            *next_action_index, state.physics_step, action.physics_step
        )));
    }
    if action.controller_update_index != *next_action_index as u64 {
        return Err(SimulationError::InvalidContext(format!(
            "action {} expected controller_update_index {}, got {}",
            *next_action_index, *next_action_index, action.controller_update_index
        )));
    }
    if (action.sim_time_s - state.sim_time_s).abs() > 1e-9 {
        return Err(SimulationError::InvalidContext(format!(
            "action {} expected sim_time_s {:.12}, got {:.12}",
            *next_action_index, state.sim_time_s, action.sim_time_s
        )));
    }
    if state.physics_step != 0 && state.physics_step % ctx.sim.control_interval_steps() != 0 {
        return Err(SimulationError::InvalidContext(format!(
            "action {} occurs on invalid control step {}",
            *next_action_index, state.physics_step
        )));
    }

    state.set_command(action.command);
    replay_actions.push(ActionLogEntry {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        controller_update_index: action.controller_update_index,
        command: state.held_command,
    });
    events.push(EventRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        kind: EventKind::ControllerUpdated,
        message: "controller_updated".to_owned(),
    });
    *next_action_index += 1;
    Ok(())
}

fn build_manifest(
    ctx: &RunContext,
    controller_id: &str,
    state: &SimulationState,
    controller_updates: u64,
) -> RunManifest {
    RunManifest {
        schema_version: RUN_SCHEMA_VERSION,
        scenario_id: ctx.scenario_id.clone(),
        scenario_name: ctx.scenario_name.clone(),
        controller_id: controller_id.to_owned(),
        physics_hz: ctx.sim.physics_hz,
        controller_hz: ctx.sim.controller_hz,
        sim_time_s: state.sim_time_s,
        physics_steps: state.physics_step,
        controller_updates,
        physical_outcome: state.physical_outcome.clone(),
        mission_outcome: state.mission_outcome.clone(),
        end_reason: state.end_reason.clone(),
    }
}

fn maybe_push_sample(
    samples: &mut Vec<SampleRecord>,
    state: &SimulationState,
    ctx: &RunContext,
    sample_interval_steps: Option<u64>,
) {
    let Some(sample_interval_steps) = sample_interval_steps else {
        return;
    };
    if state.physics_step % sample_interval_steps != 0 && !state.is_terminal() {
        return;
    }
    if samples
        .last()
        .is_some_and(|sample| sample.physics_step == state.physics_step)
    {
        return;
    }

    samples.push(SampleRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        observation: state.build_observation(ctx),
        held_command: state.held_command,
    });
}

fn shortest_angle_delta(current_rad: f64, target_rad: f64) -> f64 {
    let two_pi = std::f64::consts::TAU;
    let mut delta = (target_rad - current_rad) % two_pi;
    if delta > std::f64::consts::PI {
        delta -= two_pi;
    } else if delta < -std::f64::consts::PI {
        delta += two_pi;
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, VehicleGeometry,
        VehicleInitialState, VehicleSpec, WorldSpec,
    };
    use crate::terrain::TerrainDefinition;

    fn smoke_scenario() -> ScenarioSpec {
        ScenarioSpec {
            id: "smoke".to_owned(),
            name: "Smoke".to_owned(),
            description: "smoke test".to_owned(),
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 10.0,
                sample_hz: Some(10),
            },
            world: WorldSpec {
                gravity_mps2: 1.62,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-50.0, 0.0), Vec2::new(50.0, 0.0)],
                },
                landing_pads: vec![LandingPadSpec {
                    id: "pad_a".to_owned(),
                    center_x_m: 0.0,
                    surface_y_m: 0.0,
                    width_m: 30.0,
                }],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 4.0,
                    hull_height_m: 6.0,
                    touchdown_half_span_m: 2.0,
                    touchdown_base_offset_m: 3.2,
                },
                dry_mass_kg: 700.0,
                initial_fuel_kg: 200.0,
                max_fuel_kg: 200.0,
                max_thrust_n: 14_000.0,
                max_fuel_burn_kgps: 10.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 15.0),
                velocity_mps: Vec2::new(0.0, -5.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            mission: MissionSpec {
                goal: EvaluationGoal::LandingOnPad {
                    target_pad_id: "pad_a".to_owned(),
                },
            },
        }
    }

    #[test]
    fn run_simulation_emits_authoritative_logs() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let artifacts = run_simulation(&ctx, "idle", |_, _| Command::idle()).unwrap();

        assert!(!artifacts.actions.is_empty());
        assert!(!artifacts.events.is_empty());
        assert!(matches!(
            artifacts.manifest.end_reason,
            EndReason::Crash | EndReason::MaxTimeReached
        ));
    }

    #[test]
    fn replay_simulation_reproduces_manifest_and_events() {
        let ctx = RunContext::from_scenario(&smoke_scenario()).unwrap();
        let original = run_simulation(&ctx, "scripted", |_, observation| {
            if observation.height_above_target_m > 10.0 {
                Command {
                    throttle_frac: 0.2,
                    target_attitude_rad: 0.0,
                }
            } else {
                Command {
                    throttle_frac: 0.5,
                    target_attitude_rad: 0.0,
                }
            }
        })
        .unwrap();

        let replayed = replay_simulation(&ctx, "scripted", &original.actions).unwrap();

        assert_eq!(replayed.manifest, original.manifest);
        assert_eq!(replayed.events, original.events);
        assert_eq!(replayed.actions, original.actions);
    }
}
