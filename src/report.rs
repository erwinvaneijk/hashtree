// SPDX-License-Identifier: EUPL-1.2
use std::fmt::Write as _;
use std::io::{self, Write};

use crate::encoding::DigestEncoding;
use crate::model::{DigestResult, DirHasherResult, IndexedResult};

pub fn print_missing_entries(
    entries: &IndexedResult,
    out: &mut dyn Write,
    encoding: DigestEncoding,
) -> io::Result<()> {
    for entry in entries.values() {
        print_digest_entry(&entry.path, &entry.digests, out, encoding)?;
    }
    Ok(())
}

pub fn print_digest_entry(
    path: &str,
    digests: &DigestResult,
    out: &mut dyn Write,
    encoding: DigestEncoding,
) -> io::Result<()> {
    writeln!(out, "{path}")?;
    for (algorithm, hex_digest) in &digests.digests {
        writeln!(out, "\t{algorithm}\t{}", encoding.render(hex_digest))?;
    }
    Ok(())
}

pub fn pretty_print_result(
    result: &DirHasherResult,
    out: &mut dyn Write,
    encoding: DigestEncoding,
) -> io::Result<()> {
    writeln!(out, "{} elements", result.content.len())?;
    let mut entries: Vec<_> = result.content.iter().collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));

    for (path, digest_result) in entries {
        writeln!(out, "\t{path}")?;
        for (algorithm, hex_digest) in &digest_result.digests {
            let mut line = String::new();
            let _ = write!(
                &mut line,
                "{}\t{}",
                algorithm,
                encoding.render_grouped(hex_digest, 4)
            );
            writeln!(out, "\t\t{line}")?;
        }
    }
    out.flush()
}
