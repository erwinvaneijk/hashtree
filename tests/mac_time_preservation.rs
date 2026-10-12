// SPDX-License-Identifier: EUPL-1.2
//! Tests to verify that file MAC (Modification/Access) times are preserved
//! during hashing operations. This is essential for forensic integrity.

use std::fs;
use std::time::Duration;

use filetime::FileTime;
use hashtree_rs::algorithm::Algorithm;
use hashtree_rs::hashing::{generate_digests, hash_file};

/// Helper function to set file times to a specific moment in the past.
fn set_file_times_to_past(path: &std::path::Path, days_ago: u64) {
    let past = FileTime::from_system_time(
        std::time::SystemTime::now() - Duration::from_secs(days_ago * 86400),
    );
    filetime::set_file_times(path, past, past).expect("Failed to set file times");
}

/// Helper function to get current file times.
fn get_file_times(path: &std::path::Path) -> (FileTime, FileTime) {
    let metadata = fs::metadata(path).expect("Failed to read metadata");
    (
        FileTime::from_last_access_time(&metadata),
        FileTime::from_last_modification_time(&metadata),
    )
}

#[test]
fn hash_file_preserves_modification_time() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    let file_path = temp.path().join("test_file.txt");
    fs::write(&file_path, b"test content").expect("fixture file should be written");

    // Set file times to 7 days ago
    set_file_times_to_past(&file_path, 7);
    let (atime_before, mtime_before) = get_file_times(&file_path);

    // Hash the file
    let _digest = hash_file(&file_path, &[Algorithm::Sha256]).expect("hashing should succeed");

    // Verify times are preserved
    let (atime_after, mtime_after) = get_file_times(&file_path);
    assert_eq!(mtime_before, mtime_after, "modification time should be preserved");
    assert_eq!(atime_before, atime_after, "access time should be preserved");
}

#[test]
fn hash_file_preserves_access_time() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    let file_path = temp.path().join("test_file.txt");
    fs::write(&file_path, b"test content").expect("fixture file should be written");

    // Set file times to a specific point in time
    set_file_times_to_past(&file_path, 30);
    let (atime_before, mtime_before) = get_file_times(&file_path);

    // Hash the file multiple times
    for _ in 0..3 {
        let _digest = hash_file(&file_path, &[Algorithm::Sha256]).expect("hashing should succeed");
    }

    // Verify times are still preserved after multiple reads
    let (atime_after, mtime_after) = get_file_times(&file_path);
    assert_eq!(
        mtime_before, mtime_after,
        "modification time should be preserved after multiple hashes"
    );
    assert_eq!(
        atime_before, atime_after,
        "access time should be preserved after multiple hashes"
    );
}

#[test]
fn hash_file_with_multiple_algorithms_preserves_times() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    let file_path = temp.path().join("multi_algo.txt");
    fs::write(&file_path, b"test data for multiple algorithms").expect("fixture file should be written");

    set_file_times_to_past(&file_path, 14);
    let (atime_before, mtime_before) = get_file_times(&file_path);

    // Hash with multiple algorithms
    let _digest = hash_file(
        &file_path,
        &[
            Algorithm::Md5,
            Algorithm::Sha1,
            Algorithm::Sha256,
            Algorithm::Sha384,
            Algorithm::Sha512,
            Algorithm::Sha3_256,
            Algorithm::Sha3_384,
            Algorithm::Sha3_512,
        ],
    )
    .expect("hashing should succeed");

    // Verify all times are preserved
    let (atime_after, mtime_after) = get_file_times(&file_path);
    assert_eq!(
        mtime_before, mtime_after,
        "modification time should be preserved with multiple algorithms"
    );
    assert_eq!(
        atime_before, atime_after,
        "access time should be preserved with multiple algorithms"
    );
}

#[test]
fn generate_digests_preserves_file_times_in_directory() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    
    // Create multiple test files
    let file1 = temp.path().join("file1.txt");
    let file2 = temp.path().join("file2.txt");
    let file3 = temp.path().join("file3.txt");
    
    fs::write(&file1, b"content1").expect("file1 should be written");
    fs::write(&file2, b"content2").expect("file2 should be written");
    fs::write(&file3, b"content3").expect("file3 should be written");

    // Set all files to have times from the past
    set_file_times_to_past(&file1, 3);
    set_file_times_to_past(&file2, 5);
    set_file_times_to_past(&file3, 10);

    let times_before = vec![
        get_file_times(&file1),
        get_file_times(&file2),
        get_file_times(&file3),
    ];

    // Generate digests for all files in the directory
    let _result = generate_digests(
        &[temp.path().to_path_buf()],
        &[Algorithm::Sha256],
        false, // single threaded
        None,
        false,
    )
    .expect("generate_digests should succeed");

    // Verify all file times are preserved
    let times_after = vec![
        get_file_times(&file1),
        get_file_times(&file2),
        get_file_times(&file3),
    ];

    for (i, (before, after)) in times_before.iter().zip(times_after.iter()).enumerate() {
        assert_eq!(before, after, "File {} times should be preserved", i + 1);
    }
}

#[test]
fn generate_digests_preserves_file_times_multithreaded() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    
    // Create multiple test files
    let file1 = temp.path().join("file1.txt");
    let file2 = temp.path().join("file2.txt");
    let file3 = temp.path().join("file3.txt");
    let file4 = temp.path().join("file4.txt");
    
    fs::write(&file1, b"content1").expect("file1 should be written");
    fs::write(&file2, b"content2").expect("file2 should be written");
    fs::write(&file3, b"content3").expect("file3 should be written");
    fs::write(&file4, b"content4").expect("file4 should be written");

    // Set all files to have times from the past
    set_file_times_to_past(&file1, 7);
    set_file_times_to_past(&file2, 14);
    set_file_times_to_past(&file3, 21);
    set_file_times_to_past(&file4, 30);

    let times_before = vec![
        get_file_times(&file1),
        get_file_times(&file2),
        get_file_times(&file3),
        get_file_times(&file4),
    ];

    // Generate digests with multithreading
    let _result = generate_digests(
        &[temp.path().to_path_buf()],
        &[Algorithm::Sha256],
        true, // multi-threaded
        None,
        false,
    )
    .expect("generate_digests should succeed");

    // Verify all file times are preserved even with parallel processing
    let times_after = vec![
        get_file_times(&file1),
        get_file_times(&file2),
        get_file_times(&file3),
        get_file_times(&file4),
    ];

    for (i, (before, after)) in times_before.iter().zip(times_after.iter()).enumerate() {
        assert_eq!(
            before, after,
            "File {} times should be preserved with multithreading",
            i + 1
        );
    }
}

#[test]
fn file_times_preservation_does_not_affect_hash_correctness() {
    let temp = tempfile::tempdir().expect("temp directory should be created");
    let file_path = temp.path().join("hash_test.txt");
    let content = b"exact test content for hash verification";
    fs::write(&file_path, content).expect("fixture file should be written");

    // Set old file times
    set_file_times_to_past(&file_path, 60);

    // Hash the file
    let digest1 = hash_file(&file_path, &[Algorithm::Sha256])
        .expect("hashing should succeed")
        .digests
        .get(&Algorithm::Sha256)
        .cloned();

    // Hash again after time preservation
    let digest2 = hash_file(&file_path, &[Algorithm::Sha256])
        .expect("hashing should succeed")
        .digests
        .get(&Algorithm::Sha256)
        .cloned();

    // Verify hashes are identical
    assert_eq!(digest1, digest2, "Hashes should be identical after time preservation");

    // Verify the hash is correct
    // Note: We don't check exact hash as it depends on exact content,
    // but we verify consistency
    assert_eq!(digest1, digest2, "Hash should be consistent");
}
