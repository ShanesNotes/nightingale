mod anatomy;
mod holds;
mod keep_back;
mod packet;
mod schema;
mod table;

pub use anatomy::body_anatomy;
pub use keep_back::{FluidRoute, KEEP_BACKS, KeepBack};
pub use packet::{
    BodyBreath, body_breath, body_certificate, body_lens, body_packet, body_packet_at_viscosity_exponent,
    body_packet_with_steady_evaporation, body_packet_with_the_drive_scale_held, body_start_guess,
};
pub use schema::body_schema;
pub use table::{HOLD_TABLE, Held, Hold, Release};

use crate::{AssemblyCase, CaseAssembly};
use ng_contract::*;
use schema::{CARBON_LEAK, NITROGEN_LEAK, OXYGEN_LEAK, VASOPRESSIN_LEAK};

pub struct WholeBody;
impl CaseAssembly for WholeBody {
    fn compiled() -> Result<CompiledBody, AdmissionError> {
        whole_body()
    }
    fn packet(body: &CompiledBody) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        body_packet(body)
    }
    fn lens(body: &CompiledBody) -> Result<ObservationLens, AdmissionError> {
        body_lens(body)
    }
    fn anchored(at: SimTime) -> Vec<BoundaryEvent> {
        vec![body_awake(at), body_room(at)]
    }
}
pub type BodyCase = AssemblyCase<WholeBody>;

/// The body reads sleep, so every admission of it states sleep at the anchor, or admission refuses it as unstated.
pub fn body_awake(at: SimTime) -> BoundaryEvent {
    let id = InputId::new(Digest::of_canonical(b"awake at the anchor"));
    BoundaryEvent::new(id, at, BoundaryInput::Sleep(Sleep::Awake))
}

/// The skin's losses read the room, so every admission of the body states one: still air at 25 degrees Celsius, half
/// saturated, and light clothing.
pub fn body_room(at: SimTime) -> BoundaryEvent {
    let id = InputId::new(Digest::of_canonical(b"the body's room at the anchor"));
    let room = Room {
        air: measure(298.15),
        radiant: measure(298.15),
        humidity: measure(0.5),
        air_speed: measure(1.0),
        pressure: measure(760.0),
        oxygen: measure(0.21),
        carbon_dioxide: measure(0.0004),
        clothing: measure(0.6),
    };
    BoundaryEvent::new(id, at, BoundaryInput::Room(room))
}

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).unwrap_or_else(|_| unreachable!("a finite room"))
}

fn exits(body: &mut Assembly<'_>, leaking: bool) -> Result<(), AdmissionError> {
    let (mut laws, ports) = body.boundary()?.parts();
    let urine = ports.route::<Water>("urine")?.route;
    let rest = laws.parameter::<Water>(ParameterKey::new("bd.bladder-rest"), Admissible::nonnegative());
    let voiding =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.bladder-speed"), Admissible::positive());
    let width = laws.parameter::<Rate<Water>>(ParameterKey::new("bd.bladder-width"), Admissible::positive());
    let class = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
    let rate = laws.kernel(|mut k| {
        let (held, floor, pace) = (
            k.read(&urine.amount::<Water>()),
            k.parameter(&rest),
            k.parameter(&voiding),
        );
        let draining = k.relax(held, floor, pace);
        let (zero, soft) = (k.zero(), k.parameter(&width));
        let rate = k.smooth_max(draining, zero, soft);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&urine, rate));
    if leaking {
        leak::<Oxygen>(&mut laws, &ports, (OXYGEN_LEAK, "bd.oxygen-leak"), class)?;
        leak::<CarbonDioxide>(&mut laws, &ports, (CARBON_LEAK, "bd.carbon-dioxide-leak"), class)?;
        leak::<NitrogenGas>(&mut laws, &ports, (NITROGEN_LEAK, "bd.nitrogen-gas-leak"), class)?;
        leak::<Vasopressin>(
            &mut laws,
            &ports,
            (VASOPRESSIN_LEAK, "bd.vasopressin-leak"),
            class,
        )?;
    }
    body.add(laws.finish());
    Ok(())
}

fn leak<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    (route, key): (&'static str, &'static str),
    class: RateClass,
) -> Result<(), AdmissionError> {
    let route = ports.route::<Q>(route)?.route;
    let rate = laws.parameter::<Rate<Q>>(ParameterKey::new(key), Admissible::finite());
    let kernel = laws.kernel(|mut k| {
        let (rate, open) = (k.parameter(&rate), k.voiding());
        let value = k.scale(rate, open);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, kernel));
    Ok(())
}

/// The composition root of the whole body: the slice's systems, and every other card held.
pub fn whole_body() -> Result<CompiledBody, AdmissionError> {
    assembled(body_schema()?, false)
}

/// The whole body with four steady leaks for its certificate's tests, each into the body at the signed rate its
/// parameter states times the void gate, so a shut gate shuts them and an open one opens them: `bd.oxygen-leak` and
/// `bd.carbon-dioxide-leak` into the arteries' blood, `bd.nitrogen-gas-leak` into the pulmonary bed's gas, and
/// `bd.vasopressin-leak` into the central veins. Nothing else reads the gate.
pub fn whole_body_leaking() -> Result<CompiledBody, AdmissionError> {
    let mut schema = body_schema()?;
    schema::leaks(&mut schema)?;
    assembled(schema, true)
}

fn assembled(schema: Schema, leaking: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(schema, |mut body| {
        holds::hold_every_card(&mut body)?;
        exits(&mut body, leaking)?;
        let fragment = ng_cardiovascular::declare_in_the_whole_body(body.cardiovascular()?)?;
        body.add(fragment);
        let fragment = ng_fluids::declare(body.fluids()?)?;
        body.add(fragment);
        let fragment = ng_renal::declare(body.renal()?)?;
        body.add(fragment);
        let fragment = ng_nervous::declare_in_the_whole_body(body.nervous()?)?;
        body.add(fragment);
        let fragment = ng_receptor::declare(body.receptor()?)?;
        body.add(fragment);
        body.compile()
    })
}

#[cfg(test)]
mod placement {
    use super::*;

    type Moved = (&'static str, Species, Space, i64);

    /// Each column of `card`'s laws named in `laws`, with the pools it moves, sorted.
    fn placed(body: &CompiledBody, card: &str, laws: &[&str]) -> Vec<(&'static str, Vec<Moved>)> {
        let mut placed: Vec<(&str, Vec<Moved>)> = (body.columns().iter())
            .filter(|column| column.origin().owner == card && laws.contains(&column.origin().local))
            .map(|column| {
                let terms = (column.terms().iter())
                    .filter_map(|&(row, sign)| match body.rows()[row.index()].kind() {
                        RowKind::Amount { site, space, species } => Some((site.key(), species, space, sign)),
                        RowKind::Memory(_) | RowKind::Algebraic(_) => None,
                    })
                    .collect();
                (column.origin().local, terms)
            })
            .collect();
        placed.sort();
        placed
    }

    /// Ticket 12's placement for 12e: with the contract unchanged and the respiratory card's hold on airway heat
    /// released, the whole body compiles with the airway's heat leaving the airway store from a law that reads the
    /// expired water's pools and the room's inputs, and the card keeps a declaration that holds nothing, for the
    /// pleural fluid.
    #[test]
    fn the_airway_heat_leaves_the_airway_store_beside_the_water_it_follows() -> Result<(), AdmissionError> {
        let body = whole_body()?;
        let thermal = ("conducting airway", Species::Energy, Space::Thermal, -1);
        let gas = ("conducting airway", Species::Water, Space::Gas, -1);
        assert_eq!(
            placed(&body, Respiratory::CARD, &["airway heat", "expired water"]),
            [("airway heat", vec![thermal]), ("expired water", vec![gas])]
        );
        let declarations = body.conserved_basis().coverage().declarations();
        let respiratory = (declarations.iter())
            .find(|card| card.card() == Respiratory::CARD)
            .expect("the respiratory card's declaration for the pleural fluid");
        assert!(respiratory.routes().is_empty() && respiratory.species().is_empty());
        assert!(respiratory.spaces().is_empty());
        Ok(())
    }

    /// Ticket 12's placement: with the contract unchanged, the whole body compiles with conduction from the core
    /// to the skin and airway stores, and the skin's losses out of the skin store and the skin bed.
    #[test]
    fn each_heat_law_moves_the_stores_ticket_12_places_it_on() -> Result<(), AdmissionError> {
        let body = whole_body()?;
        let laws = [
            "conduction",
            "dry heat loss",
            "skin evaporation",
            "skin evaporative heat",
        ];
        let thermal = |site, sign| (site, Species::Energy, Space::Thermal, sign);
        assert_eq!(
            placed(&body, Metabolism::CARD, &laws),
            [
                (
                    "conduction",
                    vec![thermal("body", -1), thermal("conducting airway", 1)]
                ),
                (
                    "conduction",
                    vec![thermal("body", -1), thermal("skin heat site", 1)]
                ),
                ("dry heat loss", vec![thermal("skin heat site", -1)]),
                (
                    "skin evaporation",
                    vec![("skin bed", Species::Water, Space::Interstitium, -1)]
                ),
                ("skin evaporative heat", vec![thermal("skin heat site", -1)]),
            ]
        );
        Ok(())
    }
}
