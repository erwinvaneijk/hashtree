// SPDX-License-Identifier: EUPL-1.2
use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

#[test]
fn create_case_format_output() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("output.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    cmd.assert().success();
    assert!(output_file.exists(), "output file should be created");
}

#[test]
fn case_output_is_valid_json_ld() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("output.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    cmd.assert().success();

    let json_str = fs::read_to_string(&output_file).expect("output file should be readable");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("output should be valid JSON");

    assert!(json.get("@context").is_some(), "@context should be present");
    assert!(json.get("@graph").is_some(), "@graph should be present");
}

#[test]
fn case_output_contains_observable_file() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("output.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    cmd.assert().success();

    let json_str = fs::read_to_string(&output_file).expect("output file should be readable");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("output should be valid JSON");

    let graph = json
        .get("@graph")
        .and_then(serde_json::Value::as_array)
        .expect("@graph should be an array");

    let has_file = graph
        .iter()
        .any(|obj| obj.get("@type").and_then(serde_json::Value::as_str) == Some("observable:File"));

    assert!(has_file, "graph should contain observable:File");
}

#[test]
fn case_output_contains_hashes() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("output.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    cmd.assert().success();

    let json_str = fs::read_to_string(&output_file).expect("output file should be readable");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("output should be valid JSON");

    let graph = json
        .get("@graph")
        .and_then(serde_json::Value::as_array)
        .expect("@graph should be an array");

    let file_obj = graph
        .iter()
        .find(|obj| obj.get("@type").and_then(serde_json::Value::as_str) == Some("observable:File"))
        .expect("should have observable:File");

    let content_data_facet = file_obj
        .get("core:hasFacet")
        .and_then(serde_json::Value::as_array)
        .and_then(|facets| {
            facets.iter().find(|facet| {
                facet.get("@type").and_then(serde_json::Value::as_str)
                    == Some("observable:ContentDataFacet")
            })
        })
        .expect("file should have an observable:ContentDataFacet");
    let hashes = content_data_facet
        .get("observable:hash")
        .and_then(serde_json::Value::as_array)
        .expect("observable:hash should be an array");

    assert!(!hashes.is_empty(), "file should have at least one hash");

    let hash_obj = &hashes[0];
    assert!(
        hash_obj.get("types:hashMethod").is_some(),
        "hash should have types:hashMethod"
    );
    assert!(
        hash_obj.get("types:hashValue").is_some(),
        "hash should have types:hashValue"
    );
}

#[test]
fn case_output_contains_tool() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("output.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    cmd.assert().success();

    let json_str = fs::read_to_string(&output_file).expect("output file should be readable");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("output should be valid JSON");

    let graph = json
        .get("@graph")
        .and_then(serde_json::Value::as_array)
        .expect("@graph should be an array");

    let has_tool = graph.iter().any(|obj| {
        obj.get("@type").and_then(serde_json::Value::as_str) == Some("tool:AnalyticTool")
    });

    assert!(has_tool, "graph should contain tool:AnalyticTool");
}

#[test]
fn case_output_contains_investigative_action() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("output.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    cmd.assert().success();

    let json_str = fs::read_to_string(&output_file).expect("output file should be readable");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("output should be valid JSON");

    let graph = json
        .get("@graph")
        .and_then(serde_json::Value::as_array)
        .expect("@graph should be an array");

    let has_action = graph.iter().any(|obj| {
        obj.get("@type").and_then(serde_json::Value::as_str)
            == Some("case-investigation:InvestigativeAction")
    });

    assert!(
        has_action,
        "graph should contain case-investigation:InvestigativeAction"
    );
}

#[test]
fn verify_case_format_passes_for_unchanged_files() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("hashes.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    // Create CASE manifest
    let mut create_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    create_cmd
        .arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    create_cmd.assert().success();

    // Verify against CASE manifest
    let mut verify_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    verify_cmd
        .arg("verify")
        .arg("--input")
        .arg(&output_file)
        .arg("--style")
        .arg("case")
        .arg("--sha-256")
        .arg(&test_file);

    verify_cmd.assert().success();
}

#[test]
fn verify_case_format_detects_modified_files() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("hashes.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    // Create CASE manifest
    let mut create_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    create_cmd
        .arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    create_cmd.assert().success();

    // Modify file
    fs::write(&test_file, "modified content").expect("test file should be modified");

    // Verify against CASE manifest - should report differences
    let mut verify_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    verify_cmd
        .arg("verify")
        .arg("--input")
        .arg(&output_file)
        .arg("--style")
        .arg("case")
        .arg("--sha-256")
        .arg(&test_file);

    verify_cmd
        .assert()
        .code(1)
        .stdout(contains("There are differences"));
}

#[test]
fn verify_case_format_with_multiple_files() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("hashes.jsonld");
    let file1 = temp.path().join("file1.txt");
    let file2 = temp.path().join("file2.txt");

    fs::write(&file1, "content 1").expect("file1 should be written");
    fs::write(&file2, "content 2").expect("file2 should be written");

    // Create CASE manifest with both files
    let mut create_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    create_cmd
        .arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&file1)
        .arg(&file2);

    create_cmd.assert().success();

    // Verify both files
    let mut verify_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    verify_cmd
        .arg("verify")
        .arg("--input")
        .arg(&output_file)
        .arg("--style")
        .arg("case")
        .arg("--sha-256")
        .arg(&file1)
        .arg(&file2);

    verify_cmd.assert().success();
}

#[test]
fn verify_case_format_with_compact_json() {
    let temp = TempDir::new().expect("temp directory should be created");
    let output_file = temp.path().join("hashes.jsonld");
    let test_file = temp.path().join("test.txt");

    fs::write(&test_file, "test content").expect("test file should be written");

    // Create compact CASE manifest
    let mut create_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    create_cmd
        .arg("create")
        .arg("--sha-256")
        .arg("--style")
        .arg("case")
        .arg("--compact-json")
        .arg("-o")
        .arg(&output_file)
        .arg("-f")
        .arg(&test_file);

    create_cmd.assert().success();

    // Verify against compact CASE manifest
    let mut verify_cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    verify_cmd
        .arg("verify")
        .arg("--input")
        .arg(&output_file)
        .arg("--style")
        .arg("case")
        .arg("--sha-256")
        .arg(&test_file);

    verify_cmd.assert().success();
}

#[test]
fn verify_case_manifest_written_by_jhashtools() {
    // tests/fixtures/jhashtools_case.jsonld was written by jHashtools with
    // `-a -s case` for tests/fixtures/basic_tree, run from the repository root.
    let mut cmd = Command::cargo_bin("hashtree").expect("binary should exist");
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("verify")
        .args([
            "--md5",
            "--sha-1",
            "--sha-256",
            "--sha-384",
            "--sha-512",
            "--ripemd-160",
        ])
        .arg("--style")
        .arg("case")
        .arg("-i")
        .arg("tests/fixtures/jhashtools_case.jsonld")
        .arg("tests/fixtures/basic_tree");

    cmd.assert()
        .success()
        .stdout(contains("*** PASSED VERIFICATION ***"));
}
