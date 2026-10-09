use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use chrono::Local;

use crate::error::{AppError, AppResult};
use crate::model::{DigestResult, DirHasherResult};
use crate::style::PersistenceStyle;
use crate::util::split_hex;

pub fn persist_result(
    path: &Path,
    style: PersistenceStyle,
    result: &DirHasherResult,
) -> AppResult<()> {
    match style {
        PersistenceStyle::Json => {
            let file = File::create(path)?;
            serde_json::to_writer_pretty(file, result)?;
            Ok(())
        }
        PersistenceStyle::OldStyle => persist_oldstyle(path, result),
        PersistenceStyle::Xml => Err(AppError::XmlNotSupported),
    }
}

pub fn load_result(path: &Path, style: PersistenceStyle) -> AppResult<DirHasherResult> {
    match style {
        PersistenceStyle::Json => {
            let file = File::open(path)?;
            Ok(serde_json::from_reader(file)?)
        }
        PersistenceStyle::OldStyle => load_oldstyle(path),
        PersistenceStyle::Xml => Err(AppError::XmlNotSupported),
    }
}

fn persist_oldstyle(path: &Path, result: &DirHasherResult) -> AppResult<()> {
    let mut file = File::create(path)?;
    writeln!(
        &mut file,
        "Generated with: hashtree-rs {}",
        Local::now().to_rfc2822()
    )?;

    for (file_path, digest_result) in &result.content {
        writeln!(&mut file, "{file_path}")?;
        for (algorithm, hex_digest) in &digest_result.digests {
            let label = oldstyle_label(algorithm);
            let grouped = split_hex(hex_digest, 4);
            writeln!(&mut file, "\t{label}:\t{grouped}")?;
        }
    }

    Ok(())
}

fn load_oldstyle(path: &Path) -> AppResult<DirHasherResult> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;

    if !first_line.starts_with("Generated with: ") {
        return Err(AppError::InvalidOldStyleHeader);
    }

    let mut result = DirHasherResult::default();
    let mut current_file = String::new();
    let mut digest_result = DigestResult::default();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        if !line.starts_with('\t') {
            if !current_file.is_empty() {
                result
                    .content
                    .insert(current_file.clone(), digest_result.clone());
            }
            current_file = line;
            digest_result = DigestResult::default();
            continue;
        }

        if let Some((algorithm, hex_digest)) = parse_oldstyle_digest_line(&line) {
            digest_result.add_digest(&algorithm, &hex_digest);
        }
    }

    if !current_file.is_empty() {
        result.content.insert(current_file, digest_result);
    }

    Ok(result)
}

fn parse_oldstyle_digest_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let (prefix, rest) = trimmed.split_once(':')?;
    let algorithm = match prefix.to_ascii_uppercase().as_str() {
        "MD5" => "md5",
        "SHA" | "SHA-1" => "sha-1",
        "SHA256" | "SHA-256" => "sha-256",
        _ => return None,
    };

    let hex_digest: String = rest.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
    if hex_digest.is_empty() {
        None
    } else {
        Some((algorithm.to_string(), hex_digest.to_ascii_lowercase()))
    }
}

fn oldstyle_label(algorithm: &str) -> String {
    match algorithm {
        "md5" => "MD5".to_string(),
        "sha-1" => "SHA-1".to_string(),
        "sha-256" => "SHA-256".to_string(),
        _ => algorithm.to_ascii_uppercase(),
    }
}
