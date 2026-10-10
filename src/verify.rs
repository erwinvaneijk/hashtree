// SPDX-License-Identifier: EUPL-1.2
use std::collections::HashMap;
use std::path::Path;

use path_clean::PathClean;

use crate::model::{DigestResult, DirHasherResult, IndexedEntry, IndexedResult};
use crate::util::absolute_path;

pub struct IndexedRefEntry<'a> {
    pub path: &'a str,
    pub digests: &'a DigestResult,
}

pub type IndexedRefResult<'a> = HashMap<String, IndexedRefEntry<'a>>;

pub fn index_result_refs<'a>(
    result: &'a DirHasherResult,
    ignore_case: bool,
) -> IndexedRefResult<'a> {
    let mut indexed = IndexedRefResult::with_capacity(result.content.len());
    for (path, digests) in &result.content {
        let normalized = normalized_path_string(Path::new(path), ignore_case);
        indexed.insert(normalized, IndexedRefEntry { path, digests });
    }
    indexed
}

pub fn not_intersect_count_refs(
    left: &IndexedRefResult<'_>,
    right: &IndexedRefResult<'_>,
) -> usize {
    let mut count = 0;

    for (key, entry) in left {
        let same = right
            .get(key)
            .map(|other| entry.digests.matches(other.digests))
            .unwrap_or(false);
        if !same {
            count += 1;
        }
    }

    for (key, entry) in right {
        let same = left
            .get(key)
            .map(|other| entry.digests.matches(other.digests))
            .unwrap_or(false);
        if !same {
            count += 1;
        }
    }

    count
}

pub fn single_not_intersect_path_refs(
    left: &IndexedRefResult<'_>,
    right: &IndexedRefResult<'_>,
) -> Option<String> {
    let mut found: Option<String> = None;

    for (key, entry) in left {
        let same = right
            .get(key)
            .map(|other| entry.digests.matches(other.digests))
            .unwrap_or(false);
        if !same {
            if found.is_some() {
                return None;
            }
            found = Some(entry.path.to_string());
        }
    }

    for (key, entry) in right {
        let same = left
            .get(key)
            .map(|other| entry.digests.matches(other.digests))
            .unwrap_or(false);
        if !same {
            if found.is_some() {
                return None;
            }
            found = Some(entry.path.to_string());
        }
    }

    found
}

pub fn missing_refs<'a>(
    left: &'a IndexedRefResult<'a>,
    right: &IndexedRefResult<'_>,
) -> Vec<&'a IndexedRefEntry<'a>> {
    let mut missing = Vec::new();

    for (key, entry) in left {
        let present = right
            .get(key)
            .map(|other| entry.digests.equals_subset_of(other.digests))
            .unwrap_or(false);
        if !present {
            missing.push(entry);
        }
    }

    missing
}

pub fn find_entry_by_path<'a>(
    index: &'a IndexedRefResult<'a>,
    path: &str,
) -> Option<&'a IndexedRefEntry<'a>> {
    index.values().find(|entry| entry.path == path)
}

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
    let normalized = absolute_path(path).clean();

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
