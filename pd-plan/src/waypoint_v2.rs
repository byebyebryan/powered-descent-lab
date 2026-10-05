//! Pure bounds and clocks for the opt-in, piecewise ballistic flight planner.
//! No terrain-dependent nominal selection or plant state restoration lives here.
use serde::{Deserialize, Serialize};

pub const WAYPOINT_V2_POLICY_ID: &str = "piecewise_local_clearing_v2_policy_1";
pub const WAYPOINT_V2_POLICY_REVISION_2_ID: &str = "piecewise_local_clearing_v2_policy_2";
pub const WAYPOINT_V2_POLICY_REVISION_3_ID: &str = "piecewise_local_clearing_v2_policy_3";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2Policy {
    pub policy_id: String,
    pub maximum_corrections: u32,
}

impl Default for WaypointV2Policy {
    fn default() -> Self {
        Self::revision_3()
    }
}

impl WaypointV2Policy {
    /// Historical identity for saved records, not an executable policy.
    pub fn revision_1() -> Self {
        Self {
            policy_id: WAYPOINT_V2_POLICY_ID.into(),
            maximum_corrections: 6,
        }
    }

    /// Historical identity for saved records, not an executable policy.
    pub fn revision_2() -> Self {
        Self {
            policy_id: WAYPOINT_V2_POLICY_REVISION_2_ID.into(),
            maximum_corrections: 6,
        }
    }

    /// Policy 3 keeps policy 2's initial entries while carrying its own
    /// explicit identity for the airborne nominal integration.
    pub fn revision_3() -> Self {
        Self {
            policy_id: WAYPOINT_V2_POLICY_REVISION_3_ID.into(),
            maximum_corrections: 6,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.version().map(|_| ())
    }

    /// Recognize exact saved identities independently of the current default.
    pub fn version(&self) -> Result<u32, String> {
        if self == &Self::revision_1() {
            Ok(1)
        } else if self == &Self::revision_2() {
            Ok(2)
        } else if self == &Self::revision_3() {
            Ok(3)
        } else {
            Err("unsupported V2 policy; revisions must be explicitly versioned".into())
        }
    }

    pub fn validate_for_execution(&self) -> Result<(), String> {
        if self.version()? != 3 {
            return Err("V2 policies 1 and 2 are retired; execution requires policy 3".into());
        }
        Ok(())
    }

    pub fn initial_intervention_boundaries(
        &self,
        source_handoff: u64,
    ) -> Result<Vec<(String, u64)>, String> {
        self.validate_for_execution()?;
        let bridge = source_handoff
            .checked_sub(72)
            .ok_or("source handoff before launch")?;
        if !source_handoff.is_multiple_of(2) || bridge == 0 {
            return Err("unaligned or empty canonical bridge".into());
        }
        let mut entries = Vec::new();
        for (name, numerator, denominator) in [
            ("source_75_percent", 3_u64, 4_u64),
            ("source_62_5_percent", 5, 8),
            ("source_50_percent", 1, 2),
            ("source_37_5_percent", 3, 8),
        ] {
            let units = bridge.checked_mul(numerator).ok_or("clock overflow")? / (2 * denominator);
            let tick = 72_u64
                .checked_add(units.checked_mul(2).ok_or("clock overflow")?)
                .ok_or("clock overflow")?;
            entries.push((name.into(), tick));
        }
        Ok(entries)
    }

    /// A final Direct attempt is allowed after the last permitted correction.
    pub fn can_correct(&self, completed: u32) -> bool {
        completed < self.maximum_corrections
    }

    pub fn maximum_local_work(&self) -> Result<(u64, u64), String> {
        let rows = u64::from(self.maximum_corrections)
            .checked_mul(4 * 42)
            .ok_or("row overflow")?;
        Ok((rows, rows.checked_mul(360).ok_or("boundary overflow")?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2Stop {
    Landed,
    NoNominal,
    NominalRejected,
    NoClearing,
    CorrectionLimit,
    Deadline,
    NoProgress,
    Unsupported,
    InvalidInput,
    ImplementationError,
}

/// Forward-only aligned entries relative to the current handoff, not source.
pub fn later_intervention_boundaries(
    current: u64,
    conflict: u64,
) -> Result<Vec<(String, u64)>, String> {
    if current == 0 || !current.is_multiple_of(2) || conflict <= current {
        return Err("invalid live-origin/conflict clock".into());
    }
    let delta = conflict - current;
    let mut entries = Vec::new();
    for numerator in 0_u64..4 {
        // Divide before multiplying where possible; the remainder is <8.
        let units = (delta / 8)
            .checked_mul(numerator)
            .and_then(|x| x.checked_add((delta % 8) * numerator / 8))
            .ok_or("entry overflow")?;
        let tick = current
            .checked_add(units.checked_mul(2).ok_or("entry overflow")?)
            .ok_or("entry overflow")?;
        if tick < conflict && entries.last().is_none_or(|(_, previous)| *previous != tick) {
            entries.push((format!("live_{numerator}_quarters"), tick));
        }
    }
    Ok(entries)
}

/// Updates own [start,end); an odd terminal tick still consumes its last pair.
pub fn command_count(start: u64, end: u64) -> Result<u64, String> {
    if !start.is_multiple_of(2) || end < start {
        return Err("invalid segment interval".into());
    }
    Ok((end - start).div_ceil(2))
}

pub fn original_deadline(horizon_s: f64, budget_s: f64) -> Result<u64, String> {
    let time = horizon_s.min(budget_s);
    if !horizon_s.is_finite()
        || !budget_s.is_finite()
        || time <= 0.0
        || time * 120.0 >= u64::MAX as f64
    {
        return Err("invalid original deadline".into());
    }
    Ok((time * 120.0).floor() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entries_use_live_origin_and_round_down() {
        assert_eq!(
            later_intervention_boundaries(2820, 3120)
                .unwrap()
                .iter()
                .map(|x| x.1)
                .collect::<Vec<_>>(),
            [2820, 2894, 2970, 3044]
        );
        assert_eq!(later_intervention_boundaries(120, 121).unwrap().len(), 1);
        assert_eq!(
            later_intervention_boundaries(120, 123)
                .unwrap()
                .iter()
                .map(|x| x.1)
                .collect::<Vec<_>>(),
            [120, 122]
        );
        assert!(later_intervention_boundaries(0, 100).is_err());
        assert!(later_intervention_boundaries(121, 200).is_err());
        assert!(later_intervention_boundaries(120, 120).is_err());
        assert!(later_intervention_boundaries(u64::MAX - 3, u64::MAX).is_ok());
    }
    #[test]
    fn bounds_and_exclusive_command_ownership() {
        let p = WaypointV2Policy::default();
        assert_eq!(p.maximum_local_work().unwrap(), (1008, 362880));
        assert!(p.can_correct(5));
        assert!(!p.can_correct(6));
        assert_eq!(command_count(120, 120).unwrap(), 0);
        assert_eq!(command_count(120, 123).unwrap(), 2);
        assert!(command_count(121, 124).is_err());
        assert!(command_count(124, 120).is_err());
        assert_eq!(original_deadline(100.0, 80.0).unwrap(), 9600);
        assert_eq!(original_deadline(3.0, 80.0).unwrap(), 360);
        assert!(original_deadline(f64::INFINITY, 80.0).is_err());
    }
    #[test]
    fn current_default_preserves_policy_three_entries_and_bounds() {
        let p = WaypointV2Policy::default();
        assert_eq!(p, WaypointV2Policy::revision_3());
        assert!(p.validate_for_execution().is_ok());
        assert_eq!(
            p.initial_intervention_boundaries(1992)
                .unwrap()
                .iter()
                .map(|x| x.1)
                .collect::<Vec<_>>(),
            [1512, 1272, 1032, 792]
        );
        assert_eq!(p.maximum_local_work().unwrap(), (1008, 362880));
        assert!(p.initial_intervention_boundaries(72).is_err());
        assert!(p.initial_intervention_boundaries(u64::MAX - 1).is_err());
    }

    #[test]
    fn historical_policies_remain_recognizable_but_cannot_execute() {
        for (version, policy) in [
            (1, WaypointV2Policy::revision_1()),
            (2, WaypointV2Policy::revision_2()),
        ] {
            let saved = serde_json::to_string(&policy).unwrap();
            let restored: WaypointV2Policy = serde_json::from_str(&saved).unwrap();
            assert_eq!(restored.version().unwrap(), version);
            assert!(restored.validate().is_ok());
            assert!(
                restored
                    .validate_for_execution()
                    .unwrap_err()
                    .contains("retired")
            );
            assert!(restored.initial_intervention_boundaries(1992).is_err());
        }
    }

    #[test]
    fn policy_validation_accepts_only_exact_named_values() {
        let mut unknown = WaypointV2Policy::revision_3();
        unknown.policy_id.push_str("_unknown");
        assert!(unknown.validate().is_err());

        let mut changed_cap = WaypointV2Policy::revision_3();
        changed_cap.maximum_corrections += 1;
        assert!(changed_cap.validate().is_err());
    }
}
