# hashtree (Rust)

A Rust reimplementation of the [`jHashtools`](https://github.com/erwinvaneijk/jHashtools/)
workflow.

Current stable release: **3.0.1**

It supports:

- Recursive file hashing for files/directories
- Multiple digest algorithms (9 total: `md5`, `sha-1`, `ripemd-160`, `sha-256`,
  `sha-384`, `sha-512`, `sha3-256`, `sha3-384`, `sha3-512`)
- Writing digest manifests in multiple formats:
  - `json` - Machine-readable JSON format
  - `old` - Legacy text format
  - `case` - CASE JSON-LD format (forensically-compliant, standards-based)
- Digest output in hexadecimal or, for digests of at least 256 bits, base64
  (`--base64`)
- Verifying current files against a stored manifest
- Detecting file tampering across all algorithms
- Case-insensitive path matching during verification (`--ignorecase`)

## Build

```bash
cargo build --release
```

## Usage

```bash
# Create digest manifest in JSON format
cargo run -- create -o hashes.json --sha-256 /path/to/dir

# Create manifest in CASE JSON-LD format (forensic-compliant)
cargo run -- create -o hashes.jsonld --sha-256 --style case /path/to/dir

# Verify against manifest (any format)
cargo run -- verify -i hashes.json --sha-256 /path/to/dir

# Verify CASE manifest
cargo run -- verify -i hashes.jsonld --style case --sha-256 /path/to/dir

# Use old legacy format
cargo run -- create -o hashes.txt -s old --all /path/to/dir

# Write digests of 256 bits and longer in base64
cargo run -- create -o hashes.json --base64 --sha-256 /path/to/dir
```

## CLI structure

```text
hashtree create [OPTIONS] <dir>...
hashtree verify [OPTIONS] <dir>...
```

Common options for both subcommands:

```text
-1, --sha-1
-2, --sha-256
    --sha-384
    --sha-512
    --md5
    --ripemd-160
-a, --all
-b, --base64
-v, --verbose
    --single
    --jobs <N>
-s, --style <STYLE>    (json|old|case|xml)
```

## Base64 digests

By default every digest is written as lowercase hexadecimal. With `--base64`,
digests of at least 256 bits (`sha-256`, `sha-384`, `sha-512`, `sha3-256`,
`sha3-384` and `sha3-512`) are written in standard base64 instead. Shorter
digests (`md5`, `sha-1`, `ripemd-160`) stay hexadecimal, because base64 buys little
there and would only make the manifest harder to compare with other tools.

The encoding is a presentation choice only; it applies to all styles (`json`,
`old` and `case`) as well as to the reports printed on the console:

```bash
# Create a manifest with base64 digests
hashtree create -o hashes.json --base64 --sha-256 /path/to/dir

# Verification auto-detects the encoding, so --base64 is not required here
hashtree verify -i hashes.json --sha-256 /path/to/dir

# ... but it does control how digests are printed in the verification report
hashtree verify -i hashes.json --base64 --sha-256 /path/to/dir
```

Manifests may even mix both encodings: every digest value is decoded
individually, and both padded and unpadded base64 (standard or URL-safe
alphabet) are accepted.

Create-specific options:

```text
-o, --output <OUTPUT>
-f, --force
    --compact-json
```

Verify-specific options:

```text
-i, --input <INPUT>
-n, --ignorecase
```

## Exit codes

Every outcome has its own exit code, so scripts can tell a failed verification
apart from a problem running the tool:

| Code | Meaning |
|------|---------|
| 0 | Success: the manifest was created, or verification passed |
| 1 | Verification failed: files were modified, added or removed |
| 2 | Invalid command line, such as an unknown option or a missing argument |
| 3 | `--jobs` is zero |
| 4 | Unsupported `--style` value |
| 5 | The `xml` style is not supported yet |
| 6 | An input path does not exist |
| 7 | The output file already exists and `--force` was not given |
| 8 | The old-style manifest has an invalid header |
| 9 | The CASE manifest is not valid JSON-LD (not an object, or no `@graph`) |
| 10 | The JSON manifest could not be parsed or written |
| 11 | The written manifest could not be hashed for the summary |
| 12 | The worker thread pool could not be created |
| 13 | Any other I/O error, such as an unreadable manifest or file |

```bash
hashtree verify -i hashes.json /path/to/dir
case $? in
    0) echo "all files match" ;;
    1) echo "files have changed" ;;
    *) echo "verification could not be run" ;;
esac
```

## CASE Ontology Support

Hashtree now supports exporting and verifying file hashes in **CASE JSON-LD
format**, enabling forensically-compliant output that integrates with
cyber-investigation tools and standards.

### Quick Start

```bash
# Export as CASE JSON-LD
hashtree create --sha-256 --style case -o evidence.jsonld /path/to/files

# With all algorithms
hashtree create --all --style case -o digital_evidence.jsonld /mnt/evidence/

# Verify files against CASE manifest
hashtree verify --input digital_evidence.jsonld --style case --all /mnt/evidence/
```

### Forensic Workflow Example

```bash
# Step 1: Create trusted manifest with all algorithms
hashtree create --all --style case -o evidence_manifest.jsonld /mnt/evidence/

# Step 2: Later, verify files haven't been tampered with
hashtree verify --input evidence_manifest.jsonld --style case --all /mnt/evidence/

# If tampering is detected:
# Output shows which files changed and which algorithms detected the difference
```

See [docs/CASE.md](./docs/CASE.md) for detailed information, validation examples,
verification workflows, and the complete CASE JSON-LD structure.

## License

This project is licensed under the **European Union Public Licence v1.2 (EUPL-1.2)**.
See [LICENSE](./LICENSE).
