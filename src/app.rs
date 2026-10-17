// SPDX-License-Identifier: EUPL-1.2
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Local;
use rayon::ThreadPoolBuilder;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::algorithm::Algorithm;
use crate::cli::{
    AlgorithmSelection, Cli, Command as CliCommand, CommonArgs, CreateArgs, VerifyArgs,
};
use crate::encoding::DigestEncoding;
use crate::error::{AppError, AppResult};
use crate::hashing::{generate_digests, hash_file, hash_file_with_buffer};
use crate::model::DirHasherResult;
use crate::persistence::{load_result, persist_result};
use crate::report::{pretty_print_result, print_digest_entry};
use crate::style::PersistenceStyle;
use crate::util::is_same_file;
use crate::verify::{index_result_refs, normalized_path_string};
use crate::{DEFAULT_ALGORITHM, VERSION};

#[derive(Debug, Clone)]
struct SharedCommandOptions {
    style: PersistenceStyle,
    algorithms: Vec<Algorithm>,
    encoding: DigestEncoding,
    verbose: bool,
    single: bool,
    jobs: Option<usize>,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct CreateCommand {
    shared: SharedCommandOptions,
    output: PathBuf,
    force: bool,
    compact_json: bool,
}

#[derive(Debug, Clone)]
struct VerifyCommand {
    shared: SharedCommandOptions,
    input: PathBuf,
    ignore_case: bool,
}

#[derive(Debug, Clone)]
enum AppCommand {
    Create(CreateCommand),
    Verify(VerifyCommand),
}

impl AppCommand {
    fn from_cli(cli: Cli) -> AppResult<Self> {
        match cli.command {
            CliCommand::Create(args) => Ok(Self::Create(resolve_create(args)?)),
            CliCommand::Verify(args) => Ok(Self::Verify(resolve_verify(args)?)),
        }
    }
}

fn resolve_create(args: CreateArgs) -> AppResult<CreateCommand> {
    Ok(CreateCommand {
        shared: resolve_common(args.common)?,
        output: args.output,
        force: args.force,
        compact_json: args.compact_json,
    })
}

fn resolve_verify(args: VerifyArgs) -> AppResult<VerifyCommand> {
    Ok(VerifyCommand {
        shared: resolve_common(args.common)?,
        input: args.input,
        ignore_case: args.ignore_case,
    })
}

fn resolve_common(common: CommonArgs) -> AppResult<SharedCommandOptions> {
    if let Some(0) = common.jobs {
        return Err(AppError::Message(
            "--jobs must be greater than zero".to_string(),
        ));
    }

    let mut algorithms = selected_algorithms(&common.algorithm_selection);
    if algorithms.is_empty() {
        algorithms.push(Algorithm::from_name(DEFAULT_ALGORITHM));
    }

    Ok(SharedCommandOptions {
        style: PersistenceStyle::parse(&common.style)?,
        algorithms,
        encoding: DigestEncoding::from_base64_flag(common.base64),
        verbose: common.verbose,
        single: common.single,
        jobs: common.jobs,
        paths: common.paths,
    })
}

pub fn run(cli: Cli) -> ExitCode {
    let start = std::time::Instant::now();
    eprintln!("Version: {VERSION}");

    let command = match AppCommand::from_cli(cli) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let status = match command {
        AppCommand::Create(command) => match write_mode(&command) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(255)
            }
        },
        AppCommand::Verify(command) => match verify_mode(&command) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(254)
            }
        },
    };

    eprintln!("Done: {:.3}", start.elapsed().as_secs_f64());
    status
}

fn selected_algorithms(selection: &AlgorithmSelection) -> Vec<Algorithm> {
    let mut names = BTreeSet::new();

    if selection.all || selection.sha256 {
        names.insert(Algorithm::Sha256);
    }
    if selection.all || selection.sha1 {
        names.insert(Algorithm::Sha1);
    }
    if selection.all || selection.sha384 {
        names.insert(Algorithm::Sha384);
    }
    if selection.all || selection.sha512 {
        names.insert(Algorithm::Sha512);
    }
    if selection.all || selection.sha3_256 {
        names.insert(Algorithm::Sha3_256);
    }
    if selection.all || selection.sha3_384 {
        names.insert(Algorithm::Sha3_384);
    }
    if selection.all || selection.sha3_512 {
        names.insert(Algorithm::Sha3_512);
    }
    if selection.all || selection.md5 {
        names.insert(Algorithm::Md5);
    }
    if selection.all || selection.ripemd160 {
        names.insert(Algorithm::Ripemd160);
    }

    names.into_iter().collect()
}

fn write_mode(command: &CreateCommand) -> AppResult<()> {
    if matches!(command.shared.style, PersistenceStyle::Xml) {
        return Err(AppError::XmlNotSupported);
    }

    if command.output.exists() && !command.force {
        return Err(AppError::OutputExists(command.output.clone()));
    }

    let digests = generate_digests(
        &command.shared.paths,
        &command.shared.algorithms,
        !command.shared.single,
        command.shared.jobs,
        command.shared.verbose,
    )?;
    persist_result(
        &command.output,
        command.shared.style,
        &digests,
        !command.compact_json,
        command.shared.encoding,
    )?;

    let summary_algorithms = digests
        .content
        .values()
        .next()
        .map(|digest| digest.algorithms_enum())
        .unwrap_or_else(|| command.shared.algorithms.clone());

    let output_hash = hash_file(&command.output, &summary_algorithms).map_err(|source| {
        AppError::HashOutputFile {
            path: command.output.clone(),
            source,
        }
    })?;
    let mut summary = DirHasherResult::default();
    summary
        .content
        .insert(command.output.to_string_lossy().to_string(), output_hash);

    let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let mut stderr = io::stderr().lock();
    writeln!(&mut stderr, "Generated with hashtree (rust) by {username}")?;
    pretty_print_result(&summary, &mut stderr, command.shared.encoding)?;

    Ok(())
}

fn verify_mode(command: &VerifyCommand) -> AppResult<()> {
    if matches!(command.shared.style, PersistenceStyle::Xml) {
        return Err(AppError::XmlNotSupported);
    }

    let verification = load_result(&command.input, command.shared.style)?;
    let verification_index = index_result_refs(&verification, command.ignore_case);

    let verify_eval = evaluate_verification(
        &command.shared.paths,
        &command.shared.algorithms,
        !command.shared.single,
        command.shared.jobs,
        command.shared.verbose,
        command.ignore_case,
        &verification_index,
    )?;

    let verification_not_matching_count =
        verification_index.len() - verify_eval.verification_match_keys.len();
    let difference_count =
        verify_eval.measured_not_matching_count + verification_not_matching_count;

    let mut out = io::stdout().lock();
    writeln!(out, "*** {} ***", Local::now().to_rfc2822())?;

    if difference_count == 0 {
        writeln!(out, "*** PASSED VERIFICATION ***")?;
        return Ok(());
    }

    if difference_count == 1
        && let Some(path) = single_difference_path(
            &verify_eval,
            &verification_index,
            verification_not_matching_count,
        )
        && is_same_file(Path::new(&path), &command.input)
    {
        writeln!(out, "*** PASSED VERIFICATION ***")?;
        writeln!(out, "Printing info on {}", command.input.display())?;
        if let Some((single_path, single_digests)) = verify_eval.single_printable_entry_for(&path) {
            let mut single = DirHasherResult::default();
            single.content.insert(single_path, single_digests);
            pretty_print_result(&single, &mut out, command.shared.encoding)?;
        } else if let Some(entry) = verification_index.values().find(|entry| entry.path == path) {
            let mut single = DirHasherResult::default();
            single
                .content
                .insert(entry.path.to_string(), entry.digests.clone());
            pretty_print_result(&single, &mut out, command.shared.encoding)?;
        }
        return Ok(());
    }

    writeln!(out, "There are differences.")?;

    writeln!(
        out,
        "These entries are seen, but missing in the {} list",
        command.input.display()
    )?;
    let mut seen_missing_entries = verify_eval.seen_missing_entries;
    seen_missing_entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (path, digests) in seen_missing_entries {
        print_digest_entry(&path, &digests, &mut out, command.shared.encoding)?;
    }

    writeln!(
        out,
        "These entries are in the {} list, but not in the directory and/or files.",
        command.input.display()
    )?;
    let mut listed_missing_paths: Vec<&String> = verification_index
        .keys()
        .filter(|key| !verify_eval.verification_subset_keys.contains(*key))
        .collect();
    listed_missing_paths.sort();
    for key in listed_missing_paths {
        if let Some(entry) = verification_index.get(key) {
            print_digest_entry(entry.path, entry.digests, &mut out, command.shared.encoding)?;
        }
    }

    Ok(())
}

struct VerifyEval {
    measured_not_matching_count: usize,
    first_measured_difference_path: Option<String>,
    verification_match_keys: HashSet<String>,
    verification_subset_keys: HashSet<String>,
    seen_missing_entries: Vec<(String, crate::model::DigestResult)>,
}

impl VerifyEval {
    fn single_printable_entry_for(
        &self,
        path: &str,
    ) -> Option<(String, crate::model::DigestResult)> {
        self.seen_missing_entries
            .iter()
            .find(|(candidate_path, _)| candidate_path == path)
            .map(|(entry_path, digests)| (entry_path.clone(), digests.clone()))
    }
}

fn evaluate_verification(
    paths: &[PathBuf],
    algorithms: &[Algorithm],
    multi_threaded: bool,
    jobs: Option<usize>,
    verbose: bool,
    ignore_case: bool,
    verification_index: &crate::verify::IndexedRefResult<'_>,
) -> AppResult<VerifyEval> {
    let mut verify_eval = VerifyEval {
        measured_not_matching_count: 0,
        first_measured_difference_path: None,
        verification_match_keys: HashSet::with_capacity(verification_index.len()),
        verification_subset_keys: HashSet::with_capacity(verification_index.len()),
        seen_missing_entries: Vec::new(),
    };

    for root_path in paths {
        let metadata = fs::symlink_metadata(root_path)
            .map_err(|_| AppError::FileDoesNotExist(root_path.clone()))?;

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.file_type().is_file() {
            let mut buffer = vec![0_u8; crate::BLOCK_READ_SIZE];
            if let Some(evaluation) = evaluate_one_file(
                root_path,
                algorithms,
                &mut buffer,
                ignore_case,
                verification_index,
                verbose,
            ) {
                apply_file_evaluation(&mut verify_eval, evaluation);
            }
            continue;
        }

        if metadata.file_type().is_dir() {
            if multi_threaded {
                let evaluate_parallel = || {
                    WalkDir::new(root_path)
                        .follow_links(false)
                        .into_iter()
                        .filter_map(Result::ok)
                        .filter(|entry| entry.file_type().is_file())
                        .par_bridge()
                        .map_init(
                            || vec![0_u8; crate::BLOCK_READ_SIZE],
                            |buffer, entry| {
                                evaluate_one_file(
                                    entry.path(),
                                    algorithms,
                                    buffer,
                                    ignore_case,
                                    verification_index,
                                    verbose,
                                )
                            },
                        )
                        .filter_map(|entry| entry)
                        .collect::<Vec<_>>()
                };

                let evaluations = if let Some(job_count) = jobs {
                    ThreadPoolBuilder::new()
                        .num_threads(job_count)
                        .build()
                        .map_err(|error| AppError::Message(error.to_string()))?
                        .install(evaluate_parallel)
                } else {
                    evaluate_parallel()
                };

                for evaluation in evaluations {
                    apply_file_evaluation(&mut verify_eval, evaluation);
                }
            } else {
                let mut buffer = vec![0_u8; crate::BLOCK_READ_SIZE];
                for entry in WalkDir::new(root_path).follow_links(false) {
                    let entry = match entry {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    if !entry.file_type().is_file() {
                        continue;
                    }

                    if let Some(evaluation) = evaluate_one_file(
                        entry.path(),
                        algorithms,
                        &mut buffer,
                        ignore_case,
                        verification_index,
                        verbose,
                    ) {
                        apply_file_evaluation(&mut verify_eval, evaluation);
                    }
                }
            }
        }
    }

    Ok(verify_eval)
}

fn apply_file_evaluation(verify_eval: &mut VerifyEval, evaluation: FileEvaluation) {
    if evaluation.verification_matches {
        verify_eval
            .verification_match_keys
            .insert(evaluation.key.clone());
    }
    if evaluation.verification_subset {
        verify_eval
            .verification_subset_keys
            .insert(evaluation.key.clone());
    }
    if let Some(path) = evaluation.measured_difference_path {
        verify_eval.measured_not_matching_count += 1;
        if verify_eval.first_measured_difference_path.is_none() {
            verify_eval.first_measured_difference_path = Some(path);
        }
    }
    if let Some(entry) = evaluation.seen_missing_entry {
        verify_eval.seen_missing_entries.push(entry);
    }
}

struct FileEvaluation {
    key: String,
    measured_difference_path: Option<String>,
    verification_matches: bool,
    verification_subset: bool,
    seen_missing_entry: Option<(String, crate::model::DigestResult)>,
}

fn evaluate_one_file(
    path: &Path,
    algorithms: &[Algorithm],
    buffer: &mut [u8],
    ignore_case: bool,
    verification_index: &crate::verify::IndexedRefResult<'_>,
    verbose: bool,
) -> Option<FileEvaluation> {
    let digest = match hash_file_with_buffer(path, algorithms, buffer) {
        Ok(digest) => digest,
        Err(error) => {
            if verbose {
                eprintln!("Could not hash {}: {error}", path.display());
            }
            return None;
        }
    };

    let key = normalized_path_string(path, ignore_case);
    let verification_entry = verification_index.get(&key);

    let measured_matches = verification_entry
        .map(|entry| digest.matches(entry.digests))
        .unwrap_or(false);
    let measured_subset = verification_entry
        .map(|entry| digest.equals_subset_of(entry.digests))
        .unwrap_or(false);
    let verification_matches = verification_entry
        .map(|entry| entry.digests.matches(&digest))
        .unwrap_or(false);
    let verification_subset = verification_entry
        .map(|entry| entry.digests.equals_subset_of(&digest))
        .unwrap_or(false);

    let mut path_string_cache: Option<String> = None;
    let mut get_path_string = || {
        path_string_cache
            .get_or_insert_with(|| path.to_string_lossy().to_string())
            .clone()
    };

    let measured_difference_path = if measured_matches {
        None
    } else {
        Some(get_path_string())
    };
    let seen_missing_entry = if measured_subset {
        None
    } else {
        Some((get_path_string(), digest))
    };

    Some(FileEvaluation {
        key,
        measured_difference_path,
        verification_matches,
        verification_subset,
        seen_missing_entry,
    })
}

fn single_difference_path(
    verify_eval: &VerifyEval,
    verification_index: &crate::verify::IndexedRefResult<'_>,
    verification_not_matching_count: usize,
) -> Option<String> {
    match (
        verify_eval.measured_not_matching_count,
        verification_not_matching_count,
    ) {
        (1, 0) => verify_eval.first_measured_difference_path.clone(),
        (0, 1) => verification_index.iter().find_map(|(key, entry)| {
            (!verify_eval.verification_match_keys.contains(key)).then(|| entry.path.to_string())
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithm::Algorithm;
    use crate::cli::AlgorithmSelection;

    use super::selected_algorithms;

    #[test]
    fn selected_algorithms_returns_expected_set() {
        let selection = AlgorithmSelection {
            sha256: true,
            md5: true,
            ..AlgorithmSelection::default()
        };

        let algorithms = selected_algorithms(&selection);
        assert_eq!(algorithms, vec![Algorithm::Md5, Algorithm::Sha256]);
    }

    #[test]
    fn selected_algorithms_all_includes_all_supported() {
        let selection = AlgorithmSelection {
            all: true,
            ..AlgorithmSelection::default()
        };

        let algorithms = selected_algorithms(&selection);
        assert_eq!(
            algorithms,
            vec![
                Algorithm::Md5,
                Algorithm::Sha1,
                Algorithm::Sha256,
                Algorithm::Sha384,
                Algorithm::Sha512,
                Algorithm::Sha3_256,
                Algorithm::Sha3_384,
                Algorithm::Sha3_512,
                Algorithm::Ripemd160,
            ]
        );
    }
}
