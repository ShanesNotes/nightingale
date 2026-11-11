use super::process_route;
use crate::law::{Brand, Family};
use crate::{
    Absorbed, AbsorbingEdge, AdmissionError, BoundaryProcess, BoundaryRoute, EdgeRoute, FragmentBuilder,
    Gastrointestinal, GutEdge, Ingested, Iron, Passage, Species, Triglyceride,
};
use core::marker::PhantomData;

pub struct GastrointestinalPorts<'b> {
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> GastrointestinalPorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            processes,
            brand: PhantomData,
        }
    }
    /// Lumen to lumen along every gut edge between two lumens: gastric emptying.
    pub fn transit<Q: Ingested>(
        &self,
        laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ) -> EdgeRoute<'b, Gastrointestinal, Q, GutEdge> {
        let family = Family::Along(Passage::Transit, Q::SPECIES);
        laws.request(family);
        EdgeRoute::requested(family)
    }
    /// A gut lumen to its bed's plasma along every absorbing edge.
    /// ```compile_fail,E0277
    /// use ng_contract::*;
    /// fn absorbed<'b>(gi: &mut FragmentBuilder<'b, Gastrointestinal>, ports: &GastrointestinalPorts<'b>)
    ///     -> EdgeRoute<'b, Gastrointestinal, Triglyceride, AbsorbingEdge> {
    ///     ports.absorption::<Triglyceride>(gi)
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn absorbed<'b>(gi: &mut FragmentBuilder<'b, Gastrointestinal>, ports: &GastrointestinalPorts<'b>)
    ///     -> EdgeRoute<'b, Gastrointestinal, Glucose, AbsorbingEdge> {
    ///     ports.absorption::<Glucose>(gi)
    /// }
    /// ```
    pub fn absorption<Q: Absorbed>(
        &self,
        laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ) -> EdgeRoute<'b, Gastrointestinal, Q, AbsorbingEdge> {
        let family = Family::Along(Passage::Absorption, Q::SPECIES);
        laws.request(family);
        EdgeRoute::requested(family)
    }
    /// Fat to the bed's interstitium, which its lymph drains: the only way triglyceride is absorbed.
    pub fn chyle(
        &self,
        laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ) -> EdgeRoute<'b, Gastrointestinal, Triglyceride, AbsorbingEdge> {
        let family = Family::Along(Passage::Chyle, Species::Triglyceride);
        laws.request(family);
        EdgeRoute::requested(family)
    }
    /// The gut's cells shed into the lumen with their iron.
    pub fn shedding(
        &self,
        laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ) -> EdgeRoute<'b, Gastrointestinal, Iron, AbsorbingEdge> {
        let family = Family::Along(Passage::Shedding, Species::Iron);
        laws.request(family);
        EdgeRoute::requested(family)
    }
    pub fn boundary<P: BoundaryProcess<Card = Gastrointestinal>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Gastrointestinal>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
