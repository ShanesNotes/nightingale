use crate::boundary::PendingInputs;
use crate::identity::{Canonical, Seal};
use crate::{
    AdmissibleSet, BoundParameters, Candidate, Compensated, CompiledBody, DenseSegment, Digest, Interval,
    MethodHistory, PacketHash, RowKind, SimTime,
};

const IMAGE_FORMAT: &str = "ng-contract image 1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malformed;

/// A length is refused unless the bytes left can hold it, so a corrupt length asks for no allocation the
/// input does not back.
pub struct ByteReader<'b> {
    bytes: &'b [u8],
}
impl<'b> ByteReader<'b> {
    pub fn new(bytes: &'b [u8]) -> Self {
        Self { bytes }
    }
    pub fn take(&mut self, count: usize) -> Result<&'b [u8], Malformed> {
        let (taken, rest) = self.bytes.split_at_checked(count).ok_or(Malformed)?;
        self.bytes = rest;
        Ok(taken)
    }
    pub fn number(&mut self) -> Result<u64, Malformed> {
        let bytes = self.take(8)?.first_chunk().ok_or(Malformed)?;
        Ok(u64::from_be_bytes(*bytes))
    }
    pub fn time(&mut self) -> Result<SimTime, Malformed> {
        Ok(SimTime::from_ns(self.number()?))
    }
    pub fn digest(&mut self) -> Result<Digest, Malformed> {
        let bytes = self.take(32)?.first_chunk().ok_or(Malformed)?;
        Ok(Digest::from_bytes(*bytes))
    }
    pub fn compensated(&mut self) -> Result<Compensated, Malformed> {
        Compensated::from_bits([self.number()?, self.number()?]).ok_or(Malformed)
    }
    pub fn count(&mut self, least_item_bytes: usize) -> Result<usize, Malformed> {
        let count = usize::try_from(self.number()?).map_err(|_| Malformed)?;
        let needed = count.checked_mul(least_item_bytes).ok_or(Malformed)?;
        if needed > self.bytes.len() {
            return Err(Malformed);
        }
        Ok(count)
    }
    pub fn finish(self) -> Result<(), Malformed> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(Malformed)
        }
    }
    pub(crate) fn float(&mut self) -> Result<f64, Malformed> {
        Ok(f64::from_bits(self.number()?))
    }
    pub(crate) fn text(&mut self) -> Result<&'b str, Malformed> {
        let count = self.count(1)?;
        core::str::from_utf8(self.take(count)?).map_err(|_| Malformed)
    }
    fn exact_count(&mut self, item_bytes: usize, expected: usize) -> Result<(), Malformed> {
        if self.count(item_bytes)? == expected {
            Ok(())
        } else {
            Err(Malformed)
        }
    }
    pub fn compensated_list(&mut self, expected: usize) -> Result<Vec<Compensated>, Malformed> {
        self.exact_count(16, expected)?;
        (0..expected).map(|_| self.compensated()).collect()
    }
    fn float_list(&mut self, expected: usize) -> Result<Vec<f64>, Malformed> {
        self.exact_count(8, expected)?;
        (0..expected).map(|_| self.float()).collect()
    }
    fn number_list(&mut self, expected: usize) -> Result<Vec<u64>, Malformed> {
        self.exact_count(8, expected)?;
        (0..expected).map(|_| self.number()).collect()
    }
    fn mode_list(&mut self, body: &CompiledBody) -> Result<Vec<usize>, Malformed> {
        let modes = &body.layout.modes;
        self.exact_count(8, modes.len())?;
        let read = modes.iter().map(|mode| {
            let value = usize::try_from(self.number()?).map_err(|_| Malformed)?;
            mode.values.get(value).map(|_| value).ok_or(Malformed)
        });
        read.collect()
    }
}

/// What the contract owns of one accepted image. Parts enter a record only through `InitialRevision`,
/// which only `InitializationSystem::admit` makes, or through checkpoint bytes `ng-record` has verified,
/// so `parse` mints nothing a record takes.
pub struct ImageParts {
    pub time: SimTime,
    pub state: Candidate,
    pub continuation: MethodHistory,
    pub dense: DenseSegment,
    pub pending: PendingInputs,
    pub(crate) parameters: BoundParameters,
}
impl ImageParts {
    /// Lent, never moved, so bound values in a parsed image cannot seed a new admission.
    pub fn parameters(&self) -> &BoundParameters {
        &self.parameters
    }
    pub fn canonical(&self) -> Vec<u8> {
        let Self {
            time,
            state,
            continuation,
            dense,
            pending,
            parameters,
        } = self;
        let mut out = Canonical::new(IMAGE_FORMAT);
        time.seal(&mut out);
        let Candidate {
            amounts,
            memory,
            algebraic,
            modes,
            ordinals,
            fired,
        } = state;
        amounts.seal(&mut out);
        memory.seal(&mut out);
        algebraic.seal(&mut out);
        modes.seal(&mut out);
        ordinals.seal(&mut out);
        out.number(u64::from(*fired));
        let MethodHistory {
            schema,
            started_at,
            counters,
        } = continuation;
        out.raw(schema.as_bytes());
        started_at.seal(&mut out);
        counters.seal(&mut out);
        let DenseSegment { interval, start, end } = dense;
        interval
            .map(|interval| (interval.start(), interval.end()))
            .seal(&mut out);
        start.seal(&mut out);
        end.seal(&mut out);
        pending.write(&mut out);
        let BoundParameters {
            model,
            packet,
            values,
            starts,
            modes,
        } = parameters;
        out.raw(model.digest().as_bytes());
        out.raw(packet.digest().as_bytes());
        values.seal(&mut out);
        starts.seal(&mut out);
        modes.seal(&mut out);
        out.into_bytes()
    }
    /// Any bytes give parts or `Malformed`, never a panic.
    pub fn parse(body: &CompiledBody, bytes: &[u8]) -> Result<Self, Malformed> {
        let mut read = ByteReader::new(bytes);
        let format = read.text()?;
        if format != IMAGE_FORMAT {
            return Err(Malformed);
        }
        let time = read.time()?;
        let rows = body.rows();
        let count = |kind: fn(&RowKind) -> bool| rows.iter().filter(|row| kind(&row.kind())).count();
        let state = Candidate {
            amounts: read.compensated_list(count(|kind| matches!(kind, RowKind::Amount { .. })))?,
            memory: read.float_list(count(|kind| matches!(kind, RowKind::Memory(_))))?,
            algebraic: read.float_list(count(|kind| matches!(kind, RowKind::Algebraic(_))))?,
            modes: read.mode_list(body)?,
            ordinals: read.number_list(body.layout.streams.len())?,
            fired: match read.number()? {
                0 => false,
                1 => true,
                _ => return Err(Malformed),
            },
        };
        if !rows
            .iter()
            .all(|row| row.admissible().holds(state.value(row.id())))
        {
            return Err(Malformed);
        }
        let schema = read.digest()?;
        let started_at = read.time()?;
        let counters = (0..read.count(8)?)
            .map(|_| read.number())
            .collect::<Result<_, _>>()?;
        let continuation = MethodHistory::new(schema, started_at, counters);
        let dense = match read.number()? {
            0 => DenseSegment {
                interval: None,
                start: read.compensated_list(0)?,
                end: read.compensated_list(0)?,
            },
            1 => {
                let interval = Interval::new(read.time()?, read.time()?).ok_or(Malformed)?;
                if interval.end() != time {
                    return Err(Malformed);
                }
                DenseSegment {
                    interval: Some(interval),
                    start: read.compensated_list(rows.len())?,
                    end: read.compensated_list(rows.len())?,
                }
            }
            _ => return Err(Malformed),
        };
        let pending = PendingInputs::read(&mut read, body, time)?;
        if read.digest()? != body.model_hash().digest() {
            return Err(Malformed);
        }
        let parameters = BoundParameters {
            model: body.model_hash(),
            packet: PacketHash::of_digest(read.digest()?),
            values: read.float_list(body.layout.parameters.len())?,
            starts: read.float_list(rows.len())?,
            modes: read.mode_list(body)?,
        };
        let inside = |set: AdmissibleSet, value: &f64| set.holds(Compensated::exact(*value));
        if !(body.layout.parameters.iter().zip(&parameters.values))
            .all(|(slot, value)| inside(slot.admissible, value))
            || !(rows.iter().zip(&parameters.starts)).all(|(row, value)| inside(row.admissible(), value))
            || body.unheld_period(&parameters.values).is_some()
        {
            return Err(Malformed);
        }
        read.finish()?;
        Ok(Self {
            time,
            state,
            continuation,
            dense,
            pending,
            parameters,
        })
    }
}
