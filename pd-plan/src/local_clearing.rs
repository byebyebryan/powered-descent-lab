//! Pure finite local-clearing policy. No terrain querying, saved route or
//! evaluator snapshot is an input; physical acceptance lives in the evaluator.
use pd_core::{FlightProgramUpdateV1, Vec2, VehicleGeometry};
use serde::{Deserialize, Serialize};

pub const LOCAL_CLEARING_POLICY_ID: &str = "one_obstruction_powered_coast_clearing_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingPolicyV1 {
    pub policy_id: String,
    pub attitudes_degrees: [f64; 3],
    pub acceleration_factors: [f64; 2],
    pub powered_ticks: [u64; 7],
    pub maximum_coast_ticks: u64,
    pub continuation_ticks: u64,
    pub minimum_clearance_m: f64,
}

impl Default for LocalClearingPolicyV1 {
    fn default() -> Self {
        Self {
            policy_id: LOCAL_CLEARING_POLICY_ID.into(),
            attitudes_degrees: [-30.0, 0.0, 30.0],
            acceleration_factors: [0.75, 1.0],
            powered_ticks: [60, 120, 240, 360, 480, 720, 960],
            maximum_coast_ticks: 720,
            continuation_ticks: 240,
            minimum_clearance_m: 5.0,
        }
    }
}

impl LocalClearingPolicyV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self != &Self::default() {
            return Err("policy differs from sealed local-clearing V1 family".into());
        }
        Ok(())
    }

    pub fn templates(&self) -> Result<Vec<LocalClearingTemplateV1>, String> {
        self.validate()?;
        let mut rows = Vec::new();
        for attitude in self.attitudes_degrees {
            for factor in self.acceleration_factors {
                for ticks in self.powered_ticks {
                    rows.push(LocalClearingTemplateV1 {
                        row_index: rows.len(),
                        target_attitude_rad: attitude.to_radians(),
                        acceleration_factor: factor,
                        powered_ticks: ticks,
                    });
                }
            }
        }
        Ok(rows)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingTemplateV1 {
    pub row_index: usize,
    pub target_attitude_rad: f64,
    pub acceleration_factor: f64,
    pub powered_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingGoalV1 {
    pub first_conflict_physics_step: u64,
    pub first_conflict_position_m: Vec2,
    pub absolute_deadline_physics_step: u64,
}

impl LocalClearingGoalV1 {
    pub fn validate(&self, entry_tick: u64) -> Result<(), String> {
        if !self.first_conflict_position_m.x.is_finite()
            || !self.first_conflict_position_m.y.is_finite()
            || entry_tick >= self.first_conflict_physics_step
            || self.first_conflict_physics_step >= self.absolute_deadline_physics_step
        {
            return Err("nonfinite goal or intervention not before conflict/deadline".into());
        }
        Ok(())
    }

    pub fn progress_x(&self, entry_x: f64, geometry: &VehicleGeometry) -> f64 {
        entry_x.max(self.first_conflict_position_m.x) + conservative_body_diameter(geometry)
    }
}

pub fn conservative_body_diameter(geometry: &VehicleGeometry) -> f64 {
    2.0 * (geometry.hull_width_m * 0.5)
        .hypot(geometry.hull_height_m * 0.5)
        .max(
            geometry
                .touchdown_half_span_m
                .hypot(geometry.touchdown_base_offset_m),
        )
}

pub fn intervention_boundaries(source_handoff: u64) -> Result<Vec<(String, u64)>, String> {
    let bridge = source_handoff
        .checked_sub(72)
        .ok_or("source handoff before launch")?;
    if !source_handoff.is_multiple_of(2) || bridge == 0 {
        return Err("unaligned or empty canonical bridge".into());
    }
    let mut entries = vec![(
        "first_idle_hold".into(),
        source_handoff.checked_add(2).ok_or("clock overflow")?,
    )];
    for (name, numerator, denominator) in [
        ("source_75_percent", 3, 4),
        ("source_50_percent", 1, 2),
        ("source_25_percent", 1, 4),
    ] {
        let ticks = bridge.checked_mul(numerator).ok_or("clock overflow")? / (2 * denominator) * 2;
        entries.push((name.into(), 72 + ticks));
    }
    Ok(entries)
}

/// A local schedule has no landing contact or mission-success fields and cannot
/// be mistaken for a complete FlightProgramV1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalClearingScheduleV1 {
    pub entry_physics_step: u64,
    pub powered_end_physics_step: u64,
    pub handoff_physics_step: u64,
    pub continuation_end_physics_step: u64,
    pub absolute_deadline_physics_step: u64,
    pub updates: Vec<FlightProgramUpdateV1>,
}

impl LocalClearingScheduleV1 {
    pub fn validate(&self, template: &LocalClearingTemplateV1) -> Result<(), String> {
        let policy = LocalClearingPolicyV1::default();
        if policy.templates()?.get(template.row_index) != Some(template)
            || self.entry_physics_step == 0
            || !self.entry_physics_step.is_multiple_of(2)
            || self.entry_physics_step.checked_add(template.powered_ticks)
                != Some(self.powered_end_physics_step)
            || self.handoff_physics_step <= self.powered_end_physics_step
            || self.handoff_physics_step
                > self
                    .powered_end_physics_step
                    .saturating_add(policy.maximum_coast_ticks)
            || !self.handoff_physics_step.is_multiple_of(2)
            || self
                .handoff_physics_step
                .checked_add(policy.continuation_ticks)
                != Some(self.continuation_end_physics_step)
            || self.continuation_end_physics_step > self.absolute_deadline_physics_step
        {
            return Err("invalid local schedule clock or template".into());
        }
        let count = (self.continuation_end_physics_step - self.entry_physics_step) / 2;
        if self.updates.len() as u64 != count {
            return Err("local command coverage gap or extra endpoint update".into());
        }
        for (ordinal, update) in self.updates.iter().enumerate() {
            if update.physics_step != self.entry_physics_step + 2 * ordinal as u64
                || update.command != update.command.clamped()
                || !update.command.throttle_frac.is_finite()
                || !update.command.target_attitude_rad.is_finite()
            {
                return Err("local missing/duplicate/off-clock/invalid command".into());
            }
            if update.physics_step >= self.powered_end_physics_step
                && (update.command.throttle_frac != 0.0
                    || update.command.target_attitude_rad != 0.0)
            {
                return Err("coast/certificate requires explicit idle upright updates".into());
            }
            if update.physics_step < self.powered_end_physics_step
                && (update.command.throttle_frac <= 0.0
                    || update.command.target_attitude_rad != template.target_attitude_rad)
            {
                return Err("powered command differs from finite template".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sealed_grid_and_entry_clock() {
        assert_eq!(
            LocalClearingPolicyV1::default().templates().unwrap().len(),
            42
        );
        let entries = intervention_boundaries(1992).unwrap();
        assert_eq!(
            entries.iter().map(|e| e.1).collect::<Vec<_>>(),
            [1994, 1512, 1032, 552]
        );
        assert!(intervention_boundaries(u64::MAX).is_err());
        let mut policy = LocalClearingPolicyV1::default();
        policy.continuation_ticks -= 2;
        assert!(policy.validate().is_err());
    }
    #[test]
    fn conflicts_are_not_intervention_states() {
        let goal = LocalClearingGoalV1 {
            first_conflict_physics_step: 10,
            first_conflict_position_m: Vec2::new(0.0, 1.0),
            absolute_deadline_physics_step: 100,
        };
        assert!(goal.validate(10).is_err());
        assert!(goal.validate(8).is_ok());
    }
    #[test]
    fn local_schedule_cannot_deserialize_fake_landing_fields() {
        let fake = serde_json::json!({"entry_physics_step":2,"powered_end_physics_step":62,"handoff_physics_step":64,"continuation_end_physics_step":304,"absolute_deadline_physics_step":1000,"updates":[],"expected_contact_physics_step":304});
        assert!(serde_json::from_value::<LocalClearingScheduleV1>(fake.clone()).is_err());
        let mut non_landing = fake;
        non_landing
            .as_object_mut()
            .unwrap()
            .remove("expected_contact_physics_step");
        assert!(serde_json::from_value::<LocalClearingScheduleV1>(non_landing).is_ok());
    }

    #[test]
    fn progress_is_a_body_diameter_not_a_feature_far_edge() {
        let mut geometry = VehicleGeometry {
            hull_width_m: 8.0,
            hull_height_m: 10.0,
            touchdown_half_span_m: 4.0,
            touchdown_base_offset_m: 5.0,
        };
        assert_eq!(conservative_body_diameter(&geometry), 12.806248474865697);
        geometry.touchdown_half_span_m = 20.0;
        assert_eq!(
            conservative_body_diameter(&geometry),
            2.0 * 20.0_f64.hypot(5.0)
        );
        let goal = LocalClearingGoalV1 {
            first_conflict_physics_step: 100,
            first_conflict_position_m: Vec2::new(-350.0, 200.0),
            absolute_deadline_physics_step: 1000,
        };
        assert_eq!(
            goal.progress_x(-400.0, &geometry),
            -350.0 + conservative_body_diameter(&geometry)
        );
    }
}
