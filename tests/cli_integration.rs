// SPDX-License-Identifier: EUPL-1.2
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn generate_json_manifest_contains_expected_shape() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes.json");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success();

    assert!(output_manifest.exists(), "manifest should exist");

    let raw = fs::read_to_string(output_manifest).expect("manifest should be readable");
    let json: Value = serde_json::from_str(&raw).expect("manifest should be valid json");

    assert!(
        json.get("constructionInfo").is_some(),
        "constructionInfo must be present"
    );

    let content = json
        .get("content")
        .and_then(Value::as_object)
        .expect("content should be an object");

    assert_eq!(content.len(), 3, "fixture should hash exactly three files");

    for digest_array in content.values() {
        let digests = digest_array
            .as_array()
            .expect("digest entry should be an array");
        assert!(
            !digests.is_empty(),
            "every file should have at least one digest"
        );
        assert!(
            digests
                .iter()
                .filter_map(Value::as_str)
                .any(|digest| digest.starts_with("sha-256:")),
            "each file should include a sha-256 digest"
        );
    }
}

#[test]
fn verify_mode_passes_for_fresh_manifest() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes.json");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success();

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("verify")
        .arg("-i")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("*** PASSED VERIFICATION ***"));
}

#[test]
fn verify_mode_reports_differences_when_file_changes() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes.json");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success();

    fs::write(
        tree_dir.join("alpha.txt"),
        "This file changed after manifest generation\n",
    )
    .expect("should be able to modify fixture copy");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("verify")
        .arg("-i")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("There are differences."));
}

#[test]
fn old_style_roundtrip_verification_passes() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes-old.txt");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("-s")
        .arg("old")
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success();

    let first_line = fs::read_to_string(&output_manifest)
        .expect("old-style output should be readable")
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    assert!(
        first_line.starts_with("Generated with: "),
        "old-style output should start with expected header"
    );

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("verify")
        .arg("-i")
        .arg(&output_manifest)
        .arg("-s")
        .arg("old")
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("*** PASSED VERIFICATION ***"));
}

#[test]
fn output_file_requires_force_overwrite() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes.json");

    fs::write(&output_manifest, "already exists\n")
        .expect("should create pre-existing output file");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "exists or not forced to be overwritten",
        ));

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--force")
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success();
}

#[test]
fn create_supports_jobs_and_compact_json() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes-compact.json");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--sha-256")
        .arg("--jobs")
        .arg("2")
        .arg("--compact-json")
        .arg(&tree_dir)
        .assert()
        .success();

    let raw = fs::read_to_string(&output_manifest).expect("manifest should be readable");
    assert!(
        !raw.contains("\n  \"constructionInfo\""),
        "compact json should not be pretty-printed"
    );

    let json: Value = serde_json::from_str(&raw).expect("manifest should still be valid json");
    assert!(json.get("content").is_some(), "content must be present");
}

#[test]
fn base64_manifest_encodes_long_digests_only() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes-base64.json");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("--base64")
        .arg("--sha-256")
        .arg("--md5")
        .arg(&tree_dir)
        .assert()
        .success();

    let raw = fs::read_to_string(&output_manifest).expect("manifest should be readable");
    let json: Value = serde_json::from_str(&raw).expect("manifest should be valid json");
    let content = json
        .get("content")
        .and_then(Value::as_object)
        .expect("content should be an object");

    for digest_array in content.values() {
        let digests: Vec<&str> = digest_array
            .as_array()
            .expect("digest entry should be an array")
            .iter()
            .filter_map(Value::as_str)
            .collect();

        let sha256 = digests
            .iter()
            .find_map(|digest| digest.strip_prefix("sha-256:"))
            .expect("sha-256 digest should be present");
        assert_eq!(
            sha256.len(),
            44,
            "a 256 bit digest should be 44 base64 characters, got {sha256}"
        );
        assert!(
            sha256.ends_with('='),
            "base64 digest should be padded, got {sha256}"
        );

        let md5 = digests
            .iter()
            .find_map(|digest| digest.strip_prefix("md5:"))
            .expect("md5 digest should be present");
        assert_eq!(
            md5.len(),
            32,
            "a 128 bit digest should stay hexadecimal, got {md5}"
        );
        assert!(
            md5.chars().all(|character| character.is_ascii_hexdigit()),
            "a 128 bit digest should stay hexadecimal, got {md5}"
        );
    }
}

#[test]
fn verification_accepts_either_encoding() {
    let (temp, tree_dir) = copy_fixture_tree();
    let hex_manifest = temp.path().join("hashes-hex.json");
    let base64_manifest = temp.path().join("hashes-base64.json");

    for (manifest, extra_arguments) in [
        (&hex_manifest, Vec::new()),
        (&base64_manifest, vec!["--base64"]),
    ] {
        let mut command = Command::cargo_bin("hashtree").expect("binary should build");
        command.arg("create").arg("-o").arg(manifest);
        command.args(&extra_arguments);
        command.arg("--sha-256").arg(&tree_dir).assert().success();
    }

    // Both manifests must verify, whether or not base64 output is requested.
    for manifest in [&hex_manifest, &base64_manifest] {
        for extra_arguments in [Vec::new(), vec!["--base64"]] {
            let mut command = Command::cargo_bin("hashtree").expect("binary should build");
            command.arg("verify").arg("-i").arg(manifest);
            command.args(&extra_arguments);
            command
                .arg("--sha-256")
                .arg(&tree_dir)
                .assert()
                .success()
                .stdout(predicate::str::contains("*** PASSED VERIFICATION ***"));
        }
    }
}

#[test]
fn base64_old_style_roundtrip_verification_passes() {
    let (temp, tree_dir) = copy_fixture_tree();
    let output_manifest = temp.path().join("hashes-old-base64.txt");

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("create")
        .arg("-o")
        .arg(&output_manifest)
        .arg("-s")
        .arg("old")
        .arg("--base64")
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success();

    let raw = fs::read_to_string(&output_manifest).expect("old-style output should be readable");
    assert!(
        raw.contains('='),
        "old-style base64 output should contain padded digests"
    );

    Command::cargo_bin("hashtree")
        .expect("binary should build")
        .arg("verify")
        .arg("-i")
        .arg(&output_manifest)
        .arg("-s")
        .arg("old")
        .arg("--sha-256")
        .arg(&tree_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("*** PASSED VERIFICATION ***"));
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("basic_tree")
}

fn copy_fixture_tree() -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("temp dir creation should succeed");
    let tree_dir = temp.path().join("tree");
    copy_dir_recursive(&fixture_root(), &tree_dir).expect("fixture copy should succeed");
    (temp, tree_dir)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }

    Ok(())
}
