use std::path::{Path, PathBuf};

pub fn split_hex(input: &str, group_size: usize) -> String {
    let mut output = String::new();
    for (index, chunk) in input.as_bytes().chunks(group_size).enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push_str(std::str::from_utf8(chunk).unwrap_or(""));
    }
    output
}

pub fn normalize_hex(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

pub fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

pub fn is_same_file(left: &Path, right: &Path) -> bool {
    if !(left.exists() && right.exists()) {
        return false;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
