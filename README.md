# hashtree (Rust)

A Rust reimplementation of the `jHashtools` workflow from `../jHashtools`.

Current stable release: **1.0.0**

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
cargo run -- create -o hashes.json --sha-256 /path/to/dir

# Verify against manifest
cargo run -- verify -i hashes.json --sha-256 /path/to/dir

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
-s, --style <STYLE>    (json|old|xml)
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

## License

This project is licensed under the **European Union Public Licence v1.2 (EUPL-1.2)**.
See [LICENSE](./LICENSE).
