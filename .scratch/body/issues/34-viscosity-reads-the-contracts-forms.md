# 34: The viscosity law and the plain edge read the contract's forms

Type: task
Status: open
Blocked by: ../../native-contract/issues/66-what-the-gas-loop-works-around.md

**Why.** Hematology's viscosity law writes `x^C - 1` as `exp(C ln x) - 1`, which loses its digits as the exponent `C` nears zero, and research 02's exponent passes zero. The check of nightingale#175 measured the hole: the law loses three digits at an exponent of 1e-3 and six at 1e-6, returns a wrong value with no refusal at exponents of 1e-15 to 1e-16, and is refused from 1e-17 down and from about -390. The plain edge's resistance `R0 (mu / mu_ref)` reads a reciprocal parameter, `cv.reference-viscosity-reciprocal`, because the contract had no viscosity over viscosity. [Native-contract 66](../../native-contract/issues/66-what-the-gas-loop-works-around.md) gives the contract `expm1` and that identity. At an exponent of -390 either form keeps 13.5 digits, so `expm1` does not help there.

- [ ] The plain edge's resistance reads viscosity over its reference through `Quotient<Viscosity, Viscosity>`, and `cv.reference-viscosity-reciprocal` leaves the packet and the fixture
- [ ] The viscosity law writes `x^C - 1` with `expm1`, with a test at the exponents native-contract 66 measured: 1e-3, 1e-6, 1e-15, 1e-17 and -390
- [ ] `ng-validate/tests/operators.rs` lists `Expm1` among the operators the pinned body runs
- [ ] The body's pins move once, with one sentence on why
