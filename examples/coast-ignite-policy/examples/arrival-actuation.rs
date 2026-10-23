//! Bounded diagnostic only: compare one shared-kernel command with the native
//! minimum-throttle map and an algebraic inverse. No controller is replaced.
use pd_control::{AirborneArrivalController, ArrivalFrame, ArrivalRequest, TelemetryValue};
use pd_core::{RunContext, ScenarioSpec, SimulationState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: arrival-actuation SCENARIO.json PROGRAM.json".into());
    }
    let scenario: ScenarioSpec = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let program: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let request: ArrivalRequest = serde_json::from_value(program["arrival"].clone())?;
    let ctx = RunContext::from_scenario(&scenario)?;
    let original = SimulationState::new(&ctx)?;
    let mut controller = AirborneArrivalController::new(&scenario, 0)?;
    let observation = original.build_observation(&ctx);
    controller.submit(&request, &observation)?;
    let ArrivalFrame::Command(frame) = controller.frame(&observation)? else {
        return Err("fixture resolved without a command".into());
    };
    let TelemetryValue::Float(requested) = frame.metrics["arrival_required_accel_ratio"] else {
        return Err("missing kernel request".into());
    };
    let minimum = ctx.vehicle.min_throttle_frac;
    if requested <= minimum || requested >= 1.0 || frame.command.target_attitude_rad.abs() > 1e-12 {
        return Err("audit requires an unclipped vertical command above minimum thrust".into());
    }
    let mut inverse = frame.command;
    inverse.throttle_frac = (requested - minimum) / (1.0 - minimum);
    let mut traces = vec![];
    for (label, command) in [
        ("shared_kernel_raw", frame.command),
        ("inverse_only_diagnostic", inverse),
    ] {
        let mut state = original.clone();
        state.set_command(command);
        for _ in 0..2 {
            let _ = state.step_physics_and_classify_contact(&ctx);
        }
        traces.push(serde_json::json!({"variant":label,"command":command,
            "applied_force_fraction":minimum+command.throttle_frac*(1.0-minimum),
            "after":state.build_observation(&ctx)}));
    }
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"requested_force_fraction":requested,
        "minimum_force_fraction":minimum,"entry":observation,"traces":traces,
        "native_steps":4,"model_steps":controller.feedback().model_steps,
        "scope":"one command pair only; inverse is diagnostic, no full-flight causal attribution or controller fix"})
    );
    Ok(())
}
