# 17: Lactate's oxidation in tissue, and the lactate made for each unit of oxygen short

Type: research
Status: open
Blocked by: 11

**Question:** Which cited forms set the rate at which a bed's cells burn lactate, against the cells' lactate and their oxygen, and how much glucose a bed's cells split into lactate for each unit of oxygen its oxygen rate falls short of its demand? [Body ticket 09](../../body/issues/09-tissues-respire.md) states both as model defaults with no source, and [research 11](11-law-forms-gas-loop.md) gives no form for either.

Today's forms, in `crates/ng-metabolism/src/burn.rs`:

- Glycolysis: `g = r max(D - B, 0)` glucose a second, with `r` = 1/6, one glucose to the six oxygen a glucose burn takes. Zero wherever the oxygen rate `B` meets the demand `D`.
- Lactate's burn: `l = k N P / (P + K)` lactate a second, first order in the cells' lactate `N` at `k` = 0.02 a second and half its speed at a cell oxygen partial pressure `K` = 2 mmHg.

- [ ] A form for lactate's oxidation in tissue against the cells' lactate and oxygen, or the evidence that first order in each is as far as sources go, with its constants and a validity range
- [ ] The lactate made for each unit of oxygen short, as a yield or as a form, with its source and tier. Research 11 names 38/6 lactate an oxygen as a bound an inference record may use; this ticket checks that bound against a source before any packet uses it
- [ ] Whether cells hold lactate at rest, and what sets that amount. Today glycolysis is zero at rest, so a bed's cells admit with no lactate and owe no oxygen; a resting amount would need the debt to read lactate above it without going below zero
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it

**Non-goals:** the liver's and the kidney's uptake of lactate from blood (body ticket 14 and research 13), lactate's crossings of the cell membrane and the capillary wall, and lactate's effect on acidity, which reads plasma lactate (body ticket 18 and research 14).
