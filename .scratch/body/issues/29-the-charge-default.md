# 29: How charge closes when a reaction makes a strong anion

Type: prototype
Status: resolved
Claimed by: 29-charge-default
Blocked by: 02

**Question:** Rung 0's reactions make lactate, ketones, the fixed acid and ammonium, and the strong-ion frame keeps hydrogen ion out of the amounts. Today the contract's charge is a sum of fixed charges, and the certificate holds charge's drift at zero. [Ticket 01](01-rung-0-specification.md)'s Answer books each acid and base as a neutral moiety that speciation charges, so every reaction balances charge, and the certificate reads charge after speciation. Its alternative gives each strong anion a fixed charge and pairs each reaction that makes or burns one with a counter-charge. Test both on [ticket 02](02-reactions.md)'s bench and pick one.

- [ ] Both options run on one reaction that makes a strong anion, and on a potassium meal that carries phosphate and the diet's organic anion
- [ ] What the certificate reads for charge under each option, over the same run
- [ ] A pick with its reason
- [ ] What the pick asks of tickets 03, 14 and 18
- [ ] A refusal test and a mutant row for the charge leg of the compile's balance check. No test reaches that leg today, because every charged species is carried and states no atoms, so a reaction that names one is refused first

**Non-goals:** the acid loop's laws ([ticket 18](18-acid-loop.md)).

## Answer

The default stands. Each acid and base is a neutral moiety with a fixed charge of zero, speciation charges it, and the certificate reads charge after speciation. The bench ran both options through one reaction that makes a strong anion and through a potassium meal. With the contract as it stands, the alternative does not compile. Under both options, the meal that carries phosphate fails the certificate's present rule for charge, while charge closes in every window. Every number below is synthetic.

- **The bench** (throwaway, on the engine at this ticket's base; patch, scripts and logs in `~/lanes/logs/ng-body-wave-2/29-charge-default/proto/`). Ticket 02's `BurningBeds` gained glycolysis in each of its three beds' interstitium: glucose into two lactate, its energy into the bed's heat store, first order in glucose. Each interstitium also took a potassium meal as constant supplies that nothing clears. Meal M1 brings potassium at 3r, phosphate at r and the diet's organic anion at r, with r at 0.001 a second. Meal M2, the control, brings potassium and the organic anion at r each. Under A, lactate and the organic anion each state the atoms of their acid form, with a fixed charge of zero. Under B, each states the atoms of its anion with a fixed charge of -1, and glycolysis also makes two of a proton species, one hydrogen with +1. Phosphate is carried with a fixed charge of zero under both, because it is a weak acid that only speciation charges. Each case ran 600 s on the reference integrator and was certified on 60 s windows, charge by its real rule and the other quantities by the bench's limits.
- **The compile, with the contract as it stands.**

  | Case | Result |
  |---|---|
  | A: glycolysis | compiles |
  | A: the organic anion burned with oxygen | compiles |
  | B: glycolysis without the proton | refused, `UnbalancedReaction`, charge |
  | B: glycolysis with two protons | refused, `UnnamedConservedQuantity` |
  | B: the organic anion burned, without and with a proton | refused, the same two ways |

  With the proton the reaction balances. But the anion and the proton then form a combination that the bed's reactions keep and that carbon, hydrogen, oxygen and energy do not span. So B needs a reacting part to name charge as a conserved quantity, which changes the contract's basis rule. It also makes hydrogen ion an amount, which the strong-ion frame keeps out: [ticket 01](01-rung-0-specification.md)'s species list adds none, and [research 14](../../research/issues/14-law-forms-volume-and-acid.md)'s balance solves for it.
- **What the certificate reads for charge, over the same run.** For B to run, the prototype named charge among a reacting part's quantities. A's model hash, basis and combinations are the same with and without that change. Charge in each 60 s window, the same in all ten:

  | Case | Combinations | Certificate | Charge's change, equal to its net exchange | Closure |
  |---|---|---|---|---|
  | A, M1 | 22 | fails drift on charge in the first window | 0.54 | 0 |
  | A, M2 | 19 | fails drift on charge in the first window | 0.18 | 0 |
  | B, M1 | 25 | fails drift on charge in the first window | 0.36 | 0 |
  | B, M2 | 22 | passes | 0 | 0 |

  Each change is what the supplies predict over three beds and 60 s: under A the potassium alone, and under B the potassium less the organic anion. Phosphate counts zero under both options, so the potassium that comes with it reads as charge gained. Glycolysis ran in every bed, and B's own charge combination in each bed read zero.
- **The pick: the default, A.** First, A compiles and runs with the contract as it stands, while B needs the basis rule changed and hydrogen ion made an amount. Second, a meal that carries phosphate fails the present charge rule under either option, so the certificate must read charge after speciation either way, and B's fixed charges on strong anions buy nothing a real meal keeps. Third, A moves nothing in the compiled slice. The body map's line stands.
- **The slice's model hash and trace do not move.** This ticket changes no code. On the prototype's tree, with six species appended and charge named in a reacting part, `tests/trace.rs` passed 3 of 3 with both pins.
- **What the pick asks of [ticket 03](03-whole-body-lists.md).** Each acid and base states the atoms of its uncharged form and a fixed charge of zero, so lactate states lactic acid's atoms. Its charge after speciation is stated apart, for ticket 18, as ticket 01's species table gives it. A fixed charge belongs only to a carried species: sodium, potassium, chloride, calcium, magnesium and the cell anion. No species stands for hydrogen ion or bicarbonate. Hold that rule in the species type, so a species that states atoms cannot state a fixed charge, with its compile-fail test and legal twin. The rule makes the compile's charge leg unreachable, so ticket 03 can retire the leg (see the fifth box). This ticket appends no species.
- **What the pick asks of [ticket 14](14-liver-glucose-hub.md).** Write the liver's conversions with neutral moieties and no hydrogen ion or counter-charge term: glucose from lactate, urea from amino acids, and the fixed acid from their sulfur. Every term's fixed charge is zero, so only elements and energy can leave such a reaction open.
- **What the pick asks of [ticket 18](18-acid-loop.md).** Speciation charges each moiety, and research 14's balance already counts lactate, ketones, the fixed acid and the diet's organic anion at -1 and ammonium at +1. The certificate then reads charge after speciation. Today it sums fixed charges and holds that sum's drift to the closure floor (the contract's certificate section, [record 0004](../../../docs/adr/0004-first-milestone.md)). That sum is the body's strong ion total, and under the default it moves by design: when a meal brings a cation with phosphate or the organic anion, as the table shows, and when the kidney sends ammonium out with chloride. So ticket 18 changes what the certificate holds as charge to each space's balance after speciation, which electroneutrality holds at zero, and gives the sum of fixed charges another drift rule. That changes the contract's certificate paragraph and record 0004's charge line, a call for the developer reviewer. The SRS line that charge never drifts can stand, read as charge after speciation. Until ticket 18 lands, every synthetic meal pairs its cations with chloride, as ticket 01 says, so the sum stays put. Ticket 18's test that charge closes with the new species holds under the default: closure was zero in every window, and drift is what the present rule fails.
- **The fifth box is not met, and is flagged.** Under the default no species that states atoms carries a fixed charge, so the pick's own rule keeps the charge leg unreachable. The prototype's mutant that removes the leg survived every test under A: the reactions tests (26), the burning bench (5) and the four A cases. Only B's species reached the leg, and with the leg removed their refusal named hydrogen instead of charge. A test that reaches the leg needs a charged species with atoms, which the default forbids, or a change to `compile.rs` and the species type, outside this ticket's paths. The rule belongs in ticket 03's species type, whose compile-fail test holds it. The body map's note on the two unreached guards names this ticket for the leg, so that pointer moves to ticket 03.
- **Not in this ticket, and flagged.** `UnnamedConservedQuantity` named the bed's first reaction (`burn`), not the reaction whose ions the quantities missed. That misleads whoever meets it, though under the default no reaction does.
