use ng_contract::Digest;
use ng_record::{CheckpointError, HistoryStore};
use std::cell::Cell;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;

const TAIL: u64 = 32 + 8;

pub struct LinkLog {
    path: PathBuf,
    file: File,
    end: Cell<u64>,
    cursors: Cell<[u64; 2]>,
    newer: Cell<usize>,
}

impl LinkLog {
    pub fn create() -> Self {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        std::fs::create_dir_all(&directory).expect("the log's directory");
        let path = directory.join(format!("history-{}.log", std::process::id()));
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("the log opens");
        Self {
            path,
            file,
            end: Cell::new(0),
            cursors: Cell::new([0; 2]),
            newer: Cell::new(0),
        }
    }

    pub fn bytes(&self) -> u64 {
        self.end.get()
    }

    fn record_before(&self, end: u64) -> ([u8; 32], u64, u64) {
        let mut tail = [0; TAIL as usize];
        self.file
            .read_exact_at(&mut tail, end - TAIL)
            .expect("the log reads");
        let (seal, length) = tail.split_at(32);
        let length = u64::from_be_bytes(length.try_into().expect("eight bytes"));
        let seal = seal.try_into().expect("32 bytes");
        (seal, end - TAIL - length, length)
    }

    fn link(&self, start: u64, length: u64) -> Vec<u8> {
        let mut link = vec![0; usize::try_from(length).expect("a link fits in memory")];
        self.file.read_exact_at(&mut link, start).expect("the log reads");
        link
    }
}

impl HistoryStore for LinkLog {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let mut cursors = self.cursors.get();
        let (hit, start, length) = (0..2)
            .filter(|&which| cursors[which] > 0)
            .find_map(|which| {
                let (found, start, length) = self.record_before(cursors[which]);
                (&found == seal.as_bytes()).then_some((which, start, length))
            })
            .or_else(|| {
                let mut end = self.end.get();
                while end > 0 {
                    let (found, start, length) = self.record_before(end);
                    if &found == seal.as_bytes() {
                        return Some((1 - self.newer.get(), start, length));
                    }
                    end = start;
                }
                None
            })?;
        cursors[hit] = start;
        self.cursors.set(cursors);
        self.newer.set(hit);
        Some(self.link(start, length))
    }

    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        let mut record = link.to_vec();
        record.extend_from_slice(seal.as_bytes());
        record.extend_from_slice(&(link.len() as u64).to_be_bytes());
        let end = self.end.get();
        self.file.write_all_at(&record, end).expect("the log writes");
        self.end.set(end + record.len() as u64);
        Ok(())
    }
}

impl Drop for LinkLog {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            eprintln!("bench: {} stays: {error}", self.path.display());
        }
    }
}
