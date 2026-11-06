use crate::history::{History, Start, push_bits};
use crate::record::PatientRecord;
use core::cmp::Ordering;
use ng_contract::{
    ByteReader, Compensated, CompiledBody, Digest, ImageParts, Malformed, RecordHash, RunIdentity, SimTime,
};

/// A checkpoint's only form: the envelope's bytes, so a rewind, a recover and a resume take one path.
/// `from_bytes` checks nothing.
pub struct Checkpoint {
    bytes: Vec<u8>,
}
impl Checkpoint {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointError {
    /// Fewer bytes than the header, or than the header says the body holds.
    Truncated,
    /// Not this format.
    UnknownFormat,
    /// More bytes than the header says the body holds.
    TrailingContent,
    /// The raw digest does not match the bytes: corruption, caught with no body field decoded.
    Corrupt,
    /// A checkpoint whose raw digest holds, for another run, or an expected identity that names another body
    /// or another solver's resolution.
    IdentityMismatch,
    /// A fault's last image. It continues only through `recover`, on a new branch.
    Latched,
    /// A checkpoint whose raw digest holds, for this run, with a body that is not the canonical form of an
    /// image, its ancestry, its opening values and its origin.
    Malformed,
    /// The parts parsed, and they are not the image the chain recorded for this head.
    SealMismatch,
    /// The store has no link for this seal.
    HistoryMissing { seal: Digest },
    /// A link that does not chain to its seal, does not parse or does not reach the opening.
    HistoryAltered { seal: Digest },
    /// The oldest link the session's store has not kept. No checkpoint exists until a later step's store
    /// keeps it.
    StoreFailed { seal: Digest },
    /// `rewind` or `recover` given a branch id this branch or an ancestor already uses.
    BranchInUse,
    /// `rewind` or `recover` given a checkpoint off this branch's own chain.
    NotOnThisBranch,
}

/// Durable storage for the chain's links, addressed by seal. It is host code and trusted for nothing: a
/// walk chains what comes back to the seal it asked for. A session hands it each link as its step is
/// accepted, and a store that loses a link fails the next walk that needs it, by that link's seal.
pub trait HistoryStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>>;
    /// Keeping a seal it holds changes nothing, so every session of a run may share one store.
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchId {
    digest: Digest,
}
impl BranchId {
    pub fn new(digest: Digest) -> Self {
        Self { digest }
    }
    pub(crate) fn digest(self) -> Digest {
        self.digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchFork {
    parent: Digest,
    revision: u64,
    at: SimTime,
}
impl BranchFork {
    pub fn parent(&self) -> Digest {
        self.parent
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn at(&self) -> SimTime {
        self.at
    }
}

/// A checkpoint carries it and no seal covers it, so a fork and the same course loaded from its checkpoint
/// have equal seals.
#[derive(Clone)]
pub(crate) struct BranchAncestry {
    id: Digest,
    forks: Vec<BranchFork>,
}
impl BranchAncestry {
    pub(crate) fn of_run(run: Digest) -> Self {
        Self {
            id: run,
            forks: Vec::new(),
        }
    }
    pub(crate) fn id(&self) -> Digest {
        self.id
    }
    pub(crate) fn forks(&self) -> &[BranchFork] {
        &self.forks
    }
    pub(crate) fn uses(&self, id: Digest) -> bool {
        id == self.id || self.forks.iter().any(|fork| fork.parent == id)
    }
    fn forked(&self, branch: BranchId, revision: u64, at: SimTime) -> Result<Self, CheckpointError> {
        if self.uses(branch.digest) {
            return Err(CheckpointError::BranchInUse);
        }
        let mut forks = self.forks.clone();
        forks.push(BranchFork {
            parent: self.id,
            revision,
            at,
        });
        Ok(Self {
            id: branch.digest,
            forks,
        })
    }
    fn write(&self, out: &mut Vec<u8>) {
        let Self { id, forks } = self;
        out.extend_from_slice(id.as_bytes());
        out.extend_from_slice(&(forks.len() as u64).to_be_bytes());
        for BranchFork { parent, revision, at } in forks {
            out.extend_from_slice(parent.as_bytes());
            out.extend_from_slice(&revision.to_be_bytes());
            out.extend_from_slice(&at.nanoseconds().to_be_bytes());
        }
    }
    fn read(read: &mut ByteReader<'_>) -> Result<Self, Malformed> {
        let id = read.digest()?;
        let forks = (0..read.count(48)?)
            .map(|_| {
                Ok(BranchFork {
                    parent: read.digest()?,
                    revision: read.number()?,
                    at: read.time()?,
                })
            })
            .collect::<Result<_, Malformed>>()?;
        Ok(Self { id, forks })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecordFormat {
    V1,
    V2,
}
impl RecordFormat {
    pub(crate) const ADMISSION: Self = Self::V2;
    fn magic(self) -> [u8; MAGIC_LENGTH] {
        match self {
            Self::V1 => *b"ngcheck1",
            Self::V2 => *b"ngcheck2",
        }
    }
    pub(crate) fn hash(self) -> RecordHash {
        match self {
            Self::V1 => RecordHash::Sha256,
            Self::V2 => RecordHash::Blake2b,
        }
    }
}

const MAGIC_LENGTH: usize = 8;
/// The raw digest covers every byte after it, so checking it decodes no body field.
const RAW_DIGEST_END: usize = MAGIC_LENGTH + 32;
/// The body's length, the run identity, the revision, the seal and the latch.
const HEADER: usize = RAW_DIGEST_END + 8 + 32 + 8 + 32 + 8;

struct Header<'b> {
    format: RecordFormat,
    identity: Digest,
    revision: u64,
    seal: Digest,
    latched: bool,
    body: &'b [u8],
}

fn opened(bytes: &[u8]) -> Result<Header<'_>, CheckpointError> {
    let (header, body) = bytes.split_at_checked(HEADER).ok_or(CheckpointError::Truncated)?;
    let mut read = ByteReader::new(header);
    let short = |_: Malformed| CheckpointError::Truncated;
    let magic = read.take(MAGIC_LENGTH).map_err(short)?;
    let format = [RecordFormat::V1, RecordFormat::V2]
        .into_iter()
        .find(|format| magic == format.magic())
        .ok_or(CheckpointError::UnknownFormat)?;
    let digest = read.digest().map_err(short)?;
    let stated = read.number().map_err(short)?;
    match usize::try_from(stated).map_or(Ordering::Greater, |stated| stated.cmp(&body.len())) {
        Ordering::Less => return Err(CheckpointError::TrailingContent),
        Ordering::Greater => return Err(CheckpointError::Truncated),
        Ordering::Equal => {}
    }
    if Digest::of_canonical(&bytes[RAW_DIGEST_END..]) != digest {
        return Err(CheckpointError::Corrupt);
    }
    let identity = read.digest().map_err(short)?;
    let revision = read.number().map_err(short)?;
    let seal = read.digest().map_err(short)?;
    let latched = match read.number().map_err(short)? {
        0 => false,
        1 => true,
        _ => return Err(CheckpointError::UnknownFormat),
    };
    Ok(Header {
        format,
        identity,
        revision,
        seal,
        latched,
        body,
    })
}

/// What a restored record goes on as. Only `Recover` goes past a latch, and a fork opens a new branch from
/// a revision its parent's chain holds.
#[derive(Clone, Copy)]
pub(crate) enum Continuing<'p> {
    Resume,
    Rewind(BranchId, &'p History),
    Recover(BranchId, &'p History),
}

pub(crate) struct Sections<'r> {
    pub(crate) image: &'r ImageParts,
    pub(crate) branch: &'r BranchAncestry,
    pub(crate) history: &'r History,
}

pub(crate) fn write(
    identity: &RunIdentity,
    revision: u64,
    latched: bool,
    sections: Sections<'_>,
) -> Checkpoint {
    let Sections {
        image,
        branch,
        history,
    } = sections;
    let image = image.canonical();
    let mut body = (image.len() as u64).to_be_bytes().to_vec();
    body.extend_from_slice(&image);
    branch.write(&mut body);
    let opening = history.opening_values();
    body.extend_from_slice(&(opening.len() as u64).to_be_bytes());
    for value in opening {
        push_bits(&mut body, *value);
    }
    body.extend_from_slice(history.origin().image.as_bytes());
    let mut bytes = history.format().magic().to_vec();
    bytes.extend_from_slice(&[0; 32]);
    bytes.extend_from_slice(&(body.len() as u64).to_be_bytes());
    bytes.extend_from_slice(identity.digest().as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(history.seal().as_bytes());
    bytes.extend_from_slice(&u64::from(latched).to_be_bytes());
    bytes.extend_from_slice(&body);
    let raw = Digest::of_canonical(&bytes[RAW_DIGEST_END..]);
    bytes[MAGIC_LENGTH..RAW_DIGEST_END].copy_from_slice(raw.as_bytes());
    Checkpoint { bytes }
}

/// Any bytes, from a host's store, give a record or an error, never a panic. A fork reads its parent's chain
/// through the same store, so a rewind and a load take one path.
pub(crate) fn restore<'m>(
    saved: &Checkpoint,
    expected: &RunIdentity,
    body: &'m CompiledBody,
    store: &dyn HistoryStore,
    continuing: Continuing<'_>,
) -> Result<PatientRecord<'m>, CheckpointError> {
    let header = opened(saved.as_bytes())?;
    if header.identity != expected.digest() {
        return Err(CheckpointError::IdentityMismatch);
    }
    if header.latched && !matches!(continuing, Continuing::Recover(..)) {
        return Err(CheckpointError::Latched);
    }
    let parts = Parts::read(header.body, body).map_err(|_| CheckpointError::Malformed)?;
    let Parts {
        image,
        branch,
        opening,
        origin,
    } = parts;
    if let Continuing::Rewind(_, parent) | Continuing::Recover(_, parent) = continuing
        && (header.format != parent.format() || !parent.holds(header.seal, header.revision, store)?)
    {
        return Err(CheckpointError::NotOnThisBranch);
    }
    let start = Start {
        format: header.format,
        run: expected.digest(),
        opening,
        image: origin,
        parameters: image.parameters().digest(),
    };
    let history = History::walked(body, start, header.seal, header.revision, store)?;
    let branch = match continuing {
        Continuing::Resume => branch,
        Continuing::Rewind(fork, _) | Continuing::Recover(fork, _) => {
            branch.forked(fork, header.revision, image.time)?
        }
    };
    let parts = Restored {
        branch,
        image,
        history,
    };
    PatientRecord::restored(body, expected.clone(), parts)
}

pub(crate) struct Restored {
    pub(crate) branch: BranchAncestry,
    pub(crate) image: ImageParts,
    pub(crate) history: History,
}

struct Parts {
    image: ImageParts,
    branch: BranchAncestry,
    opening: Vec<Compensated>,
    origin: Digest,
}
impl Parts {
    fn read(bytes: &[u8], body: &CompiledBody) -> Result<Self, Malformed> {
        let mut read = ByteReader::new(bytes);
        let count = read.count(1)?;
        let image = ImageParts::parse(body, read.take(count)?)?;
        let branch = BranchAncestry::read(&mut read)?;
        let opening = read.compensated_list(body.conserved_basis().combinations().len())?;
        let origin = read.digest()?;
        read.finish()?;
        Ok(Self {
            image,
            branch,
            opening,
            origin,
        })
    }
}
