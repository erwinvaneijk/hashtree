# hashtree (Rust)

A Rust reimplementation of the `jHashtools` workflow from `../jHashtools`.

Current stable release: **1.0.0**

It supports:
- Recursive file hashing for files/directories
- Multiple digest algorithms (9 total: `md2`, `md5`, `sha-1`, `sha-256`, `sha-384`, `sha-512`, `sha3-256`, `sha3-384`, `sha3-512`)
- Writing digest manifests in multiple formats:
  - `json` - Machine-readable JSON format
  - `old` - Legacy text format
  - `case` - CASE JSON-LD format (forensically-compliant, standards-based, with verification support)
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

# Verify CASE manifest (detects tampering)
cargo run -- verify -i hashes.jsonld --style case --sha-256 /path/to/dir

# Use old legacy format
cargo run -- create -o hashes.txt -s old --all /path/to/dir
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
    --md2
-a, --all
-v, --verbose
    --single
    --jobs <N>
-s, --style <STYLE>    (json|old|case|xml)
```

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

`xml` is recognized for compatibility but intentionally not implemented (matching `jHashtools`).

## CASE Ontology Support

Hashtree now supports exporting and verifying file hashes in **CASE JSON-LD format**, enabling forensically-compliant output that integrates with cyber-investigation tools and standards.

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

See [docs/CASE.md](./docs/CASE.md) for detailed information, validation examples, verification workflows, and the complete CASE JSON-LD structure.

## License

This project is licensed under the **European Union Public Licence v1.2 (EUPL-1.2)**.
See [LICENSE](./LICENSE).
