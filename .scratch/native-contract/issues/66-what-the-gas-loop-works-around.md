# 66: Two rows and a function the gas loop works around

Spec: `crates/ng-contract/`

Type: task

Status: claimed
Claimed by: nc66

Blocked by: none

**Why.** The gas loop (body 08 to 11) is the longest chain to the first milestone (SRS invariant I19), and [body 08](../../body/issues/08-blood-carries-gases.md)'s plan found three things the contract cannot state, each with a workaround in the body (`~/lanes/logs/ng-wave-4/b08-plan/PLAN.md`, sections 3 and 8).

- **A read of one bed class at its one site.** The held lung exchange reads the pulmonary bed's plasma. Today it is a `sum_at` over every bed, with its conductance zero off the lung. The smallest change is `impl Includes<Beds<Lung>> for CapillaryBed`.
- **A viscosity-over-viscosity identity.** The plain edge's resistance is `R0 (mu / mu_ref)`. The contract has no `Quotient<Viscosity, Viscosity>` identity, so a reciprocal parameter, `cv.reference-viscosity-reciprocal`, stands in.
- **`expm1`.** From the break-it pass on nightingale#174: the contract has no `expm1`, so a form like `x^C - 1` loses its value as `C` nears zero, and research 02's viscosity exponent passes zero. The check of nightingale#175 measured the range: the viscosity law holds to 2.4 units in the last place for exponents of 0.5 to 8 of either sign and to 10 at 50, loses three digits at 1e-3 and six at 1e-6, returns a wrong value with no refusal from 1e-15 to 1e-16, and is refused from 1e-17 down and from about -390, where its denominator's square overflows.

- [x] A law reads one bed class at its one site, with its test
- [x] A viscosity over a viscosity is a dimensionless identity, with its test
- [x] The contract has `expm1`, with a test near zero
- [ ] The body drops each workaround: the lung's `sum_at` rows, the reciprocal parameter, and any form that stands in for `expm1`. The lung's `sum_at` rows and `mt.lung-share` go with [body 10](../../body/issues/10-lungs-breathe.md), and the reciprocal parameter and the viscosity form go in one small body PR after this ticket's contract PR.

## Answer

The contract part is in `crates/ng-contract/` and moves no pin.

- **One bed class at its one site.** The beds include each tissue's beds (`Includes<Beds<T>> for CapillaryBed`), so a site kernel on `Beds<Lung>` reads a capillary bed's entries and site parameters with `at_site_in` and `site_parameter_in`. That relation alone would still sum two lung beds and give an empty sum for none, so `FragmentBuilder::only_at` makes a body row the kernel's value at the only site of its set, and compile refuses the row where the map gives the set no site or more than one (`CompileDefect::NotOneSite`, naming the law and the set). `tests/one_site.rs` holds one, none and two lung beds.
- **Viscosity over viscosity.** `Quotient<Viscosity, Viscosity>` is `Dimensionless` in the identity table, with a doctest and a compile-fail twin for a viscosity over a pressure.
- **`expm1`.** `Kernel::expm1`, by `exp`'s reduction and polynomial with the reduction's rounding carried and fdlibm's rebuild. It is exact at zero, within one unit in the last place of offline references across its range of either sign, and its slope is `e^x`. Past `exp`'s overflow a law that takes it is refused as not finite, by name. `tests/expm1.rs` holds the form `x^C - 1` written as `expm1(C ln x)` at the exponents this ticket measured.

