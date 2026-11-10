# 08: Blood carries oxygen and carbon dioxide

Type: task
Status: open
Blocked by: 06, 26, 29, `.scratch/research/issues/11-law-forms-gas-loop.md`

**What to build:** Oxygen and carbon dioxide are amounts in every blood and tissue space, moved by the flows that already move water, and crossing the capillary wall and the cell membrane by diffusion. Blood chemistry turns each total into its partial pressure and its saturation as algebraic rows that never make or lose mass. It also states acidity in every watery space from the strong ion difference, the weak acids and total carbon dioxide, with the lumped anion and the diet's organic anion held. Hemoglobin is a mass in blood, so its concentration follows dilution, and the hematocrit it gives sets viscosity by the form research 02 gives. No vessel law reads viscosity yet, so this ticket adds that reader. The lungs and the tissues stay held: a fixed exchange at the lung and a fixed use at each bed.

**Releases:** from ticket 06's hold table, gas content in blood and tissue, the carrier's capacity, and viscosity on coupling arrow 114.

- [ ] Tests: content against partial pressure matches each form at stated points, and each gas's total is conserved through speciation to the ledger's floor
- [ ] Run: with the held exchange and use, arterial and venous contents settle where the Fick identity puts them. A water bolus lowers hemoglobin concentration and arterial content while the oxygen amount holds
- [ ] Run: a change in hematocrit changes viscosity and flow through the declared law, and a mutant row that disconnects coupling arrow 114 fails a test
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against [ticket 26](26-cost-budget.md)'s budget

**Non-goals:** the making and excretion of the lumped anion and the diet's organic anion (ticket 18), abnormal hemoglobins, and red cell turnover (ticket 21).

From [research 14](../../research/issues/14-law-forms-volume-and-acid.md): the ticket that states acidity, this one under the default, writes hemoglobin's charge in blood, the balance's last row. Whether compile accepts a site field that reads its own row at its own site was not checked, and if it does not, `[H+]` and bicarbonate become two fields that read each other.
