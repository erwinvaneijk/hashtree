# hashtree (Rust)

A Rust reimplementation of the `jHashtools` workflow from `../jHashtools`.

It supports:
- Recursive file hashing for files/directories
- Multiple digest algorithms (`sha-1`, `sha-256`, `sha-384`, `sha-512`, `md5`, `md2`)
- Writing digest manifests (`json` and `old` style)
- Verifying current files against a stored manifest
- Case-insensitive path matching during verification (`--ignorecase`)

## Build

```bash
cargo build --release
```

## Usage

```bash
# Create digest manifest
cargo run -- -o hashes.json --sha-256 /path/to/dir

# Verify against manifest
cargo run -- -i hashes.json --sha-256 /path/to/dir

# Use old legacy format
cargo run -- -o hashes.txt -s old --all /path/to/dir
```

## CLI options

```text
-1, --sha-1
-2, --sha-256
    --sha-384
    --sha-512
    --md5
    --md2
-a, --all
-n, --ignorecase
-v, --verbose
-f, --force
    --single
-o, --output <OUTPUT>
-i, --input <INPUT>
-s, --style <STYLE>    (json|old|xml)
```

`xml` is recognized for compatibility but intentionally not implemented (matching `jHashtools`).
