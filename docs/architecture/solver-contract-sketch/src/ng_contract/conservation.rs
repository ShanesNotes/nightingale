//! The conserved-quantity basis. The compiler derives it from the schema's incidence and balanced
//! reactions: every combination of pool amounts that no internal transfer or reaction changes.
//! Only exterior routes change a combination, so its inventory change must equal net boundary
//! exchange. Combinations, not free species: total norepinephrine is vesicular + junctional free
//! + receptor-bound + metabolite pools; the choline moiety is acetylcholine (free and bound) +
//! choline. A free-species total need not close; a moiety total must.
use super::Digest;

pub struct ConservedBasis {
    _layout: Digest,
    _combinations: Vec<Combination>,
    _coverage: CoverageManifest,
}
impl ConservedBasis {
    pub fn digest(&self) -> Digest {
        todo!()
    }
    pub fn combinations(&self) -> &[Combination] {
        todo!()
    }
    pub fn coverage(&self) -> &CoverageManifest {
        todo!()
    }
}
/// Issued by a basis; certificate policies and history views check the basis digest they carry.
#[derive(Clone, Copy)]
pub struct QuantityId {
    _index: u32,
}
impl QuantityId {
    pub fn index(self) -> usize {
        todo!()
    }
    pub fn book(self) -> Book {
        todo!()
    }
}
pub struct Combination {
    _id: QuantityId,
    _book: Book,
    _terms: Vec<Term>,
}
impl Combination {
    pub fn id(&self) -> QuantityId {
        todo!()
    }
    pub fn book(&self) -> &Book {
        todo!()
    }
    pub fn terms(&self) -> &[Term] {
        todo!()
    }
}
/// One pool's coefficient in a combination (moiety count, element count, charge or joules).
pub struct Term {
    _pool: u32,
    _coefficient: f64,
}
pub enum Book {
    Water,
    Element(Element),
    Charge,
    /// Chemical, thermal, elastic and exterior-work accounts paired through work and heat routes.
    Energy,
    Moiety(Moiety),
    /// Albumin and fixed carrier mass: transported, not produced, inside the slice.
    Carrier,
}
pub enum Element {
    Sodium,
    Potassium,
    Chloride,
    Calcium,
    Magnesium,
    Phosphorus,
    Nitrogen,
}
pub enum Moiety {
    Catecholamine,
    Choline,
    /// Glucose: meal cargo and renal filtration and reclamation inside the slice; hepatic release
    /// and tissue uptake route to held receipts until those cards are active.
    Glucose,
    /// A labeled endocrine stand-in ligand (AVP, angiotensin II, aldosterone, ANP surrogates).
    HeldLigand,
}
/// Which combinations close inside the slice and which route to a held stand-in's receipt pool.
/// A held route is declared coverage, not proof of the outside system's physiology.
pub struct CoverageManifest {
    _internal: Vec<QuantityId>,
    _held: Vec<(QuantityId, &'static str)>,
    /// Digests of every `HeldDeclaration`, also part of the model hash.
    _declarations: Vec<Digest>,
    _routes: Vec<HeldRoute>,
    _external_physiology: Vec<&'static str>,
}
impl CoverageManifest {
    /// Every route on which a held stand-in supplies or removes material or energy (`HeldSupply`),
    /// in declaration order.
    pub fn held_routes(&self) -> &[HeldRoute] {
        todo!()
    }
}
/// One held route and the combination it moves. The history keeps its gross exchange apart from
/// that combination's totals, so support from a stand-in stays visible beside ordinary intake.
pub struct HeldRoute {
    _id: HeldRouteId,
    _quantity: QuantityId,
    _name: &'static str,
}
impl HeldRoute {
    pub fn id(&self) -> HeldRouteId {
        todo!()
    }
    pub fn quantity(&self) -> QuantityId {
        todo!()
    }
    pub fn name(&self) -> &'static str {
        todo!()
    }
}
/// Issued by a basis's coverage manifest, as `QuantityId` is by the basis.
#[derive(Clone, Copy)]
pub struct HeldRouteId {
    _index: u32,
}
