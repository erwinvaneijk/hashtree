use std::fmt::Write as _;
use std::io::{self, Write};

use crate::model::{DigestResult, DirHasherResult, IndexedResult};
use crate::util::split_hex;

pub fn print_missing_entries(entries: &IndexedResult, out: &mut dyn Write) -> io::Result<()> {
    for entry in entries.values() {
        print_digest_entry(&entry.path, &entry.digests, out)?;
    }
    Ok(())
}

pub fn print_digest_entry(
    path: &str,
    digests: &DigestResult,
    out: &mut dyn Write,
) -> io::Result<()> {
    writeln!(out, "{path}")?;
    for (algorithm, hex_digest) in &digests.digests {
        writeln!(out, "\t{algorithm}\t{hex_digest}")?;
    }
    Ok(())
}

pub fn pretty_print_result(result: &DirHasherResult, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{} elements", result.content.len())?;
    for (path, digest_result) in &result.content {
        writeln!(out, "\t{path}")?;
        for (algorithm, hex_digest) in &digest_result.digests {
            let mut line = String::new();
            let _ = write!(&mut line, "{}\t{}", algorithm, split_hex(hex_digest, 4));
            writeln!(out, "\t\t{line}")?;
        }
    }
    out.flush()
}
