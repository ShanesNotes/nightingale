# 02: Law forms the slice needs

Type: research
Status: resolved
Claimed by: claude
Blocked by: none

**Question:** Which cited form does each slice law take, where the fills leave it open? Ticket [10](../../native-contract/issues/10-slice-laws.md) writes the laws, so each answer is a form, its constants and a stated validity, never a value bound to the fixture.

- [x] Valve flow: linear conductance against quadratic orifice loss, with inertance and the opening guards
- [x] Arteriolar conductance against alpha-1 occupancy and tubuloglomerular feedback
- [x] Venous unstressed volume against alpha-1 occupancy
- [x] Viscosity against hematocrit and diameter
- [x] Central integration past the sinus node, and traffic to transmitter release
- [x] Urine concentration against AVP, and the chemoreflex's sensed medium
- [x] A validity range for each form, with its source

## Answer

Each row is a form, where its constants go and where it holds. Constants become records under [Parameters](05-parameters.md); none is bound here. Where a source states a validity range the row cites it. Otherwise `Holds for` says "model default", because none was found. Each source says how far it was read: full text, abstract, or restated, where an open paper reproduces a closed one, and a restated form names its upgrade. "Model adaptation" means a sourced form was changed to fit the contract, and the row says how.

| Law | Form | Constants | Holds for | Source and reading |
|---|---|---|---|---|
| Valve flow, open mode | `L dq/dt = dp - B q abs(q)`, `B = rho / (2 A^2)`, `L = rho l / A` | Effective area `A` and length `l`, one pair per valve | Model default: competent valves with no stenosis, at rest and in ordinary activity | Mynard 2012, restated by Contarino and Toro 2018 Eqs 48 to 51, which add a viscous term the heart does not take. Upgrade: Mynard 2012 |
| Valve modes | Open while `dp > 0` or `q > 0`. Shut at `q = 0` with `dp <= 0`, where `q` holds at zero | None | All four valves, tricuspid included | Guyton 12e p.106, full text: valves open and close passively on the gradient. The `q > 0` guard follows from the inertance term and is a model default |
| Arteriolar resistance | `R = R_base (1 + S E_a1)`, with `E_a1` the operational effect of alpha-1 occupancy at the junction. On a renal afferent edge, `R = R_base (1 + S E_a1) f_tgf^-4` with `R_base = 128 mu L / (pi d_base^4)`, because TGF acts on diameter | `S` per bed class | Model default: basal to maximal traffic in a healthy adult | Model adaptation of Ursino 1998, restated by Kana 2010 Eq 1.7.16: `R = R0 + delta_R`, with `delta_R` relaxing toward `G ln(f - f_min + 1)`. The contract reads occupancy, so the receptor effect replaces that log of traffic. Hallow 2017 Eq 2, full text, for Poiseuille. Upgrade: Ursino 1998 |
| TGF on the afferent edge | `f_tgf = 1 + S (1 / (1 + exp((x - x0) / m)) - 0.5)`, with `x` the macula densa Na concentration. It is 1 at the set point and bounded by `1 - S/2` and `1 + S/2`, and with `S` and `m` positive a raised `x` narrows the edge | `S`, `m`, `x0` | Model default: around the operating point, with `S` under 2 so the diameter stays positive | Hallow 2017 Eq 30, full text, its form for a control with a steady-state error, and its text for the signal. Its TGF constants sit in the Supplement, not read. Guidi 1995 for human gain, not read |
| Venous unstressed volume | `V_u = V_u_base - S E_a1`, with compliance held | `S` per venous bed class | Model default: muscle and splanchnic veins, the two beds Kana's mass balance moves | Kana 2010 Eqs A1.10 and A1.11 show `V_u` moving in those beds. The gain's form is by analogy with the resistance law, and Ursino's own equation is unread. Upgrade: Ursino 1998, Halliwill 1999 |
| Viscosity | `mu = mu_plasma mu_rel`, with Pries's in vivo `mu_rel`, `D` in micrometres and `C` from Eq 5. `H_D` is discharge hematocrit, red cell flow over total flow on the edge, so no Fahraeus correction applies | `mu_plasma`, and each bed class's `D` | Model default: mean shear `U / D` of 50 /s or more, the bar Secomb and Pries set on the glass tube data. The in vivo curve comes from rat mesentery networks | Secomb and Pries 2013 Eqs 5 to 7, full text. Their Eqs 3 and 4 give the glass tube curve, from human blood |
| Integration at a junction | `rate = R0 m n` at the sinus node, with `m = 1 + S_b E_b1` and `n = 1 - S_m E_m2`. Past the sinus node there is no interaction term: each junction's receptor effect drives that junction's own law | `R0`, `S_b`, `S_m`, fitted to Katona's `m` and `n` at rest | Sinus node only for the measured form; past it, model default | Katona 1982, abstract, for `R0 m n`, where `m` and `n` are defined by blockade. Their maps to receptor effect are a model adaptation. No measurement past the sinus node was found |
| Traffic to release | `release = q_i f` per junction, linear in traffic | `q_i`, calibrated so resting spillover closes | Model default: resting traffic. Saturation at high traffic is not sourced | Wallin 1996, abstract: renal spillover rises with muscle traffic across men, r 0.76. Fairfax 2013, full text, is the check on the chain: a single burst lowers leg conductance 2.9% at the sixth cycle, multiple bursts 11.0% |
| Urine concentration | `eta = eta_0 mu_avp` on collecting duct water, with `mu_avp = E_v2 / E_v2_base` | `eta_0` and the V2 constants | Model default. Observations to hold it against: urine 50 to 1200-1400 mOsm/L (Guyton 12e p.345); plasma AVP 1.4 +/- 0.8 pg/mL after water loading and 5.4 +/- 3.4 after fluid deprivation (Robertson 1973, abstract) | Model adaptation of Hallow 2017 Eq 17, full text, whose multiplier is normalized vasopressin from a sodium controller (Eq 31). The slice reads V2-bound, so the V2 effect replaces it. Upgrade: Robertson 1976 for the curve's shape |
| Chemoreflex sensed medium | Central drive reads brain extracellular H+. CO2 crosses the blood-brain barrier and H+ barely does, so arterial pH acts slowly | Outside the slice | Healthy adult. A brain space in the chemistry layer is the structural ask | Guyton 12e p.507, full text. The chemoreflex sits outside slice coverage in the contract, so ticket 10 does not wait on it |

Decisions:

- **Quadratic, not linear.** Klabunde writes a valve as a resistance, linear in flow, and its 16-fold gradient at a quarter of the area holds under both forms, because both vary as `1 / A^2`. They differ only in flow. The valve area inference needs the quadratic form, and inertance is what lets a valve shut at zero flow instead of on a gradient, so ticket 10 takes the quadratic form.
- **Resistance, not conductance, is linear in the stimulus.** Ursino writes his fit in resistance, and this keeps a saturating receptor from driving conductance through zero. Taking the receptor effect in place of his log of traffic is the adaptation, because the contract's laws read occupancy and never a tone scalar.
- **No separate lag on an effector.** The contract carries release, reuptake and binding, so the junction pool supplies the lag Ursino writes as a first-order filter and a latency. That is a model default. Fairfax's nadir at the sixth cycle checks the whole chain's timing and does not show which step supplies the delay.
- **A restated form is usable now.** The valve law stands on Contarino and Toro's restatement, the traffic-to-resistance shape on Kana's, and the venous gain on analogy. No open route to Ursino 1998, Mynard 2012 or Halliwill 1999 has been found: APS refuses scripts and Wiley's copy is closed.

The size of each `S` is not settled here. [Parameters](05-parameters.md) owes it.
