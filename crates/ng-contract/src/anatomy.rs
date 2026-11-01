use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::{AdmissionError, Digest, Length, Measure, ParameterKey};
use core::cmp::Reverse;
use core::iter::successors;
use core::marker::PhantomData;
use core::ops::Range;

macro_rules! site_classes {
    ($($name:ident),*) => {
        mod sealed {
            pub trait SiteClass {
                const CLASS: Class;
            }
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
            pub enum Class {
                $($name),*
            }
        }
        $(
            pub enum $name {}
            impl sealed::SiteClass for $name {
                const CLASS: sealed::Class = sealed::Class::$name;
            }
            impl SiteClass for $name {}
        )*
    };
}

pub trait SiteClass: sealed::SiteClass {}

site_classes!(
    CapillaryBed,
    VesselSegment,
    Junction,
    EnclosedSpace,
    TissueSite,
    NerveSegment
);

pub use sealed::Class;

pub struct Site<C> {
    key: &'static str,
    class: PhantomData<C>,
}
impl<C: SiteClass> Site<C> {
    pub fn id(&self) -> SiteId {
        SiteId {
            key: self.key,
            class: C::CLASS,
        }
    }
}

/// A site named by its key and class. Every anatomy resolves it by key at `finish`, so an id from
/// another builder names the site declared here under the same key and class, or no site at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SiteId {
    key: &'static str,
    class: Class,
}
impl SiteId {
    pub(crate) fn class(&self) -> Class {
        self.class
    }
}
impl Seal for SiteId {
    fn seal(&self, out: &mut Canonical) {
        let Self { key, class } = self;
        out.text(key);
        out.number(*class as u64);
    }
}

pub struct Position {
    axial: Measure<Length>,
    lateral: Measure<Length>,
    sagittal: Measure<Length>,
}
impl Position {
    pub fn new(axial: Measure<Length>, lateral: Measure<Length>, sagittal: Measure<Length>) -> Self {
        Self {
            axial,
            lateral,
            sagittal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Network {
    Vascular,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Compliance {
    Rigid,
    Elastic {
        elastance: ParameterKey,
        unstressed: ParameterKey,
    },
    Held {
        card: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Afferent {
    Stretch,
    Nociceptive,
    Chemical,
    Thermal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionId {
    key: &'static str,
}

/// Each names the least key that shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnatomyDefect {
    DeclaredTwice {
        site: &'static str,
    },
    /// A relation names a site this anatomy does not declare under that key and class.
    Undeclared {
        site: &'static str,
    },
    RegionTwice {
        region: &'static str,
    },
    /// A vascular edge at a site that is neither a capillary bed nor a vessel segment.
    WrongRelation {
        site: &'static str,
    },
    ContainedTwice {
        site: &'static str,
    },
    ContainmentCycle {
        site: &'static str,
    },
    /// A parentless site other than the root, which is the parentless site holding the most sites,
    /// ties to the least key.
    OrphanSite {
        site: &'static str,
    },
    /// What contains a site is in an enclosure that does not name the site, so the site would read the
    /// wrong pressure or none.
    TwoEnclosureChains {
        site: &'static str,
    },
    EnclosedTwice {
        site: &'static str,
    },
    ComplianceTwice {
        space: &'static str,
    },
    NoCompliance {
        space: &'static str,
    },
    EnclosureCycle {
        space: &'static str,
    },
    /// A bed no blood reaches from the circulation, or from any vessel segment when there is none.
    UnreachableBed {
        bed: &'static str,
    },
    /// A bed blood reaches that cannot return it to the circulation, or any such bed when there is no
    /// circulation.
    UndrainedBed {
        bed: &'static str,
    },
}

struct Declared {
    key: &'static str,
    class: Class,
    position: Position,
}

#[derive(Default)]
pub struct AnatomyBuilder {
    sites: Vec<Declared>,
    contains: Vec<(SiteId, SiteId)>,
    vascular: Vec<(SiteId, SiteId)>,
    encloses: Vec<(SiteId, SiteId)>,
    compliances: Vec<(SiteId, Compliance)>,
    served_by: Vec<(SiteId, SiteId, Afferent)>,
    regions: Vec<(&'static str, SiteId)>,
}
impl AnatomyBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn site<C: SiteClass>(&mut self, key: &'static str, position: Position) -> Site<C> {
        self.sites.push(Declared {
            key,
            class: C::CLASS,
            position,
        });
        Site {
            key,
            class: PhantomData,
        }
    }
    pub fn contain(&mut self, parent: SiteId, child: SiteId) {
        self.contains.push((parent, child));
    }
    pub fn connect(&mut self, network: Network, from: SiteId, to: SiteId) {
        match network {
            Network::Vascular => self.vascular.push((from, to)),
        }
    }
    /// A repeat with the same compliance adds to what the space encloses.
    pub fn enclose(&mut self, space: &Site<EnclosedSpace>, sites: &[SiteId], compliance: Compliance) {
        self.compliances.push((space.id(), compliance));
        self.encloses.extend(sites.iter().map(|&site| (space.id(), site)));
    }
    pub fn served_by(&mut self, site: SiteId, segment: &Site<NerveSegment>, afferent: Afferent) {
        self.served_by.push((site, segment.id(), afferent));
    }
    pub fn region(&mut self, key: &'static str, root: SiteId) -> RegionId {
        self.regions.push((key, root));
        RegionId { key }
    }
    /// Runs every check in [`AnatomyDefect`]'s order and refuses at the first that fails. An
    /// identical repeat of a relation is one fact, so neither repeats nor declaration order change
    /// the result.
    pub fn finish(self) -> Result<Anatomy, AdmissionError> {
        let facts = Facts::resolve(self)?;
        facts.check_relations()?;
        let tree = Tree::of(&facts)?;
        let enclosure = Enclosure::of(&facts, &tree)?;
        facts.check_circulation()?;
        Ok(Anatomy::lay_out(facts, tree, enclosure))
    }
}

struct Facts {
    sites: Vec<Declared>,
    contains: Vec<(usize, usize)>,
    vascular: Vec<(usize, usize)>,
    encloses: Vec<(usize, usize)>,
    compliances: Vec<(usize, Compliance)>,
    served_by: Vec<(usize, usize, Afferent)>,
    regions: Vec<(&'static str, usize)>,
}
impl Facts {
    fn resolve(builder: AnatomyBuilder) -> Result<Self, AnatomyDefect> {
        let mut sites = builder.sites;
        sites.sort_by_key(|site| site.key);
        if let Some(pair) = sites.windows(2).find(|pair| pair[0].key == pair[1].key) {
            return Err(AnatomyDefect::DeclaredTwice { site: pair[0].key });
        }
        let place = |id: SiteId| match sites.binary_search_by_key(&id.key, |site| site.key) {
            Ok(index) if sites[index].class == id.class => Ok(index),
            _ => Err(AnatomyDefect::Undeclared { site: id.key }),
        };
        let named = (builder
            .contains
            .iter()
            .chain(&builder.vascular)
            .chain(&builder.encloses))
        .flat_map(|&(first, second)| [first, second])
        .chain(builder.compliances.iter().map(|&(space, _)| space))
        .chain(
            builder
                .served_by
                .iter()
                .flat_map(|&(site, segment, _)| [site, segment]),
        )
        .chain(builder.regions.iter().map(|&(_, root)| root));
        if let Some(site) = named.filter(|&id| place(id).is_err()).map(|id| id.key).min() {
            return Err(AnatomyDefect::Undeclared { site });
        }
        let pair = |(first, second): (SiteId, SiteId)| Ok((place(first)?, place(second)?));
        let regions = relation(builder.regions, |(key, root)| Ok((key, place(root)?)))?;
        if let Some(pair) = regions.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(AnatomyDefect::RegionTwice { region: pair[0].0 });
        }
        Ok(Self {
            contains: relation(builder.contains, pair)?,
            vascular: relation(builder.vascular, pair)?,
            encloses: relation(builder.encloses, pair)?,
            compliances: relation(builder.compliances, |(space, how)| Ok((place(space)?, how)))?,
            served_by: relation(builder.served_by, |(site, segment, kind)| {
                Ok((place(site)?, place(segment)?, kind))
            })?,
            regions,
            sites,
        })
    }

    fn key(&self, site: usize) -> &'static str {
        self.sites[site].key
    }

    fn class(&self, site: usize) -> Class {
        self.sites[site].class
    }

    fn check_relations(&self) -> Result<(), AnatomyDefect> {
        let carries_blood = |site| matches!(self.class(site), Class::CapillaryBed | Class::VesselSegment);
        let ends = self.vascular.iter().flat_map(|&(from, to)| [from, to]);
        match ends.filter(|&site| !carries_blood(site)).min() {
            Some(site) => Err(AnatomyDefect::WrongRelation { site: self.key(site) }),
            None => Ok(()),
        }
    }

    fn check_circulation(&self) -> Result<(), AnatomyDefect> {
        let count = self.sites.len();
        let forward = Digraph::new(count, &self.vascular);
        let component = forward.strongly_connected_components();
        let mut size = vec![0; count];
        let mut has_vessel = vec![false; count];
        for (site, &part) in component.iter().enumerate() {
            size[part] += 1;
            has_vessel[part] |= self.class(site) == Class::VesselSegment;
        }
        let circulation = (0..count)
            .filter(|&site| size[component[site]] > 1 && has_vessel[component[site]])
            .max_by_key(|&site| (size[component[site]], Reverse(site)))
            .map(|site| component[site]);
        let (sources, drains): (Vec<usize>, Vec<usize>) = match circulation {
            Some(part) => {
                let members: Vec<usize> = (0..count).filter(|&site| component[site] == part).collect();
                (members.clone(), members)
            }
            None => (
                (0..count)
                    .filter(|&site| self.class(site) == Class::VesselSegment)
                    .collect(),
                Vec::new(),
            ),
        };
        let reached = forward.reachable(&sources);
        let draining = forward.reversed().reachable(&drains);
        let mut beds = (0..count).filter(|&site| self.class(site) == Class::CapillaryBed);
        if let Some(bed) = beds.clone().find(|&bed| !reached[bed]) {
            return Err(AnatomyDefect::UnreachableBed { bed: self.key(bed) });
        }
        match beds.find(|&bed| !draining[bed]) {
            Some(bed) => Err(AnatomyDefect::UndrainedBed { bed: self.key(bed) }),
            None => Ok(()),
        }
    }
}

fn relation<T, U: Ord>(
    declared: Vec<T>,
    place: impl Fn(T) -> Result<U, AnatomyDefect>,
) -> Result<Vec<U>, AnatomyDefect> {
    let mut facts = declared.into_iter().map(place).collect::<Result<Vec<U>, _>>()?;
    facts.sort();
    facts.dedup();
    Ok(facts)
}

fn least_on_cycle(next: &[Option<usize>]) -> Option<usize> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        New,
        OnWalk,
        Done,
    }
    let mut mark = vec![Mark::New; next.len()];
    let mut least: Option<usize> = None;
    for start in 0..next.len() {
        let mut walk = Vec::new();
        let mut at = Some(start);
        while let Some(site) = at {
            match mark[site] {
                Mark::New => {
                    mark[site] = Mark::OnWalk;
                    walk.push(site);
                    at = next[site];
                }
                Mark::OnWalk => {
                    if let Some(found) = walk.iter().skip_while(|&&step| step != site).min().copied() {
                        least = Some(least.map_or(found, |least| least.min(found)));
                    }
                    break;
                }
                Mark::Done => break,
            }
        }
        for site in walk {
            mark[site] = Mark::Done;
        }
    }
    least
}

fn single_valued<T: Copy + PartialEq>(
    slots: &mut [Option<T>],
    facts: impl Iterator<Item = (usize, T)>,
) -> Option<usize> {
    let mut twice: Option<usize> = None;
    for (site, value) in facts {
        match slots[site] {
            None => slots[site] = Some(value),
            Some(held) if held != value => twice = Some(twice.map_or(site, |least| least.min(site))),
            Some(_) => {}
        }
    }
    twice
}

struct Tree {
    parent: Vec<Option<usize>>,
    preorder: Vec<usize>,
    subtree: Vec<Range<usize>>,
}
impl Tree {
    fn of(facts: &Facts) -> Result<Self, AnatomyDefect> {
        let count = facts.sites.len();
        let mut parent = vec![None; count];
        if let Some(site) = single_valued(&mut parent, facts.contains.iter().map(|&(up, site)| (site, up))) {
            return Err(AnatomyDefect::ContainedTwice {
                site: facts.key(site),
            });
        }
        if let Some(site) = least_on_cycle(&parent) {
            return Err(AnatomyDefect::ContainmentCycle {
                site: facts.key(site),
            });
        }
        let mut children = vec![Vec::new(); count];
        for &(up, site) in &facts.contains {
            children[up].push(site);
        }
        let roots: Vec<usize> = (0..count).filter(|&site| parent[site].is_none()).collect();
        let mut preorder = Vec::with_capacity(count);
        let mut stack: Vec<usize> = roots.iter().rev().copied().collect();
        while let Some(site) = stack.pop() {
            preorder.push(site);
            stack.extend(children[site].iter().rev());
        }
        let mut size = vec![1; count];
        for &site in preorder.iter().rev() {
            if let Some(up) = parent[site] {
                size[up] += size[site];
            }
        }
        let root = roots
            .iter()
            .copied()
            .max_by_key(|&site| (size[site], Reverse(site)));
        if let Some(stray) = roots.iter().copied().find(|&site| Some(site) != root) {
            return Err(AnatomyDefect::OrphanSite {
                site: facts.key(stray),
            });
        }
        let mut subtree = vec![0..0; count];
        for (at, &site) in preorder.iter().enumerate() {
            subtree[site] = at..at + size[site];
        }
        Ok(Self {
            parent,
            preorder,
            subtree,
        })
    }
}

struct Enclosure {
    enclosed_by: Vec<Option<usize>>,
    compliance: Vec<Option<Compliance>>,
}
impl Enclosure {
    fn of(facts: &Facts, tree: &Tree) -> Result<Self, AnatomyDefect> {
        let count = facts.sites.len();
        let mut enclosed_by = vec![None; count];
        if let Some(site) = single_valued(
            &mut enclosed_by,
            facts.encloses.iter().map(|&(space, site)| (site, space)),
        ) {
            return Err(AnatomyDefect::EnclosedTwice {
                site: facts.key(site),
            });
        }
        let mut compliance = vec![None; count];
        if let Some(space) = single_valued(&mut compliance, facts.compliances.iter().copied()) {
            return Err(AnatomyDefect::ComplianceTwice {
                space: facts.key(space),
            });
        }
        let unset = |site: usize| facts.class(site) == Class::EnclosedSpace && compliance[site].is_none();
        if let Some(space) = (0..count).find(|&site| unset(site)) {
            return Err(AnatomyDefect::NoCompliance {
                space: facts.key(space),
            });
        }
        if let Some(space) = least_on_cycle(&enclosed_by) {
            return Err(AnatomyDefect::EnclosureCycle {
                space: facts.key(space),
            });
        }
        let agrees = |site: usize| {
            let holder = tree.parent[site].and_then(|up| {
                let inside = (facts.class(up) == Class::EnclosedSpace).then_some(up);
                inside.or(enclosed_by[up])
            });
            holder.is_none_or(|space| {
                successors(enclosed_by[site], |&at| enclosed_by[at]).any(|at| at == space)
            })
        };
        if let Some(site) = (0..count).find(|&site| !agrees(site)) {
            return Err(AnatomyDefect::TwoEnclosureChains {
                site: facts.key(site),
            });
        }
        Ok(Self {
            enclosed_by,
            compliance,
        })
    }
}

struct Placed {
    id: SiteId,
    position: Position,
    parent: Option<usize>,
    subtree: Range<usize>,
    enclosed_by: Option<usize>,
    compliance: Option<Compliance>,
}

pub struct Anatomy {
    sites: Vec<Placed>,
    preorder: Vec<SiteId>,
    vascular: Vec<(usize, usize)>,
    served_by: Vec<(usize, usize, Afferent)>,
    regions: Vec<(&'static str, usize)>,
}
impl Anatomy {
    fn lay_out(facts: Facts, tree: Tree, enclosure: Enclosure) -> Self {
        let preorder = tree.preorder.iter().map(|&site| SiteId {
            key: facts.sites[site].key,
            class: facts.sites[site].class,
        });
        let preorder = preorder.collect();
        let placed = facts
            .sites
            .into_iter()
            .enumerate()
            .map(|(site, declared)| Placed {
                id: SiteId {
                    key: declared.key,
                    class: declared.class,
                },
                position: declared.position,
                parent: tree.parent[site],
                subtree: tree.subtree[site].clone(),
                enclosed_by: enclosure.enclosed_by[site],
                compliance: enclosure.compliance[site],
            });
        Self {
            sites: placed.collect(),
            preorder,
            vascular: facts.vascular,
            served_by: facts.served_by,
            regions: facts.regions,
        }
    }

    /// SHA-256 over the anatomy's canonical bytes: the map's part of the model hash.
    pub fn digest(&self) -> Digest {
        let Self {
            sites,
            preorder: _,
            vascular,
            served_by,
            regions,
        } = self;
        let mut bytes = Canonical::new("ng-contract anatomy 1");
        let index = |bytes: &mut Canonical, site: usize| bytes.number(site as u64);
        bytes.number(sites.len() as u64);
        let (mut contains, mut encloses, mut spaces) = (Vec::new(), Vec::new(), Vec::new());
        for (at, site) in sites.iter().enumerate() {
            let Placed {
                id,
                position,
                parent,
                subtree: _,
                enclosed_by,
                compliance,
            } = site;
            bytes.text(id.key);
            bytes.number(id.class as u64);
            for measure in [&position.axial, &position.lateral, &position.sagittal] {
                bytes.number(measure.bits());
            }
            contains.extend(parent.map(|up| (at, up)));
            encloses.extend(enclosed_by.map(|space| (at, space)));
            spaces.extend(compliance.map(|how| (at, how)));
        }
        for link in [contains, encloses] {
            bytes.number(link.len() as u64);
            for (from, to) in link {
                index(&mut bytes, from);
                index(&mut bytes, to);
            }
        }
        bytes.number(spaces.len() as u64);
        for (space, how) in spaces {
            index(&mut bytes, space);
            match how {
                Compliance::Rigid => bytes.number(0),
                Compliance::Elastic {
                    elastance,
                    unstressed,
                } => {
                    bytes.number(1);
                    bytes.text(elastance.key());
                    bytes.text(unstressed.key());
                }
                Compliance::Held { card } => {
                    bytes.number(2);
                    bytes.text(card);
                }
            }
        }
        bytes.number(vascular.len() as u64);
        for &(from, to) in vascular {
            index(&mut bytes, from);
            index(&mut bytes, to);
        }
        bytes.number(served_by.len() as u64);
        for &(site, segment, kind) in served_by {
            index(&mut bytes, site);
            index(&mut bytes, segment);
            bytes.number(kind as u64);
        }
        bytes.number(regions.len() as u64);
        for &(key, root) in regions {
            bytes.text(key);
            index(&mut bytes, root);
        }
        bytes.digest()
    }

    /// The region's root and every site under it, in pre-order with children in key order; `None`
    /// when this anatomy has no region under that key.
    pub fn members(&self, region: RegionId) -> Option<&[SiteId]> {
        let at = self
            .regions
            .binary_search_by_key(&region.key, |&(key, _)| key)
            .ok()?;
        Some(&self.preorder[self.sites[self.regions[at].1].subtree.clone()])
    }

    pub(crate) fn sites(&self) -> impl Iterator<Item = SiteId> + '_ {
        self.sites.iter().map(|site| site.id)
    }

    fn placed(&self, site: SiteId) -> Option<&Placed> {
        let at = self.sites.binary_search_by_key(&site.key, |placed| placed.id.key);
        at.ok()
            .map(|at| &self.sites[at])
            .filter(|placed| placed.id == site)
    }

    pub(crate) fn encloser(&self, site: SiteId) -> Option<(SiteId, Compliance)> {
        let space = &self.sites[self.placed(site)?.enclosed_by?];
        Some((space.id, space.compliance?))
    }

    pub(crate) fn compliance(&self, space: SiteId) -> Option<Compliance> {
        self.placed(space)?.compliance
    }

    pub(crate) fn held_enclosures(&self) -> impl Iterator<Item = (SiteId, &'static str)> + '_ {
        (self.sites.iter()).filter_map(|site| match site.compliance {
            Some(Compliance::Held { card }) => Some((site.id, card)),
            _ => None,
        })
    }

    pub(crate) fn vascular(&self) -> impl Iterator<Item = (SiteId, SiteId)> + '_ {
        (self.vascular.iter()).map(|&(from, to)| (self.sites[from].id, self.sites[to].id))
    }
}
