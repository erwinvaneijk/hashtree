use std::path::Path;

use path_clean::PathClean;

use crate::hashing::is_symlink;
use crate::model::{DirHasherResult, IndexedEntry, IndexedResult};
use crate::util::absolute_path;

pub fn index_result(result: &DirHasherResult, ignore_case: bool) -> IndexedResult {
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

pub fn normalized_path_string(path: &Path, ignore_case: bool) -> String {
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

pub fn not_intersect(left: &IndexedResult, right: &IndexedResult) -> IndexedResult {
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

pub fn missing(left: &IndexedResult, right: &IndexedResult) -> IndexedResult {
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
