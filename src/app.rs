use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use chrono::Local;

use crate::algorithm::Algorithm;
use crate::cli::Cli;
use crate::error::{AppError, AppResult};
use crate::hashing::{generate_digests, hash_file};
use crate::model::DirHasherResult;
use crate::persistence::{load_result, persist_result};
use crate::report::{pretty_print_result, print_missing_entries};
use crate::style::PersistenceStyle;
use crate::util::is_same_file;
use crate::verify::{index_result, missing, not_intersect};
use crate::{DEFAULT_ALGORITHM, VERSION};

pub fn run(cli: Cli) -> ExitCode {
    let start = std::time::Instant::now();
    eprintln!("Version: {VERSION}");

    let style = match PersistenceStyle::parse(&cli.style) {
        Ok(style) => style,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let mut algorithms = selected_algorithms(&cli);
    if algorithms.is_empty() {
        algorithms.push(Algorithm::from_name(DEFAULT_ALGORITHM));
    }

    let status = if cli.input.is_some() && cli.output.is_some() {
        eprintln!("Make up your mind. Cannot do -i and -o at the same time.");
        ExitCode::from(1)
    } else if let Some(input_file) = &cli.input {
        match verify_mode(&cli, style, &algorithms, input_file) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(254)
            }
        }
    } else if let Some(output_file) = &cli.output {
        match write_mode(&cli, style, &algorithms, output_file) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(255)
            }
        }
    } else {
        eprintln!("You need either -i or -o");
        ExitCode::from(2)
    };

    eprintln!("Done: {:.3}", start.elapsed().as_secs_f64());
    status
}

fn selected_algorithms(cli: &Cli) -> Vec<Algorithm> {
    let mut names = BTreeSet::new();

    if cli.all || cli.sha256 {
        names.insert(Algorithm::Sha256);
    }
    if cli.all || cli.sha1 {
        names.insert(Algorithm::Sha1);
    }
    if cli.all || cli.sha384 {
        names.insert(Algorithm::Sha384);
    }
    if cli.all || cli.sha512 {
        names.insert(Algorithm::Sha512);
    }
    if cli.all || cli.md5 {
        names.insert(Algorithm::Md5);
    }
    if cli.all || cli.md2 {
        names.insert(Algorithm::Md2);
    }

    names.into_iter().collect()
}

fn write_mode(
    cli: &Cli,
    style: PersistenceStyle,
    algorithms: &[Algorithm],
    output_file: &Path,
) -> AppResult<()> {
    if matches!(style, PersistenceStyle::Xml) {
        return Err(AppError::XmlNotSupported);
    }

    if output_file.exists() && !cli.force {
        return Err(AppError::OutputExists(output_file.to_path_buf()));
    }

    let digests = generate_digests(&cli.paths, algorithms, !cli.single, cli.verbose)?;
    persist_result(output_file, style, &digests)?;

    let summary_algorithms = digests
        .content
        .values()
        .next()
        .map(|digest| digest.algorithms_enum())
        .unwrap_or_else(|| algorithms.to_vec());

    let output_hash =
        hash_file(output_file, &summary_algorithms).map_err(|source| AppError::HashOutputFile {
            path: output_file.to_path_buf(),
            source,
        })?;
    let mut summary = DirHasherResult::default();
    summary
        .content
        .insert(output_file.to_string_lossy().to_string(), output_hash);

    let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let mut stderr = io::stderr().lock();
    writeln!(&mut stderr, "Generated with hashtree (rust) by {username}")?;
    pretty_print_result(&summary, &mut stderr)?;

    Ok(())
}

fn verify_mode(
    cli: &Cli,
    style: PersistenceStyle,
    algorithms: &[Algorithm],
    input_file: &Path,
) -> AppResult<()> {
    if matches!(style, PersistenceStyle::Xml) {
        return Err(AppError::XmlNotSupported);
    }

    let verification = load_result(input_file, style)?;
    let measured = generate_digests(&cli.paths, algorithms, !cli.single, cli.verbose)?;

    let measured_index = index_result(&measured, cli.ignore_case);
    let verification_index = index_result(&verification, cli.ignore_case);

    let differences = not_intersect(&measured_index, &verification_index);

    let mut out = io::stdout().lock();
    writeln!(out, "*** {} ***", Local::now().to_rfc2822())?;

    if differences.is_empty() {
        writeln!(out, "*** PASSED VERIFICATION ***")?;
        return Ok(());
    }

    if differences.len() == 1
        && let Some(entry) = differences.values().next()
        && is_same_file(Path::new(&entry.path), input_file)
    {
        writeln!(out, "*** PASSED VERIFICATION ***")?;
        writeln!(out, "Printing info on {}", input_file.display())?;
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
        input_file.display()
    )?;
    print_missing_entries(&seen_missing, &mut out)?;

    let listed_missing = missing(&verification_index, &measured_index);
    writeln!(
        out,
        "These entries are in the {} list, but not in the directory and/or files.",
        input_file.display()
    )?;
    print_missing_entries(&listed_missing, &mut out)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::algorithm::Algorithm;
    use crate::cli::Cli;

    use super::selected_algorithms;

    fn base_cli() -> Cli {
        Cli {
            sha1: false,
            sha256: false,
            sha384: false,
            sha512: false,
            md5: false,
            md2: false,
            all: false,
            ignore_case: false,
            verbose: false,
            force: false,
            single: false,
            output: None,
            input: None,
            style: "json".to_string(),
            paths: vec![PathBuf::from(".")],
        }
    }

    #[test]
    fn selected_algorithms_returns_expected_set() {
        let mut cli = base_cli();
        cli.sha256 = true;
        cli.md5 = true;

        let algorithms = selected_algorithms(&cli);
        assert_eq!(algorithms, vec![Algorithm::Md5, Algorithm::Sha256]);
    }

    #[test]
    fn selected_algorithms_all_includes_all_supported() {
        let mut cli = base_cli();
        cli.all = true;

        let algorithms = selected_algorithms(&cli);
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
