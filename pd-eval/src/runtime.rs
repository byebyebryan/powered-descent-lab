use super::*;

pub(super) fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario file {}", path.display()))?;
    let scenario: ScenarioSpec = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse scenario json {}", path.display()))?;
    scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("scenario '{}' failed validation", path.display()))?;
    Ok(scenario)
}

pub(super) fn load_controller_spec(
    base_dir: &Path,
    controller_name: &str,
    controller_config_path: Option<&str>,
) -> Result<ControllerSpec> {
    if let Some(path) = controller_config_path {
        let full_path = base_dir.join(path);
        let raw = fs::read_to_string(&full_path).with_context(|| {
            format!(
                "failed to read controller config file {}",
                full_path.display()
            )
        })?;
        return serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse controller config json {}",
                full_path.display()
            )
        });
    }

    built_in_controller_spec(controller_name)
        .ok_or_else(|| anyhow!("unknown controller '{}'", controller_name))
}

pub(super) fn write_artifact_bundle(
    path: &Path,
    scenario: &ScenarioSpec,
    controller_spec: &ControllerSpec,
    artifacts: &ControlledRunArtifacts,
) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create artifact bundle dir {}", path.display()))?;
    write_json(&path.join("scenario.json"), scenario)?;
    write_json(&path.join("controller.json"), controller_spec)?;
    write_json(
        &path.join("controller_updates.json"),
        &artifacts.controller_updates,
    )?;
    write_json(&path.join("performance.json"), &artifacts.performance)?;
    write_json(&path.join("manifest.json"), &artifacts.run.manifest)?;
    write_json(&path.join("actions.json"), &artifacts.run.actions)?;
    write_json(&path.join("events.json"), &artifacts.run.events)?;
    write_json(&path.join("samples.json"), &artifacts.run.samples)?;
    pd_report::write_run_report_with_context(
        &path.join("report.html"),
        scenario,
        Some(controller_spec),
        &artifacts.run.manifest,
        &artifacts.run.events,
        &artifacts.run.samples,
        &artifacts.controller_updates,
        Some(&artifacts.performance),
        Some(&pd_report::RunReportContext {
            parent_report_href: Some("../../report.html".to_owned()),
            parent_report_label: Some("Batch report".to_owned()),
            run_index_href: Some("../".to_owned()),
        }),
    )?;
    pd_report::write_run_preview_svg(
        &path.join("preview.svg"),
        scenario,
        &artifacts.run.manifest,
        &artifacts.run.samples,
        &artifacts.controller_updates,
    )?;
    Ok(())
}

pub(super) fn batch_identity_for_pack(
    pack: &ScenarioPackSpec,
    resolved_runs: &[ResolvedBatchRun],
) -> Result<BatchIdentity> {
    Ok(BatchIdentity {
        schema_version: BATCH_REPORT_SCHEMA_VERSION,
        pack_spec_digest: stable_digest(pack)?,
        resolved_run_digest: stable_digest(
            &resolved_runs
                .iter()
                .map(|run| &run.descriptor)
                .collect::<Vec<_>>(),
        )?,
    })
}

pub(super) fn batch_cache_stem(pack_id: &str, identity: &BatchIdentity) -> String {
    format!(
        "{}__spec_{}__runs_{}",
        sanitize_token(pack_id),
        short_digest(&identity.pack_spec_digest),
        short_digest(&identity.resolved_run_digest),
    )
}

pub(super) fn cache_dir_for_batch_key(workspace_key: &str, batch_stem: &str) -> PathBuf {
    eval_cache_root().join(workspace_key).join(batch_stem)
}

pub(super) fn eval_cache_root() -> PathBuf {
    repo_root().join("outputs").join("eval").join("cache")
}

pub(super) fn current_workspace_state() -> Result<WorkspaceState> {
    let commit_key = git_commit_key_for_ref("HEAD")?;
    let status_output = git_stdout(&["status", "--porcelain=v1", "--untracked-files=normal"])?;
    let dirty = !status_output.trim().is_empty();
    let workspace_key = if dirty {
        format!(
            "{}-dirty-{}",
            commit_key,
            short_bytes_digest(status_output.as_bytes())
        )
    } else {
        commit_key.clone()
    };
    Ok(WorkspaceState {
        commit_key,
        workspace_key,
        dirty,
    })
}

pub(super) fn git_commit_key_for_ref(reference: &str) -> Result<String> {
    let resolved = git_stdout(&["rev-parse", "--short=12", reference])?;
    let key = resolved.trim();
    if key.is_empty() {
        bail!("git rev-parse produced empty commit key for {}", reference);
    }
    Ok(key.to_owned())
}

pub(super) fn git_stdout(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root())
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(super) fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(super) fn short_bytes_digest(bytes: &[u8]) -> String {
    format!("{:08x}", fnv1a64(bytes))
}

pub(super) fn short_digest(value: &str) -> String {
    value.chars().take(8).collect()
}

pub(super) fn resolve_compare_provenance(
    baseline_dir: Option<&Path>,
    compare_ref: Option<&str>,
    missing_compare: MissingComparePolicy,
    workspace: &WorkspaceState,
) -> Result<BatchCompareProvenance> {
    if let Some(baseline_dir) = baseline_dir {
        return Ok(BatchCompareProvenance {
            source: BatchCompareSource::ExplicitDir,
            requested_ref: None,
            resolved_ref: None,
            baseline_dir: Some(baseline_dir.to_string_lossy().into_owned()),
            status: BatchCompareResolutionStatus::Resolved,
            note: Some("explicit baseline report directory".to_owned()),
        });
    }

    let requested_ref = compare_ref.unwrap_or("auto");
    if requested_ref == "none" {
        return Ok(BatchCompareProvenance::default());
    }

    let resolved_ref = if requested_ref == "auto" {
        if workspace.dirty {
            Some(workspace.commit_key.clone())
        } else {
            git_commit_key_for_ref("HEAD^").ok()
        }
    } else {
        Some(git_commit_key_for_ref(requested_ref)?)
    };

    let Some(resolved_ref) = resolved_ref else {
        return Ok(BatchCompareProvenance {
            source: BatchCompareSource::CacheRef,
            requested_ref: Some(requested_ref.to_owned()),
            resolved_ref: None,
            baseline_dir: None,
            status: BatchCompareResolutionStatus::Missing,
            note: Some(match missing_compare {
                MissingComparePolicy::Skip => {
                    "no compare cache ref could be resolved; continuing without external compare"
                        .to_owned()
                }
                MissingComparePolicy::Error => "no compare cache ref could be resolved".to_owned(),
            }),
        });
    };

    Ok(BatchCompareProvenance {
        source: BatchCompareSource::CacheRef,
        requested_ref: Some(requested_ref.to_owned()),
        resolved_ref: Some(resolved_ref.clone()),
        baseline_dir: Some(
            eval_cache_root()
                .join(&resolved_ref)
                .to_string_lossy()
                .into_owned(),
        ),
        status: BatchCompareResolutionStatus::NotRequested,
        note: Some(if requested_ref == "auto" && workspace.dirty {
            "auto compare requested; using clean HEAD cache".to_owned()
        } else if requested_ref == "auto" {
            "auto compare requested; using previous clean commit cache".to_owned()
        } else {
            "explicit compare cache ref".to_owned()
        }),
    })
}

pub(super) fn load_requested_baseline(
    pack: &ScenarioPackSpec,
    identity: &BatchIdentity,
    mut provenance: BatchCompareProvenance,
    missing_compare: MissingComparePolicy,
) -> Result<(BatchCompareProvenance, Option<ResolvedBaselineReport>)> {
    match provenance.source {
        BatchCompareSource::None => Ok((provenance, None)),
        BatchCompareSource::ExplicitDir => {
            let baseline_dir = provenance
                .baseline_dir
                .as_deref()
                .ok_or_else(|| anyhow!("explicit baseline compare is missing baseline_dir"))?;
            let dir = PathBuf::from(baseline_dir);
            provenance.status = BatchCompareResolutionStatus::Resolved;
            Ok((
                provenance,
                Some(ResolvedBaselineReport {
                    report: load_batch_report(&dir)?,
                    dir,
                }),
            ))
        }
        BatchCompareSource::CacheRef => {
            if provenance.resolved_ref.is_none() {
                return if missing_compare == MissingComparePolicy::Skip {
                    provenance.status = BatchCompareResolutionStatus::Missing;
                    Ok((provenance, None))
                } else {
                    bail!("cache compare is missing resolved_ref")
                };
            }
            let resolved_ref = provenance
                .resolved_ref
                .as_deref()
                .expect("resolved_ref handled above");
            let batch_stem = batch_cache_stem(&pack.id, identity);
            let baseline_dir = cache_dir_for_batch_key(resolved_ref, &batch_stem);
            if let Some(report) = validate_cached_batch_dir(&baseline_dir, pack, identity)? {
                provenance.status = BatchCompareResolutionStatus::Resolved;
                provenance.baseline_dir = Some(baseline_dir.to_string_lossy().into_owned());
                Ok((
                    provenance,
                    Some(ResolvedBaselineReport {
                        dir: baseline_dir,
                        report,
                    }),
                ))
            } else if missing_compare == MissingComparePolicy::Skip {
                provenance.status = BatchCompareResolutionStatus::Missing;
                provenance.baseline_dir = Some(baseline_dir.to_string_lossy().into_owned());
                provenance.note = Some(format!(
                    "no compare cache found for ref '{}' at {}; continuing without external compare",
                    resolved_ref,
                    baseline_dir.display()
                ));
                Ok((provenance, None))
            } else {
                bail!(
                    "no compare cache found for ref '{}' at {}",
                    resolved_ref,
                    baseline_dir.display()
                )
            }
        }
    }
}

pub(super) fn validate_cached_batch_dir(
    cache_dir: &Path,
    pack: &ScenarioPackSpec,
    identity: &BatchIdentity,
) -> Result<Option<BatchReport>> {
    let required_files = [
        cache_dir.join("pack.json"),
        cache_dir.join("resolved_runs.json"),
        cache_dir.join("summary.json"),
        cache_dir.join("meta.json"),
        cache_dir.join("report.html"),
    ];
    if required_files.iter().any(|path| !path.exists()) {
        return Ok(None);
    }

    let Ok(meta) = read_json::<BatchCacheMeta>(&cache_dir.join("meta.json")) else {
        return Ok(None);
    };
    if meta.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || meta.identity.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || meta.pack_id != pack.id
        || meta.identity.pack_spec_digest != identity.pack_spec_digest
        || meta.identity.resolved_run_digest != identity.resolved_run_digest
    {
        return Ok(None);
    }

    let Ok(report) = load_batch_report(cache_dir) else {
        return Ok(None);
    };
    if report.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || report.identity.schema_version != BATCH_REPORT_SCHEMA_VERSION
        || report.pack_id != pack.id
        || report.identity.pack_spec_digest != identity.pack_spec_digest
        || report.identity.resolved_run_digest != identity.resolved_run_digest
        || report.records.len() != report.resolved_runs.len()
    {
        return Ok(None);
    }
    if !validate_cached_run_bundles(&report.records) {
        return Ok(None);
    }
    Ok(Some(report))
}

pub(super) fn validate_cached_run_bundles(records: &[BatchRunRecord]) -> bool {
    const REQUIRED_BUNDLE_FILES: [&str; 10] = [
        "scenario.json",
        "controller.json",
        "controller_updates.json",
        "performance.json",
        "manifest.json",
        "actions.json",
        "events.json",
        "samples.json",
        "report.html",
        "preview.svg",
    ];

    records.iter().all(|record| {
        let Some(bundle_dir) = record.bundle_dir.as_deref() else {
            return false;
        };
        let bundle_dir = if Path::new(bundle_dir).is_absolute() {
            PathBuf::from(bundle_dir)
        } else {
            repo_root().join(bundle_dir)
        };
        REQUIRED_BUNDLE_FILES
            .iter()
            .all(|name| bundle_dir.join(name).exists())
    })
}

pub(super) fn write_batch_cache_dir(
    output_dir: &Path,
    pack: &ScenarioPackSpec,
    report: &BatchReport,
    update_latest: bool,
    render_cache: &report::BatchReportRenderCache,
) -> Result<()> {
    write_batch_manifest_files(output_dir, pack, report)?;
    let cache = report
        .provenance
        .cache
        .clone()
        .ok_or_else(|| anyhow!("cannot write cache metadata without cache provenance"))?;
    write_json(
        &output_dir.join("meta.json"),
        &BatchCacheMeta {
            schema_version: BATCH_REPORT_SCHEMA_VERSION,
            pack_id: report.pack_id.clone(),
            pack_name: report.pack_name.clone(),
            identity: report.identity.clone(),
            total_runs: report.total_runs,
            workers_used: report.workers_used,
            cache,
        },
    )?;
    report::write_batch_report_artifacts_with_cache(output_dir, report, None, render_cache)?;
    if update_latest {
        maybe_update_latest_link(output_dir)?;
        if let Some(last_record) = report.records.last()
            && let Some(bundle_dir) = last_record.bundle_dir.as_deref()
        {
            maybe_update_latest_link(Path::new(bundle_dir))?;
        }
    }
    Ok(())
}

pub(super) fn write_batch_output_dir(
    output_dir: &Path,
    pack: &ScenarioPackSpec,
    report: &BatchReport,
    baseline: Option<(&Path, &BatchReport)>,
    render_cache: &report::BatchReportRenderCache,
) -> Result<()> {
    sync_output_run_bundles(output_dir, report)?;
    let localized_report = localize_report_bundle_dirs(report, output_dir);
    write_batch_manifest_files(output_dir, pack, &localized_report)?;
    report::write_batch_report_artifacts_with_cache(
        output_dir,
        &localized_report,
        baseline,
        render_cache,
    )?;
    maybe_update_latest_link(output_dir)?;
    if let Some(last_record) = localized_report.records.last()
        && let Some(bundle_dir) = last_record.bundle_dir.as_deref()
    {
        maybe_update_latest_link(Path::new(bundle_dir))?;
    }
    Ok(())
}

pub(super) fn write_batch_manifest_files(
    output_dir: &Path,
    pack: &ScenarioPackSpec,
    report: &BatchReport,
) -> Result<()> {
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create batch eval output directory {}",
            output_dir.display()
        )
    })?;
    write_json(&output_dir.join("pack.json"), pack)?;
    write_json(
        &output_dir.join("resolved_runs.json"),
        &report.resolved_runs,
    )?;
    write_json(&output_dir.join("summary.json"), report)?;
    Ok(())
}

pub(super) fn rewrite_report_bundle_dirs(
    report: &mut BatchReport,
    source_dir: &Path,
    target_dir: &Path,
) {
    for record in &mut report.records {
        if let Some(bundle_dir) = record.bundle_dir.as_deref() {
            record.bundle_dir = Some(rewrite_dir_string(bundle_dir, source_dir, target_dir));
        }
    }
}

pub(super) fn localize_report_bundle_dirs(report: &BatchReport, output_dir: &Path) -> BatchReport {
    let mut localized = report.clone();
    let runs_dir = output_dir.join("runs");
    for record in &mut localized.records {
        if record.bundle_dir.is_some() {
            record.bundle_dir = Some(
                runs_dir
                    .join(&record.resolved.run_id)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    localized
}

pub(super) fn sync_output_run_bundles(output_dir: &Path, report: &BatchReport) -> Result<()> {
    let runs_dir = output_dir.join("runs");
    remove_path_if_exists(&runs_dir)?;
    let bundle_records = report
        .records
        .iter()
        .filter_map(|record| {
            record
                .bundle_dir
                .as_deref()
                .map(|bundle_dir| (record.resolved.run_id.as_str(), PathBuf::from(bundle_dir)))
        })
        .collect::<Vec<_>>();
    if bundle_records.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(&runs_dir)
        .with_context(|| format!("failed to create runs directory {}", runs_dir.display()))?;
    for (run_id, bundle_dir) in bundle_records {
        let target = runs_dir.join(run_id);
        platform_fs::symlink(&bundle_dir, &target).with_context(|| {
            format!(
                "failed to link stable output run {} -> {}",
                target.display(),
                bundle_dir.display()
            )
        })?;
    }
    Ok(())
}

pub(super) fn remove_path_if_exists(path: &Path) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove path {}", path.display()))?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove directory {}", path.display()))?;
    }
    Ok(())
}

pub(super) fn rewrite_dir_string(path_str: &str, source_dir: &Path, target_dir: &Path) -> String {
    let path = Path::new(path_str);
    if let Ok(relative) = path.strip_prefix(source_dir) {
        target_dir.join(relative).to_string_lossy().into_owned()
    } else {
        path_str.to_owned()
    }
}

pub(super) fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)
        .with_context(|| format!("failed to create directory {}", target.display()))?;
    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

pub(super) fn find_latest_dirty_workspace_key(
    commit_key: &str,
    batch_stem: &str,
) -> Result<Option<String>> {
    let mut candidates = Vec::<(u64, String)>::new();
    let cache_root = eval_cache_root();
    if !cache_root.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(&cache_root)
        .with_context(|| format!("failed to read cache root {}", cache_root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let workspace_key = entry.file_name().to_string_lossy().into_owned();
        if !workspace_key.starts_with(&format!("{commit_key}-dirty-")) {
            continue;
        }
        let meta_path = entry.path().join(batch_stem).join("meta.json");
        if !meta_path.exists() {
            continue;
        }
        let Ok(meta) = read_json::<BatchCacheMeta>(&meta_path) else {
            continue;
        };
        candidates.push((meta.cache.created_at_unix_s, workspace_key));
    }
    candidates.sort();
    Ok(candidates.pop().map(|(_, key)| key))
}

pub(super) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read json file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse json file {}", path.display()))
}

pub(super) fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw)
        .with_context(|| format!("failed to write json file {}", path.display()))?;
    Ok(())
}

pub(super) fn enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_owned())
        .trim_matches('"')
        .to_owned()
}

pub(super) fn effective_worker_count(requested_workers: usize, total_runs: usize) -> usize {
    if total_runs == 0 {
        return 1;
    }
    requested_workers.max(1).min(total_runs)
}

pub(super) fn stable_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:012x}", fnv1a64(&bytes)))
}

pub(super) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) fn stable_unit_interval(seed: u64, salt: &str) -> f64 {
    let mixed = splitmix64(seed ^ fnv1a64(salt.as_bytes()));
    let mantissa = mixed >> 11;
    (mantissa as f64) / ((1_u64 << 53) as f64)
}

pub(super) fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = value;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

pub(super) fn sanitize_token(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    let mut last_was_sep = false;
    for ch in token.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            out.push(normalized);
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    out.trim_matches('_').to_owned()
}

pub(super) fn resolved_family_run_id(entry_id: &str, seed: u64) -> String {
    sanitize_token(&format!("{entry_id}__seed_{seed:04}"))
}

pub(super) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval crate should live under repo root")
        .to_path_buf()
}

pub(super) fn maybe_update_latest_link(target_dir: &Path) -> Result<()> {
    let repo_root = repo_root();
    let outputs_root = repo_root.join("outputs");
    let resolved_target_dir = if target_dir.is_absolute() {
        target_dir.to_path_buf()
    } else {
        repo_root.join(target_dir)
    };

    if !resolved_target_dir.starts_with(&outputs_root) {
        return Ok(());
    }

    let Some(parent_dir) = resolved_target_dir.parent() else {
        return Ok(());
    };
    let Some(target_name) = resolved_target_dir.file_name() else {
        return Ok(());
    };

    let latest_path = parent_dir.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest_path) {
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(&latest_path).with_context(|| {
                format!(
                    "failed to remove existing latest link {}",
                    latest_path.display()
                )
            })?;
        } else {
            eprintln!(
                "skipping latest link update because '{}' exists and is not a symlink",
                latest_path.display()
            );
            return Ok(());
        }
    }

    platform_fs::symlink(PathBuf::from(target_name), &latest_path).with_context(|| {
        format!(
            "failed to create latest link {} -> {}",
            latest_path.display(),
            target_name.to_string_lossy()
        )
    })?;
    Ok(())
}
