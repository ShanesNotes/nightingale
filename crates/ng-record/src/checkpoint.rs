use crate::history::{History, Start, push_bits};
use crate::record::PatientRecord;
use core::cmp::Ordering;
use ng_contract::{
    ByteReader, Compensated, CompiledBody, Digest, ImageParts, Malformed, RunIdentity, SimTime,
};

/// A checkpoint's only form: the envelope's bytes, so an in-process retry and a loaded save are one path.
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
    /// Well sealed, for another run, another body or another solver's resolution.
    IdentityMismatch,
    /// A resolution with a floor of zero, which no admission takes, so only forged bytes can hold one, and a
    /// landing's search would never close on it.
    ZeroMinimumInterval,
    /// A fault's last image. It continues only through `recover`, on a new branch.
    Latched,
    /// Well sealed and for this run, and not the canonical form of an image.
    Malformed,
    /// The parts parsed, and they are not the image the chain recorded for this head.
    SealMismatch,
    /// The store has no link for this seal.
    HistoryMissing { seal: Digest },
    /// A link that does not chain to its seal, does not parse or does not reach the opening.
    HistoryAltered { seal: Digest },
    /// A store that could not keep a link. No checkpoint exists until it does.
    StoreFailed { seal: Digest },
    /// `rewind` or `recover` given a branch id this branch or an ancestor already uses.
    BranchInUse,
    /// `rewind` or `recover` given a checkpoint off this branch's own chain.
    NotOnThisBranch,
}

/// Durable storage for the chain's links, addressed by seal. It is host code and trusted for nothing: a
/// walk chains what comes back to the seal it asked for. A checkpoint is complete only with a store that
/// keeps what it took, and a store that loses a link fails at `resume`, by that link's seal.
pub trait HistoryStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>>;
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError>;
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

const MAGIC: [u8; 8] = *b"ngcheck1";
/// The raw digest covers every byte after it, so checking it decodes no body field.
const RAW_DIGEST_END: usize = MAGIC.len() + 32;
/// The body's length, the run identity, the revision, the seal and the latch.
const HEADER: usize = RAW_DIGEST_END + 8 + 32 + 8 + 32 + 8;

struct Header<'b> {
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
    if read.take(MAGIC.len()).map_err(short)? != MAGIC {
        return Err(CheckpointError::UnknownFormat);
    }
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
        identity,
        revision,
        seal,
        latched,
        body,
    })
}

/// What a restored record goes on as. Only `Recover` goes past a latch, and it opens a new branch.
#[derive(Clone, Copy)]
pub(crate) enum Continuing {
    Resume,
    Rewind(BranchId),
    Recover(BranchId),
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
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&[0; 32]);
    bytes.extend_from_slice(&(body.len() as u64).to_be_bytes());
    bytes.extend_from_slice(identity.digest().as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(history.seal().as_bytes());
    bytes.extend_from_slice(&u64::from(latched).to_be_bytes());
    bytes.extend_from_slice(&body);
    let raw = Digest::of_canonical(&bytes[RAW_DIGEST_END..]);
    bytes[MAGIC.len()..RAW_DIGEST_END].copy_from_slice(raw.as_bytes());
    Checkpoint { bytes }
}

/// Any bytes, from a host's store on a load or this branch's chain on a rewind, give a record or an error,
/// never a panic.
pub(crate) fn restore<'m>(
    saved: &Checkpoint,
    expected: &RunIdentity,
    body: &'m CompiledBody,
    resolve: &dyn Fn(&Digest) -> Option<Vec<u8>>,
    continuing: Continuing,
) -> Result<PatientRecord<'m>, CheckpointError> {
    let header = opened(saved.as_bytes())?;
    if header.identity != expected.digest() {
        return Err(CheckpointError::IdentityMismatch);
    }
    if header.latched && !matches!(continuing, Continuing::Recover(_)) {
        return Err(CheckpointError::Latched);
    }
    let parts = Parts::read(header.body, body).map_err(|_| CheckpointError::Malformed)?;
    let Parts {
        image,
        branch,
        opening,
        origin,
    } = parts;
    let start = Start {
        run: expected.digest(),
        opening,
        image: origin,
        parameters: image.parameters().digest(),
    };
    let history = History::walked(body, start, header.seal, header.revision, resolve)?;
    let branch = match continuing {
        Continuing::Resume => branch,
        Continuing::Rewind(fork) | Continuing::Recover(fork) => {
            branch.forked(fork, header.revision, image.time)?
        }
    };
    let parts = Restored {
        branch,
        revision: header.revision,
        image,
        history,
    };
    PatientRecord::restored(body, expected.clone(), parts)
}

pub(crate) struct Restored {
    pub(crate) branch: BranchAncestry,
    pub(crate) revision: u64,
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
