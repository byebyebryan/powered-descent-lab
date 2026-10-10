//! Maintained nominal-flight construction and fixed-program terrain audit.
//! Pure ballistic math lives in pd-plan; physical realization stays here.

mod acquisition;
mod airborne;
mod canonical_initial;
mod geometry;
mod math;
mod source_fit;
mod terminal;

pub mod input;
pub(crate) mod terrain;

pub use input::{
    NominalDirectFlightDecisionV1, NominalDirectFlightPreflightV1,
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointDirectNominalDirectGenerationValidation, nominal_direct_flight_identity,
    preflight_nominal_direct_flight, validate_waypoint_direct_nominal_direct_generation_request,
};

pub(crate) use acquisition::MAX_THRUST_FRACTION;
pub use acquisition::runtime::*;
pub(crate) use acquisition::turn_ticks_for;
pub use airborne::{AirborneDirectAuditV1, AirborneFlightStateV1};
pub use canonical_initial::*;
pub use geometry::{
    AirborneClearanceMinimumEvidence, ContactFootEvidence, ContactPredicateMirrorEvidence,
    GeometryClearanceScanEvidence, GeometryClearanceViolationEvidence,
    PreterminalContactStateEvidence, ScheduledFirstContactMarginsEvidence,
    TerminalContactAuditEvidence,
};
pub use terminal::BodyAwareTerminalPolicyV1;

pub(crate) use airborne::live_rejection;
use geometry::*;
use input::nominal_direct_flight_identity as stable_digest;
use math::shortest_angle_delta;
use math::*;
use source_fit::*;
use terminal::*;
pub(crate) use terminal::{
    clearing_body_reserve_query, nominal_body_reserve_query, paired_throttle,
};
