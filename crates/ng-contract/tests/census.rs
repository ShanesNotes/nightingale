use ng_contract::Sites;
use ng_contract::catalog::{
    ARROWS, Arrow, Carries, Census, Disposition, HeldField, HeldRead, Projection, SPECIATION_READERS,
    SpeciationReader, WiredField, WiredRead,
};

const ATRIAL: WiredField = WiredField {
    name: "atrial_wall_stress",
    unit: "Pressure",
    body_ticket: 17,
    projected: None,
};

const fn held(name: &'static str, holder: &'static str) -> HeldField {
    HeldField {
        name,
        unit: "Pressure",
        holder,
        reason: "a synthetic reason",
    }
}

const fn wired(body_ticket: u8) -> WiredField {
    WiredField {
        body_ticket,
        ..ATRIAL
    }
}

const fn wall_stress(carries: Carries) -> Arrow {
    Arrow {
        line: 79,
        from: "CV",
        to: "EN",
        sender_in_slice: true,
        receiver_in_slice: false,
        slice: Disposition::ExternalSink,
        carries,
    }
}

#[test]
fn the_census_refuses_a_second_holder_of_one_field() {
    const ONCE: Arrow = wall_stress(Carries::Fields {
        wired: &[ATRIAL],
        held: &[held("ventricular_wall_stress", "CV")],
    });
    const WIRED_AND_HELD: Arrow = wall_stress(Carries::Fields {
        wired: &[ATRIAL],
        held: &[held("atrial_wall_stress", "CV")],
    });
    const HELD_BY_BOTH_ENDS: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[
            held("ventricular_wall_stress", "CV"),
            held("ventricular_wall_stress", "EN"),
        ],
    });
    assert!(ONCE.one_holder_per_field());
    assert!(
        !WIRED_AND_HELD.one_holder_per_field(),
        "a wired field held as well has a second holder"
    );
    assert!(
        !HELD_BY_BOTH_ENDS.one_holder_per_field(),
        "a field held at both ends has a second holder"
    );
}

#[test]
fn the_census_refuses_a_holder_at_neither_end_of_the_arrow() {
    const AT_THE_SENDER: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[held("ventricular_wall_stress", "CV")],
    });
    const AT_THE_RECEIVER: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[held("ventricular_wall_stress", "EN")],
    });
    const ELSEWHERE: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[held("ventricular_wall_stress", "MT")],
    });
    assert!(AT_THE_SENDER.held_at_its_ends() && AT_THE_RECEIVER.held_at_its_ends());
    assert!(!ELSEWHERE.held_at_its_ends(), "MT is neither end of CV to EN");
}

#[test]
fn the_census_refuses_a_wired_field_no_body_ticket_wires() {
    const FIRST: Arrow = wall_stress(Carries::Fields {
        wired: &[wired(6)],
        held: &[],
    });
    const LAST: Arrow = wall_stress(Carries::Fields {
        wired: &[wired(25)],
        held: &[],
    });
    const BEFORE: Arrow = wall_stress(Carries::Fields {
        wired: &[wired(5)],
        held: &[],
    });
    const AFTER: Arrow = wall_stress(Carries::Fields {
        wired: &[wired(26)],
        held: &[],
    });
    assert!(FIRST.wired_by_body_tickets() && LAST.wired_by_body_tickets());
    assert!(!BEFORE.wired_by_body_tickets(), "ticket 05 builds no system");
    assert!(!AFTER.wired_by_body_tickets(), "ticket 26 builds no system");
}

#[test]
fn the_census_refuses_an_arrow_that_carries_nothing() {
    const RELATION: Arrow = wall_stress(Carries::Relation {
        reason: "a synthetic relation",
    });
    const ONE_FIELD: Arrow = wall_stress(Carries::Fields {
        wired: &[ATRIAL],
        held: &[],
    });
    const NOTHING: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[],
    });
    assert!(RELATION.accounted() && ONE_FIELD.accounted());
    assert!(
        !NOTHING.accounted(),
        "an arrow with no field and no relation is not accounted for"
    );
}

#[test]
fn the_census_refuses_an_empty_reason() {
    const REASONED: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[held("ventricular_wall_stress", "CV")],
    });
    const HELD_WITHOUT_ONE: Arrow = wall_stress(Carries::Fields {
        wired: &[],
        held: &[HeldField {
            reason: "",
            ..held("ventricular_wall_stress", "CV")
        }],
    });
    const RELATION_WITHOUT_ONE: Arrow = wall_stress(Carries::Relation { reason: "" });
    const SLICE_WITHOUT_ONE: Arrow = Arrow {
        slice: Disposition::OutsideSlice { reason: "" },
        ..REASONED
    };
    assert!(REASONED.every_reason_given());
    assert!(
        !HELD_WITHOUT_ONE.every_reason_given(),
        "a held field names no reason"
    );
    assert!(
        !RELATION_WITHOUT_ONE.every_reason_given(),
        "an anatomy relation names no reason"
    );
    assert!(
        !SLICE_WITHOUT_ONE.every_reason_given(),
        "a slice disposition names no reason"
    );
}

#[test]
fn the_whole_body_wires_eighteen_fields_holds_thirty_two_and_mixes_six_arrows() {
    let fields = |arrow: &Arrow| (arrow.carries.wired().len(), arrow.carries.held().len());
    let wired: usize = ARROWS.iter().map(|arrow| fields(arrow).0).sum();
    let held: usize = ARROWS.iter().map(|arrow| fields(arrow).1).sum();
    let mixed: Vec<u16> = (ARROWS.iter())
        .filter(|arrow| fields(arrow).0 > 0 && fields(arrow).1 > 0)
        .map(|arrow| arrow.line)
        .collect();
    assert_eq!((ARROWS.len(), wired, held), (38, 18, 32));
    assert_eq!(mixed, [78, 79, 87, 89, 95, 114]);
    let whole = Census::WholeBody.count();
    assert_eq!((whole.live, whole.relation, whole.held), (14, 6, 18));
    let slice = Census::Slice.count();
    assert_eq!((slice.mapped(), slice.unmapped()), (27, 11));
    for line in [90, 91] {
        let row = Census::Slice.row(&ARROWS[line - 77]);
        assert_eq!(row.disposition, Disposition::ExternalInput, "line {line}");
    }
}

#[test]
fn arrows_90_and_91_wire_their_projected_temperatures_for_ticket_12_and_keep_every_other_entry() {
    let wired = |line: usize| -> Vec<(&str, u8, Option<Projection>)> {
        (ARROWS[line - 77].carries.wired().iter())
            .map(|field| (field.name, field.body_ticket, field.projected))
            .collect()
    };
    assert_eq!(
        wired(90),
        [
            ("oxygen_debt_per_bed", 9, None),
            ("temperature", 12, Some(Projection::CarriageEnds))
        ]
    );
    assert_eq!(
        wired(91),
        [
            ("core_temperature", 12, None),
            ("skin_temperature", 12, Some(Projection::SkinThermoreceptor))
        ]
    );
    assert!(ARROWS[90 - 77].carries.held().is_empty() && ARROWS[91 - 77].carries.held().is_empty());
    let flow = ARROWS[78 - 77].carries;
    assert_eq!(wired(78), [("myocardial_work", 9, None)]);
    let held: Vec<(&str, &str, &str)> = (flow.held().iter())
        .map(|field| (field.name, field.holder, field.reason))
        .collect();
    let reason = "MT's oxygen supply arrives by plasma transport. CV reads its own inflow for carriage.";
    assert_eq!(held, [("flow_per_bed", "CV", reason)]);
    let projected = (ARROWS.iter())
        .flat_map(|arrow| arrow.carries.wired())
        .filter(|field| field.projected.is_some())
        .count();
    assert_eq!(projected, 2);
}

#[test]
fn every_arrow_of_the_catalog_keeps_the_census_rules() {
    for arrow in &ARROWS {
        let line = arrow.line;
        assert!(arrow.accounted(), "line {line} carries nothing");
        assert!(
            arrow.held_at_its_ends(),
            "line {line} has a holder at neither end"
        );
        assert!(
            arrow.wired_by_body_tickets(),
            "line {line} names no body ticket from 06 to 25"
        );
        assert!(arrow.one_holder_per_field(), "line {line} has a second holder");
        assert!(arrow.every_reason_given(), "line {line} has an empty reason");
    }
}

const fn reader(
    reader: &'static str,
    wired: &'static [WiredRead],
    held: &'static [HeldRead],
) -> SpeciationReader {
    SpeciationReader {
        reader,
        sites: Sites::Speciated,
        label: "a synthetic reader",
        wired,
        held,
    }
}
const ACIDITY: WiredRead = WiredRead {
    quantity: "acidity",
    body_ticket: 11,
};
const HELD: HeldRead = HeldRead {
    quantity: "acidity",
    reason: "a synthetic reason",
};

#[test]
fn the_readers_table_refuses_a_reader_that_reads_nothing() {
    assert!(reader("NS", &[ACIDITY], &[]).accounted());
    assert!(!reader("NS", &[], &[]).accounted());
}

#[test]
fn the_readers_table_refuses_blood_chemistry_as_a_reader() {
    assert!(reader("NS", &[ACIDITY], &[]).read_by_another_card());
    assert!(!reader("BC", &[ACIDITY], &[]).read_by_another_card());
}

#[test]
fn the_readers_table_refuses_one_quantity_twice_in_a_row() {
    assert!(reader("NS", &[ACIDITY], &[]).each_quantity_once());
    assert!(!reader("NS", &[ACIDITY], &[HELD]).each_quantity_once());
    assert!(!reader("NS", &[ACIDITY, ACIDITY], &[]).each_quantity_once());
}

#[test]
fn the_readers_table_refuses_a_wired_read_no_body_ticket_wires() {
    const EARLY: WiredRead = WiredRead {
        body_ticket: 5,
        ..ACIDITY
    };
    const LATE: WiredRead = WiredRead {
        body_ticket: 26,
        ..ACIDITY
    };
    assert!(reader("NS", &[ACIDITY], &[]).wired_by_body_tickets());
    assert!(!reader("NS", &[EARLY], &[]).wired_by_body_tickets());
    assert!(!reader("NS", &[LATE], &[]).wired_by_body_tickets());
}

#[test]
fn the_readers_table_refuses_an_empty_reason() {
    const UNREASONED: HeldRead = HeldRead { reason: "", ..HELD };
    assert!(reader("NS", &[], &[HELD]).every_reason_given());
    assert!(!reader("NS", &[], &[UNREASONED]).every_reason_given());
}

#[test]
fn the_readers_table_refuses_two_rows_for_one_reader_and_set() {
    let brain = SpeciationReader {
        sites: Sites::Tissue(ng_contract::Tissue::Brain),
        ..reader("NS", &[], &[HELD])
    };
    let once = [reader("NS", &[ACIDITY], &[]), brain, reader("RN", &[], &[HELD])];
    assert!(SpeciationReader::one_row_per_reader_and_set(&once));
    let twice = [reader("NS", &[ACIDITY], &[]), reader("NS", &[], &[HELD])];
    assert!(!SpeciationReader::one_row_per_reader_and_set(&twice));
}

#[test]
fn every_row_of_the_readers_table_keeps_its_rules() {
    assert!(SpeciationReader::one_row_per_reader_and_set(&SPECIATION_READERS));
    for row in &SPECIATION_READERS {
        let label = row.label;
        assert!(row.accounted(), "{label} reads nothing");
        assert!(row.read_by_another_card(), "{label} is blood chemistry's own");
        assert!(row.each_quantity_once(), "{label} names a quantity twice");
        assert!(
            row.wired_by_body_tickets(),
            "{label} names no body ticket from 06 to 25"
        );
        assert!(row.every_reason_given(), "{label} has an empty reason");
    }
}
