use crate::archive::{SUMMARY, summary_value};
use crate::capture::digest_of_words;
use crate::npy::Table;
use crate::spec::hex;
use ng_validate::anchor_moment;
use std::fs;
use std::path::Path;

pub const SMOKE_SPEC: &str = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 20000\nevent at-ms 10000 trunk 1000 legs \
                              standing\nvariant none\n";
pub const SMOKE_PACKET: &str = "2dde20ac6a13aa9e6de955f917fb1bff57b9d62ecb8ae3f33e3cc377e893045b";
pub const SMOKE_SEAL: &str = "fe5bf7dec9c890f5f948765fe013aa5f62a704fc7fb3aa41257c77abb3545c4a";
pub const SMOKE_HEAD_NS: u64 = 1_000_226_006;
const SMOKE_BEATS: u64 = 15;
const SECOND_NS: u64 = 1_000_000_000;

pub fn check_smoke(directory: &Path) -> Result<Vec<String>, String> {
    let read = |name: &str| directory.join(name);
    let summary = fs::read_to_string(read(SUMMARY)).map_err(|error| format!("summary.txt: {error}"))?;
    let value = |key: &str| summary_value(&summary, key).ok_or(format!("summary.txt has no {key}"));
    let mut checks = Vec::new();
    let packet = value("packet")?;
    if packet != SMOKE_PACKET {
        return Err(format!("packet {packet}, pinned {SMOKE_PACKET}"));
    }
    checks.push(format!("packet {packet} as pinned"));
    let steps = Table::<u64>::read(&read("steps.npy")).map_err(|error| format!("steps.npy: {error}"))?;
    let anchor = anchor_moment().nanoseconds();
    let first = (steps.iter())
        .find(|row| row[1] >= anchor + SECOND_NS)
        .ok_or("no accepted head at or after 1000 ms")?;
    let words = <[u64; 4]>::try_from(&first[3..7]).map_err(|_| "a step row without four seal words")?;
    let (head, seal) = (first[1] - anchor, hex(&digest_of_words(words)));
    if head != SMOKE_HEAD_NS || seal != SMOKE_SEAL {
        return Err(format!(
            "seal {seal} at {head} ns, pinned {SMOKE_SEAL} at {SMOKE_HEAD_NS} ns"
        ));
    }
    checks.push(format!(
        "seal {seal} at {head} ns after the anchor, revision {}, as pinned",
        first[0]
    ));
    let beats: u64 = value("beats")?.parse().map_err(|_| "a beat count")?;
    if beats < SMOKE_BEATS {
        return Err(format!("{beats} beats, fewer than {SMOKE_BEATS}"));
    }
    checks.push(format!("{beats} beats"));
    let fault = value("fault")?;
    if fault != "none" {
        return Err(format!("fault {fault}"));
    }
    checks.push("no fault".to_string());
    let applied =
        Table::<u64>::read(&read("applied.npy")).map_err(|error| format!("applied.npy: {error}"))?;
    let stand = applied
        .iter()
        .find(|row| row[0] == 0)
        .ok_or("the stand is not in applied.npy")?;
    checks.push(format!("the stand applied at revision {}", stand[1]));
    Ok(checks)
}
