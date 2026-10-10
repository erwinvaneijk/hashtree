// SPDX-License-Identifier: EUPL-1.2
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use digest::Digest as _;
use md2::Md2;
use md5::Md5;
use rayon::ThreadPoolBuilder;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use walkdir::WalkDir;

use crate::BLOCK_READ_SIZE;
use crate::algorithm::Algorithm;
use crate::error::{AppError, AppResult};
use crate::model::{DigestResult, DirHasherResult};

pub fn generate_digests(
    paths: &[PathBuf],
    algorithms: &[Algorithm],
    multi_threaded: bool,
    jobs: Option<usize>,
    verbose: bool,
) -> AppResult<DirHasherResult> {
    let mut result = DirHasherResult::default();

    for path in paths {
        let metadata =
            fs::symlink_metadata(path).map_err(|_| AppError::FileDoesNotExist(path.clone()))?;

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.file_type().is_file() {
            if let Some((file_path, digest)) = hash_one_path(path, algorithms, verbose) {
                result.content.insert(file_path, digest);
            }
            continue;
        }

        if metadata.file_type().is_dir() {
            if multi_threaded {
                let run_parallel = || {
                    WalkDir::new(path)
                        .follow_links(false)
                        .into_iter()
                        .filter_map(Result::ok)
                        .filter(|entry| entry.file_type().is_file())
                        .par_bridge()
                        .map_init(
                            || vec![0_u8; BLOCK_READ_SIZE],
                            |buffer, entry| match hash_file_with_buffer(
                                entry.path(),
                                algorithms,
                                buffer,
                            ) {
                                Ok(digest) => {
                                    Some((entry.path().to_string_lossy().to_string(), digest))
                                }
                                Err(error) => {
                                    if verbose {
                                        eprintln!(
                                            "Could not hash {}: {error}",
                                            entry.path().display()
                                        );
                                    }
                                    None
                                }
                            },
                        )
                        .filter_map(|entry| entry)
                        .collect::<Vec<_>>()
                };

                let digest_pairs = if let Some(job_count) = jobs {
                    ThreadPoolBuilder::new()
                        .num_threads(job_count)
                        .build()
                        .map_err(|error| AppError::Message(error.to_string()))?
                        .install(run_parallel)
                } else {
                    run_parallel()
                };

                for (file_path, digest) in digest_pairs {
                    result.content.insert(file_path, digest);
                }
            } else {
                let mut buffer = vec![0_u8; BLOCK_READ_SIZE];
                for entry in WalkDir::new(path).follow_links(false) {
                    let entry = match entry {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    if !entry.file_type().is_file() {
                        continue;
                    }

                    match hash_file_with_buffer(entry.path(), algorithms, &mut buffer) {
                        Ok(digest) => {
                            result
                                .content
                                .insert(entry.path().to_string_lossy().to_string(), digest);
                        }
                        Err(error) => {
                            if verbose {
                                eprintln!("Could not hash {}: {error}", entry.path().display());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

fn hash_one_path(
    path: &Path,
    algorithms: &[Algorithm],
    verbose: bool,
) -> Option<(String, DigestResult)> {
    let mut buffer = vec![0_u8; BLOCK_READ_SIZE];
    match hash_file_with_buffer(path, algorithms, &mut buffer) {
        Ok(digest) => Some((path.to_string_lossy().to_string(), digest)),
        Err(error) => {
            if verbose {
                eprintln!("Could not hash {}: {error}", path.display());
            }
            None
        }
    }
}

pub fn collect_files(paths: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        let metadata =
            fs::symlink_metadata(path).map_err(|_| AppError::FileDoesNotExist(path.clone()))?;

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.file_type().is_file() {
            files.push(path.clone());
            continue;
        }

        if metadata.file_type().is_dir() {
            for entry in WalkDir::new(path).follow_links(false) {
                let entry = match entry {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                if entry.file_type().is_symlink() {
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

pub fn hash_file(path: &Path, algorithms: &[Algorithm]) -> io::Result<DigestResult> {
    let mut buffer = vec![0_u8; BLOCK_READ_SIZE];
    hash_file_with_buffer(path, algorithms, &mut buffer)
}

pub fn hash_file_with_buffer(
    path: &Path,
    algorithms: &[Algorithm],
    buffer: &mut [u8],
) -> io::Result<DigestResult> {
    let mut file = File::open(path)?;
    let mut hashers: Vec<(Algorithm, AnyHasher)> = algorithms
        .iter()
        .filter_map(|algorithm| create_hasher(algorithm).map(|hasher| (algorithm.clone(), hasher)))
        .collect();

    loop {
        let read = file.read(buffer)?;
        if read == 0 {
            break;
        }

        for (_, hasher) in &mut hashers {
            hasher.update(&buffer[..read]);
        }
    }

    let mut result = DigestResult::default();
    for (algorithm, hasher) in hashers {
        result.add_digest(algorithm, &hasher.finalize_hex());
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

pub fn create_hasher(name: &Algorithm) -> Option<AnyHasher> {
    match name {
        Algorithm::Md2 => Some(AnyHasher::Md2(Md2::new())),
        Algorithm::Md5 => Some(AnyHasher::Md5(Md5::new())),
        Algorithm::Sha1 => Some(AnyHasher::Sha1(Sha1::new())),
        Algorithm::Sha256 => Some(AnyHasher::Sha256(Sha256::new())),
        Algorithm::Sha384 => Some(AnyHasher::Sha384(Sha384::new())),
        Algorithm::Sha512 => Some(AnyHasher::Sha512(Sha512::new())),
        _ => None,
    }
}
