use super::{Arrival, BoundaryEvent, BoundaryInput, BoundaryPort, Delivery, Kind, Meal, time_of_day};
use crate::identity::{Canonical, Seal};
use crate::{AdmissionError, CompiledBody, InputId, SimTime};
use core::mem::discriminant;

/// The day a packet holds: the boundary events its run's are made from, in `(time, id)` order. A meal stays one
/// item, its id, instant, end and amounts, so how a meal reaches the timeline can change without changing the day
/// or the packet's hash.
pub struct Day {
    items: Vec<Item>,
}
enum Item {
    Meal {
        id: InputId,
        at: SimTime,
        ends: SimTime,
        amounts: [f64; Meal::SPECIES.len()],
    },
    Event(Delivery),
}
const EVENT: u64 = 0;
const MEAL: u64 = 1;
impl Item {
    /// A meal sorts after an event of its own id and instant, so the order never rests on how events came.
    fn key(&self) -> (SimTime, InputId, u64) {
        match *self {
            Item::Meal { id, at, .. } => (at, id, MEAL),
            Item::Event(delivery) => (delivery.at, delivery.id, EVENT),
        }
    }
    /// What the item delivers: a meal's parts as the port parses them, or the event's one delivery.
    fn arrival(&self) -> Result<Arrival, AdmissionError> {
        match *self {
            Item::Meal {
                id,
                at,
                ends,
                amounts,
            } => Delivery::meal(id, at, Meal { ends, amounts }),
            Item::Event(delivery) => Ok(Arrival::one(delivery)),
        }
    }
}
impl Day {
    /// Parses each event as `BoundaryPort::new` does: the same refusals, one id with two payloads refused and one
    /// id repeated with one payload taken once.
    pub fn new(events: Vec<BoundaryEvent>) -> Result<Self, AdmissionError> {
        let mut known: Vec<Delivery> = Vec::with_capacity(events.len());
        let mut items = Vec::with_capacity(events.len());
        for event in events {
            let (id, at) = (event.id, event.at);
            let meal = match &event.payload {
                BoundaryInput::Meal(Meal { ends, amounts }) => Some((*ends, *amounts)),
                _ => None,
            };
            let arrival = Delivery::parsed(event)?;
            if !arrival.new_to(&known)? {
                continue;
            }
            items.push(match meal {
                Some((ends, amounts)) => Item::Meal {
                    id,
                    at,
                    ends,
                    amounts,
                },
                None => Item::Event(arrival.parts[0]),
            });
            known.extend(arrival.parts);
        }
        items.sort_by_key(Item::key);
        Ok(Self { items })
    }
    /// Refuses an event the body refuses on its port, whatever its instant.
    pub(crate) fn checked(self, body: &CompiledBody) -> Result<Self, AdmissionError> {
        for item in &self.items {
            for part in &item.arrival()?.parts {
                body.check_delivery(part, SimTime::from_ns(0))?;
            }
        }
        Ok(self)
    }
}
impl Seal for Day {
    /// A meal seals as its one item, never as its parts, and every other event as its delivery.
    fn seal(&self, out: &mut Canonical) {
        let Self { items } = self;
        out.length(items.len());
        for item in items {
            match item {
                Item::Event(delivery) => {
                    out.number(EVENT);
                    delivery.seal(out);
                }
                Item::Meal {
                    id,
                    at,
                    ends,
                    amounts,
                } => {
                    out.number(MEAL);
                    id.seal(out);
                    at.seal(out);
                    ends.seal(out);
                    amounts.seal(out);
                }
            }
        }
    }
}

impl BoundaryPort {
    /// The port with `day` merged in, as admission at `moment` reads it. An event at or after the moment enters as
    /// it is, a meal as its parts. The latest held input of each kind before the moment is restamped at it, a clock
    /// moved on to the moment's time of day, unless one of its kind stands at the moment. A delivery that ended by
    /// the moment is dropped, and one that runs across it is refused, because admission would read only its end.
    /// One id on the day and the port is one input when its payload is the same.
    pub(crate) fn with_day(self, day: Day, moment: SimTime) -> Result<Self, AdmissionError> {
        let Self { mut deliveries } = self;
        let mut arrivals: Vec<Arrival> = Vec::new();
        let mut held: Vec<Delivery> = Vec::new();
        for item in &day.items {
            let arrival = item.arrival()?;
            let first = arrival.parts[0];
            if first.at >= moment {
                arrivals.push(arrival);
            } else if first.held_event() {
                held.retain(|known| discriminant(&known.kind) != discriminant(&first.kind));
                held.push(first);
            } else if arrival.parts.iter().any(|part| part.ends > moment) {
                return Err(AdmissionError::DeliveryAcrossAdmission { id: arrival.id });
            }
        }
        let stands = |latest: &Delivery| {
            (deliveries
                .iter()
                .chain(arrivals.iter().flat_map(|arrival| &arrival.parts)))
            .any(|known| known.at == moment && discriminant(&known.kind) == discriminant(&latest.kind))
        };
        let restamped: Vec<Arrival> = (held.into_iter())
            .filter(|latest| !stands(latest))
            .map(|latest| Arrival::one(latest.restamped(moment)))
            .collect();
        arrivals.extend(restamped);
        for arrival in arrivals {
            if arrival.new_to(&deliveries)? {
                deliveries.extend(arrival.parts);
            }
        }
        deliveries.sort_by_key(|delivery| (delivery.at, delivery.id));
        Ok(Self { deliveries })
    }
}
impl Delivery {
    fn restamped(self, moment: SimTime) -> Self {
        let kind = match self.kind {
            Kind::Clock(reading) => Kind::Clock(time_of_day(self.at, reading, moment)),
            kind => kind,
        };
        Self {
            at: moment,
            kind,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        BoundaryEvent, BoundaryInput, BoundaryPort, Clock, Delivered, InputId, Legs, MINUTE_NS, Meal,
        Posture, Sleep, part_id,
    };
    use super::Day;
    use crate::identity::{Canonical, Seal};
    use crate::{AdmissionError, Digest, Glucose, Measure, SimTime, Sodium, Water};

    const SECOND: u64 = 1_000_000_000;
    const HOUR: u64 = 60 * MINUTE_NS;

    fn id(name: &[u8]) -> InputId {
        InputId::new(Digest::of_canonical(name))
    }
    fn event(name: &[u8], at: u64, payload: BoundaryInput) -> BoundaryEvent {
        BoundaryEvent::new(id(name), SimTime::from_ns(at), payload)
    }
    fn water(name: &[u8], at: u64, amount: f64, ends: u64) -> Result<BoundaryEvent, AdmissionError> {
        let delivered = Delivered {
            boundary: "gut water",
            amount: Measure::new(amount)?,
            ends: SimTime::from_ns(ends),
        };
        Ok(event(name, at, BoundaryInput::Water(delivered)))
    }
    fn lunch(ends: u64) -> Result<BoundaryInput, AdmissionError> {
        let meal = Meal::new(SimTime::from_ns(ends))
            .with(Measure::<Water>::new(60.0)?)
            .with(Measure::<Sodium>::new(6.0)?)
            .with(Measure::<Glucose>::new(3.0)?);
        Ok(BoundaryInput::Meal(meal))
    }
    fn sitting(name: &[u8], at: u64) -> Result<BoundaryEvent, AdmissionError> {
        let posture = Posture::new(Measure::new(1.0)?, Legs::SeatedDependent)?;
        Ok(event(name, at, BoundaryInput::Posture(posture)))
    }
    fn lying(name: &[u8], at: u64) -> Result<BoundaryEvent, AdmissionError> {
        let posture = Posture::new(Measure::new(0.0)?, Legs::Horizontal)?;
        Ok(event(name, at, BoundaryInput::Posture(posture)))
    }
    fn clock(name: &[u8], at: u64, since_midnight_ns: u64) -> BoundaryEvent {
        event(name, at, BoundaryInput::Clock(Clock { since_midnight_ns }))
    }
    fn merged(
        day: Vec<BoundaryEvent>,
        port: Vec<BoundaryEvent>,
        moment: u64,
    ) -> Result<Digest, AdmissionError> {
        let port = BoundaryPort::new(port)?.with_day(Day::new(day)?, SimTime::from_ns(moment))?;
        Ok(port.digest())
    }
    fn plain(events: Vec<BoundaryEvent>) -> Result<Digest, AdmissionError> {
        Ok(BoundaryPort::new(events)?.digest())
    }
    fn bytes(day: &Day) -> Vec<u8> {
        let mut out = Canonical::new("day");
        day.seal(&mut out);
        out.into_bytes()
    }
    fn holds(bytes: &[u8], wanted: &[u8]) -> bool {
        bytes.windows(wanted.len()).any(|window| window == wanted)
    }
    /// An id as a seal writes it, with no tag before it.
    fn written(id: InputId) -> Vec<u8> {
        let mut out = Canonical::new("");
        id.seal(&mut out);
        out.into_bytes().split_off(8)
    }

    #[test]
    fn one_id_with_two_payloads_is_refused_and_one_repeated_with_one_is_taken_once()
    -> Result<(), AdmissionError> {
        let twice = vec![water(b"drink", 0, 1.0, SECOND)?, water(b"drink", 0, 2.0, SECOND)?];
        assert_eq!(
            Day::new(twice).err(),
            Some(AdmissionError::InputConflict { id: id(b"drink") })
        );
        let repeated = Day::new(vec![
            water(b"drink", 0, 1.0, SECOND)?,
            water(b"drink", 0, 1.0, SECOND)?,
        ])?;
        let once = Day::new(vec![water(b"drink", 0, 1.0, SECOND)?])?;
        assert_eq!(bytes(&repeated), bytes(&once));
        Ok(())
    }

    #[test]
    fn a_meal_seals_as_one_item_and_no_part_enters_the_day() -> Result<(), AdmissionError> {
        let day = Day::new(vec![event(b"lunch", 80 * SECOND, lunch(140 * SECOND)?)])?;
        let sealed = bytes(&day);
        assert!(holds(&sealed, &written(id(b"lunch"))), "the meal's own id");
        for place in 0..Meal::SPECIES.len() {
            let part = part_id(id(b"lunch"), place);
            assert!(!holds(&sealed, &written(part)), "part {place}");
            assert!(
                !holds(&sealed, Meal::BOUNDARIES[place].as_bytes()),
                "boundary {place}"
            );
        }
        Ok(())
    }

    #[test]
    fn an_event_at_or_after_the_moment_enters_as_the_port_parses_it_and_a_meal_as_its_parts()
    -> Result<(), AdmissionError> {
        let events = || -> Result<Vec<BoundaryEvent>, AdmissionError> {
            Ok(vec![
                event(b"lunch", 80 * SECOND, lunch(140 * SECOND)?),
                water(b"drink", 30 * SECOND, 1.0, 40 * SECOND)?,
                sitting(b"sit", 30 * SECOND)?,
                clock(b"clock", 30 * SECOND, 6 * HOUR),
            ])
        };
        assert_eq!(merged(events()?, Vec::new(), 30 * SECOND)?, plain(events()?)?);
        Ok(())
    }

    #[test]
    fn the_latest_held_input_of_each_kind_before_the_moment_is_restamped_at_it_and_a_clock_moves_on()
    -> Result<(), AdmissionError> {
        let day = vec![
            clock(b"clock", 0, 24 * HOUR - MINUTE_NS),
            event(b"asleep", 0, BoundaryInput::Sleep(Sleep::Nrem)),
            lying(b"lie", 0)?,
            sitting(b"sit", 10 * SECOND)?,
            water(b"drink", SECOND, 1.0, 2 * SECOND)?,
        ];
        let restamped = vec![
            clock(b"clock", 2 * MINUTE_NS, MINUTE_NS),
            event(b"asleep", 2 * MINUTE_NS, BoundaryInput::Sleep(Sleep::Nrem)),
            sitting(b"sit", 2 * MINUTE_NS)?,
        ];
        assert_eq!(merged(day, Vec::new(), 2 * MINUTE_NS)?, plain(restamped)?);
        Ok(())
    }

    #[test]
    fn a_held_input_at_the_moment_stands_over_the_one_before_it_whatever_their_ids()
    -> Result<(), AdmissionError> {
        for (before, at) in [(&b"a"[..], &b"b"[..]), (&b"b"[..], &b"a"[..])] {
            let day = vec![
                event(before, 0, BoundaryInput::Sleep(Sleep::Nrem)),
                event(at, HOUR, BoundaryInput::Sleep(Sleep::Awake)),
            ];
            let only = vec![event(at, HOUR, BoundaryInput::Sleep(Sleep::Awake))];
            assert_eq!(merged(day, Vec::new(), HOUR)?, plain(only)?);
            let day = vec![event(before, 0, BoundaryInput::Sleep(Sleep::Nrem))];
            let port = vec![event(at, HOUR, BoundaryInput::Sleep(Sleep::Awake))];
            let only = vec![event(at, HOUR, BoundaryInput::Sleep(Sleep::Awake))];
            assert_eq!(merged(day, port, HOUR)?, plain(only)?, "the port's");
        }
        Ok(())
    }

    #[test]
    fn one_id_on_the_day_and_the_port_is_one_input_or_a_conflict() -> Result<(), AdmissionError> {
        let drink = || water(b"drink", HOUR, 1.0, 2 * HOUR);
        assert_eq!(merged(vec![drink()?], vec![drink()?], 0)?, plain(vec![drink()?])?);
        let other = water(b"drink", HOUR, 2.0, 2 * HOUR)?;
        assert_eq!(
            merged(vec![drink()?], vec![other], 0).err(),
            Some(AdmissionError::InputConflict { id: id(b"drink") })
        );
        Ok(())
    }
}
