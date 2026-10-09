use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use digest::Digest as _;
use md2::Md2;
use md5::Md5;
use rayon::prelude::*;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use walkdir::WalkDir;

use crate::BLOCK_READ_SIZE;
use crate::error::{AppError, AppResult};
use crate::model::{DigestResult, DirHasherResult};
use crate::util::normalize_algorithm_name;

pub fn generate_digests(
    paths: &[PathBuf],
    algorithms: &[String],
    multi_threaded: bool,
    verbose: bool,
) -> AppResult<DirHasherResult> {
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

pub fn collect_files(paths: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            return Err(AppError::FileDoesNotExist(path.clone()));
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

pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn hash_file(path: &Path, algorithms: &[String]) -> io::Result<DigestResult> {
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

pub enum AnyHasher {
    Md2(Md2),
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

impl AnyHasher {
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Self::Md2(hasher) => hasher.update(data),
            Self::Md5(hasher) => hasher.update(data),
            Self::Sha1(hasher) => hasher.update(data),
            Self::Sha256(hasher) => hasher.update(data),
            Self::Sha384(hasher) => hasher.update(data),
            Self::Sha512(hasher) => hasher.update(data),
        }
    }

    pub fn finalize_hex(self) -> String {
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

pub fn create_hasher(name: &str) -> Option<AnyHasher> {
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
