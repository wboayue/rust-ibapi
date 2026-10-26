//! Read a capture back. Lives beside `raw_capture` because the file-naming scheme is that
//! module's private business, and three test files need to inspect one.

use std::fs;
use std::path::{Path, PathBuf};

/// Capture files in `dir` with the given extension, in segment order.
pub(crate) fn segments(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .expect("capture directory must exist")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .collect();
    paths.sort();
    paths
}

/// Every captured byte, segments concatenated in order. For a single-segment
/// capture this is the inbound stream verbatim.
pub(crate) fn frames(dir: &Path) -> Vec<u8> {
    segments(dir, "bin")
        .iter()
        .flat_map(|path| fs::read(path).expect("readable capture"))
        .collect()
}

/// Index lines across all segments, in order, newlines stripped.
pub(crate) fn index(dir: &Path) -> Vec<String> {
    segments(dir, "idx")
        .iter()
        .flat_map(|path| {
            fs::read_to_string(path)
                .expect("readable index")
                .lines()
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect()
}
