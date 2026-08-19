pub mod eval;
pub mod math;
pub mod model;
pub mod planning;
pub mod sim;
pub mod terrain;

pub use eval::ContactClassification;
pub use math::Vec2;
pub use model::{
    ActionLogEntry, CheckpointRunSummary, Command, EndReason, EvaluationGoal, EventKind,
    EventRecord, LandingPadSpec, LandingRunSummary, MissionOutcome, MissionSpec, Observation,
    PhysicalOutcome, RunArtifacts, RunContext, RunManifest, RunSummary, SampleRecord, ScenarioSpec,
    SimConfig, TransferRouteSpec, TransferWaypointSpec, VehicleGeometry, VehicleInitialState,
    VehicleSpec, WaypointHandoffAssessment, WaypointHandoffKinematics, WaypointHandoffViolation,
    WorldSpec,
};
pub use planning::{
    HEIGHTFIELD_VISIBILITY_ALGORITHM_ID, NormalizedRouteGeometry, PlannerComputeEvidence,
    PlanningRejection, PlanningRejectionCode, PlanningValidationError,
    ROUTE_PLANNING_POLICY_VERSION, RouteLegDiagnostics, RoutePlan, RoutePlanDiagnostics,
    RoutePlanningPolicy, RoutePlanningRequest, RouteTopology, RouteValidation,
    RouteValidationError, SafetyProfile, WaypointAuthorityDiagnostics, build_endpoint_profile,
    compute_waypoint_authority, endpoint_shaped_centerline, normalized_geometry, validate_route,
};
pub use sim::{SimulationError, SimulationState, replay_simulation, run_simulation};
pub use terrain::{
    CorridorClearance, CorridorEnvelope, CorridorResidual, TerrainDefinition, TerrainQueryError,
};
