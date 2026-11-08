use ng_contract::Digest;
use ng_record::{CheckpointError, CompactLink, HistoryStore, compact, expand};
use std::cell::{Cell, RefCell};
use std::fs::{self, File};
use std::io;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

const HEAD: u64 = 8;
const TAIL: u64 = 32 + 8;

pub struct LinkFile {
    path: PathBuf,
    file: File,
    end: Cell<u64>,
    cursors: Cell<[u64; 2]>,
    newer: Cell<usize>,
    newest: RefCell<Option<(Digest, Vec<u8>)>>,
    replay: Cell<u64>,
    failure: RefCell<Option<io::Error>>,
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

impl LinkFile {
    pub fn create(path: &Path) -> io::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            end: Cell::new(0),
            cursors: Cell::new([0; 2]),
            newer: Cell::new(0),
            newest: RefCell::new(None),
            replay: Cell::new(0),
            failure: RefCell::new(None),
        })
    }

    pub fn check(&self) -> io::Result<()> {
        match self.failure.borrow_mut().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub fn remove(&self) -> io::Result<()> {
        fs::remove_file(&self.path)
    }

    fn fail(&self, error: io::Error) {
        let mut failure = self.failure.borrow_mut();
        if failure.is_none() {
            *failure = Some(error);
        }
    }

    fn read(&self, at: u64, length: u64) -> Option<Vec<u8>> {
        let mut bytes = vec![0; usize::try_from(length).ok()?];
        match self.file.read_exact_at(&mut bytes, at) {
            Ok(()) => Some(bytes),
            Err(error) => {
                self.fail(error);
                None
            }
        }
    }

    fn number(bytes: &[u8]) -> Option<u64> {
        Some(u64::from_be_bytes(bytes.try_into().ok()?))
    }

    fn record_before(&self, end: u64) -> Option<Record> {
        let tail = self.read(end.checked_sub(TAIL)?, TAIL)?;
        let (seal, length) = tail.split_at(32);
        let length = Self::number(length)?;
        let seal = Digest::from_bytes(seal.try_into().ok()?);
        let start = end.checked_sub(TAIL + length + HEAD)?;
        Some(Record { seal, start, length })
    }

    fn record_at(&self, start: u64) -> Option<Record> {
        let length = Self::number(&self.read(start, HEAD)?)?;
        let seal = self.read(start + HEAD + length, 32)?;
        let seal = Digest::from_bytes(seal.as_slice().try_into().ok()?);
        Some(Record { seal, start, length })
    }

    fn compact_link(&self, record: &Record) -> Option<Vec<u8>> {
        self.read(record.start + HEAD, record.length)
    }

    fn find(&self, seal: &Digest) -> Option<Record> {
        let mut end = self.end.get();
        while end > 0 {
            let record = self.record_before(end)?;
            if &record.seal == seal {
                return Some(record);
            }
            end = record.start;
        }
        None
    }

    fn append(&self, seal: Digest, link: &[u8]) -> io::Result<()> {
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
        self.file.write_all_at(&record, end)?;
        self.end.set(end + record.len() as u64);
        self.replay.set(self.end.get());
        *newest = Some((seal, kept));
        Ok(())
    }

    fn replays(&self, seal: &Digest) -> bool {
        let at = self.replay.get();
        let expected = (at < self.end.get()).then(|| self.record_at(at)).flatten();
        let replayed = expected.filter(|expected| &expected.seal == seal);
        if let Some(expected) = &replayed {
            self.replay.set(expected.end());
        }
        replayed.is_some()
    }
}

impl HistoryStore for LinkFile {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let mut cursors = self.cursors.get();
        let (hit, record) = (0..2)
            .filter(|&which| cursors[which] > 0)
            .find_map(|which| {
                let record = self.record_before(cursors[which])?;
                (&record.seal == seal).then_some((which, record))
            })
            .or_else(|| Some((1 - self.newer.get(), self.find(seal)?)))?;
        cursors[hit] = record.start;
        self.cursors.set(cursors);
        self.newer.set(hit);
        let previous = match record.start {
            0 => None,
            start => {
                let previous = self.record_before(start)?;
                let kept = self.compact_link(&previous)?;
                Some((previous.seal, kept))
            }
        };
        let previous = previous.as_ref().map(|(seal, kept)| CompactLink {
            seal: *seal,
            bytes: kept,
        });
        expand(previous, &self.compact_link(&record)?)
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
            Some(held) => {
                self.replay.set(held.end());
                Ok(())
            }
            None => self.append(seal, link).map_err(|error| {
                self.fail(error);
                CheckpointError::StoreFailed { seal }
            }),
        }
    }
}
