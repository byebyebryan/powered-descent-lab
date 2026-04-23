use std::collections::BTreeMap;

use pd_core::{Command, Observation, RunContext};
use serde::{Deserialize, Serialize};

use crate::kit::{ControllerFrameBuilder, ControllerView, metric, phase, standard_marker};
use crate::{Controller, ControllerFrame, TelemetryValue};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalPdgControllerConfig {
    pub max_tilt_rad: f64,
    pub terminal_dynamic_tilt_max_rad: f64,
    pub max_tilt_low_alt_rad: f64,
    pub max_tilt_low_alt_far_rad: f64,
    pub low_alt_tilt_alt_m: f64,
    pub low_alt_tilt_dx_m: f64,
    pub low_alt_tilt_vx_mps: f64,
    pub braking_alt_margin_m: f64,
    pub braking_tilt_scale: f64,
    pub braking_accel_safety: f64,
    pub braking_min_speed_mps: f64,
    pub braking_max_speed_mps: f64,
    pub braking_target_ratio: f64,
    pub vy_low_alt_cap_alt_m: f64,
    pub vy_low_alt_cap_mps: f64,
    pub vy_touch_cap_alt_m: f64,
    pub vy_touch_cap_mps: f64,
    #[serde(alias = "touchdown_zero_alt_m")]
    pub touchdown_idle_clearance_m: f64,
    pub touchdown_zero_vx_mps: f64,
    pub touchdown_zero_vy_mps: f64,
    pub touchdown_low_clearance_trigger_m: f64,
    #[serde(alias = "touchdown_rescue_altitude_m")]
    pub touchdown_rescue_clearance_m: f64,
    pub touchdown_rescue_vy_ratio: f64,
    pub touchdown_rescue_tilt_rad: f64,
    pub touchdown_rescue_vx_full_tilt_mps: f64,
    pub touchdown_rescue_dx_full_tilt_m: f64,
    pub touchdown_rescue_vx_margin_mps: f64,
    pub touchdown_rescue_alt_floor_m: f64,
    pub terminal_gate_nominal_ratio: f64,
    pub terminal_gate_nominal_min_up_accel_mps2: f64,
    pub terminal_gate_hysteresis_ticks: u32,
    pub terminal_gate_nominal_buffer_s: f64,
    pub terminal_gate_burn_time_min_s: f64,
    pub terminal_gate_burn_time_max_s: f64,
    pub terminal_gate_burn_time_offset_short_s: f64,
    pub terminal_gate_burn_time_offset_long_s: f64,
    pub terminal_gate_latest_safe_buffer_s: f64,
    pub terminal_gate_latest_safe_aggressive_dx_abs_m: f64,
    pub terminal_gate_latest_safe_aggressive_dx_ratio: f64,
    pub terminal_overshoot_tilt_altitude_min_m: f64,
    pub terminal_overshoot_tilt_projected_dx_abs_m: f64,
    pub terminal_overshoot_tilt_projected_dx_ratio: f64,
    pub terminal_overshoot_tilt_vx_min_mps: f64,
    pub terminal_overshoot_tilt_max_rad: f64,
}

impl Default for TerminalPdgControllerConfig {
    fn default() -> Self {
        Self {
            max_tilt_rad: 0.78,
            terminal_dynamic_tilt_max_rad: 0.95,
            max_tilt_low_alt_rad: 0.18,
            max_tilt_low_alt_far_rad: 0.34,
            low_alt_tilt_alt_m: 20.0,
            low_alt_tilt_dx_m: 12.0,
            low_alt_tilt_vx_mps: 2.4,
            braking_alt_margin_m: 6.0,
            braking_tilt_scale: 1.0,
            braking_accel_safety: 0.58,
            braking_min_speed_mps: 0.8,
            braking_max_speed_mps: 55.0,
            braking_target_ratio: 0.48,
            vy_low_alt_cap_alt_m: 40.0,
            vy_low_alt_cap_mps: 9.5,
            vy_touch_cap_alt_m: 8.0,
            vy_touch_cap_mps: 1.8,
            touchdown_idle_clearance_m: 0.05,
            touchdown_zero_vx_mps: 0.55,
            touchdown_zero_vy_mps: 0.6,
            touchdown_low_clearance_trigger_m: 1.25,
            touchdown_rescue_clearance_m: 4.5,
            touchdown_rescue_vy_ratio: 1.8,
            touchdown_rescue_tilt_rad: 0.42,
            touchdown_rescue_vx_full_tilt_mps: 3.0,
            touchdown_rescue_dx_full_tilt_m: 8.0,
            touchdown_rescue_vx_margin_mps: 0.15,
            touchdown_rescue_alt_floor_m: 0.25,
            terminal_gate_nominal_ratio: 0.92,
            terminal_gate_nominal_min_up_accel_mps2: 0.5,
            terminal_gate_hysteresis_ticks: 2,
            terminal_gate_nominal_buffer_s: 0.4,
            terminal_gate_burn_time_min_s: 3.0,
            terminal_gate_burn_time_max_s: 14.0,
            terminal_gate_burn_time_offset_short_s: 0.8,
            terminal_gate_burn_time_offset_long_s: 0.8,
            terminal_gate_latest_safe_buffer_s: 0.6,
            terminal_gate_latest_safe_aggressive_dx_abs_m: 24.0,
            terminal_gate_latest_safe_aggressive_dx_ratio: 1.25,
            terminal_overshoot_tilt_altitude_min_m: 35.0,
            terminal_overshoot_tilt_projected_dx_abs_m: 28.0,
            terminal_overshoot_tilt_projected_dx_ratio: 2.0,
            terminal_overshoot_tilt_vx_min_mps: 8.0,
            terminal_overshoot_tilt_max_rad: 1.22,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GuidanceMode {
    NominalPending,
    NominalReady,
    LatestSafe,
}

impl GuidanceMode {
    fn label(self) -> &'static str {
        match self {
            Self::NominalPending => "nominal pending",
            Self::NominalReady => "nominal ready",
            Self::LatestSafe => "latest safe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BallisticProjection {
    projected_dx_m: f64,
    time_to_cross_s: f64,
    has_target_y_solution: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TerminalGateCandidate {
    burn_time_s: f64,
    required_accel_ratio: f64,
    upward_accel_mps2: f64,
    tilt_feasible: bool,
    ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TerminalCommandState {
    mode: GuidanceMode,
    candidate: TerminalGateCandidate,
    projected_dx_m: f64,
    projected_time_s: f64,
    has_target_y_solution: bool,
    desired_vertical_speed_mps: f64,
    target_attitude_rad: f64,
    throttle_frac: f64,
    max_tilt_rad: f64,
    latest_safe_margin_s: f64,
}

#[derive(Debug)]
pub struct TerminalPdgController {
    config: TerminalPdgControllerConfig,
    last_phase: Option<String>,
    last_mode: Option<GuidanceMode>,
    nominal_ready_ticks: u32,
}

impl Default for TerminalPdgController {
    fn default() -> Self {
        Self::new(TerminalPdgControllerConfig::default())
    }
}

impl TerminalPdgController {
    pub fn new(config: TerminalPdgControllerConfig) -> Self {
        Self {
            config,
            last_phase: None,
            last_mode: None,
            nominal_ready_ticks: 0,
        }
    }

    fn phase_for_height(&self, height_above_target_m: f64) -> &'static str {
        if height_above_target_m > self.config.vy_low_alt_cap_alt_m {
            phase::DESCENT
        } else if height_above_target_m > self.config.vy_touch_cap_alt_m {
            phase::FLARE
        } else {
            phase::TOUCHDOWN
        }
    }

    fn compute_command_state(&mut self, view: &ControllerView<'_>) -> TerminalCommandState {
        let dx_m = view.target_dx_m();
        let height_above_target_m = view.height_above_target_m().max(0.0);
        let touchdown_clearance_m = view.touchdown_clearance_m();
        let dy_m = -height_above_target_m;
        let vx_mps = view.observation.velocity_mps.x;
        let vy_up_mps = view.observation.velocity_mps.y;
        let max_thrust_accel_mps2 =
            view.ctx.vehicle.max_thrust_n / view.observation.mass_kg.max(1.0);
        let nominal_thrust_accel_mps2 =
            max_thrust_accel_mps2 * self.config.terminal_gate_nominal_ratio;
        let gravity_mps2 = view.observation.gravity_mps2;
        let projection = estimate_target_y_projection(
            dx_m,
            dy_m,
            vx_mps,
            vy_up_mps,
            view.observation.position_m.y,
            gravity_mps2,
        );
        let lateral_dx_m = projection.projected_dx_m;

        let latest_safe = self.latest_safe_candidate(
            dx_m,
            dy_m,
            touchdown_clearance_m,
            lateral_dx_m,
            vx_mps,
            vy_up_mps,
            max_thrust_accel_mps2,
            gravity_mps2,
            view.observation.target_pad_half_width_m,
        );
        let nominal = self.best_nominal_candidate(
            dx_m,
            dy_m,
            touchdown_clearance_m,
            lateral_dx_m,
            vx_mps,
            vy_up_mps,
            max_thrust_accel_mps2,
            nominal_thrust_accel_mps2,
            gravity_mps2,
            view.observation.target_pad_half_width_m,
        );

        if nominal.ready {
            self.nominal_ready_ticks = self.nominal_ready_ticks.saturating_add(1);
        } else {
            self.nominal_ready_ticks = 0;
        }

        let guidance_mode = if latest_safe.latest_safe_margin_s <= 0.0 {
            GuidanceMode::LatestSafe
        } else if nominal.ready
            && self.nominal_ready_ticks >= self.config.terminal_gate_hysteresis_ticks
        {
            GuidanceMode::NominalReady
        } else {
            GuidanceMode::NominalPending
        };

        let selected = match guidance_mode {
            GuidanceMode::LatestSafe => latest_safe.best_candidate,
            GuidanceMode::NominalReady | GuidanceMode::NominalPending => nominal.best_candidate,
        };

        let max_tilt_rad = self.resolve_max_tilt(
            touchdown_clearance_m,
            dx_m,
            vx_mps,
            dy_m,
            vy_up_mps,
            max_thrust_accel_mps2,
            lateral_dx_m,
            gravity_mps2,
            view.observation.target_pad_half_width_m,
        );
        let desired_vertical_speed_mps = self.desired_terminal_vertical_speed(
            touchdown_clearance_m,
            max_thrust_accel_mps2,
            max_tilt_rad,
            gravity_mps2,
        );
        let (ax_req, ay_req) = required_control_accel(
            dx_m,
            dy_m,
            vx_mps,
            vy_up_mps,
            0.0,
            desired_vertical_speed_mps,
            selected.burn_time_s,
            view.observation.gravity_mps2,
        );
        let (throttle_frac, target_attitude_rad) =
            self.allocate_command(ax_req, ay_req, max_thrust_accel_mps2, max_tilt_rad);

        TerminalCommandState {
            mode: guidance_mode,
            candidate: selected,
            projected_dx_m: projection.projected_dx_m,
            projected_time_s: projection.time_to_cross_s,
            has_target_y_solution: projection.has_target_y_solution,
            desired_vertical_speed_mps,
            target_attitude_rad,
            throttle_frac,
            max_tilt_rad,
            latest_safe_margin_s: latest_safe.latest_safe_margin_s,
        }
    }

    fn braking_speed_limit(
        &self,
        altitude_m: f64,
        max_thrust_accel_mps2: f64,
        max_tilt_rad: f64,
        gravity_mps2: f64,
    ) -> f64 {
        let alt_eff = (altitude_m - self.config.braking_alt_margin_m).max(0.0);
        let tilt_eff = self.config.braking_tilt_scale * max_tilt_rad;
        let vertical_brake = (self.config.braking_accel_safety
            * ((max_thrust_accel_mps2 * tilt_eff.cos()) - gravity_mps2))
            .max(0.7);
        let speed = ((self.config.vy_touch_cap_mps * self.config.vy_touch_cap_mps)
            + (2.0 * vertical_brake * alt_eff))
            .max(0.0)
            .sqrt();
        speed.clamp(
            self.config.braking_min_speed_mps,
            self.config.braking_max_speed_mps,
        )
    }

    fn desired_terminal_vertical_speed(
        &self,
        altitude_m: f64,
        max_thrust_accel_mps2: f64,
        max_tilt_rad: f64,
        gravity_mps2: f64,
    ) -> f64 {
        let mut vy_mag = self.config.braking_target_ratio
            * self.braking_speed_limit(
                altitude_m,
                max_thrust_accel_mps2,
                max_tilt_rad,
                gravity_mps2,
            );
        if altitude_m <= self.config.vy_low_alt_cap_alt_m {
            vy_mag = vy_mag.min(self.config.vy_low_alt_cap_mps);
        }
        if altitude_m <= self.config.vy_touch_cap_alt_m {
            vy_mag = vy_mag.min(self.config.vy_touch_cap_mps);
        }
        -vy_mag.max(self.config.braking_min_speed_mps)
    }

    fn terminal_lateral_correction_time(
        &self,
        dx_m: f64,
        vx_mps: f64,
        lateral_accel_mps2: f64,
    ) -> f64 {
        let accel = lateral_accel_mps2.max(1e-3);
        if dx_m.abs() <= 1e-6 && vx_mps.abs() <= 1e-6 {
            return 0.0;
        }
        let t_stop = vx_mps.abs() / accel;
        let x_stop = 0.5 * vx_mps * t_stop;
        let residual_dx = dx_m - x_stop;
        let t_translate = if residual_dx.abs() <= 1e-6 {
            0.0
        } else {
            2.0 * (residual_dx.abs() / accel).sqrt()
        };
        t_stop + t_translate
    }

    fn resolve_static_max_tilt(&self, altitude_m: f64, dx_m: f64, vx_mps: f64) -> f64 {
        if altitude_m < self.config.low_alt_tilt_alt_m {
            if dx_m.abs() <= self.config.low_alt_tilt_dx_m
                && vx_mps.abs() <= self.config.low_alt_tilt_vx_mps
            {
                self.config.max_tilt_low_alt_rad
            } else {
                self.config.max_tilt_low_alt_far_rad
            }
        } else {
            self.config.max_tilt_rad
        }
    }

    fn terminal_tilt_is_recoverable(
        &self,
        tilt_rad: f64,
        height_to_target_m: f64,
        lateral_dx_m: f64,
        vx_mps: f64,
        vy_up_mps: f64,
        max_thrust_accel_mps2: f64,
        gravity_mps2: f64,
    ) -> bool {
        if height_to_target_m <= 0.0 {
            return false;
        }
        let lateral_accel = (max_thrust_accel_mps2 * tilt_rad.sin()).max(0.5);
        let side_burn_time =
            self.terminal_lateral_correction_time(lateral_dx_m, vx_mps, lateral_accel);
        let upward_accel = max_thrust_accel_mps2 * tilt_rad.cos();
        let net_vertical_accel = upward_accel - gravity_mps2;
        let height_after = height_to_target_m
            + (vy_up_mps * side_burn_time)
            + (0.5 * net_vertical_accel * side_burn_time * side_burn_time);
        if !height_after.is_finite() || height_after <= 0.0 {
            return false;
        }
        let vy_after = vy_up_mps + (net_vertical_accel * side_burn_time);
        let down_speed_after = (-vy_after).max(0.0);
        let recover_tilt = self.resolve_static_max_tilt(height_after, 0.0, 0.0);
        let recover_limit = self.braking_speed_limit(
            height_after,
            max_thrust_accel_mps2,
            recover_tilt,
            gravity_mps2,
        );
        down_speed_after <= recover_limit
    }

    fn terminal_overshoot_tilt_cap(
        &self,
        altitude_m: f64,
        dx_m: f64,
        lateral_dx_m: f64,
        vx_mps: f64,
        target_half_width_m: f64,
    ) -> Option<f64> {
        if altitude_m < self.config.terminal_overshoot_tilt_altitude_min_m {
            return None;
        }
        if dx_m.abs() <= 1e-3 || vx_mps.abs() < self.config.terminal_overshoot_tilt_vx_min_mps {
            return None;
        }
        if dx_m * vx_mps <= 0.0 {
            return None;
        }
        if dx_m * lateral_dx_m >= 0.0 {
            return None;
        }

        let projected_dx_threshold = self
            .config
            .terminal_overshoot_tilt_projected_dx_abs_m
            .max(self.config.terminal_overshoot_tilt_projected_dx_ratio * target_half_width_m);
        let projected_dx_abs = lateral_dx_m.abs();
        if projected_dx_abs <= projected_dx_threshold {
            return None;
        }

        let base_cap = self.config.terminal_dynamic_tilt_max_rad.max(0.0);
        let overshoot_cap = base_cap.max(self.config.terminal_overshoot_tilt_max_rad);
        if overshoot_cap <= base_cap + 1e-6 {
            return None;
        }
        let severity = ((projected_dx_abs - projected_dx_threshold)
            / projected_dx_threshold.max(1e-3))
        .clamp(0.0, 1.0);
        Some(base_cap + (severity * (overshoot_cap - base_cap)))
    }

    fn resolve_max_tilt(
        &self,
        altitude_m: f64,
        dx_m: f64,
        vx_mps: f64,
        dy_m: f64,
        vy_up_mps: f64,
        max_thrust_accel_mps2: f64,
        lateral_dx_m: f64,
        gravity_mps2: f64,
        target_half_width_m: f64,
    ) -> f64 {
        let base_tilt = self.resolve_static_max_tilt(altitude_m, dx_m, vx_mps);
        let mut dynamic_cap = base_tilt.max(self.config.terminal_dynamic_tilt_max_rad);
        if let Some(overshoot_cap) = self.terminal_overshoot_tilt_cap(
            altitude_m,
            dx_m,
            lateral_dx_m,
            vx_mps,
            target_half_width_m,
        ) {
            dynamic_cap = dynamic_cap.max(overshoot_cap);
        }
        if dynamic_cap <= base_tilt + 1e-6 {
            return base_tilt;
        }

        let height_to_target_m = (-dy_m).max(0.0);
        if !self.terminal_tilt_is_recoverable(
            base_tilt,
            height_to_target_m,
            lateral_dx_m,
            vx_mps,
            vy_up_mps,
            max_thrust_accel_mps2,
            gravity_mps2,
        ) {
            return base_tilt;
        }
        let mut lo = base_tilt;
        let mut hi = dynamic_cap;
        if self.terminal_tilt_is_recoverable(
            hi,
            height_to_target_m,
            lateral_dx_m,
            vx_mps,
            vy_up_mps,
            max_thrust_accel_mps2,
            gravity_mps2,
        ) {
            return hi;
        }
        for _ in 0..8 {
            let mid = 0.5 * (lo + hi);
            if self.terminal_tilt_is_recoverable(
                mid,
                height_to_target_m,
                lateral_dx_m,
                vx_mps,
                vy_up_mps,
                max_thrust_accel_mps2,
                gravity_mps2,
            ) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    fn candidate_times(
        &self,
        dx_m: f64,
        dy_m: f64,
        altitude_m: f64,
        vx_mps: f64,
        vy_up_mps: f64,
        thrust_accel_mps2: f64,
        max_thrust_accel_mps2: f64,
        lateral_dx_m: f64,
        gravity_mps2: f64,
        target_half_width_m: f64,
        include_max_time: bool,
    ) -> (Vec<f64>, f64, f64) {
        let max_tilt = self.resolve_max_tilt(
            altitude_m,
            dx_m,
            vx_mps,
            dy_m,
            vy_up_mps,
            max_thrust_accel_mps2,
            lateral_dx_m,
            gravity_mps2,
            target_half_width_m,
        );
        let target_vy_up = self.desired_terminal_vertical_speed(
            altitude_m,
            thrust_accel_mps2,
            max_tilt,
            gravity_mps2,
        );
        let down_speed = (-vy_up_mps).max(0.0);
        let target_down_speed = (-target_vy_up).max(0.0);
        let vertical_up_accel = ((thrust_accel_mps2 * max_tilt.cos()) - gravity_mps2).max(0.1);
        let lateral_accel = (thrust_accel_mps2 * max_tilt.sin()).max(0.5);
        let t_v_nom = (down_speed - target_down_speed).max(0.0) / vertical_up_accel;
        let t_x_nom = vx_mps.abs() / lateral_accel;
        let burn_time_nom = (t_v_nom.max(t_x_nom) + self.config.terminal_gate_nominal_buffer_s)
            .clamp(
                self.config.terminal_gate_burn_time_min_s,
                self.config.terminal_gate_burn_time_max_s,
            );
        let mut candidate_times = Vec::new();
        for raw in [
            burn_time_nom - self.config.terminal_gate_burn_time_offset_short_s,
            burn_time_nom,
            burn_time_nom + self.config.terminal_gate_burn_time_offset_long_s,
        ] {
            let t = raw.clamp(
                self.config.terminal_gate_burn_time_min_s,
                self.config.terminal_gate_burn_time_max_s,
            );
            if !candidate_times
                .iter()
                .any(|existing| (existing - t).abs() <= 1e-6)
            {
                candidate_times.push(t);
            }
        }
        if include_max_time {
            let max_time = self.config.terminal_gate_burn_time_max_s;
            if !candidate_times
                .iter()
                .any(|existing| (existing - max_time).abs() <= 1e-6)
            {
                candidate_times.push(max_time);
            }
        }
        candidate_times.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap());
        (candidate_times, max_tilt, target_vy_up)
    }

    fn evaluate_candidate(
        &self,
        dx_m: f64,
        dy_m: f64,
        vx_mps: f64,
        vy_up_mps: f64,
        burn_time_s: f64,
        target_vy_up_mps: f64,
        max_tilt_rad: f64,
        thrust_accel_mps2: f64,
        ratio_limit: f64,
        min_upward_accel_mps2: f64,
        gravity_mps2: f64,
    ) -> TerminalGateCandidate {
        let (ax_req, ay_req) = required_control_accel(
            dx_m,
            dy_m,
            vx_mps,
            vy_up_mps,
            0.0,
            target_vy_up_mps,
            burn_time_s,
            gravity_mps2,
        );
        let required_norm = (ax_req * ax_req + ay_req * ay_req).sqrt();
        let required_ratio = required_norm / thrust_accel_mps2.max(1e-6);
        let tilt_tan = max_tilt_rad.max(0.02).tan();
        let tilt_feasible = ay_req > 0.0 && ax_req.abs() <= (tilt_tan * ay_req);
        let ready =
            ay_req >= min_upward_accel_mps2 && tilt_feasible && required_ratio <= ratio_limit;
        TerminalGateCandidate {
            burn_time_s,
            required_accel_ratio: required_ratio,
            upward_accel_mps2: ay_req,
            tilt_feasible,
            ready,
        }
    }

    fn latest_safe_candidate(
        &self,
        dx_m: f64,
        dy_m: f64,
        altitude_m: f64,
        lateral_dx_m: f64,
        vx_mps: f64,
        vy_up_mps: f64,
        max_thrust_accel_mps2: f64,
        gravity_mps2: f64,
        target_half_width_m: f64,
    ) -> LatestSafeState {
        let height_to_target_m = (-dy_m).max(0.0);
        let down_speed = (-vy_up_mps).max(0.0);
        let max_tilt = self.resolve_max_tilt(
            altitude_m,
            dx_m,
            vx_mps,
            dy_m,
            vy_up_mps,
            max_thrust_accel_mps2,
            lateral_dx_m,
            gravity_mps2,
            target_half_width_m,
        );
        let vertical_up_accel = ((max_thrust_accel_mps2 * max_tilt.cos()) - gravity_mps2).max(0.1);
        let lateral_accel = (max_thrust_accel_mps2 * max_tilt.sin()).max(0.5);
        let time_to_impact =
            estimate_ground_time_to_impact(height_to_target_m, vy_up_mps, gravity_mps2);
        let (candidate_times, max_tilt, target_vy_up) = self.candidate_times(
            dx_m,
            dy_m,
            altitude_m,
            vx_mps,
            vy_up_mps,
            max_thrust_accel_mps2,
            max_thrust_accel_mps2,
            lateral_dx_m,
            gravity_mps2,
            target_half_width_m,
            true,
        );
        let t_brake_v = down_speed / vertical_up_accel;
        let t_brake_x = self.terminal_lateral_correction_time(lateral_dx_m, vx_mps, lateral_accel);

        let mut candidates: Vec<TerminalGateCandidate> = candidate_times
            .into_iter()
            .map(|burn_time_s| {
                self.evaluate_candidate(
                    dx_m,
                    dy_m,
                    vx_mps,
                    vy_up_mps,
                    burn_time_s,
                    target_vy_up,
                    max_tilt,
                    max_thrust_accel_mps2,
                    1.0,
                    0.0,
                    gravity_mps2,
                )
            })
            .collect();
        let best_candidate =
            self.select_latest_safe_candidate(&mut candidates, lateral_dx_m, target_half_width_m);

        LatestSafeState {
            latest_safe_margin_s: time_to_impact
                - (t_brake_v.max(t_brake_x) + self.config.terminal_gate_latest_safe_buffer_s),
            best_candidate,
        }
    }

    fn best_nominal_candidate(
        &self,
        dx_m: f64,
        dy_m: f64,
        altitude_m: f64,
        lateral_dx_m: f64,
        vx_mps: f64,
        vy_up_mps: f64,
        max_thrust_accel_mps2: f64,
        nominal_thrust_accel_mps2: f64,
        gravity_mps2: f64,
        target_half_width_m: f64,
    ) -> NominalState {
        let (candidate_times, max_tilt, target_vy_up) = self.candidate_times(
            dx_m,
            dy_m,
            altitude_m,
            vx_mps,
            vy_up_mps,
            nominal_thrust_accel_mps2,
            max_thrust_accel_mps2,
            lateral_dx_m,
            gravity_mps2,
            target_half_width_m,
            false,
        );
        let mut candidates: Vec<TerminalGateCandidate> = candidate_times
            .into_iter()
            .map(|burn_time_s| {
                self.evaluate_candidate(
                    dx_m,
                    dy_m,
                    vx_mps,
                    vy_up_mps,
                    burn_time_s,
                    target_vy_up,
                    max_tilt,
                    nominal_thrust_accel_mps2,
                    self.config.terminal_gate_nominal_ratio,
                    self.config.terminal_gate_nominal_min_up_accel_mps2,
                    gravity_mps2,
                )
            })
            .collect();
        candidates.sort_by(candidate_preference_order);
        let best_candidate = candidates
            .iter()
            .copied()
            .find(|candidate| candidate.ready)
            .unwrap_or_else(|| candidates[0]);
        NominalState {
            best_candidate,
            ready: candidates.iter().any(|candidate| candidate.ready),
        }
    }

    fn allocate_command(
        &self,
        ax_req_mps2: f64,
        ay_req_mps2: f64,
        max_thrust_accel_mps2: f64,
        max_tilt_rad: f64,
    ) -> (f64, f64) {
        let ay = ay_req_mps2.clamp(0.0, max_thrust_accel_mps2);
        let tilt_tan = max_tilt_rad.tan();
        let ax = ax_req_mps2.clamp(-tilt_tan * ay.max(0.2), tilt_tan * ay.max(0.2));
        let thrust_accel = (ax * ax + ay * ay).sqrt();
        let throttle_frac = (thrust_accel / max_thrust_accel_mps2.max(1e-6)).clamp(0.0, 1.0);
        let target_attitude_rad = ax.atan2(ay.max(0.2)).clamp(-max_tilt_rad, max_tilt_rad);
        (throttle_frac, target_attitude_rad)
    }

    fn touchdown_cut_command(
        &self,
        view: &ControllerView<'_>,
        current_state: &TerminalCommandState,
    ) -> Option<Command> {
        let touchdown_clearance_m = view.touchdown_clearance_m();
        let dx_m = view.target_dx_m();
        let vx_mps = view.observation.velocity_mps.x;
        let vy_up_mps = view.observation.velocity_mps.y;
        let down_speed = (-vy_up_mps).max(0.0);
        let on_pad = dx_m.abs() <= view.observation.target_pad_half_width_m;
        let settle_cut = touchdown_clearance_m <= self.config.touchdown_rescue_clearance_m.min(1.0)
            && on_pad
            && vx_mps.abs() <= self.config.touchdown_zero_vx_mps
            && down_speed <= self.config.touchdown_zero_vy_mps
            && vy_up_mps >= -0.05;
        if settle_cut {
            return Some(Command::idle());
        }
        let low_clearance = touchdown_clearance_m <= self.config.touchdown_low_clearance_trigger_m;
        let rescue_limit = self.braking_speed_limit(
            touchdown_clearance_m,
            view.ctx.vehicle.max_thrust_n / view.observation.mass_kg.max(1.0),
            current_state.max_tilt_rad,
            view.observation.gravity_mps2,
        );
        let low_clearance_trigger = low_clearance && down_speed > self.config.touchdown_zero_vy_mps;
        if touchdown_clearance_m <= self.config.touchdown_rescue_clearance_m
            && (down_speed > (self.config.touchdown_rescue_vy_ratio * rescue_limit)
                || low_clearance_trigger)
        {
            let safe_touchdown_vx_mps = view
                .ctx
                .vehicle
                .safe_touchdown_tangential_speed_mps
                .max(0.0);
            let inside_pad_safe_vx_mps =
                (safe_touchdown_vx_mps - self.config.touchdown_rescue_vx_margin_mps).max(0.0);
            let rescue_tilt_limit = current_state
                .max_tilt_rad
                .min(self.config.touchdown_rescue_tilt_rad)
                .max(0.0);
            let moving_toward_target = (dx_m * vx_mps) > 0.0;
            let inside_pad = dx_m.abs() <= view.observation.target_pad_half_width_m;
            let vx_term = if inside_pad {
                let vx_excess = (vx_mps.abs() - inside_pad_safe_vx_mps).max(0.0);
                let vx_full_tilt_excess = (self.config.touchdown_rescue_vx_full_tilt_mps
                    - inside_pad_safe_vx_mps)
                    .max(1e-3);
                (vx_excess / vx_full_tilt_excess).clamp(0.0, 1.0)
            } else {
                (vx_mps.abs() / self.config.touchdown_rescue_vx_full_tilt_mps.max(1e-3))
                    .clamp(0.0, 1.0)
            };
            let dx_term = if inside_pad {
                0.0
            } else {
                (dx_m.abs() / self.config.touchdown_rescue_dx_full_tilt_m.max(1e-3)).clamp(0.0, 1.0)
            };
            let rescue_sign = if vx_mps.abs() > self.config.touchdown_zero_vx_mps
                && (moving_toward_target || inside_pad)
            {
                -vx_mps.signum()
            } else if !inside_pad && dx_m.abs() > 1e-3 {
                dx_m.signum()
            } else if vx_mps.abs() > self.config.touchdown_zero_vx_mps {
                -vx_mps.signum()
            } else {
                0.0
            };
            let rescue_weight = if rescue_sign == -vx_mps.signum() {
                vx_term.max(0.5 * dx_term)
            } else {
                dx_term.max(0.25 * vx_term)
            };
            let rescue_angle_target = rescue_sign * rescue_tilt_limit * rescue_weight;
            let mass = view.observation.mass_kg.max(0.5);
            let alt_eff = if low_clearance {
                touchdown_clearance_m.max(self.config.touchdown_rescue_alt_floor_m)
            } else {
                (touchdown_clearance_m - self.config.touchdown_idle_clearance_m)
                    .max(self.config.touchdown_rescue_alt_floor_m)
            };
            let target_down_speed = if low_clearance {
                self.config.touchdown_zero_vy_mps
            } else {
                self.config.vy_touch_cap_mps
            };
            let v_excess = (down_speed - target_down_speed).max(0.0);
            let required_net_brake = (v_excess * v_excess) / (2.0 * alt_eff.max(1e-6));
            let required_ay = view.observation.gravity_mps2 + required_net_brake;
            let required_accel = required_ay / rescue_angle_target.cos().max(0.2);
            let throttle_frac =
                (required_accel / (view.ctx.vehicle.max_thrust_n / mass).max(1e-6)).clamp(0.0, 1.0);
            return Some(Command {
                throttle_frac,
                target_attitude_rad: rescue_angle_target,
            });
        }
        None
    }

    fn select_latest_safe_candidate(
        &self,
        candidates: &mut [TerminalGateCandidate],
        lateral_dx_m: f64,
        target_half_width_m: f64,
    ) -> TerminalGateCandidate {
        let aggressive_dx_threshold = self
            .config
            .terminal_gate_latest_safe_aggressive_dx_abs_m
            .max(self.config.terminal_gate_latest_safe_aggressive_dx_ratio * target_half_width_m);
        let urgent_lateral_recovery = lateral_dx_m.abs() > aggressive_dx_threshold;
        if urgent_lateral_recovery {
            candidates.sort_by(aggressive_latest_safe_preference_order);
            candidates
                .iter()
                .copied()
                .find(|candidate| candidate.tilt_feasible)
                .unwrap_or_else(|| candidates[0])
        } else {
            candidates.sort_by(latest_safe_preference_order);
            candidates
                .iter()
                .copied()
                .find(|candidate| candidate.tilt_feasible && candidate.required_accel_ratio <= 1.0)
                .unwrap_or_else(|| candidates[0])
        }
    }
}

impl Controller for TerminalPdgController {
    fn id(&self) -> &str {
        "terminal_pdg_v1"
    }

    fn reset(&mut self, _ctx: &RunContext) {
        self.last_phase = None;
        self.last_mode = None;
        self.nominal_ready_ticks = 0;
    }

    fn update(&mut self, ctx: &RunContext, observation: &Observation) -> ControllerFrame {
        let view = ControllerView::new(ctx, observation);
        let planning_height_m = view.height_above_target_m().max(0.0);
        let phase = self.phase_for_height(planning_height_m).to_owned();
        let command_state = self.compute_command_state(&view);
        let status = match command_state.mode {
            GuidanceMode::NominalReady => "terminal pdg nominal",
            GuidanceMode::NominalPending => "terminal pdg trimming into envelope",
            GuidanceMode::LatestSafe => "terminal pdg latest safe brake",
        };
        let command = self
            .touchdown_cut_command(&view, &command_state)
            .unwrap_or(Command {
                throttle_frac: command_state.throttle_frac,
                target_attitude_rad: command_state.target_attitude_rad,
            });

        let mut builder = ControllerFrameBuilder::new(command)
            .status(status)
            .phase(phase.clone())
            .standard_kinematics(&view)
            .phase_transition_marker(self.last_phase.as_deref(), &phase, &view)
            .metric(metric::GUIDANCE_ACTIVE, true)
            .metric(
                metric::DESIRED_VERTICAL_SPEED_MPS,
                command_state.desired_vertical_speed_mps,
            )
            .metric(
                metric::DESIRED_ATTITUDE_RAD,
                command_state.target_attitude_rad,
            )
            .metric(metric::PROJECTED_DX_M, command_state.projected_dx_m)
            .metric(metric::PROJECTED_TIME_S, command_state.projected_time_s)
            .metric(metric::GUIDANCE_MODE, command_state.mode.label())
            .metric(
                metric::GUIDANCE_BURN_TIME_S,
                command_state.candidate.burn_time_s,
            )
            .metric(
                metric::GUIDANCE_REQUIRED_ACCEL_RATIO,
                command_state.candidate.required_accel_ratio,
            )
            .metric(metric::GUIDANCE_MAX_TILT_RAD, command_state.max_tilt_rad)
            .metric(
                metric::GUIDANCE_LATEST_SAFE_MARGIN_S,
                command_state.latest_safe_margin_s,
            )
            .metric(
                metric::VERTICAL_ERROR_MPS,
                command_state.desired_vertical_speed_mps - view.vertical_speed_mps(),
            )
            .metric(metric::LATERAL_ERROR_MPS, -view.observation.velocity_mps.x)
            .metric(metric::HOVER_THROTTLE, view.hover_throttle_frac());

        if self.last_mode != Some(command_state.mode) {
            builder = builder.marker(standard_marker(
                crate::kit::marker::TERMINAL_GATE,
                &format!("terminal gate: {}", command_state.mode.label()),
                &view,
                BTreeMap::from([
                    ("kind".to_owned(), TelemetryValue::from("gate")),
                    (
                        "guidance_mode".to_owned(),
                        TelemetryValue::from(command_state.mode.label()),
                    ),
                    (
                        metric::PROJECTED_DX_M.to_owned(),
                        TelemetryValue::from(command_state.projected_dx_m),
                    ),
                    (
                        metric::GUIDANCE_REQUIRED_ACCEL_RATIO.to_owned(),
                        TelemetryValue::from(command_state.candidate.required_accel_ratio),
                    ),
                ]),
            ));
        }

        self.last_phase = Some(phase);
        self.last_mode = Some(command_state.mode);
        builder.build()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LatestSafeState {
    latest_safe_margin_s: f64,
    best_candidate: TerminalGateCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NominalState {
    best_candidate: TerminalGateCandidate,
    ready: bool,
}

fn candidate_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    (
        !lhs.tilt_feasible,
        OrderedF64(lhs.required_accel_ratio),
        OrderedF64(lhs.burn_time_s),
    )
        .cmp(&(
            !rhs.tilt_feasible,
            OrderedF64(rhs.required_accel_ratio),
            OrderedF64(rhs.burn_time_s),
        ))
}

fn latest_safe_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    (
        !lhs.tilt_feasible,
        lhs.required_accel_ratio > 1.0,
        OrderedF64(lhs.burn_time_s),
        OrderedF64(lhs.required_accel_ratio),
    )
        .cmp(&(
            !rhs.tilt_feasible,
            rhs.required_accel_ratio > 1.0,
            OrderedF64(rhs.burn_time_s),
            OrderedF64(rhs.required_accel_ratio),
        ))
}

fn aggressive_latest_safe_preference_order(
    lhs: &TerminalGateCandidate,
    rhs: &TerminalGateCandidate,
) -> std::cmp::Ordering {
    (
        !lhs.tilt_feasible,
        OrderedF64(lhs.burn_time_s),
        OrderedF64(lhs.required_accel_ratio),
    )
        .cmp(&(
            !rhs.tilt_feasible,
            OrderedF64(rhs.burn_time_s),
            OrderedF64(rhs.required_accel_ratio),
        ))
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OrderedF64(f64);

impl Eq for OrderedF64 {}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgent_lateral_latest_safe_prefers_shorter_candidate() {
        let controller = TerminalPdgController::default();
        let mut candidates = vec![
            TerminalGateCandidate {
                burn_time_s: 14.0,
                required_accel_ratio: 0.9,
                upward_accel_mps2: 3.0,
                tilt_feasible: true,
                ready: true,
            },
            TerminalGateCandidate {
                burn_time_s: 6.0,
                required_accel_ratio: 1.2,
                upward_accel_mps2: 5.0,
                tilt_feasible: true,
                ready: false,
            },
        ];

        let selected = controller.select_latest_safe_candidate(&mut candidates, 32.0, 18.0);

        assert_eq!(selected.burn_time_s, 6.0);
        assert_eq!(selected.required_accel_ratio, 1.2);
    }
}

fn required_control_accel(
    dx_m: f64,
    dy_m: f64,
    vx_mps: f64,
    vy_up_mps: f64,
    target_vx_mps: f64,
    target_vy_up_mps: f64,
    time_to_go_s: f64,
    gravity_mps2: f64,
) -> (f64, f64) {
    let t = time_to_go_s.max(1e-3);
    let t2 = t * t;
    let zem_x = dx_m - (vx_mps * t);
    let zem_y = dy_m - (vy_up_mps * t) + (0.5 * gravity_mps2 * t2);
    let zev_x = target_vx_mps - vx_mps;
    let zev_y = target_vy_up_mps - vy_up_mps + (gravity_mps2 * t);
    let ax = ((6.0 * zem_x) / t2) - ((2.0 * zev_x) / t);
    let ay = ((6.0 * zem_y) / t2) - ((2.0 * zev_y) / t);
    (ax, ay)
}

fn estimate_target_y_projection(
    dx_m: f64,
    dy_m: f64,
    vx_mps: f64,
    vy_up_mps: f64,
    current_y_m: f64,
    gravity_mps2: f64,
) -> BallisticProjection {
    let target_y_cross = current_y_m + dy_m;
    let apex = ballistic_apex_from_state(current_y_m, vy_up_mps, gravity_mps2);
    if dy_m > 0.0 && (apex.time_to_apex_s <= 0.0 || apex.apex_y_m <= target_y_cross) {
        return BallisticProjection {
            projected_dx_m: dx_m - (vx_mps * apex.time_to_apex_s),
            time_to_cross_s: apex.time_to_apex_s.max(0.0),
            has_target_y_solution: false,
        };
    }

    if let Some(time_to_cross_s) = time_to_target_y_crossing(dy_m, vy_up_mps, gravity_mps2, true) {
        return BallisticProjection {
            projected_dx_m: dx_m - (vx_mps * time_to_cross_s),
            time_to_cross_s,
            has_target_y_solution: true,
        };
    }
    BallisticProjection {
        projected_dx_m: dx_m - (vx_mps * apex.time_to_apex_s),
        time_to_cross_s: apex.time_to_apex_s.max(0.0),
        has_target_y_solution: false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BallisticApex {
    time_to_apex_s: f64,
    apex_y_m: f64,
}

fn ballistic_apex_from_state(current_y_m: f64, vy_up_mps: f64, gravity_mps2: f64) -> BallisticApex {
    let time_to_apex_s = if gravity_mps2 <= 1e-6 {
        0.0
    } else {
        vy_up_mps / gravity_mps2
    };
    let apex_y_m = current_y_m + (vy_up_mps * time_to_apex_s)
        - (0.5 * gravity_mps2 * time_to_apex_s * time_to_apex_s);
    BallisticApex {
        time_to_apex_s,
        apex_y_m,
    }
}

fn time_to_target_y_crossing(
    dy_m: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
    prefer_descending: bool,
) -> Option<f64> {
    let eps = 1e-6;
    if gravity_mps2 <= eps {
        if vy_up_mps.abs() <= eps {
            return if dy_m.abs() <= eps { Some(0.0) } else { None };
        }
        let t_lin = dy_m / vy_up_mps;
        return (t_lin >= 0.0).then_some(t_lin);
    }
    let disc = (vy_up_mps * vy_up_mps) - (2.0 * gravity_mps2 * dy_m);
    if disc < -eps {
        return None;
    }
    let sqrt_disc = disc.max(0.0).sqrt();
    let mut roots = [
        (vy_up_mps - sqrt_disc) / gravity_mps2,
        (vy_up_mps + sqrt_disc) / gravity_mps2,
    ];
    roots.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap());
    let positive: Vec<f64> = roots.into_iter().filter(|root| *root >= 0.0).collect();
    if positive.is_empty() {
        return None;
    }
    if prefer_descending && positive.len() >= 2 && dy_m >= 0.0 {
        return positive.last().copied();
    }
    positive
        .iter()
        .copied()
        .find(|root| *root > 1e-4)
        .or_else(|| positive.first().copied())
}

fn estimate_ground_time_to_impact(
    height_to_target_m: f64,
    vy_up_mps: f64,
    gravity_mps2: f64,
) -> f64 {
    let altitude = height_to_target_m.max(0.0);
    let disc = (vy_up_mps * vy_up_mps) + (2.0 * gravity_mps2 * altitude);
    if disc <= 0.0 || gravity_mps2 <= 1e-6 {
        return 0.0;
    }
    let sqrt_disc = disc.sqrt();
    ((vy_up_mps + sqrt_disc) / gravity_mps2).max(0.0)
}
