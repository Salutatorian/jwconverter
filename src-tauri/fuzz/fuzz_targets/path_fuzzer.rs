#![no_main]
#![allow(dead_code)]

use std::path::Path;

use libfuzzer_sys::fuzz_target;

#[path = "../../src/errors.rs"]
mod errors;
#[path = "../../src/fs_safety/temp.rs"]
mod temp;
#[path = "../../src/fs_safety/finalize.rs"]
mod finalize;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut parts = text.splitn(3, '\n');
    let stem = parts.next().unwrap_or("");
    let extension = parts.next().unwrap_or("");
    let job_id = parts.next().unwrap_or("");

    // Never exists on disk, so filesystem lookups bail out immediately.
    let dir = Path::new("__jw_fuzz_no_such_dir__");

    let primary = finalize::primary_final_path(dir, stem, extension);
    let _ = temp::is_our_temp_file(&primary);
    let _ = temp::is_our_temp_file(Path::new(&*text));
    let _ = temp::temp_output_path(dir, stem, extension, job_id);

    let unique = finalize::unique_final_path(dir, stem, extension);
    let _ = temp::is_our_temp_file(&unique);
});
