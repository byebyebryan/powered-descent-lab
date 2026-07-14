//! Frozen transfer boost-scoring experiments retained for reproducibility.
//!
//! These modes are not promotion candidates. Production transfer guidance uses
//! endpoint scoring; the aliases and weights remain available so historical
//! diagnostic packs and serialized controller specs continue to resolve.

use super::TransferPdgControllerConfig;

pub(super) const TRANSFER_BOOST_SCORE_NO_TARGET_Y: f64 = 10_000.0;
pub(super) const TRANSFER_BOOST_SCORE_PROJECTED_DX: f64 = 100.0;
pub(super) const TRANSFER_BOOST_SCORE_PROJECTED_DX_CENTERING: f64 = 45.0;
pub(super) const TRANSFER_BOOST_SCORE_SHORTFALL: f64 = 45.0;
pub(super) const TRANSFER_BOOST_SCORE_MIN_ANGLE: f64 = 60.0;
pub(super) const TRANSFER_BOOST_SCORE_TARGET_ANGLE: f64 = 20.0;
pub(super) const TRANSFER_BOOST_SCORE_APEX_UNDERSHOOT: f64 = 18.0;
pub(super) const TRANSFER_BOOST_SCORE_APEX_OVERSHOOT: f64 = 10.0;
pub(super) const TRANSFER_BOOST_SCORE_THROTTLE_EFFORT: f64 = 1.0;
pub(super) const TRANSFER_BOOST_SCORE_TILT_EFFORT: f64 = 0.4;
pub(super) const TRANSFER_BOOST_RECOVERY_SCORE_ENDPOINT_WEIGHT: f64 = 0.05;
pub(super) const TRANSFER_BOOST_RECOVERY_SCORE_SETTLED_WEIGHT: f64 = 0.02;
pub(super) const TRANSFER_BOOST_RECOVERY_SCORE_LATEST_SAFE_MARGIN: f64 = 14.0;
pub(super) const TRANSFER_BOOST_RECOVERY_SCORE_ACCEL_RATIO: f64 = 1.2;
pub(super) const TRANSFER_BOOST_RECOVERY_SCORE_PASS_NOT_READY: f64 = 45.0;
pub(super) const TRANSFER_BOOST_RECOVERY_SCORE_TERRAIN_UNSAFE: f64 = 1_200.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TransferBoostScoringMode {
    Endpoint,
    ExperimentalPathwise,
    ExperimentalRecoverability,
}

impl TransferBoostScoringMode {
    pub(super) fn from_config(config: &TransferPdgControllerConfig) -> Self {
        if config.boost_recoverability_scoring_enabled {
            Self::ExperimentalRecoverability
        } else if config.boost_pathwise_scoring_enabled {
            Self::ExperimentalPathwise
        } else {
            Self::Endpoint
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Endpoint => "legacy_endpoint",
            Self::ExperimentalPathwise => "pathwise_geometry",
            Self::ExperimentalRecoverability => "recoverability",
        }
    }
}
