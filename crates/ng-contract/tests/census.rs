use ng_contract::catalog::{Arrow, Carries, Disposition, HeldField, WiredField};

const ATRIAL: WiredField = WiredField {
    name: "atrial_wall_stress",
    unit: "Pressure",
    ticket: 17,
};

const fn held(name: &'static str, holder: &'static str) -> HeldField {
    HeldField {
        name,
        unit: "Pressure",
        holder,
        reason: "a synthetic reason",
    }
}

const fn wired(ticket: u8) -> WiredField {
    WiredField { ticket, ..ATRIAL }
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
