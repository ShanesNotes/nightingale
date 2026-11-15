# 08: Blood carries oxygen and carbon dioxide

Type: task
Status: open
Blocked by: 06, 26, 29, `.scratch/research/issues/11-law-forms-gas-loop.md`

**What to build:** Oxygen and carbon dioxide are amounts in every blood and tissue space, moved by the flows that already move water, and crossing the capillary wall and the cell membrane by diffusion. Blood chemistry turns each total into its partial pressure and its saturation as algebraic rows that never make or lose mass. It also states acidity in every watery space from the strong ion difference, the weak acids and total carbon dioxide, with the lumped anion and the diet's organic anion held. Hemoglobin is a mass in blood, so its concentration follows dilution, and the hematocrit it gives sets viscosity by the form research 02 gives. No vessel law reads viscosity yet, so this ticket adds that reader. The lungs and the tissues stay held: a fixed exchange at the lung and a fixed use at each bed.

**Releases:** from ticket 06's hold table, gas content in blood and tissue, the carrier's capacity, and viscosity on coupling arrow 114.

- [ ] Tests: content against partial pressure matches each form at stated points, and each gas's total is conserved through speciation to the ledger's floor
- [ ] Run: with the held exchange and use, arterial and venous contents settle where the Fick identity puts them. A water bolus lowers hemoglobin concentration and arterial content while the oxygen amount holds
- [x] Run: a change in hematocrit changes viscosity and flow through the declared law, and a mutant row that disconnects coupling arrow 114 fails a test. Met by the first PR: a 400 mL water bolus takes the central veins' hematocrit from 0.425 to 0.385 and viscosity from 7.115 to 6.407 in 30 s, the coronary edge's pressure drop over its flow falls by the same ratio, 0.9005, and its flow rises 12.1%: 0.9% from the drop and the rest from viscosity. The row `viscosity-arrow-114-disconnected` fails `a_plain_edges_flow_falls_as_viscosity_over_its_reference_rises` in 2 s, with no admission.

The first PR (08a) gives hematology its law crate, `ng-hematology`. Hematocrit is the central veins' hemoglobin over the red cells' own, and viscosity is research 02's row at one fixed diameter, one body-wide field on arrow 114. The whole body's plain edges read it through `ng_cardiovascular::declare_reading_viscosity`, which only the body's root calls, so the slice's model is unchanged. Every value is synthetic. The reference viscosity is the admitted start's, so the ratio starts at 1 to 4e-7. Released: arrow 114's viscosity field. The hold table loses no row, because it lists routes and pools and hemoglobin's routes stay held. Cost on the beelink, base `686eb631a720` against this PR's head: rows 1,372 to 1,373, nonzeros 8,892 to 8,916, steps 11,794 to 11,797, solves 15,264 to 14,510 (ordinary 12,519 to 12,522, landing 2,745 to 1,988), 6,257.2 to 5,987.2 M a simulated second, and admission 212 s to 214 s. The landing knife-edge of nightingale#172 moved back, so the instructions a solve rose 0.66% from the mix of solves, while those a Newton update rose 0.05%. Left to the next two PRs: oxygen carriage (08b), then carbon dioxide and plasma acidity (08c). Bleeding competence and thrombosis stay held on arrow 114 until ticket 21.
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against [ticket 26](26-cost-budget.md)'s budget

**Non-goals:** the making and excretion of the lumped anion and the diet's organic anion (ticket 18), abnormal hemoglobins, and red cell turnover (ticket 21).

From [ticket 06](06-whole-body-on-holds.md)'s plan: body 01 releases the held gas in blood and tissue here (01:871) and gives carbon dioxide's tissue store to [ticket 09](09-tissues-respire.md) (01:146). So this ticket takes carbon dioxide's crossings of the capillary wall and the cell membrane, with its pools in the cells and the interstitium of every bed, and ticket 09 takes its tissue store: the bed's burn, which fills those pools.

From [research 14](../../research/issues/14-law-forms-volume-and-acid.md): the ticket that states acidity, this one under the default, writes hemoglobin's charge in blood, the last row of research 14's acid balance. Whether compile accepts a site field that reads its own row at its own site was not checked, and if it does not, `[H+]` and bicarbonate become two fields that read each other.
