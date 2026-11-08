use crate::capture::Arrival;
use crate::spec::{SpecId, digest_of_hex, hex};
use ng_contract::{Digest, EngineFaultKind, NumericalRefusal, SimTime};
use ng_record::RecordDigests;
use std::fs::{self, File, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const LOCK: &str = "lock";
pub const SUMMARY: &str = "summary.txt";

pub fn summary_value<'t>(summary: &'t str, key: &str) -> Option<&'t str> {
    summary
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(' '))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityDigests {
    pub model: Digest,
    pub packet: Digest,
    pub resolution: Digest,
    pub run_identity: Digest,
}
impl IdentityDigests {
    pub fn read(summary: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(summary)?;
        let digest = |key: &str| {
            summary_value(&text, key).and_then(digest_of_hex).ok_or_else(|| {
                let place = summary.display();
                io::Error::new(io::ErrorKind::InvalidData, format!("{place} has no {key} digest"))
            })
        };
        Ok(Self {
            model: digest("model")?,
            packet: digest("packet")?,
            resolution: digest("resolution")?,
            run_identity: digest("run_identity")?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RunState {
    Missing,
    /// A live run holds the lock inside its directory for its whole life, so a reused pid cannot pass for one.
    Partial {
        paths: Vec<PathBuf>,
        held: bool,
    },
    Present(IdentityDigests),
    Fault(PathBuf),
}
impl RunState {
    pub fn of(out: &Path, id: SpecId) -> io::Result<Self> {
        let present = out.join(id.to_string());
        if present.is_dir() {
            return Ok(Self::Present(IdentityDigests::read(&present.join(SUMMARY))?));
        }
        let fault = out.join(format!("{id}.fault"));
        if fault.is_dir() {
            return Ok(Self::Fault(fault));
        }
        let entries = match fs::read_dir(out) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::Missing),
            Err(error) => return Err(error),
        };
        let prefix = format!("{id}.partial-");
        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(&prefix))
            {
                paths.push(entry.path());
            }
        }
        if paths.is_empty() {
            return Ok(Self::Missing);
        }
        paths.sort();
        let mut held = false;
        for path in &paths {
            held |= is_held(path)?;
        }
        Ok(Self::Partial { paths, held })
    }
    pub fn word(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Partial { .. } => "partial",
            Self::Present(_) => "present",
            Self::Fault(_) => "fault",
        }
    }
}

fn is_held(directory: &Path) -> io::Result<bool> {
    match File::open(directory.join(LOCK)) {
        Ok(lock) => match lock.try_lock() {
            Ok(()) => Ok(false),
            Err(TryLockError::WouldBlock) => Ok(true),
            Err(TryLockError::Error(error)) => Err(error),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn write_synced(path: &Path, text: &str) -> io::Result<()> {
    let mut file = File::create_new(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}

fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[derive(Debug)]
pub enum Settled {
    Renamed(PathBuf),
    FinishedByAnother(PathBuf),
}

/// Under a name `RunState::of` never reads until its lock is held, so no other run takes a live run's
/// directory for an abandoned partial and removes it.
struct Staged {
    out: PathBuf,
    id: SpecId,
    directory: PathBuf,
    lock: File,
}
impl Staged {
    fn create(out: &Path, id: SpecId) -> io::Result<Self> {
        let directory = out.join(format!(".{id}.staging-{}", std::process::id()));
        fs::create_dir(&directory)?;
        let lock = File::create_new(directory.join(LOCK))?;
        lock.lock()?;
        Ok(Self {
            out: out.to_path_buf(),
            id,
            directory,
            lock,
        })
    }
    /// The lock belongs to the open file, so it holds across the rename.
    fn publish(self) -> io::Result<PartialRun> {
        let Self {
            out,
            id,
            directory,
            lock,
        } = self;
        let partial = out.join(format!("{id}.partial-{}", std::process::id()));
        fs::rename(&directory, &partial)?;
        Ok(PartialRun {
            out,
            id,
            directory: partial,
            _lock: lock,
        })
    }
}

pub struct PartialRun {
    out: PathBuf,
    id: SpecId,
    directory: PathBuf,
    _lock: File,
}
impl PartialRun {
    pub fn create(out: &Path, id: SpecId) -> io::Result<Self> {
        Staged::create(out, id)?.publish()
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn file(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }
    pub fn complete(self) -> io::Result<Settled> {
        let target = self.out.join(self.id.to_string());
        self.settle(target)
    }
    pub fn fault(self) -> io::Result<Settled> {
        let target = self.out.join(format!("{}.fault", self.id));
        self.settle(target)
    }
    fn settle(self, target: PathBuf) -> io::Result<Settled> {
        sync_directory(&self.directory)?;
        match fs::rename(&self.directory, &target) {
            Ok(()) => {
                fs::remove_file(target.join(LOCK))?;
                sync_directory(&self.out)?;
                Ok(Settled::Renamed(target))
            }
            Err(_) if target.is_dir() => {
                fs::remove_dir_all(&self.directory)?;
                Ok(Settled::FinishedByAnother(target))
            }
            Err(error) => Err(error),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Course {
    Original,
    Branch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaultKind {
    Engine {
        kind: EngineFaultKind,
        last_refusal: Option<NumericalRefusal>,
    },
    MemoryCap {
        resident_bytes: u64,
    },
    Protocol(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultSummary {
    pub course: Course,
    pub kind: FaultKind,
    pub at: SimTime,
    pub attempted_ns: u64,
    pub step_refusals: u64,
}
impl FaultSummary {
    fn lines(&self) -> Vec<String> {
        let (kind, detail) = match &self.kind {
            FaultKind::Engine { kind, last_refusal } => (
                format!("{kind:?}"),
                last_refusal.map_or("none".to_string(), |refusal| format!("{refusal:?}")),
            ),
            FaultKind::MemoryCap { resident_bytes } => (
                "MemoryCap".to_string(),
                format!("resident_bytes {resident_bytes}"),
            ),
            FaultKind::Protocol(reason) => ("Protocol".to_string(), reason.clone()),
        };
        let course = match self.course {
            Course::Original => "original",
            Course::Branch => "branch",
        };
        vec![
            format!("fault {kind}"),
            format!("fault_course {course}"),
            format!("fault_at_ns {}", self.at.nanoseconds()),
            format!("fault_attempted_ns {}", self.attempted_ns),
            format!("fault_step_refusals {}", self.step_refusals),
            format!("fault_detail {detail}"),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchSummary {
    pub from_revision: u64,
    pub enqueued: Vec<Arrival>,
    pub revisions: u64,
    pub refusals: u64,
    pub last_seal: Digest,
    pub end: RecordDigests,
}

fn entries(enqueued: &[Arrival]) -> String {
    if enqueued.is_empty() {
        return "none".to_string();
    }
    let entries: Vec<String> = (enqueued.iter())
        .map(|arrival| format!("{}@{}", arrival.event.0, arrival.revision))
        .collect();
    entries.join(" ")
}

fn digest_lines(prefix: &str, digests: &RecordDigests) -> Vec<String> {
    let RecordDigests {
        revision,
        time,
        state,
        continuation,
        segment,
        modes,
        streams,
        applied,
        waiting,
        ledger,
    } = digests;
    vec![
        format!("{prefix}_revision {revision}"),
        format!("{prefix}_time_ns {}", time.nanoseconds()),
        format!("{prefix}_state {}", hex(state)),
        format!("{prefix}_continuation {}", hex(continuation)),
        format!("{prefix}_segment {}", hex(segment)),
        format!("{prefix}_modes {}", hex(modes)),
        format!("{prefix}_streams {}", hex(streams)),
        format!("{prefix}_applied {}", hex(applied)),
        format!("{prefix}_waiting {}", hex(waiting)),
        format!("{prefix}_ledger {}", hex(ledger)),
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunSummary {
    pub id: SpecId,
    pub capture_version: u32,
    pub source_commit: String,
    pub identity: IdentityDigests,
    pub first_seal: Digest,
    pub last_seal: Digest,
    pub revisions: u64,
    pub refusals: u64,
    pub beats: u64,
    pub enqueued: Vec<Arrival>,
    pub fault: Option<FaultSummary>,
    pub admission_s: f64,
    pub step_wall_s: f64,
    pub peak_rss_bytes: u64,
    pub files: Vec<(String, Digest)>,
    pub memory_cap_gb: u32,
    pub variant: String,
    pub end: RecordDigests,
    pub branch: Option<BranchSummary>,
}
impl RunSummary {
    pub fn render(&self) -> String {
        let IdentityDigests {
            model,
            packet,
            resolution,
            run_identity,
        } = &self.identity;
        let mut lines = vec![
            format!("spec_id {}", self.id),
            format!("capture_version {}", self.capture_version),
            format!("source_commit {}", self.source_commit),
            format!("model {}", hex(model)),
            format!("packet {}", hex(packet)),
            format!("resolution {}", hex(resolution)),
            format!("run_identity {}", hex(run_identity)),
            format!("first_seal {}", hex(&self.first_seal)),
            format!("last_seal {}", hex(&self.last_seal)),
            format!("revisions {}", self.revisions),
            format!("refusals {}", self.refusals),
            format!("beats {}", self.beats),
            format!("enqueued {}", entries(&self.enqueued)),
        ];
        match &self.fault {
            None => lines.push("fault none".to_string()),
            Some(fault) => lines.extend(fault.lines()),
        }
        lines.push(format!("admission_s {:.3}", self.admission_s));
        lines.push(format!("step_wall_s {:.3}", self.step_wall_s));
        lines.push(format!("peak_rss_bytes {}", self.peak_rss_bytes));
        for (name, digest) in &self.files {
            lines.push(format!("file {name} {}", hex(digest)));
        }
        lines.push(format!("memory_cap_gb {}", self.memory_cap_gb));
        lines.push(format!("variant {}", self.variant));
        lines.extend(digest_lines("end", &self.end));
        if let Some(branch) = &self.branch {
            lines.push(format!("branch_from_revision {}", branch.from_revision));
            lines.push(format!("branch_enqueued {}", entries(&branch.enqueued)));
            lines.push(format!("branch_revisions {}", branch.revisions));
            lines.push(format!("branch_refusals {}", branch.refusals));
            lines.push(format!("branch_last_seal {}", hex(&branch.last_seal)));
            lines.extend(digest_lines("branch_end", &branch.end));
        }
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::{RunState, Staged};
    use crate::spec::RunSpec;
    use std::fs;

    #[test]
    fn a_run_directory_is_a_partial_only_once_its_lock_is_held() {
        let out = std::env::temp_dir().join(format!("gp-driver-archive-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).expect("a scratch directory");
        let text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 2000\nvariant none\n";
        let id = RunSpec::parse(text).expect("canonical").id();
        let staged = Staged::create(&out, id).expect("staged");
        assert_eq!(
            RunState::of(&out, id).expect("a state"),
            RunState::Missing,
            "a staged run is no partial, so no other run removes it"
        );
        let partial = staged.publish().expect("published");
        let published = RunState::Partial {
            paths: vec![partial.directory().to_path_buf()],
            held: true,
        };
        assert_eq!(RunState::of(&out, id).expect("a state"), published);
        drop(partial);
        let state = RunState::of(&out, id).expect("a state");
        assert!(
            matches!(state, RunState::Partial { held: false, .. }),
            "{state:?}"
        );
        fs::remove_dir_all(&out).expect("the scratch directory removed");
    }
}
