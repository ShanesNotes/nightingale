//! The heart and vessels: one pressure law at every blood site, one flow law on every plain edge, one inertial law on
//! every valve, the rhythm, and the heart's work into the elastic store.
//!
//! # Type rules
//!
//! A neighbouring system reads what a port grants, never this crate's own forms.
//! ```compile_fail,E0603
//! use ng_cardiovascular::walls::elastance;
//! ```
//! ```
//! use ng_cardiovascular::declare;
//! ```

use ng_contract::*;

mod afferents;
mod blood_pressure;
mod capillary_fields;
mod circulation;
mod rhythm;
mod shared;
mod valves;
mod walls;

use afferents::afferents;
use capillary_fields::capillary_fields;
use circulation::plain_edges;
use valves::valves;

const WAVE_NS: u64 = 10_000_000;

/// Emits baroreceptor afferents (line 77) and capillary and venous pressure (line 80). Reads tubuloglomerular feedback
/// (line 97) and the effect of each transmitter at its junction. Each law is declared once, and the map says where.
pub fn declare<'b>(scope: CardiovascularScope<'b>) -> Result<Fragment<'b, Cardiovascular>, AdmissionError> {
    let (mut cv, ports) = scope.parts();
    let wave = RateClass::new(RateBand::Wave, WAVE_NS)?;
    let heart = rhythm::declare(&mut cv, &ports, wave)?;
    let (pressure_row, pressure) = ports.blood_pressure(&mut cv);
    let walls = walls::Walls::declare(&mut cv, &ports, &heart);
    let head = cv.parameter::<Quotient<Pressure, Length>>(
        ParameterKey::new("cv.head-per-length"),
        Admissible::positive(),
    );
    blood_pressure::declare(&mut cv, pressure_row, &walls, wave);

    let circulation = ports.circulation(&mut cv);
    plain_edges(&mut cv, &ports, &circulation, &pressure, &head, wave)?;
    valves(&mut cv, &ports, &pressure, &head, wave)?;
    afferents(&mut cv, &pressure, &head, wave)?;
    capillary_fields(&mut cv, &pressure, wave);
    walls.energy(&mut cv, &ports, &heart, &head, wave)?;
    Ok(cv.finish())
}
