use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Local;

use crate::algorithm::Algorithm;
use crate::cli::{
    AlgorithmSelection, Cli, Command as CliCommand, CommonArgs, CreateArgs, VerifyArgs,
};
use crate::error::{AppError, AppResult};
use crate::hashing::{generate_digests, hash_file};
use crate::model::DirHasherResult;
use crate::persistence::{load_result, persist_result};
use crate::report::{pretty_print_result, print_missing_entries};
use crate::style::PersistenceStyle;
use crate::util::is_same_file;
use crate::verify::{index_result, missing, not_intersect};
use crate::{DEFAULT_ALGORITHM, VERSION};

#[derive(Debug, Clone)]
struct SharedCommandOptions {
    style: PersistenceStyle,
    algorithms: Vec<Algorithm>,
    verbose: bool,
    single: bool,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
struct CreateCommand {
    shared: SharedCommandOptions,
    output: PathBuf,
    force: bool,
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
    let mut algorithms = selected_algorithms(&common.algorithm_selection);
    if algorithms.is_empty() {
        algorithms.push(Algorithm::from_name(DEFAULT_ALGORITHM));
    }

    Ok(SharedCommandOptions {
        style: PersistenceStyle::parse(&common.style)?,
        algorithms,
        verbose: common.verbose,
        single: common.single,
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
    if selection.all || selection.md5 {
        names.insert(Algorithm::Md5);
    }
    if selection.all || selection.md2 {
        names.insert(Algorithm::Md2);
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
        command.shared.verbose,
    )?;
    persist_result(&command.output, command.shared.style, &digests)?;

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
    pretty_print_result(&summary, &mut stderr)?;

    Ok(())
}

fn verify_mode(command: &VerifyCommand) -> AppResult<()> {
    if matches!(command.shared.style, PersistenceStyle::Xml) {
        return Err(AppError::XmlNotSupported);
    }

    let verification = load_result(&command.input, command.shared.style)?;
    let measured = generate_digests(
        &command.shared.paths,
        &command.shared.algorithms,
        !command.shared.single,
        command.shared.verbose,
    )?;

    let measured_index = index_result(&measured, command.ignore_case);
    let verification_index = index_result(&verification, command.ignore_case);

    let differences = not_intersect(&measured_index, &verification_index);

    let mut out = io::stdout().lock();
    writeln!(out, "*** {} ***", Local::now().to_rfc2822())?;

    if differences.is_empty() {
        writeln!(out, "*** PASSED VERIFICATION ***")?;
        return Ok(());
    }

    if differences.len() == 1
        && let Some(entry) = differences.values().next()
        && is_same_file(Path::new(&entry.path), &command.input)
    {
        writeln!(out, "*** PASSED VERIFICATION ***")?;
        writeln!(out, "Printing info on {}", command.input.display())?;
        let mut single = DirHasherResult::default();
        single
            .content
            .insert(entry.path.clone(), entry.digests.clone());
        pretty_print_result(&single, &mut out)?;
        return Ok(());
    }

    writeln!(out, "There are differences.")?;

    let seen_missing = missing(&measured_index, &verification_index);
    writeln!(
        out,
        "These entries are seen, but missing in the {} list",
        command.input.display()
    )?;
    print_missing_entries(&seen_missing, &mut out)?;

    let listed_missing = missing(&verification_index, &measured_index);
    writeln!(
        out,
        "These entries are in the {} list, but not in the directory and/or files.",
        command.input.display()
    )?;
    print_missing_entries(&listed_missing, &mut out)?;

    Ok(())
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
                Algorithm::Md2,
                Algorithm::Md5,
                Algorithm::Sha1,
                Algorithm::Sha256,
                Algorithm::Sha384,
                Algorithm::Sha512,
            ]
        );
    }
}
