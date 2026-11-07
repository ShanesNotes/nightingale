use ng_contract::Digest;
use ng_record::{CheckpointError, CompactLink, HistoryStore, compact, expand};
use std::cell::{Cell, RefCell};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;

const HEAD: u64 = 8;
const TAIL: u64 = 32 + 8;

pub struct LinkLog {
    path: PathBuf,
    file: File,
    end: Cell<u64>,
    cursors: Cell<[u64; 2]>,
    newer: Cell<usize>,
    newest: RefCell<Option<(Digest, Vec<u8>)>>,
    replay: Cell<u64>,
}

struct Record {
    seal: Digest,
    start: u64,
    length: u64,
}
impl Record {
    fn end(&self) -> u64 {
        self.start + HEAD + self.length + TAIL
    }
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
            newest: RefCell::new(None),
            replay: Cell::new(0),
        }
    }

    pub fn bytes(&self) -> u64 {
        self.end.get()
    }

    fn read(&self, at: u64, length: usize) -> Vec<u8> {
        let mut bytes = vec![0; length];
        self.file.read_exact_at(&mut bytes, at).expect("the log reads");
        bytes
    }

    fn number(bytes: &[u8]) -> u64 {
        u64::from_be_bytes(bytes.try_into().expect("eight bytes"))
    }

    fn record_before(&self, end: u64) -> Record {
        let tail = self.read(end - TAIL, TAIL as usize);
        let (seal, length) = tail.split_at(32);
        let length = Self::number(length);
        let seal = Digest::from_bytes(seal.try_into().expect("32 bytes"));
        let start = end - TAIL - length - HEAD;
        Record { seal, start, length }
    }

    fn record_at(&self, start: u64) -> Record {
        let length = Self::number(&self.read(start, HEAD as usize));
        let seal = self.read(start + HEAD + length, 32);
        let seal = Digest::from_bytes(seal.as_slice().try_into().expect("32 bytes"));
        Record { seal, start, length }
    }

    fn compact_link(&self, record: &Record) -> Vec<u8> {
        let length = usize::try_from(record.length).expect("a link fits in memory");
        self.read(record.start + HEAD, length)
    }

    fn find(&self, seal: &Digest) -> Option<Record> {
        let mut end = self.end.get();
        while end > 0 {
            let record = self.record_before(end);
            if &record.seal == seal {
                return Some(record);
            }
            end = record.start;
        }
        None
    }

    fn append(&self, seal: Digest, link: &[u8]) {
        let mut newest = self.newest.borrow_mut();
        let kept = compact(
            newest.as_ref().map(|(seal, kept)| CompactLink {
                seal: *seal,
                bytes: kept,
            }),
            link,
        );
        let length = (kept.len() as u64).to_be_bytes();
        let mut record = length.to_vec();
        record.extend_from_slice(&kept);
        record.extend_from_slice(seal.as_bytes());
        record.extend_from_slice(&length);
        let end = self.end.get();
        self.file.write_all_at(&record, end).expect("the log writes");
        self.end.set(end + record.len() as u64);
        self.replay.set(self.end.get());
        *newest = Some((seal, kept));
    }

    fn replays(&self, seal: &Digest) -> bool {
        let at = self.replay.get();
        let expected = (at < self.end.get()).then(|| self.record_at(at));
        let replayed = expected.filter(|expected| &expected.seal == seal);
        if let Some(expected) = &replayed {
            self.replay.set(expected.end());
        }
        replayed.is_some()
    }
}

impl HistoryStore for LinkLog {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let mut cursors = self.cursors.get();
        let (hit, record) = (0..2)
            .filter(|&which| cursors[which] > 0)
            .find_map(|which| {
                let record = self.record_before(cursors[which]);
                (&record.seal == seal).then_some((which, record))
            })
            .or_else(|| Some((1 - self.newer.get(), self.find(seal)?)))?;
        cursors[hit] = record.start;
        self.cursors.set(cursors);
        self.newer.set(hit);
        let previous = (record.start > 0).then(|| {
            let previous = self.record_before(record.start);
            (previous.seal, self.compact_link(&previous))
        });
        let previous = previous.as_ref().map(|(seal, kept)| CompactLink {
            seal: *seal,
            bytes: kept,
        });
        expand(previous, &self.compact_link(&record))
    }

    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        if self.replays(&seal) {
            return Ok(());
        }
        let follows = match &*self.newest.borrow() {
            Some((newest, _)) => link.get(..32) == Some(newest.as_bytes().as_slice()),
            None => true,
        };
        match (!follows).then(|| self.find(&seal)).flatten() {
            Some(held) => self.replay.set(held.end()),
            None => self.append(seal, link),
        }
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
