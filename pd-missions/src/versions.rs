//! Explicit schema dispatch; never relax either generation contract to read the
//! other version. Serialized enum variants retain their original JSON shapes.
use pd_core::ScenarioSpec;
use serde::Serialize;

use crate::{
    GenerationResultV1, GenerationResultV2, GeometrySpecV1, MissionRecipeV1, MissionRecipeV2,
    ResolvedMissionV1, ResolvedMissionV2, generate_mission, generate_mission_v2,
};

fn schema(bytes: &[u8]) -> Result<u64, String> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("versioned mission input exceeds bounded size".into());
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "explicit integer schema_version is required".into())
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VersionedMissionRecipe {
    V1(MissionRecipeV1),
    V2(MissionRecipeV2),
}

impl VersionedMissionRecipe {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("recipe exceeds bounded size".into());
        }
        let recipe = match schema(bytes)? {
            1 => Self::V1(serde_json::from_slice(bytes).map_err(|e| e.to_string())?),
            2 => Self::V2(serde_json::from_slice(bytes).map_err(|e| e.to_string())?),
            _ => return Err("unsupported mission recipe schema_version".into()),
        };
        match &recipe {
            Self::V1(r) => r.validate()?,
            Self::V2(r) => r.validate()?,
        }
        Ok(recipe)
    }

    pub fn family(family: &str) -> Result<Self, String> {
        match family {
            "value-noise" => Ok(Self::V1(MissionRecipeV1::default())),
            "layered" => Ok(Self::V1(MissionRecipeV1::layered())),
            "value-noise-v2" => Ok(Self::V2(MissionRecipeV2::value_noise())),
            "layered-v2" => Ok(Self::V2(MissionRecipeV2::default())),
            _ => Err("unknown terrain family".into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VersionedResolvedMission {
    V1(Box<ResolvedMissionV1>),
    V2(Box<ResolvedMissionV2>),
}

impl VersionedResolvedMission {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let result = match schema(bytes)? {
            1 => Self::V1(serde_json::from_slice(bytes).map_err(|e| e.to_string())?),
            2 => Self::V2(serde_json::from_slice(bytes).map_err(|e| e.to_string())?),
            _ => return Err("unsupported resolved mission schema_version".into()),
        };
        match &result {
            Self::V1(m) => m.validate()?,
            Self::V2(m) => m.validate()?,
        }
        Ok(result)
    }

    pub fn scenario(&self) -> &ScenarioSpec {
        match self {
            Self::V1(m) => &m.scenario,
            Self::V2(m) => &m.scenario,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VersionedGenerationResult {
    V1(Box<GenerationResultV1>),
    V2(Box<GenerationResultV2>),
}

pub fn generate_versioned_mission(
    recipe: &VersionedMissionRecipe,
    seed: u64,
    index: u64,
    spec: Option<&GeometrySpecV1>,
) -> Result<VersionedGenerationResult, String> {
    match recipe {
        VersionedMissionRecipe::V1(r) => {
            if spec.is_some() {
                return Err("V1 generation does not accept a geometry specification".into());
            }
            Ok(VersionedGenerationResult::V1(Box::new(generate_mission(
                r, seed, index,
            )?)))
        }
        VersionedMissionRecipe::V2(r) => Ok(VersionedGenerationResult::V2(Box::new(
            generate_mission_v2(r, spec.unwrap_or(&GeometrySpecV1::default()), seed, index)?,
        ))),
    }
}
