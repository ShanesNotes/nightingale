# 03: Does the slice's kidney give pressure natriuresis?

Type: research
Status: resolved
Claimed by: claude
Blocked by: none

**Question:** Do the slice's renal laws raise sodium and water excretion with arterial pressure as a published human pressure-natriuresis relation does, with only the couplings the census already has?

The tilt neighborhood's largest gap ([ticket 01](01-tilt-neighborhood.md)) is the kidney's answer to arterial pressure and body fluid volume: pressure natriuresis, extracellular volume raising sodium excretion, and renal sympathetic activity and arteriolar tone setting filtration. The census has no cardiovascular-to-renal arrow for it. Under [record 0002](../../../docs/adr/0002-map-starts-grouped.md) a coupling earns its arrow when a sourced check fails without it, so this ticket finds that check's target and runs the slice against it.

- [x] A human pressure-natriuresis relation from the library, with its method, window and tier
- [x] The slice run against it, with the result and the reason
- [x] If the slice fails, the coupling's proposal under record 0002

## Answer

No. The slice raises sodium excretion with arterial pressure only through the filtered load. Its kidney misses the human relation's route at every parameter value, and its size at the fixture's values. What it lacks is a renal law form, not a census arrow.

**The human relation.** The library holds no primary human measurement of pressure against sodium excretion. Its two primary salt-balance studies, PMC4567387 and PMC4919532, report excretion against intake, not pressure. The relation therefore comes from Guyton and Hall, Textbook of Medical Physiology, 12th edition (2011), at the textbook tier, and a primary human source would upgrade it.

- Acute (p. 371, Figure 29-13): a rise of 30 to 50 mmHg in arterial pressure raises urinary sodium output two- to threefold. Pressure natriuresis persists in an isolated kidney, away from nerves and hormones.
- Route (p. 337): between 75 and 160 mmHg, autoregulation leaves GFR only slightly changed. The natriuresis comes mostly from less tubular reabsorption: higher peritubular and renal interstitial pressure raises sodium backleak, and angiotensin II falls.
- Glomerulotubular balance (pp. 334 to 335): when GFR rises from 125 to 150 mL/min, the proximal tubule keeps reabsorbing 65% of it. This buffers a spontaneous change in GFR, without hormones.
- Chronic (p. 374): in most people, a tenfold rise in sodium intake raises arterial pressure by only a few mmHg, because angiotensin II falls.
- The textbook names no method or species for the acute curve; its isolated-kidney sentence points to animal preparations. The chronic statement concerns people at steady state on each intake, over days.

**The run.** A throwaway probe ran the slice on main `1d21f4e` with its synthetic fixture, on the `ng-solver` core, on the tower. It is kept outside the repo. Re-admitting the fixture with a changed systemic resistance was refused, so the probe raised pressure with isotonic saline instead: 150, 300 or 600 water units over 2 minutes, through the gut boundaries, at plasma sodium and chloride. Saline also dilutes plasma albumin, which adds to filtration. Each load is read against its own no-load run in the same window, 9 to 10 minutes after the start. Pressure is in the fixture's units, where baseline mean arterial pressure is 100.

| Feedback | Load | Pressure rise | GFR | Sodium excretion | Fractional excretion |
|---|---|---|---|---|---|
| on | 150 | +3.8 | x1.03 | x1.8 | 1.00% (control 0.58%) |
| on | 300 | +8.5 | x1.06 | x2.7 | 1.51% |
| on | 600 | +18.3 | x1.13 | x5.6 | 2.90% |
| off | 150 | +3.4 | x1.27 | x7.7 | 8.68% (control 1.42%) |
| off | 600 | +16.2 | x2.17 | x49.9 | 36.5% |

The bars were written and hashed before the first result.

| Bar | Result |
|---|---|
| Acute size: x1.5 to x4 for a rise of 30 to 50, feedback on | not reached; at a rise of 18 excretion was already x5.6 |
| Acute route: GFR within 10% over that rise | fail: GFR rose 12.5% at a rise of 18 |
| Chronic: tenfold intake raises pressure by at most 10 | not run: admission refuses the steady state with the fit held, at the body's elastic energy and the arteriolar junction's norepinephrine store |
| Route by law: a tubular law reads pressure or a hormone pressure moves | fail |

What it shows:

- **Excretion follows the filtered load, not pressure.** With feedback off, 150 units raised pressure 3.4 and GFR 27%, and excretion rose 7.7-fold. With feedback on, 600 units raised pressure 18.3 and GFR 12.5%, and excretion rose 5.6-fold: five times the pressure rise, and less natriuresis. Sodium crosses back by a saturable law that reads only the lumen's sodium and chloride (`crates/ng-renal/src/lib.rs:68`, `:81`). No renal law reads peritubular or interstitial pressure, angiotensin, aldosterone, natriuretic peptide or renal nerves for sodium, and renin (census line 98) is a sink with no receiver.
- **The saturable law breaks glomerulotubular balance.** With feedback on, a 12% rise in filtered load raised fractional excretion from 0.58% to 2.9%: tubule and duct together reclaimed only 78% of the extra load. The fixture sets this operating point on purpose, fitting the tubule's capacity so that "salt balance fixes filtration at what the tubule can take back" (`crates/ng-validate/src/slice.rs:600`).
- **Tubuloglomerular feedback is the slice's only autoregulator.** Without it, GFR doubles for a rise of 16 and excretion rises 50-fold. No myogenic law acts on the afferent edge.

**Proposal under record 0002.** In the slice, sodium excretion is set by the filtered load, the plasma's composition and the vasopressin effect. A pressure rise and a GFR rise that carry the same filtered load give the same excretion at every parameter value. The human kidney gives a large natriuresis to a pressure rise that barely moves GFR (pp. 337, 371) and buffers a GFR rise that comes without one (pp. 334 to 335). No parameter set holds both, so the slice misses the check at every value its sources allow, and new structure is earned.

That structure is a law form, not a census arrow. Renal perfusion already reaches the kidney through substrate S1, by the port the filtration law reads (`crates/ng-contract/src/scope.rs:599`). The proposal has three forms, and each needs its source and its parameter range at its tier before it enters:

1. Sodium reclaim reads the peritubular bed's pressure through that port, so higher peritubular and interstitial pressure lowers net reclaim.
2. The tubule reclaims a share of the load it receives, as the human proximal tubule does, rather than up to a fixed capacity.
3. Angiotensin II acts on reclaim as a ligand once renin has a receiver. This is the chronic curve's route, so the chronic check this probe could not run is its test.

Limits: the human relation is textbook-tier; saline is not a pure pressure step; each control drifts over its 10 minutes, the feedback-off control's excretion 2.4-fold; and the fixture's values are synthetic, so the sizes are the fixture's. Only the route belongs to the laws.

Local evidence, outside the repo: `~/lanes/logs/ng-evidence-library/ticket-03/` holds the bars (`bars.md`, sha256 `e47953a4`), both probe logs, the probe patch, the tower script, the scoring script and `verdict.json`.
