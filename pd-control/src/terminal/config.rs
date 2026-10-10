use serde::{Deserialize, Serialize};

/// Complete constructor configuration, including opt-in adapters whose coupled
/// effects are absent from the historical `ControllerSpec` payload. This is a
/// separate versioned contract; the historical tuning encoding stays intact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalPdgResolvedConfigV1 {
    pub schema_version: u32,
    pub tuning: TerminalPdgControllerConfig,
    pub guidance_plan_retention_enabled: bool,
    pub ballistic_landing_duration_fallback_enabled: bool,
    pub ballistic_landing_countdown_enabled: bool,
    pub ballistic_landing_braking_guard_enabled: bool,
    pub ballistic_landing_body_centering_enabled: bool,
    pub ballistic_terminal_coordination_enabled: bool,
}

impl TerminalPdgResolvedConfigV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("unsupported terminal constructor configuration version".into());
        }
        if self.ballistic_landing_countdown_enabled
            && !self.ballistic_landing_duration_fallback_enabled
        {
            return Err("landing countdown requires duration fallback".into());
        }
        if self.ballistic_terminal_coordination_enabled
            && !(self.ballistic_landing_braking_guard_enabled
                && self.ballistic_landing_body_centering_enabled)
        {
            return Err("terminal coordination requires braking guard and body centering".into());
        }
        // JSON cannot represent nonfinite tuning. Check before accepting an
        // in-memory constructor so its recorded payload is also complete.
        if [
            self.tuning.max_tilt_rad,
            self.tuning.terminal_dynamic_tilt_max_rad,
            self.tuning.max_tilt_low_alt_rad,
            self.tuning.max_tilt_low_alt_far_rad,
            self.tuning.low_alt_tilt_alt_m,
            self.tuning.low_alt_tilt_dx_m,
            self.tuning.low_alt_tilt_vx_mps,
            self.tuning.braking_alt_margin_m,
            self.tuning.braking_tilt_scale,
            self.tuning.braking_accel_safety,
            self.tuning.braking_min_speed_mps,
            self.tuning.braking_max_speed_mps,
            self.tuning.braking_target_ratio,
            self.tuning.vy_low_alt_cap_alt_m,
            self.tuning.vy_low_alt_cap_mps,
            self.tuning.vy_touch_cap_alt_m,
            self.tuning.vy_touch_cap_mps,
            self.tuning.lateral_hold_alt_m,
            self.tuning.lateral_hold_vx_min_mps,
            self.tuning.lateral_hold_time_ratio_start,
            self.tuning.lateral_hold_time_ratio_full,
            self.tuning.lateral_hold_vy_cap_mps,
            self.tuning.touchdown_idle_clearance_m,
            self.tuning.touchdown_zero_vx_mps,
            self.tuning.touchdown_zero_vy_mps,
            self.tuning.touchdown_low_clearance_trigger_m,
            self.tuning.touchdown_rescue_clearance_m,
            self.tuning.touchdown_rescue_vy_ratio,
            self.tuning.touchdown_rescue_tilt_rad,
            self.tuning.touchdown_rescue_vx_full_tilt_mps,
            self.tuning.touchdown_rescue_vx_margin_mps,
            self.tuning.touchdown_rescue_alt_floor_m,
            self.tuning.terminal_gate_nominal_ratio,
            self.tuning.terminal_gate_nominal_min_up_accel_mps2,
            self.tuning.terminal_gate_nominal_buffer_s,
            self.tuning.terminal_gate_burn_time_min_s,
            self.tuning.terminal_gate_burn_time_max_s,
            self.tuning.terminal_gate_burn_time_offset_short_s,
            self.tuning.terminal_gate_burn_time_offset_long_s,
            self.tuning.terminal_gate_latest_safe_buffer_s,
            self.tuning.terminal_gate_latest_safe_release_buffer_s,
            self.tuning.terminal_gate_latest_safe_aggressive_dx_abs_m,
            self.tuning.terminal_gate_latest_safe_aggressive_dx_ratio,
            self.tuning.terminal_overshoot_tilt_altitude_min_m,
            self.tuning.terminal_overshoot_tilt_projected_dx_abs_m,
            self.tuning.terminal_overshoot_tilt_projected_dx_ratio,
            self.tuning.terminal_overshoot_tilt_vx_min_mps,
            self.tuning.terminal_overshoot_tilt_max_rad,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err("terminal tuning must be finite".into());
        }
        Ok(())
    }
}

fn default_terrain_clearance_enabled() -> bool {
    true
}

fn default_terminal_gate_latest_safe_release_buffer_s() -> f64 {
    0.20
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalPdgControllerConfig {
    #[serde(default = "default_terrain_clearance_enabled")]
    pub terrain_clearance_enabled: bool,
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
    pub lateral_hold_alt_m: f64,
    pub lateral_hold_vx_min_mps: f64,
    pub lateral_hold_time_ratio_start: f64,
    pub lateral_hold_time_ratio_full: f64,
    pub lateral_hold_vy_cap_mps: f64,
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
    #[serde(default = "default_terminal_gate_latest_safe_release_buffer_s")]
    pub terminal_gate_latest_safe_release_buffer_s: f64,
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
            terrain_clearance_enabled: true,
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
            lateral_hold_alt_m: 24.0,
            lateral_hold_vx_min_mps: 4.0,
            lateral_hold_time_ratio_start: 1.0,
            lateral_hold_time_ratio_full: 1.45,
            lateral_hold_vy_cap_mps: 3.2,
            touchdown_idle_clearance_m: 0.05,
            touchdown_zero_vx_mps: 0.55,
            touchdown_zero_vy_mps: 0.6,
            touchdown_low_clearance_trigger_m: 1.25,
            touchdown_rescue_clearance_m: 4.5,
            touchdown_rescue_vy_ratio: 1.8,
            touchdown_rescue_tilt_rad: 0.42,
            touchdown_rescue_vx_full_tilt_mps: 3.0,
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
            terminal_gate_latest_safe_release_buffer_s:
                default_terminal_gate_latest_safe_release_buffer_s(),
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
