use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use chrono::Local;

use crate::algorithm::Algorithm;
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

    let mut entries: Vec<_> = result.content.iter().collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));

    for (file_path, digest_result) in entries {
        writeln!(&mut file, "{file_path}")?;
        for (algorithm, hex_digest) in &digest_result.digests {
            let label = algorithm.oldstyle_label();
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
            digest_result.add_digest(algorithm, &hex_digest);
        }
    }

    if !current_file.is_empty() {
        result.content.insert(current_file, digest_result);
    }

    Ok(result)
}

fn parse_oldstyle_digest_line(line: &str) -> Option<(Algorithm, String)> {
    let trimmed = line.trim();
    let (prefix, rest) = trimmed.split_once(':')?;
    let algorithm = Algorithm::from_oldstyle_prefix(prefix)?;

    let hex_digest: String = rest.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
    if hex_digest.is_empty() {
        None
    } else {
        Some((algorithm, hex_digest.to_ascii_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::algorithm::Algorithm;
    use crate::style::PersistenceStyle;

    use super::load_result;

    #[test]
    fn load_oldstyle_parses_known_hash_lines() {
        let temp = TempDir::new().expect("temp directory should be created");
        let file_path = temp.path().join("oldstyle.txt");

        let data = "Generated with: test\nexample.txt\n\tMD5:\t9001 5098 3cd2 4fb0 d696 3f7d 28e1 7f72\n\tSHA-1:\ta999 3e36 4706 816a ba3e 2571 7850 c26c 9cd0 d89d\n";
        fs::write(&file_path, data).expect("fixture file should be written");

        let result = load_result(&file_path, PersistenceStyle::OldStyle)
            .expect("old-style file should parse correctly");

        let digest = result
            .content
            .get("example.txt")
            .expect("example.txt should be present");

        assert_eq!(
            digest.digests.get(&Algorithm::Md5).map(String::as_str),
            Some("900150983cd24fb0d6963f7d28e17f72")
        );
        assert_eq!(
            digest.digests.get(&Algorithm::Sha1).map(String::as_str),
            Some("a9993e364706816aba3e25717850c26c9cd0d89d")
        );
    }
}
