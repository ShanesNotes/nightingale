use super::{
    Assembly, BloodChemistryPorts, BloodChemistryScope, EndocrinePorts, EndocrineScope,
    GastrointestinalPorts, GastrointestinalScope, HeldScope, HematologyPorts, HematologyScope, HepaticPorts,
    HepaticScope, ImmunePorts, ImmuneScope, MetabolismPorts, MetabolismScope, MusculoskeletalPorts,
    MusculoskeletalScope, RespiratoryPorts, RespiratoryScope,
};
use crate::{
    AdmissionError, BloodChemistry, Endocrine, FragmentBuilder, Gastrointestinal, Hematology, Hepatic,
    Immune, Metabolism, Musculoskeletal, OutsideSlice, Respiratory,
};

macro_rules! outside {
    ($($method:ident: $owner:ident, $scope:ident, $ports:ident;)*) => {
        impl<'b> Assembly<'b> {
            $(
                /// The card's scope, issued once: fresh, or taking over the builder its held stand-ins wrote to.
                pub fn $method(
                    &mut self,
                    held: Option<HeldScope<'b, $owner>>,
                ) -> Result<$scope<'b>, AdmissionError> {
                    Ok($scope {
                        laws: self.outside(held)?,
                        ports: $ports::issued(),
                    })
                }
            )*
        }
    };
}
outside! {
    respiratory: Respiratory, RespiratoryScope, RespiratoryPorts;
    hepatic: Hepatic, HepaticScope, HepaticPorts;
    gastrointestinal: Gastrointestinal, GastrointestinalScope, GastrointestinalPorts;
    endocrine: Endocrine, EndocrineScope, EndocrinePorts;
    blood_chemistry: BloodChemistry, BloodChemistryScope, BloodChemistryPorts;
    hematology: Hematology, HematologyScope, HematologyPorts;
    immune: Immune, ImmuneScope, ImmunePorts;
    metabolism: Metabolism, MetabolismScope, MetabolismPorts;
    musculoskeletal: Musculoskeletal, MusculoskeletalScope, MusculoskeletalPorts;
}

impl<'b> Assembly<'b> {
    /// Issues one of the nine once. Each enclosure the map names it on owes its pressure from here, held or built.
    pub(super) fn issue_outside<S: OutsideSlice>(
        &mut self,
    ) -> Result<FragmentBuilder<'b, S>, AdmissionError> {
        let mut laws: FragmentBuilder<'b, S> = self.issue()?;
        if self
            .schema
            .anatomy
            .held_enclosures()
            .any(|(_, card)| card == S::CARD)
        {
            laws.owe_enclosure_pressure();
        }
        Ok(laws)
    }
    fn outside<S: OutsideSlice>(
        &mut self,
        held: Option<HeldScope<'b, S>>,
    ) -> Result<FragmentBuilder<'b, S>, AdmissionError> {
        match held {
            None => self.issue_outside::<S>(),
            Some(HeldScope { laws, .. }) => Ok(laws),
        }
    }
}
