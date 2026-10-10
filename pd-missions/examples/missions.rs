use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pd_missions::*;
use serde::Serialize;

#[derive(Parser)]
#[command(about = "Input-only procedural mission and immutable collection tools")]
struct Args {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    Generate {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 64)]
        count: u64,
        #[arg(long, default_value = "development")]
        role: String,
        #[arg(long)]
        recipe: Option<PathBuf>,
    },
    Verify {
        collection: PathBuf,
    },
    Use {
        collection: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        exposure: String,
        #[arg(long)]
        purpose: String,
    },
    Characterize {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 10_000)]
        count: u64,
        #[arg(long)]
        recipe: Option<PathBuf>,
    },
}

fn recipe(path: Option<PathBuf>) -> Result<MissionRecipeV1, String> {
    let recipe = match path {
        Some(path) => serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
        None => MissionRecipeV1::default(),
    };
    recipe.validate()?;
    Ok(recipe)
}

#[derive(Serialize)]
struct Characterization {
    schema_version: u32,
    recipe: MissionRecipeV1,
    seed: u64,
    scheduled_requests: u64,
    accepted: u64,
    rejected_proposals: u64,
    exhausted_requests: u64,
    unique_physical_inputs: usize,
    duplicate_physical_inputs: u64,
    stratum_direction_counts: BTreeMap<String, u64>,
    observed_ranges: BTreeMap<String, [f64; 2]>,
    repeated_reverse_requests: u64,
    repeat_and_order_equivalent: bool,
}

fn observe(ranges: &mut BTreeMap<String, [f64; 2]>, name: &str, value: f64) {
    let range = ranges.entry(name.to_owned()).or_insert([value, value]);
    range[0] = range[0].min(value);
    range[1] = range[1].max(value);
}

fn characterize(
    recipe: MissionRecipeV1,
    seed: u64,
    count: u64,
    output: PathBuf,
) -> Result<(), String> {
    if count == 0 || count > 100_000 {
        return Err("characterization count must be 1..100,000".to_owned());
    }
    let mut report = Characterization {
        schema_version: 1,
        recipe,
        seed,
        scheduled_requests: count,
        accepted: 0,
        rejected_proposals: 0,
        exhausted_requests: 0,
        unique_physical_inputs: 0,
        duplicate_physical_inputs: 0,
        stratum_direction_counts: BTreeMap::new(),
        observed_ranges: BTreeMap::new(),
        repeated_reverse_requests: count,
        repeat_and_order_equivalent: true,
    };
    let mut identities = Vec::new();
    let mut physical = BTreeSet::new();
    for index in 0..count {
        let result = generate_mission(&report.recipe, seed, index)?;
        identities.push(sha256(&json_bytes(&result)?));
        report.rejected_proposals += result.rejections.len() as u64;
        if let Some(mission) = result.mission {
            mission.validate()?;
            report.accepted += 1;
            if !physical.insert(mission.physical_sha256.clone()) {
                report.duplicate_physical_inputs += 1;
            }
            *report
                .stratum_direction_counts
                .entry(format!(
                    "stratum-{}-direction-{}",
                    mission.terrain_stratum, mission.direction
                ))
                .or_default() += 1;
            let pads = &mission.scenario.world.landing_pads;
            observe(
                &mut report.observed_ranges,
                "travel_distance_m",
                (pads[1].center_x_m - pads[0].center_x_m).abs(),
            );
            observe(
                &mut report.observed_ranges,
                "source_x_m",
                pads[0].center_x_m,
            );
            observe(
                &mut report.observed_ranges,
                "source_pad_width_m",
                pads[0].width_m,
            );
            observe(
                &mut report.observed_ranges,
                "target_pad_width_m",
                pads[1].width_m,
            );
            observe(
                &mut report.observed_ranges,
                "target_height_relative_to_source_m",
                pads[1].surface_y_m - pads[0].surface_y_m,
            );
            observe(
                &mut report.observed_ranges,
                "terrain_amplitude_m",
                mission.terrain_amplitude_m,
            );
            observe(
                &mut report.observed_ranges,
                "terrain_wavelength_m",
                mission.terrain_wavelength_m,
            );
            let baseline = pads[0].surface_y_m.max(pads[1].surface_y_m);
            let max_between = mission
                .scenario
                .world
                .terrain
                .points()
                .iter()
                .filter(|p| {
                    p.x > pads[0].center_x_m.min(pads[1].center_x_m)
                        && p.x < pads[0].center_x_m.max(pads[1].center_x_m)
                })
                .map(|p| p.y)
                .fold(f64::NEG_INFINITY, f64::max);
            observe(
                &mut report.observed_ranges,
                "interpad_crest_above_higher_pad_m",
                max_between - baseline,
            );
        } else {
            report.exhausted_requests += 1;
        }
    }
    report.unique_physical_inputs = physical.len();
    for index in (0..count).rev() {
        let result = generate_mission(&report.recipe, seed, index)?;
        if sha256(&json_bytes(&result)?) != identities[index as usize] {
            report.repeat_and_order_equivalent = false;
        }
    }
    write_json_create_only(&output, &report)?;
    println!(
        "{} accepted, {} rejected proposals, {} exhausted; repeat/order equivalent: {}",
        report.accepted,
        report.rejected_proposals,
        report.exhausted_requests,
        report.repeat_and_order_equivalent
    );
    if !report.repeat_and_order_equivalent {
        return Err("generator repeat/order drift".to_owned());
    }
    Ok(())
}

fn main() -> Result<(), String> {
    match Args::parse().command {
        Operation::Generate {
            output,
            seed,
            count,
            role,
            recipe: path,
        } => {
            if count == 0 || count > 100_000 {
                return Err("collection count must be 1..100,000".to_owned());
            }
            let role = match role.as_str() {
                "training" => CollectionRoleV1::Training,
                "development" => CollectionRoleV1::Development,
                "validation" => CollectionRoleV1::Validation,
                "held-out" => CollectionRoleV1::HeldOut,
                _ => {
                    return Err(
                        "role must be training, development, validation or held-out".to_owned()
                    );
                }
            };
            let manifest = write_collection(
                &output,
                CollectionRequestV1 {
                    schema_version: 1,
                    role,
                    recipe: recipe(path)?,
                    seed,
                    indices: (0..count).collect(),
                },
            )?;
            println!(
                "{} missions, {} exhausted requests",
                manifest.entries.len(),
                manifest.exhausted_indices.len()
            );
        }
        Operation::Verify { collection } => {
            let manifest = verify_collection(&collection)?;
            let receipts = read_collection_usage(&collection)?;
            println!(
                "Verified {} immutable missions, {} exhausted requests, {} usage receipts; original role: {:?}",
                manifest.entries.len(),
                manifest.exhausted_indices.len(),
                receipts.len(),
                manifest.request.role
            );
            if manifest.request.role == CollectionRoleV1::HeldOut && !receipts.is_empty() {
                println!(
                    "Held-out inputs have recorded exposure; consult receipts before any performance claim."
                );
            }
        }
        Operation::Use {
            collection,
            id,
            exposure,
            purpose,
        } => {
            let exposure = match exposure.as_str() {
                "inspection" => CollectionExposureV1::InputInspection,
                "training" => CollectionExposureV1::Training,
                "tuning" => CollectionExposureV1::Tuning,
                "evaluation" => CollectionExposureV1::Evaluation,
                _ => {
                    return Err(
                        "exposure must be inspection, training, tuning or evaluation".to_owned(),
                    );
                }
            };
            record_collection_usage(&collection, &id, exposure, &purpose)?;
            println!("Recorded create-only usage receipt {id}");
        }
        Operation::Characterize {
            output,
            seed,
            count,
            recipe: path,
        } => characterize(recipe(path)?, seed, count, output)?,
    }
    Ok(())
}
