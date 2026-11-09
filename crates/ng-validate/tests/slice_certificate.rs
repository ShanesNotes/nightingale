mod certificate_oracle;

use certificate_oracle::{Ledger, certified};
use core::cell::Cell;
use core::num::NonZeroU32;
use ng_contract::*;
use ng_record::ConservationView;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const FIXED: &str = "arena fixture: synthetic, with no feedback from the slice";

struct Ticking {
    reads: Cell<u64>,
}
impl WallClock for Ticking {
    fn elapsed_ns(&self) -> Option<u64> {
        let at = self.reads.get();
        self.reads.set(at + 1);
        Some(at * SECOND)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

fn number(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn seconds(window: Interval) -> f64 {
    window.length_ns() as f64 / SECOND as f64
}

fn two_seconds(certify: impl FnOnce(&CompiledBody, &ConservationView<'_>)) {
    two_seconds_with(Vec::new(), certify);
}

fn two_seconds_with(events: Vec<BoundaryEvent>, certify: impl FnOnce(&CompiledBody, &ConservationView<'_>)) {
    let body = arena_body().expect("compiled");
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let core = ReferenceIntegrator::new(5 * MILLISECOND)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"));
    let store = MemoryStore::default();
    let end = SimTime::from_ns(anchor_moment().nanoseconds() + 2 * SECOND);
    let spec = ArenaSpec {
        window: Interval::new(anchor_moment(), end).expect("an interval"),
        packet: arena_packet(&body).expect("a packet"),
        lens: arena_lens(&body).expect("a lens"),
        boundary: BoundaryPort::new(events).expect("parsed"),
        seed: Seed::new([1, 2, 3, 4]),
        repeats: NonZeroU32::MIN,
    };
    let clock = Ticking { reads: Cell::new(0) };
    let Ok(arena) = run(&body, &core, spec, &clock, &store) else {
        panic!("the slice ran two seconds");
    };
    certify(&body, &arena.session().conservation());
}

#[test]
fn a_short_slice_window_passes_closure_and_drift_and_states_its_coverage_and_held_support() {
    two_seconds(|body, view| {
        let ledger = Ledger::sliding(view.basis(), SECOND, 2 * SECOND, 0.0, 1e-12, 1e4);
        let certificate = certified(view, &ledger).expect("the slice passes closure and drift");
        assert_eq!(certificate.span().start(), anchor_moment());
        assert!(certificate.span().length_ns() >= 2 * SECOND);
        assert!(certificate.pieces() > 0);
        assert_eq!(certificate.results().len(), view.basis().combinations().len());
        for result in certificate.results() {
            assert_eq!(seconds(result.closure_window()), 1.0);
            assert_eq!(seconds(result.drift_window()), 1.0);
        }

        let coverage = certificate.coverage();
        let declared: Vec<_> = (coverage.declarations().iter())
            .map(|card| {
                (
                    card.card(),
                    card.routes().to_vec(),
                    card.spaces().to_vec(),
                    card.validity(),
                )
            })
            .collect();
        let routes = |names: &[&'static str]| names.to_vec();
        assert_eq!(
            declared,
            [
                ("BC", routes(&[]), vec![Space::Cells], FIXED),
                (
                    "EN",
                    routes(&["vasopressin clearance", "vasopressin secretion"]),
                    vec![],
                    FIXED
                ),
                (
                    "GI",
                    routes(&["gut chloride", "gut potassium", "gut sodium", "gut water"]),
                    vec![],
                    FIXED
                ),
                ("HC", routes(&[]), vec![], FIXED),
                ("HP", routes(&[]), vec![], FIXED),
                ("IM", routes(&[]), vec![], FIXED),
                ("MS", routes(&["skin water"]), vec![], FIXED),
                ("MT", routes(&[]), vec![], FIXED),
                ("RS", routes(&[]), vec![], FIXED),
            ]
        );

        let support: Vec<_> = (certificate.support().iter())
            .map(|support| {
                let route = support.route();
                let (inflow, outflow) = (number(support.most_inflow()), number(support.most_outflow()));
                assert_eq!(seconds(support.inflow_window()), 1.0);
                assert_eq!(seconds(support.outflow_window()), 1.0);
                let book = route.quantity().book();
                (route.card(), route.name(), book, route.forward(), inflow, outflow)
            })
            .collect();
        let names: Vec<_> = support.iter().map(|&(card, name, ..)| (card, name)).collect();
        assert_eq!(
            names,
            [
                ("EN", "vasopressin clearance"),
                ("EN", "vasopressin secretion"),
                ("GI", "gut chloride"),
                ("GI", "gut potassium"),
                ("GI", "gut sodium"),
                ("GI", "gut water"),
                ("MS", "skin water"),
            ]
        );
        use BoundaryDirection::{IntoBody, OutOfBody};
        let near = |seen: f64, rate: f64| (seen - rate).abs() <= 1e-12 * rate;
        for (card, name, book, forward, inflow, outflow) in support {
            let basal = match name {
                "gut chloride" => Some((Species::Chloride, 1.05)),
                "gut potassium" => Some((Species::Potassium, 0.05)),
                "gut sodium" => Some((Species::Sodium, 1.0)),
                "gut water" => Some((Species::Water, 0.03)),
                _ => None,
            };
            match (card, name, basal) {
                ("GI", _, Some((species, rate))) => {
                    assert_eq!((book, forward), (Book::Of(species), IntoBody), "{name}");
                    assert!(near(inflow, rate) && outflow == 0.0, "{name} {inflow} {outflow}");
                }
                ("MS", "skin water", None) => {
                    assert_eq!((book, forward), (Book::Of(Species::Water), OutOfBody));
                    assert!(inflow == 0.0 && (0.005..=0.015).contains(&outflow), "{outflow}");
                }
                ("EN", "vasopressin secretion", None) => {
                    assert_eq!((book, forward), (Book::Of(Species::Vasopressin), IntoBody));
                    assert!(inflow > 0.0 && outflow == 0.0, "{inflow} {outflow}");
                }
                ("EN", "vasopressin clearance", None) => {
                    assert_eq!((book, forward), (Book::Of(Species::Vasopressin), OutOfBody));
                    assert!(inflow == 0.0 && outflow > 0.0, "{inflow} {outflow}");
                }
                _ => panic!("an undeclared route {card} {name}"),
            }
        }

        let mut pools: Vec<_> = (coverage.pools().iter())
            .map(|pool| {
                let combination = &view.basis().combinations()[pool.quantity().index()];
                assert_eq!(
                    combination.terms(),
                    [(pool.row(), 1)],
                    "a held pool is its own combination"
                );
                let RowKind::Amount { site, space, species } = body.rows()[pool.row().index()].kind() else {
                    panic!("a held pool is an amount");
                };
                (pool.card(), site.key(), space, species)
            })
            .collect();
        pools.sort_unstable();
        assert_eq!(
            pools,
            [
                ("BC", "glomerular bed", Space::Cells, Species::Water),
                ("BC", "leg bed", Space::Cells, Species::Water),
                ("BC", "peritubular bed", Space::Cells, Species::Water),
                ("BC", "pulmonary bed", Space::Cells, Species::Water),
                ("BC", "splanchnic bed", Space::Cells, Species::Water),
                ("GI", "abdominal space", Space::Fluid, Species::Water),
                ("RS", "pleural space", Space::Fluid, Species::Water),
            ]
        );
    });
}

#[test]
fn the_slice_fails_closure_with_no_allowance_and_drift_under_a_tighter_bound() {
    two_seconds(|_, view| {
        let exact = Ledger::sliding(view.basis(), SECOND, 2 * SECOND, 0.0, 0.0, 1e4);
        let failure = certified(view, &exact).expect_err("compensated sums still round");
        assert_eq!(failure.check(), CertificateCheck::Closure);
        assert!(number(failure.residual()) > 0.0);

        let tight = Ledger::sliding(view.basis(), SECOND, 2 * SECOND, 0.0, 1e-12, 1e-3);
        let failure = certified(view, &tight).expect_err("water moves more than 1e-3 in a second");
        assert_eq!(failure.check(), CertificateCheck::Drift);
        assert_eq!(failure.window().map(seconds), Some(1.0));
        assert!(number(failure.residual()) > 1e-3);
    });
}

#[test]
fn a_slice_that_closes_its_ledger_but_gains_charge_is_not_certified() {
    let after = |ms: u64| SimTime::from_ns(anchor_moment().nanoseconds() + ms * MILLISECOND);
    let sodium = BoundaryInput::Sodium(Delivered {
        boundary: "gut sodium",
        amount: Measure::new(0.5).expect("finite"),
        ends: after(900),
    });
    let id = InputId::new(Digest::of_canonical(b"synthetic sodium alone"));
    two_seconds_with(vec![BoundaryEvent::new(id, after(400), sodium)], |_, view| {
        let ledger = Ledger::sliding(view.basis(), SECOND, 2 * SECOND, 0.0, 1e-12, 1e4);
        let failure = certified(view, &ledger).expect_err("charge holds to the closure floor");
        assert_eq!(failure.check(), CertificateCheck::Drift);
        assert_eq!(failure.quantity().map(QuantityId::book), Some(Book::Charge));
        assert!(number(failure.residual()) > 0.0);
    });
}

#[test]
fn a_slice_run_one_window_long_passes_on_that_window_with_its_held_support() {
    two_seconds(|_, view| {
        let ledger = Ledger::sliding(view.basis(), 2 * SECOND, 2 * SECOND, 0.0, 1e-12, 1e4);
        let certificate = certified(view, &ledger).expect("the slice passes closure and drift");
        assert_eq!(certificate.span().start(), anchor_moment());
        for result in certificate.results() {
            assert_eq!(seconds(result.closure_window()), 2.0);
            assert_eq!(seconds(result.drift_window()), 2.0);
        }
        let held = view.basis().coverage().held_routes();
        let supported: Vec<HeldRoute> = certificate.support().iter().map(HeldSupport::route).collect();
        assert_eq!(supported, held);
        for support in certificate.support() {
            assert_eq!(seconds(support.inflow_window()), 2.0);
            assert_eq!(seconds(support.outflow_window()), 2.0);
        }
    });
}
