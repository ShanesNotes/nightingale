# Body wayfinder map

## Destination

All 13 systems run at rung 0 in one body, on a synthetic fixture. The body admits at 06:55 and lives a synthetic day of sleep, posture, three meals, drinks and room air. Every amount the day moves closes in the ledger with its own controller holding it. No stand-in remains except the few rung 0 keeps, which ticket 01 lists with their reasons. The day's checks pass by direction, and the cost of a simulated second stays inside a stated budget. [Foundation 01](../foundation/issues/01-arena-and-milestone.md), step 4, then binds the reviewed packet to this body and runs the first milestone (SRS invariant I19).

## Notes

- **Rung 0 is defined by the day's books.** A system's rung 0 is the smallest set of laws without which an amount the day moves cannot close or has no controller, or a reading in the milestone's lens cannot be produced. [Ticket 01](issues/01-rung-0-specification.md) writes it down system by system. No doc in the repo did before.
- **The work is ordered by loop, not by card** (SRS rules 9 and 10). A loop is one conserved amount with its ways in and out and its controller: the gases, heat, the fuels, water and salt, acid. A loop ticket lands laws in more than one system crate and ends when that amount's book closes on the whole body.
- **Stand-ins are the to-do list.** The slice already holds each of the nine cards outside it as a declared stand-in. [Ticket 06](issues/06-whole-body-on-holds.md) builds the whole body on such holds, and each later ticket releases the ones it names. The count of holds left is the build's progress.
- **The slice does not move.** The whole body is a second assembly. The arena scores cores on the slice, and a deeper body needs a simpler one to be compared with.
- **Shared lists change first, once.** The species, the spaces and networks, each system's ports and the census are shared by every crate. Tickets 02 to 05 change them before any law needs them, so two builders never edit the contract at the same time.
- **Numbers stay synthetic.** Only a law form can hold a build ticket up ([research map](../research/map.md)). Research tickets 11 to 15 give the forms, one ticket for each loop.
- **Every build ticket is one PR that ends in three checks.** Tests: a closed-form case for each new law, a refusal test for each new rule, and a mutant row that puts the stand-in back and must fail. A run: the whole body admits, the certificate passes on a short window, each new link of a loop moves its reader the right way, and the body's expect trace is updated with one sentence on why it moved. A cost line: unknowns and rows before and after, and wall time for each simulated second inside [ticket 26](issues/26-cost-budget.md)'s budget. A ticket over its budget does not merge until an exception is recorded with its reason.
- **Loop PRs merge one at a time.** Each one moves the body's trace and its size pins, and a code proof runs on the heavy-run machine. A PR that waited merges main and regenerates its trace before its proof.
- **Docs tickets go first and in parallel.** Ticket 01 and the research tickets prove on any machine.
- The two drafts that derived the body from its books are outside the repo: `~/lanes/logs/ng-q1-healthy-body-draft.md` and `~/lanes/logs/ng-q1-frame-followup.md`. They are model proposals from before the contract, never sources (SRS rule 16).

## Order

Tickets 01, 02, 07 and 23 and research tickets 11 to 15 can start today. Ticket 27 follows 07.

| Ticket | Waits for | What it lands |
|---|---|---|
| [01](issues/01-rung-0-specification.md) Rung 0, system by system | nothing | The specification, as tables |
| [02](issues/02-reactions.md) A reaction and its books | nothing | The law language can turn one species into another |
| [03](issues/03-whole-body-lists.md) Species, spaces and networks | 01, 02 | The contract's closed lists, grown once |
| [04](issues/04-scopes-ports-and-census.md) Scopes, ports and a census for each assembly | 03 | Nine systems can declare laws, and no system ticket edits a shared constant |
| [05](issues/05-the-days-inputs.md) The day's inputs | 03 | The clock, sleep, the room and meals reach the body |
| [06](issues/06-whole-body-on-holds.md) The whole body on holds | 04, 05 | The second assembly and its hold table |
| [07](issues/07-what-limits-the-step.md) What limits the step | nothing | Where a step's cost goes on the slice, and its limiter |
| [27](issues/27-step-plan.md) A step plan that can reach the speed aim | 07 | Prototypes and a pick |
| [26](issues/26-cost-budget.md) A cost budget for the whole body | 06, 07 | The budget each loop is held to |
| [08](issues/08-blood-carries-gases.md) Blood carries the gases | 06, 26, research 11 | Gas loop |
| [09](issues/09-tissues-respire.md) Tissues respire | 02, 08, research 11 | Gas loop |
| [10](issues/10-lungs-breathe.md) The lungs breathe | 08, 09, research 11 | Gas loop |
| [11](issues/11-breathing-is-controlled.md) Breathing is controlled | 05, 10, research 11 | Gas loop, closed |
| [12](issues/12-heat-balance.md) Heat balance | 05, 09, research 12 | Heat loop, closed |
| [25](issues/25-sleep-and-the-circulation.md) Sleep reaches the circulation and the metabolic rate | 05, 09, research 12 | The night's fall in rate, pressure and oxygen use |
| [13](issues/13-gut-absorbs-a-meal.md) The gut absorbs a meal | 05, 06, 26, research 13 | Fuel loop |
| [14](issues/14-liver-glucose-hub.md) The liver is the glucose hub | 02, 13, research 13 | Fuel loop |
| [15](issues/15-insulin-and-glucagon.md) Insulin and glucagon | 14, research 13 | Fuel loop |
| [16](issues/16-fuel-stores.md) Fuel stores close over a day | 09, 15, research 13 | Fuel loop, closed |
| [17](issues/17-volume-hormones.md) Volume hormones and the kidney's pressure answer | 06, 26, research 14 | Water and salt loop, closed |
| [18](issues/18-acid-loop.md) The acid loop | 08, 09, 11, 17, research 14 | Acid loop, closed |
| [19](issues/19-minerals-and-bone.md) Minerals and bone | 13, 15, 17, research 15 | Pass 2 |
| [20](issues/20-cortisol-rhythm.md) Cortisol's daily rhythm | 05, 14, research 15 | Pass 2 |
| [21](issues/21-blood-cells-and-systems-at-rest.md) Blood cells, plasma proteins and the systems at rest | 08, 14, research 15 | Pass 2 |
| [22](issues/22-the-bladder.md) The bladder | 05, 06, 26 | Pass 2 |
| [23](issues/23-healthy-range-and-return-checks.md) The healthy-range check and the return check | nothing | The two milestone checks no code holds |
| [24](issues/24-the-synthetic-day.md) The synthetic day | 11, 12, 16, 17, 18, 23, 25 | Pass 1 checked: 24 hours on the whole body |
| [28](issues/28-two-synthetic-days.md) The whole body lives two synthetic days | 19, 20, 21, 22, 24 | The destination: both passes, 48 hours |

After tickets 06 and 26, three lanes can be built side by side: the gases (08), the gut (13) and the volume hormones (17). They share the body's assembly and add laws to the cardiovascular and renal crates, so ticket 04 puts each law in its own module and the PRs merge one at a time.

## Model defaults, open to review

- **Two passes.** Pass 1 is every loop that a monitor reading depends on or that drifts within a day: the gases, heat, the fuels, water and salt, acid. Pass 2 is the slow books: the minerals and bone, cortisol, the blood's cells and proteins, the bladder. Until its ticket lands, a pass 2 amount is a held store with balanced flows. Alternative: one pass, with the slow books before the synthetic day.
- **Immunity and coagulation at rest are declared with zero flux** and a list of what they cannot show, as record 0002 asks of a grouped region. Alternative: a reduced model of each now, which no check of a healthy day could judge.
- **Breathing is tidal and the heart pulsatile at rung 0**, unless ticket 26's budget forbids a tidal breath. Ticket 10 prototypes the alternative, quasi-steady alveolar gas, before it chooses.
- **Acid-base is stated from the strong ion difference, the weak acids and carbon dioxide** in every space, with one lumped anion. Alternative: bicarbonate from carbon dioxide alone at rung 0, which a later ticket would have to replace.

## Not yet specified

- **Which readings the lens carries at the milestone.** [Research 06](../research/issues/06-healthy-ranges-and-lens.md) proposes the list. If it keeps total calcium and phosphate, ticket 19 moves into pass 1 and joins ticket 24's blockers.
- **The speed plan.** The slice runs 14.5 times over the speed aim's budget, and about a third of a step is the record's own work. Ticket 07 measures what limits the step and ticket 27 picks a plan. Until then, no one knows whether the whole body can reach the aim on one coupled step.
- **Owners the docs leave open**: the nitrogen, acid and carbon dioxide ledgers, the potassium shift into cells, the bone pools, the natriuretic peptide's secretion, cell water, the pleural space and intracranial pressure. Ticket 01 names a default for each, and tickets 03 and 04 build it.
- **The membrane list by site class** (SRS section 5). Ticket 03 lands the one membrane type. Each loop ticket adds the walls it crosses.

## Out of scope

- Disease, interventions, and any new chance law (scope stages 2 to 4).
- Clinical numbers. The research map sources them, and they reach a crate only through the reviewed packet.
- The arena's pick between cores, and anything that moves the slice.
- The monitor's display and the chart.
- A split of any site beyond what rung 0 names (record 0002).
- Anything in nightingale-pulse.
