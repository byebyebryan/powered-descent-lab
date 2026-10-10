//! Sustained runtime instrumentation, separate from procedural policy competence.
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Instant;

use pd_core::{
    AllowAllBoundedRunGuard, Command, RunContext, ScenarioSpec, replay_simulation_bounded,
};
use pd_run::{CaptureModeV1, Episode, scenario_horizon_limits};

fn main() -> Result<(), String> {
    let output = std::env::args()
        .nth(1)
        .ok_or("supply a new JSON output path")?;
    let mut scenario: ScenarioSpec = serde_json::from_str(include_str!(
        "../../fixtures/scenarios/flat_terminal_descent.json"
    ))
    .map_err(|e| e.to_string())?;
    // A derived synthetic instrumentation input; frozen fixture bytes stay intact.
    scenario.id = "runtime-held-airborne-trace-v1".to_owned();
    scenario.name = "Runtime held airborne trace".to_owned();
    scenario.sim.max_time_s = 30.0;
    let ctx = RunContext::from_scenario(&scenario)?;
    let limits = scenario_horizon_limits(&ctx).map_err(|e| e.to_string())?;
    let initial_mass = ctx.vehicle.dry_mass_kg + ctx.vehicle.initial_fuel_kg;
    let physical_ratio = initial_mass * ctx.world.gravity_mps2 / ctx.vehicle.max_thrust_n;
    let command = Command {
        throttle_frac: (physical_ratio - ctx.vehicle.min_throttle_frac)
            / (1.0 - ctx.vehicle.min_throttle_frac),
        target_attitude_rad: 0.0,
    };
    let repeats = 8;
    let mut expected = None;
    let mut measurements = Vec::new();
    let mut complete_replays = 0;
    let mut replay_s = 0.0;
    let mut serialization_s = 0.0;
    for mode in [CaptureModeV1::Summary, CaptureModeV1::Captured] {
        let started = Instant::now();
        let mut steps = 0;
        let mut updates = 0;
        let mut trace_records = 0;
        let mut serialized_bytes = 0;
        let mut mode_replay_s = 0.0;
        let mut mode_serialization_s = 0.0;
        for _ in 0..repeats {
            let mut episode = Episode::reset(&scenario, "held-trace-v1", limits, mode)
                .map_err(|e| e.to_string())?;
            while episode.stop().is_none() {
                episode.step(command).map_err(|e| e.to_string())?;
            }
            let result = episode.finish().map_err(|e| e.to_string())?;
            if let Some(expected) = &expected {
                if *expected != result.summary {
                    return Err("held trace physical summary drift".to_owned());
                }
            } else {
                expected = Some(result.summary.clone());
            }
            steps += result.summary.final_state.physics_step;
            updates += result.summary.manifest.controller_updates;
            if let Some(capture) = result.capture {
                trace_records += capture.run.actions.len()
                    + capture.run.events.len()
                    + capture.run.samples.len();
                let serial = Instant::now();
                serialized_bytes += serde_json::to_vec(&capture)
                    .map_err(|e| e.to_string())?
                    .len();
                mode_serialization_s += serial.elapsed().as_secs_f64();
                let replay_started = Instant::now();
                let replay = replay_simulation_bounded(
                    &ctx,
                    "held-trace-v1",
                    &capture.run.actions,
                    limits,
                    &mut AllowAllBoundedRunGuard,
                )
                .map_err(|e| e.to_string())?;
                mode_replay_s += replay_started.elapsed().as_secs_f64();
                if replay != capture {
                    return Err("held trace complete native replay drift".to_owned());
                }
                complete_replays += 1;
            } else if result.agent_updates.is_some() {
                return Err("summary retained updates".to_owned());
            }
        }
        let total_s = started.elapsed().as_secs_f64();
        replay_s += mode_replay_s;
        serialization_s += mode_serialization_s;
        measurements.push(
            serde_json::json!({"mode": mode, "episodes": repeats, "physics_steps": steps,
            "agent_updates": updates, "forecast_steps": 0, "trace_records": trace_records,
            "serialized_bytes": serialized_bytes, "total_wall_s": total_s,
            "execution_wall_s": total_s - mode_replay_s - mode_serialization_s}),
        );
    }
    let peak_rss = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|s| s.parse::<u64>().ok())
        });
    let report = serde_json::json!({"schema_version":1, "derived_synthetic_scenario":scenario,
        "held_command":command, "measurements": measurements, "complete_replays":complete_replays,
        "replay_wall_s": replay_s, "serialization_wall_s":serialization_s,
        "peak_process_rss_kib":peak_rss, "summary_capture_equivalent":true,
        "summary":expected, "scope":"Synthetic sustained runtime trace; no landing competence claim"});
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    println!(
        "Measured {repeats} summary and {repeats} captured held traces; {complete_replays} exact native replays"
    );
    Ok(())
}
