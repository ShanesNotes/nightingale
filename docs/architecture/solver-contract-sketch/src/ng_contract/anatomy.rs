//! Anatomy as data: sites, each of one site class, and the relations over them. Containment
//! names organs and territories, flow networks join sites (series beds and cycles included),
//! enclosures say which space surrounds which beds, served-by edges say which nerve segment a
//! site reports to, and every site has a position in the body frame. A finer map is a data
//! change: the laws are declared once per site class and the compiler instantiates them.
use super::{AcceptedStateView, AdmissionError, Compensated, CompiledBody, Derived, Digest, Length, Measure,
    ParameterKey, QuantityId};
use core::marker::PhantomData;

mod sealed {
    pub trait SiteClass {}
}
/// A kind of site a law can be declared for once. Closed, like the dimensions: a new class is a
/// contract version, and every site belongs to exactly one class.
pub trait SiteClass: sealed::SiteClass {
    const NAME: &'static str;
}
macro_rules! site_classes {
    ($($(#[$doc:meta])* $name:ident = $text:literal;)*) => {$(
        $(#[$doc])*
        pub enum $name {}
        impl sealed::SiteClass for $name {}
        impl SiteClass for $name {
            const NAME: &'static str = $text;
        }
    )*};
}
site_classes! {
    /// A capillary bed with its plasma, interstitium and cells: the Starling and exchange site.
    CapillaryBed = "capillary bed";
    /// A conducting vessel segment between two junctions of a flow network.
    VesselSegment = "vessel segment";
    /// A neuroeffector junction: where a transmitter is released and cleared.
    Junction = "neuroeffector junction";
    /// A space that encloses beds (pleural, pericardial, peritoneal, intracranial, a limb
    /// compartment); its pressure follows from the volumes inside it and its compliance law.
    EnclosedSpace = "enclosed space";
    /// A tissue site that reports afferent drive (stretch, nociception) to a nerve segment.
    TissueSite = "tissue site";
    /// A spinal or cranial nerve segment where afferents from many sites converge.
    NerveSegment = "nerve segment";
}

/// A site of class `C` in one anatomy. Issued by `AnatomyBuilder::site`; only relations read it.
pub struct Site<C> {
    _index: u32,
    _class: PhantomData<C>,
}
/// A class-erased site reference, for relations that join sites of different classes.
#[derive(Clone, Copy)]
pub struct SiteId {
    _index: u32,
}
impl<C: SiteClass> Site<C> {
    pub fn id(&self) -> SiteId {
        todo!()
    }
}
/// Height above the body-frame origin along the long axis, with the lateral and sagittal offsets.
/// Posture rotates the frame; the compiler derives every site's hydrostatic head from it in one
/// place, so no law and no wire carries gravity.
pub struct Position {
    _axial: Measure<Length>,
    _lateral: Measure<Length>,
    _sagittal: Measure<Length>,
}
impl Position {
    pub fn new(_axial: Measure<Length>, _lateral: Measure<Length>, _sagittal: Measure<Length>) -> Self {
        todo!()
    }
}
/// Which flow network an edge belongs to. Each is its own graph over the one site set.
pub enum Network {
    Vascular,
    Lymph,
    Airway,
    Duct,
    CerebrospinalFluid,
}
/// How an enclosed space's pressure answers the volume inside it. Each space has exactly one.
pub enum Compliance {
    Rigid,
    /// Elastance and unstressed volume, bound by key from the packet.
    Elastic { elastance: ParameterKey, unstressed: ParameterKey },
    /// Prescribed by the named held card (the arena's chest and belly). Compilation links it to
    /// that card's `HeldDeclaration` and coverage entry, with exactly one pressure source.
    Held { card: &'static str },
}
/// The afferent kind a served-by edge carries. `Nociceptive` names the per-site nociceptive drive
/// (from injury, ischemia, inflammation and distension); its law arrives with the nervous system's
/// deepening, not in the arena slice.
pub enum Afferent {
    Stretch,
    Nociceptive,
    Chemical,
    Thermal,
}
/// How a coarse law stands in for detail. A `Reduced` law names the region whose subtree it
/// replaces, so a review or a debrief can see what is coarse.
pub enum Representation {
    Mechanistic,
    Reduced { region: RegionId },
}
/// A named containment node: an organ, a territory or the whole body. Its members are derived
/// from the containment tree, and each book's aggregate over it is an exact sum over its sites.
#[derive(Clone, Copy)]
pub struct RegionId {
    _index: u32,
}

pub struct AnatomyBuilder {
    _sites: Vec<(&'static str, &'static str, Position)>,
    _contains: Vec<(u32, u32)>,
    _edges: Vec<(Network, u32, u32)>,
    _enclosures: Vec<(u32, Vec<u32>, Compliance)>,
    _served_by: Vec<(u32, u32, Afferent)>,
    _regions: Vec<(&'static str, u32)>,
}
impl AnatomyBuilder {
    pub fn new() -> Self {
        todo!()
    }
    /// A site under a stable key. Every site has a position; parameters bind per site by key.
    pub fn site<C: SiteClass>(&mut self, _key: &'static str, _position: Position) -> Site<C> {
        todo!()
    }
    /// `child` lies inside `parent`. Containment is a tree, separate from every flow network.
    pub fn contain(&mut self, _parent: SiteId, _child: SiteId) {
        todo!()
    }
    /// A directed edge of one network. Series beds (portal, glomerular then peritubular) and
    /// cycles (the circle of Willis) are legal; reachability is checked at `finish`.
    pub fn connect(&mut self, _network: Network, _from: SiteId, _to: SiteId) {
        todo!()
    }
    /// `space` surrounds `beds`; every bed it encloses reads its pressure as extravascular pressure.
    pub fn enclose(&mut self, _space: &Site<EnclosedSpace>, _beds: &[SiteId], _compliance: Compliance) {
        todo!()
    }
    /// `site` reports `afferent` drive to `segment`. Referral follows from convergence there. The
    /// edge carries block and lesion parameters keyed by its two sites, so a nerve block or a
    /// neuropathy is packet data on the edge.
    pub fn served_by(&mut self, _site: SiteId, _segment: &Site<NerveSegment>, _afferent: Afferent) {
        todo!()
    }
    /// Names a containment node (an organ, a territory) as a region.
    pub fn region(&mut self, _key: &'static str, _root: SiteId) -> RegionId {
        todo!()
    }
    /// Declares that this anatomy refines a coarser one (see `RefinementRule`).
    pub fn refines(&mut self, _rule: RefinementRule) {
        todo!()
    }
    /// Checks the graph: every bed on the circulation, one enclosure chain per enclosed site, a
    /// position for every site, containment a tree, no orphan site.
    pub fn finish(self) -> Result<Anatomy, AdmissionError> {
        todo!()
    }
}
/// The checked anatomy. It enters the model hash; a refined anatomy is a new model.
pub struct Anatomy {
    _builder: AnatomyBuilder,
    _digest: [u8; 32],
}

/// A region's exact totals: each book summed over the region's sites from the registry, in
/// compensated arithmetic. Derived, never new state; zooming out and the refinement check read it.
pub struct AggregationMap {
    _region: RegionId,
    _terms: Vec<(QuantityId, u32)>,
}
impl CompiledBody {
    pub fn aggregation(&self, _region: RegionId) -> Result<&AggregationMap, AdmissionError> {
        todo!()
    }
}
impl AggregationMap {
    pub fn inventory(&self, _state: AcceptedStateView<'_>, _book: QuantityId) -> Compensated {
        todo!()
    }
}
/// What replacing a coarse region by a finer subtree must show. A request for evidence, never a
/// certificate that the checks ran: equal port sets, the same books over the aggregates, port
/// behavior inside the envelope on manufactured histories, loop margins kept, sensors remapped,
/// only the anatomy and packet changed, and the whole-body regression passed.
pub struct RefinementRule {
    _coarse: RegionId,
    _replacement: RegionId,
    _splits: Vec<SplitRule>,
    _sensors: Vec<(&'static str, SiteId)>,
    _equivalence: PortEquivalence,
}
impl RefinementRule {
    pub fn new(
        _coarse: RegionId,
        _replacement: RegionId,
        _splits: Vec<SplitRule>,
        _sensors: Vec<(&'static str, SiteId)>,
        _equivalence: PortEquivalence,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// How a coarse parameter divides among its children, so a refined body with no new data starts
/// equivalent to the coarse one. Each child carries its own weight; weights are checked to close.
pub struct SplitRule {
    _kind: SplitKind,
    _parent: ParameterKey,
    _children: Vec<SplitChild>,
}
pub enum SplitKind {
    ParallelConductance,
    SeriesResistance,
    AdditiveCompliance,
}
pub struct SplitChild {
    _key: ParameterKey,
    _weight: f64,
}
impl SplitChild {
    pub fn new(_key: ParameterKey, _weight: f64) -> Self {
        todo!()
    }
}
impl SplitRule {
    pub fn new(_kind: SplitKind, _parent: ParameterKey, _children: Vec<SplitChild>) -> Result<Self, AdmissionError> {
        todo!()
    }
    /// A child's value as a `Derived` identity of the parent, for the refined packet.
    pub fn derive<U>(&self, _child: &ParameterKey) -> Result<Derived<U>, AdmissionError> {
        todo!()
    }
}
/// The ports, envelope, manufactured histories, tolerance and loop margin a refinement is judged
/// on, and the regression it must pass.
pub struct PortEquivalence {
    _ports: Vec<&'static str>,
    _envelope: Digest,
    _histories: Vec<Digest>,
    _tolerance: f64,
    _minimum_loop_margin: f64,
    _regression: Digest,
}
impl PortEquivalence {
    pub fn new(
        _ports: Vec<&'static str>,
        _envelope: Digest,
        _histories: Vec<Digest>,
        _tolerance: f64,
        _minimum_loop_margin: f64,
        _regression: Digest,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
