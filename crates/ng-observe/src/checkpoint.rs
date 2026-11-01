use crate::observer::{Filling, Held, Observer, Reading};
use ng_contract::{
    AdmissibleSet, ByteReader, Channel, Compensated, Digest, Malformed, ObservationLens, ObservationSeed,
};
use ng_record::ObservationRefusal;

const MAGIC: [u8; 8] = *b"ngwatch1";
/// The raw digest covers every byte after it. No seal covers an observer's state, so nothing else would
/// notice a flipped value.
const RAW_DIGEST_END: usize = MAGIC.len() + 32;

/// An observer's only saved form, stored beside the record's checkpoint. `from_bytes` checks nothing.
pub struct ObserverCheckpoint {
    bytes: Vec<u8>,
}
impl ObserverCheckpoint {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

fn push_pair(bytes: &mut Vec<u8>, value: Compensated) {
    for limb in value.bits() {
        bytes.extend_from_slice(&limb.to_be_bytes());
    }
}

fn finite(value: Compensated) -> Result<Compensated, Malformed> {
    if AdmissibleSet::Finite.holds(value) {
        Ok(value)
    } else {
        Err(Malformed)
    }
}

/// The digest has no key, so a file that recomputes it reaches here. This refuses what a field alone shows no
/// observer left. A false value that looks true still passes, as does a reading from the future.
fn filling(read: &mut ByteReader<'_>, channel: &Channel) -> Result<Filling, Malformed> {
    match read.number()? {
        0 => {
            let sum = finite(read.compensated()?)?;
            let seen = read.number()?;
            if seen < channel.window() {
                Ok(Filling::Mean { sum, seen })
            } else {
                Err(Malformed)
            }
        }
        1 => Ok(Filling::Invalid),
        _ => Err(Malformed),
    }
}

fn held(read: &mut ByteReader<'_>, channel: &Channel) -> Result<Vec<Held>, Malformed> {
    let mut queue: Vec<Held> = Vec::new();
    for _ in 0..read.count(24)? {
        let collected = read.time()?;
        let value = finite(read.compensated()?)?;
        let available = channel.available(collected).ok_or(Malformed)?;
        if queue.last().is_some_and(|last| collected <= last.collected) {
            return Err(Malformed);
        }
        queue.push(Held {
            collected,
            available,
            value,
        });
    }
    Ok(queue)
}

struct Parts {
    last: Option<Digest>,
    readings: Vec<Reading>,
}
fn parts(body: &[u8], channels: &[Channel]) -> Result<Parts, Malformed> {
    let mut read = ByteReader::new(body);
    let last = match read.number()? {
        0 => None,
        1 => Some(read.digest()?),
        _ => return Err(Malformed),
    };
    let readings = (channels.iter())
        .map(|channel| {
            Ok(Reading {
                filling: filling(&mut read, channel)?,
                held: held(&mut read, channel)?,
            })
        })
        .collect::<Result<_, Malformed>>()?;
    read.finish()?;
    Ok(Parts { last, readings })
}

impl<'l> Observer<'l> {
    /// Taken after reading a revision, it stands beside that revision's checkpoint and every fork that keeps
    /// its seal.
    pub fn checkpoint(&self) -> ObserverCheckpoint {
        let mut body = self.lens.observer_digest(self.seed).as_bytes().to_vec();
        match self.last {
            None => body.extend_from_slice(&0_u64.to_be_bytes()),
            Some(seal) => {
                body.extend_from_slice(&1_u64.to_be_bytes());
                body.extend_from_slice(seal.as_bytes());
            }
        }
        for Reading { filling, held } in &self.readings {
            match filling {
                Filling::Mean { sum, seen } => {
                    body.extend_from_slice(&0_u64.to_be_bytes());
                    push_pair(&mut body, *sum);
                    body.extend_from_slice(&seen.to_be_bytes());
                }
                Filling::Invalid => body.extend_from_slice(&1_u64.to_be_bytes()),
            }
            body.extend_from_slice(&(held.len() as u64).to_be_bytes());
            for held in held {
                body.extend_from_slice(&held.collected.nanoseconds().to_be_bytes());
                push_pair(&mut body, held.value);
            }
        }
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(Digest::of_canonical(&body).as_bytes());
        bytes.extend_from_slice(&body);
        ObserverCheckpoint { bytes }
    }
    /// Any bytes give an observer or a refusal, never a panic. The seal needs no check here: the next `advance`
    /// refuses a view that does not follow it, the one place order is judged.
    pub fn resume(
        lens: &'l ObservationLens,
        seed: ObservationSeed,
        saved: &ObserverCheckpoint,
    ) -> Result<Self, ObservationRefusal> {
        let malformed = ObservationRefusal::Malformed;
        let (head, body) = saved.bytes.split_at_checked(RAW_DIGEST_END).ok_or(malformed)?;
        let (magic, raw) = head.split_at(MAGIC.len());
        let raw = Digest::from_bytes(*raw.first_chunk().ok_or(malformed)?);
        if magic != MAGIC || Digest::of_canonical(body) != raw {
            return Err(malformed);
        }
        let (bound, rest) = body.split_first_chunk::<32>().ok_or(malformed)?;
        if Digest::from_bytes(*bound) != lens.observer_digest(seed) {
            return Err(ObservationRefusal::OtherLens);
        }
        let Parts { last, readings } = parts(rest, lens.channels()).map_err(|_| malformed)?;
        Ok(Self {
            lens,
            seed,
            last,
            readings,
        })
    }
}
