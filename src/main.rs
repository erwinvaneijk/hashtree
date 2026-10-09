use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Local;
use clap::Parser;
use digest::Digest as _;
use md2::Md2;
use md5::Md5;
use path_clean::PathClean;
use rayon::prelude::*;
use serde::de::Error as _;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use walkdir::WalkDir;

const BLOCK_READ_SIZE: usize = 1024 * 1024;
const DEFAULT_ALGORITHM: &str = "sha-256";
const VERSION: &str = "1.2-rust";

#[derive(Debug, Clone, Parser)]
#[command(
    name = "hashtree",
    about = "Creating a list of digests for files and/or directories"
)]
struct Cli {
    #[arg(short = '1', long = "sha-1")]
    sha1: bool,

    #[arg(short = '2', long = "sha-256")]
    sha256: bool,

    #[arg(long = "sha-384")]
    sha384: bool,

    #[arg(long = "sha-512")]
    sha512: bool,

    #[arg(long = "md5")]
    md5: bool,

    #[arg(long = "md2")]
    md2: bool,

    #[arg(short = 'a', long = "all")]
    all: bool,

    #[arg(short = 'n', long = "ignorecase")]
    ignore_case: bool,

    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    #[arg(short = 'f', long = "force")]
    force: bool,

    #[arg(long = "single")]
    single: bool,

    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    #[arg(short = 'i', long = "input")]
    input: Option<PathBuf>,

    #[arg(short = 's', long = "style", default_value = "json")]
    style: String,

    #[arg(value_name = "dir")]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
enum PersistenceStyle {
    Json,
    OldStyle,
    Xml,
}

impl PersistenceStyle {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "json" => Ok(Self::Json),
            "old" => Ok(Self::OldStyle),
            "xml" => Ok(Self::Xml),
            _ => Err(format!("We do not support option {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstructionInfo {
    username: String,
    version_information: String,
    construction_date: serde_json::Value,
    operating_system: String,
}

impl Default for ConstructionInfo {
    fn default() -> Self {
        let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let operating_system = format!("{}:unknown", std::env::consts::OS);
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string();

        Self {
            username,
            version_information: VERSION.to_string(),
            construction_date: serde_json::Value::String(timestamp),
            operating_system,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DigestResult {
    digests: BTreeMap<String, String>,
}

impl DigestResult {
    fn add_digest(&mut self, algorithm: &str, hex_digest: &str) {
        let algorithm = normalize_algorithm_name(algorithm);
        let hex_digest = normalize_hex(hex_digest);
        self.digests.insert(algorithm, hex_digest);
    }

    fn matches(&self, other: &Self) -> bool {
        let common: Vec<&str> = self
            .digests
            .keys()
            .filter(|name| other.digests.contains_key(*name))
            .map(String::as_str)
            .collect();

        if common.is_empty() {
            return false;
        }

        common
            .into_iter()
            .all(|algorithm| self.digests.get(algorithm) == other.digests.get(algorithm))
    }

    fn equals_subset_of(&self, other: &Self) -> bool {
        self.digests
            .iter()
            .all(|(algorithm, hex)| other.digests.get(algorithm) == Some(hex))
    }

    fn algorithms(&self) -> Vec<String> {
        self.digests.keys().cloned().collect()
    }
}

impl Serialize for DigestResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.digests.len()))?;
        for (algorithm, hex_digest) in &self.digests {
            seq.serialize_element(&format!("{algorithm}:{hex_digest}"))?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DigestResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Vec::<String>::deserialize(deserializer)?;
        let mut result = Self::default();
        for digest in raw {
            let (algorithm, value) = digest
                .split_once(':')
                .ok_or_else(|| D::Error::custom(format!("Invalid digest format: {digest}")))?;
            result.add_digest(algorithm, value);
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirHasherResult {
    #[serde(rename = "constructionInfo")]
    construction_info: ConstructionInfo,
    content: BTreeMap<String, DigestResult>,
}

impl Default for DirHasherResult {
    fn default() -> Self {
        Self {
            construction_info: ConstructionInfo::default(),
            content: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct IndexedEntry {
    path: String,
    digests: DigestResult,
}

type IndexedResult = BTreeMap<String, IndexedEntry>;

fn main() -> ExitCode {
    let start = std::time::Instant::now();
    eprintln!("Version: {VERSION}");

    let cli = Cli::parse();
    let style = match PersistenceStyle::parse(&cli.style) {
        Ok(style) => style,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let mut algorithms = selected_algorithms(&cli);
    if algorithms.is_empty() {
        algorithms.push(DEFAULT_ALGORITHM.to_string());
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

fn selected_algorithms(cli: &Cli) -> Vec<String> {
    let mut names = BTreeSet::new();

    if cli.all || cli.sha256 {
        names.insert("sha-256".to_string());
    }
    if cli.all || cli.sha1 {
        names.insert("sha-1".to_string());
    }
    if cli.all || cli.sha384 {
        names.insert("sha-384".to_string());
    }
    if cli.all || cli.sha512 {
        names.insert("sha-512".to_string());
    }
    if cli.all || cli.md5 {
        names.insert("md5".to_string());
    }
    if cli.all || cli.md2 {
        names.insert("md2".to_string());
    }

    names.into_iter().collect()
}

fn write_mode(
    cli: &Cli,
    style: PersistenceStyle,
    algorithms: &[String],
    output_file: &Path,
) -> Result<(), String> {
    if matches!(style, PersistenceStyle::Xml) {
        return Err("XML is not yet supported.".to_string());
    }

    if output_file.exists() && !cli.force {
        return Err(format!(
            "File {} exists or not forced to be overwritten. Stop.",
            output_file.display()
        ));
    }

    let digests = generate_digests(&cli.paths, algorithms, !cli.single, cli.verbose)?;
    persist_result(output_file, style, &digests)?;

    let summary_algorithms = digests
        .content
        .values()
        .next()
        .map(DigestResult::algorithms)
        .unwrap_or_else(|| algorithms.to_vec());

    let output_hash = hash_file(output_file, &summary_algorithms)
        .map_err(|error| format!("Cannot hash output file {}: {error}", output_file.display()))?;
    let mut summary = DirHasherResult::default();
    summary
        .content
        .insert(output_file.to_string_lossy().to_string(), output_hash);

    let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let mut stderr = io::stderr().lock();
    writeln!(&mut stderr, "Generated with hashtree (rust) by {username}")
        .map_err(|error| error.to_string())?;
    pretty_print_result(&summary, &mut stderr).map_err(|error| error.to_string())?;

    Ok(())
}

fn verify_mode(
    cli: &Cli,
    style: PersistenceStyle,
    algorithms: &[String],
    input_file: &Path,
) -> Result<(), String> {
    if matches!(style, PersistenceStyle::Xml) {
        return Err("XML is not yet supported.".to_string());
    }

    let verification = load_result(input_file, style)?;
    let measured = generate_digests(&cli.paths, algorithms, !cli.single, cli.verbose)?;

    let measured_index = index_result(&measured, cli.ignore_case);
    let verification_index = index_result(&verification, cli.ignore_case);

    let differences = not_intersect(&measured_index, &verification_index);

    let mut out = io::stdout().lock();
    writeln!(out, "*** {} ***", Local::now().to_rfc2822()).map_err(|error| error.to_string())?;

    if differences.is_empty() {
        writeln!(out, "*** PASSED VERIFICATION ***").map_err(|error| error.to_string())?;
        return Ok(());
    }

    if differences.len() == 1 {
        if let Some(entry) = differences.values().next()
            && is_same_file(Path::new(&entry.path), input_file)
        {
            writeln!(out, "*** PASSED VERIFICATION ***").map_err(|error| error.to_string())?;
            writeln!(out, "Printing info on {}", input_file.display())
                .map_err(|error| error.to_string())?;
            let mut single = DirHasherResult::default();
            single
                .content
                .insert(entry.path.clone(), entry.digests.clone());
            pretty_print_result(&single, &mut out).map_err(|error| error.to_string())?;
            return Ok(());
        }
    }

    writeln!(out, "There are differences.").map_err(|error| error.to_string())?;

    let seen_missing = missing(&measured_index, &verification_index);
    writeln!(
        out,
        "These entries are seen, but missing in the {} list",
        input_file.display()
    )
    .map_err(|error| error.to_string())?;
    print_missing_entries(&seen_missing, &mut out).map_err(|error| error.to_string())?;

    let listed_missing = missing(&verification_index, &measured_index);
    writeln!(
        out,
        "These entries are in the {} list, but not in the directory and/or files.",
        input_file.display()
    )
    .map_err(|error| error.to_string())?;
    print_missing_entries(&listed_missing, &mut out).map_err(|error| error.to_string())?;

    Ok(())
}

fn generate_digests(
    paths: &[PathBuf],
    algorithms: &[String],
    multi_threaded: bool,
    verbose: bool,
) -> Result<DirHasherResult, String> {
    let mut result = DirHasherResult::default();
    let files = collect_files(paths)?;

    let digest_pairs: Vec<(String, DigestResult)> = if multi_threaded {
        files
            .par_iter()
            .filter_map(|path| match hash_file(path, algorithms) {
                Ok(digest) => Some((path.to_string_lossy().to_string(), digest)),
                Err(error) => {
                    if verbose {
                        eprintln!("Could not hash {}: {error}", path.display());
                    }
                    None
                }
            })
            .collect()
    } else {
        files
            .iter()
            .filter_map(|path| match hash_file(path, algorithms) {
                Ok(digest) => Some((path.to_string_lossy().to_string(), digest)),
                Err(error) => {
                    if verbose {
                        eprintln!("Could not hash {}: {error}", path.display());
                    }
                    None
                }
            })
            .collect()
    };

    for (path, digest) in digest_pairs {
        result.content.insert(path, digest);
    }

    Ok(result)
}

fn collect_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            return Err(format!("File {} does not exist", path.display()));
        }

        if is_symlink(path) {
            continue;
        }

        if path.is_file() {
            files.push(path.clone());
            continue;
        }

        if path.is_dir() {
            for entry in WalkDir::new(path).follow_links(false) {
                let entry = match entry {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                if is_symlink(entry.path()) {
                    continue;
                }

                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn hash_file(path: &Path, algorithms: &[String]) -> io::Result<DigestResult> {
    let mut file = File::open(path)?;
    let mut hashers: Vec<(String, AnyHasher)> = algorithms
        .iter()
        .filter_map(|name| {
            create_hasher(name).map(|hasher| (normalize_algorithm_name(name), hasher))
        })
        .collect();

    let mut buffer = vec![0_u8; BLOCK_READ_SIZE];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        for (_, hasher) in &mut hashers {
            hasher.update(&buffer[..read]);
        }
    }

    let mut result = DigestResult::default();
    for (algorithm, hasher) in hashers {
        result.add_digest(&algorithm, &hasher.finalize_hex());
    }

    Ok(result)
}

enum AnyHasher {
    Md2(Md2),
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

impl AnyHasher {
    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Md2(hasher) => hasher.update(data),
            Self::Md5(hasher) => hasher.update(data),
            Self::Sha1(hasher) => hasher.update(data),
            Self::Sha256(hasher) => hasher.update(data),
            Self::Sha384(hasher) => hasher.update(data),
            Self::Sha512(hasher) => hasher.update(data),
        }
    }

    fn finalize_hex(self) -> String {
        match self {
            Self::Md2(hasher) => hex::encode(hasher.finalize()),
            Self::Md5(hasher) => hex::encode(hasher.finalize()),
            Self::Sha1(hasher) => hex::encode(hasher.finalize()),
            Self::Sha256(hasher) => hex::encode(hasher.finalize()),
            Self::Sha384(hasher) => hex::encode(hasher.finalize()),
            Self::Sha512(hasher) => hex::encode(hasher.finalize()),
        }
    }
}

fn create_hasher(name: &str) -> Option<AnyHasher> {
    match normalize_algorithm_name(name).as_str() {
        "md2" => Some(AnyHasher::Md2(Md2::new())),
        "md5" => Some(AnyHasher::Md5(Md5::new())),
        "sha-1" => Some(AnyHasher::Sha1(Sha1::new())),
        "sha-256" => Some(AnyHasher::Sha256(Sha256::new())),
        "sha-384" => Some(AnyHasher::Sha384(Sha384::new())),
        "sha-512" => Some(AnyHasher::Sha512(Sha512::new())),
        _ => None,
    }
}

fn persist_result(
    path: &Path,
    style: PersistenceStyle,
    result: &DirHasherResult,
) -> Result<(), String> {
    match style {
        PersistenceStyle::Json => {
            let file = File::create(path).map_err(|error| error.to_string())?;
            serde_json::to_writer_pretty(file, result).map_err(|error| error.to_string())
        }
        PersistenceStyle::OldStyle => persist_oldstyle(path, result),
        PersistenceStyle::Xml => Err("XML is not yet supported.".to_string()),
    }
}

fn load_result(path: &Path, style: PersistenceStyle) -> Result<DirHasherResult, String> {
    match style {
        PersistenceStyle::Json => {
            let file = File::open(path).map_err(|error| error.to_string())?;
            serde_json::from_reader(file).map_err(|error| error.to_string())
        }
        PersistenceStyle::OldStyle => load_oldstyle(path),
        PersistenceStyle::Xml => Err("XML is not yet supported.".to_string()),
    }
}

fn persist_oldstyle(path: &Path, result: &DirHasherResult) -> Result<(), String> {
    let mut file = File::create(path).map_err(|error| error.to_string())?;
    writeln!(
        &mut file,
        "Generated with: hashtree-rs {}",
        Local::now().to_rfc2822()
    )
    .map_err(|error| error.to_string())?;

    for (file_path, digest_result) in &result.content {
        writeln!(&mut file, "{file_path}").map_err(|error| error.to_string())?;
        for (algorithm, hex_digest) in &digest_result.digests {
            let label = oldstyle_label(algorithm);
            let grouped = split_hex(hex_digest, 4);
            writeln!(&mut file, "\t{label}:\t{grouped}").map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn load_oldstyle(path: &Path) -> Result<DirHasherResult, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .map_err(|error| error.to_string())?;

    if !first_line.starts_with("Generated with: ") {
        return Err("The first line is not ok.".to_string());
    }

    let mut result = DirHasherResult::default();
    let mut current_file = String::new();
    let mut digest_result = DigestResult::default();

    for line_result in reader.lines() {
        let line = line_result.map_err(|error| error.to_string())?;
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

fn split_hex(input: &str, group_size: usize) -> String {
    let mut output = String::new();
    for (index, chunk) in input.as_bytes().chunks(group_size).enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push_str(std::str::from_utf8(chunk).unwrap_or(""));
    }
    output
}

fn normalize_algorithm_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn normalize_hex(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn index_result(result: &DirHasherResult, ignore_case: bool) -> IndexedResult {
    let mut indexed = IndexedResult::new();
    for (path, digests) in &result.content {
        let normalized = normalized_path_string(Path::new(path), ignore_case);
        indexed.insert(
            normalized,
            IndexedEntry {
                path: path.clone(),
                digests: digests.clone(),
            },
        );
    }
    indexed
}

fn normalized_path_string(path: &Path, ignore_case: bool) -> String {
    let normalized = if path.exists() {
        if is_symlink(path) {
            absolute_path(path).clean()
        } else {
            path.canonicalize()
                .unwrap_or_else(|_| absolute_path(path).clean())
        }
    } else {
        absolute_path(path).clean()
    };

    let value = normalized.to_string_lossy().to_string();
    if ignore_case {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn not_intersect(left: &IndexedResult, right: &IndexedResult) -> IndexedResult {
    let mut result = IndexedResult::new();

    for (key, entry) in left {
        let should_insert = match right.get(key) {
            Some(other) => !entry.digests.matches(&other.digests),
            None => true,
        };

        if should_insert {
            result.insert(key.clone(), entry.clone());
        }
    }

    for (key, entry) in right {
        let should_insert = match left.get(key) {
            Some(other) => !entry.digests.matches(&other.digests),
            None => true,
        };

        if should_insert {
            result.insert(key.clone(), entry.clone());
        }
    }

    result
}

fn missing(left: &IndexedResult, right: &IndexedResult) -> IndexedResult {
    let mut result = IndexedResult::new();

    for (key, entry) in left {
        let present = right
            .get(key)
            .map(|other| entry.digests.equals_subset_of(&other.digests))
            .unwrap_or(false);

        if !present {
            result.insert(key.clone(), entry.clone());
        }
    }

    result
}

fn print_missing_entries(entries: &IndexedResult, out: &mut dyn Write) -> io::Result<()> {
    for entry in entries.values() {
        writeln!(out, "{}", entry.path)?;
        for (algorithm, hex_digest) in &entry.digests.digests {
            writeln!(out, "\t{algorithm}\t{hex_digest}")?;
        }
    }
    Ok(())
}

fn pretty_print_result(result: &DirHasherResult, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{} elements", result.content.len())?;
    for (path, digest_result) in &result.content {
        writeln!(out, "\t{path}")?;
        for (algorithm, hex_digest) in &digest_result.digests {
            let mut line = String::new();
            let _ = write!(&mut line, "{}\t{}", algorithm, split_hex(hex_digest, 4));
            writeln!(out, "\t\t{line}")?;
        }
    }
    out.flush()
}

fn is_same_file(left: &Path, right: &Path) -> bool {
    if !(left.exists() && right.exists()) {
        return false;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
