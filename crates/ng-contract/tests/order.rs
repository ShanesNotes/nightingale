use ng_contract::*;

// A linear congruential generator (Knuth's MMIX constants).
struct Draws(u64);
impl Draws {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) % bound as u64) as usize
    }
}

const KEYS: [&str; 7] = ["a", "b", "c", "d", "e", "f", "g"];

#[derive(Clone, Copy)]
struct Name {
    key: usize,
    class: usize,
}

#[derive(Clone, Copy)]
enum Declaration {
    Site(Name, usize),
    Contain(Name, Name),
    Connect(Name, Name),
    Enclose(usize, Name, bool),
    Serve(Name, usize),
    Region(usize, Name),
}

fn at(height: usize) -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(height as f64)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn declare(map: &mut AnatomyBuilder, name: Name, height: usize) -> Result<SiteId, AdmissionError> {
    let (key, place) = (KEYS[name.key], at(height)?);
    Ok(match name.class {
        0 => map.site::<CapillaryBed>(key, place).id(),
        1 => map.site::<VesselSegment>(key, place).id(),
        2 => map.site::<Junction>(key, place).id(),
        3 => map.site::<EnclosedSpace>(key, place).id(),
        4 => map.site::<TissueSite>(key, place).id(),
        _ => map.site::<NerveSegment>(key, place).id(),
    })
}

fn id(name: Name) -> Result<SiteId, AdmissionError> {
    declare(&mut AnatomyBuilder::new(), name, 0)
}

type Outcome = Result<(Digest, Vec<Option<Vec<SiteId>>>), AdmissionError>;

fn build(declarations: &[Declaration]) -> Result<Outcome, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    for &declaration in declarations {
        match declaration {
            Declaration::Site(name, height) => {
                declare(&mut map, name, height)?;
            }
            Declaration::Contain(parent, child) => map.contain(id(parent)?, id(child)?),
            Declaration::Connect(from, to) => map.connect(Network::Vascular, id(from)?, id(to)?),
            Declaration::Enclose(space, inside, rigid) => {
                let space = AnatomyBuilder::new().site::<EnclosedSpace>(KEYS[space], at(0)?);
                let held = Compliance::Held { card: "held card" };
                map.enclose(
                    &space,
                    &[id(inside)?],
                    if rigid { Compliance::Rigid } else { held },
                );
            }
            Declaration::Serve(site, segment) => {
                let segment = AnatomyBuilder::new().site::<NerveSegment>(KEYS[segment], at(0)?);
                map.served_by(id(site)?, &segment, Afferent::Stretch);
            }
            Declaration::Region(key, root) => {
                map.region(KEYS[key], id(root)?);
            }
        }
    }
    let mut namer = AnatomyBuilder::new();
    let root = namer.site::<TissueSite>("root", at(0)?).id();
    let regions: Vec<RegionId> = KEYS.iter().map(|&key| namer.region(key, root)).collect();
    Ok(map.finish().map(|anatomy| {
        let members = regions
            .iter()
            .map(|&region| anatomy.members(region).map(<[SiteId]>::to_vec));
        (anatomy.digest(), members.collect())
    }))
}

fn draw_map(draws: &mut Draws) -> Vec<Declaration> {
    let mut keys: Vec<usize> = (0..KEYS.len()).collect();
    for last in (1..keys.len()).rev() {
        keys.swap(last, draws.below(last + 1));
    }
    let mut sites = vec![
        Name {
            key: keys[0],
            class: 4,
        },
        Name {
            key: keys[1],
            class: 1,
        },
    ];
    for &key in &keys[2..2 + draws.below(KEYS.len() - 1)] {
        sites.push(Name {
            key,
            class: draws.below(6),
        });
    }
    let (root, vessel) = (sites[0], sites[1]);
    let mut declarations: Vec<Declaration> = sites
        .iter()
        .map(|&name| Declaration::Site(name, draws.below(3)))
        .collect();
    for &site in &sites[1..] {
        declarations.push(Declaration::Contain(root, site));
        match site.class {
            0 => {
                declarations.push(Declaration::Connect(vessel, site));
                declarations.push(Declaration::Connect(site, vessel));
            }
            3 => declarations.push(Declaration::Enclose(site.key, vessel, true)),
            _ => {}
        }
    }
    declarations.push(Declaration::Region(
        draws.below(KEYS.len()),
        sites[draws.below(sites.len())],
    ));
    let pick = |draws: &mut Draws| match draws.below(10) {
        0 => Name {
            key: draws.below(KEYS.len()),
            class: draws.below(6),
        },
        _ => sites[draws.below(sites.len())],
    };
    for _ in 0..draws.below(4) {
        declarations.push(match draws.below(6) {
            0 => Declaration::Contain(pick(draws), pick(draws)),
            1 | 2 => Declaration::Connect(pick(draws), pick(draws)),
            3 => Declaration::Enclose(draws.below(KEYS.len()), pick(draws), draws.below(4) > 0),
            4 => Declaration::Serve(pick(draws), draws.below(KEYS.len())),
            _ => Declaration::Region(draws.below(KEYS.len()), pick(draws)),
        });
    }
    declarations
}

fn reordered(declarations: &[Declaration], draws: &mut Draws) -> Vec<Declaration> {
    let mut copy = declarations.to_vec();
    for last in (1..copy.len()).rev() {
        copy.swap(last, draws.below(last + 1));
    }
    let repeats: Vec<Declaration> = (copy.iter())
        .filter(|declaration| !matches!(declaration, Declaration::Site(..)))
        .filter(|_| draws.below(3) == 0)
        .copied()
        .collect();
    copy.extend(repeats);
    copy
}

#[test]
fn finish_gives_one_outcome_whatever_the_declaration_order() -> Result<(), AdmissionError> {
    let mut draws = Draws(2);
    let (mut admitted, mut refused) = (0, 0);
    for _ in 0..4000 {
        let declarations = draw_map(&mut draws);
        let outcome = build(&declarations)?;
        assert_eq!(build(&reordered(&declarations, &mut draws))?, outcome);
        match outcome {
            Ok(_) => admitted += 1,
            Err(_) => refused += 1,
        }
    }
    assert!(
        admitted > 1000 && refused > 1000,
        "{admitted} admitted, {refused} refused"
    );
    Ok(())
}
