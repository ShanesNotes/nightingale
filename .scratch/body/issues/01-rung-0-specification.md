# 01: Rung 0, system by system

Type: research
Status: claimed
Claimed by: body-01-rung-0
Blocked by: none

**Question:** What is the smallest thing each of the 13 systems must do for the body to live a day? No doc in this repo says. The glossary defines rung 0 as a system's simplest working version, and the only system-by-system account is in two drafts outside the repo that predate the contract.

Write the answer as tables a builder can work from:

- [ ] The day's books. For each amount the day moves (water, sodium, potassium, chloride, oxygen, carbon dioxide, heat and energy, the fuels, nitrogen, fixed acid, the minerals, the blood's cells and proteins): where it enters, where it is made or used, where it leaves, and which law holds it. Each book closes on paper for the reference man's day before any law is written
- [ ] For each system: what is dynamic at rung 0, what is a held store with explicit balanced flows, what is absent, and the behaviors the system cannot show at rung 0 (record 0002 asks a grouped region for the same list)
- [ ] The species, spaces and sites those laws need, as the list ticket 03 builds from. Each site the body adds beyond the slice's names the check or the reading that earns it (record 0002)
- [ ] Which pass each item belongs to. Pass 1 is every loop that a monitor reading depends on or that drifts within a day. Pass 2 is the slow books
- [ ] The holds rung 0 keeps, such as abdominal pressure, each with its reason
- [ ] For every coupling arrow and each of its fields: the ticket that wires it, an anatomy relation, or a hold with its reason
- [ ] An owner for each fork the docs leave open: the nitrogen, acid and carbon dioxide ledgers, the potassium shift into cells, the bone pools, the natriuretic peptide's secretion, cell water, the pleural space, the bladder
- [ ] The atlas card trims this implies, listed for [foundation 06](../../foundation/issues/06-atlas.md) to apply as each system is built
- [ ] For each loop, the links across systems that the evidence library holds, set beside the coupling map's arrows: wired, held, or a lead the coupling map lacks. A lead goes to the evidence library's process and adds no arrow here

**Start from:** the evidence on hand, which the [body map](../map.md)'s Notes locate: the curation study's `needs.md` for what each system needs, and the evidence library's links for each loop. Then read `~/lanes/logs/ng-q1-healthy-body-draft.md` (sections 2 to 8: the balance sheets, the systems derived from them, a model named for each rung, and the rules that contain growth) and `~/lanes/logs/ng-q1-frame-followup.md` (sections 3 and 4: what must be dynamic once the day has meals and sleep, and the smallest clock and sleep). They are model proposals, never sources (SRS rule 16), and they predate the contract, so restate them in the contract's and the glossary's words. Numbers stay out: this ticket fixes shape, and the research tickets fix forms and values.

**Done when** each book closes on paper with only the laws the tables list, and a builder can tell from the Answer alone which holds each later ticket releases.

## Answer

Rung 0 is the whole body at the body map's destination. Each system has the smallest set of laws without which an amount the day moves cannot close, has no controller, or cannot produce a reading in the milestone's lens. The tables below are written for the builders of tickets 03 to 06 and for each loop ticket. This Answer fixes shape only. The research tickets fix forms and values, so no clinical number appears here.

### How to read the tables

- The ticket says "books". The glossary's word is conserved quantity, which this Answer uses.
- Each row names a law and the system whose laws hold it. No system owns a pool: the state holds every amount (SRS section 2).
- Ticket 06 builds the whole body with every row on a held stand-in. The **Ticket** column names the ticket that replaces that stand-in with the row's law. "slice" means the slice already has the law and ticket 06 carries it over. "05" means a boundary input. "kept" means rung 0 keeps the hold, for the reason given.
- In a system table, **dynamic** rows are laws the system declares at rung 0. **Held** rows are stores or values a held stand-in keeps at rung 0, with their reason. **Absent** rows are not in the body at all. **Cannot show** lists the behaviors record 0002 asks a grouped region to state.
- Sites are those of the slice ([solver contract](../../../docs/architecture/solver-contract.md) section 1) plus the sites listed for ticket 03 below. The slice's splanchnic bed is the gut bed, its splanchnic veins are the portal vein, and its leg bed is the muscle bed.

### The day's conserved quantities

Each table closes on paper for the reference man's day: the closure line sums every way in against every way out and the change in what the body stores. Over 24 hours the change in stores is held to the certificate's drift bound, not to zero (record 0004). The two drafts give each quantity a balance with numbers (healthy-body draft section 2), which these tables restate in the contract's words without the numbers.

#### Water

Pass 1. Controller: vasopressin on the collecting duct, which answers osmolality, volume and the night (17), and the kidney's pressure answer (17). Thirst is absent: the day's drinks are scheduled (record 0004).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| drinks | enters | stomach lumen | a drink delivered on the boundary port | boundary | 05 |
| food water | enters | stomach lumen | the meal's water, in the meal's delivery | boundary | 05 |
| metabolic water | made | cells of every bed | the bed's oxidation reaction | MT | 09 |
| absorption | moves | small intestine lumen to portal blood | absorption against the lumen's content | GI | 13 |
| gut secretion and reabsorption | moves | gut wall and lumen | one balanced held flow | GI | kept |
| circulation | moves | every vessel and bed | advection along the solved flow | CV | slice |
| capillary exchange and lymph | moves | plasma, interstitium | Starling exchange and lymph return | FL | slice |
| cell water | moves | interstitium and cells of every bed | water follows tonicity across the cell membrane | FL | 17 |
| filtration and reclaim | moves | glomerular bed, tubule, collecting duct | filtration, and reclaim under vasopressin | RN | slice |
| urine | leaves | bladder | voiding: the slice's drain, then a void event | RN | slice, 22 |
| skin evaporation | leaves | skin bed | evaporation, which takes water and its latent heat in one transfer | MT | 12 |
| sweat | leaves | skin bed | sweat under the thermostat's sweat drive | MT | 12 |
| airway evaporation | leaves | conducting airway | water that humidifies inspired gas leaves in expired gas | RS | 10 |
| feces water | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: drinks + food water + metabolic water = urine + skin evaporation + sweat + airway evaporation + feces water + change in stores.

Stores: every watery space, cells included. Metabolic water needs ticket 02's reaction, because a transfer never changes a species today (`crates/ng-contract/src/conservation.rs`).

#### Sodium

Pass 1. Controller: aldosterone through renin and angiotensin II, the natriuretic peptide, and pressure natriuresis (17). The store integrates, and pressure natriuresis is the loop's integral term (healthy-body draft section 2.2).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal sodium | enters | stomach lumen | the meal's delivery | boundary | 05 |
| absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | slice |
| cell sodium | moves | interstitium and cells | the sodium pump against a leak | FL | 17 |
| filtration and reclaim | moves | glomerular bed, tubule, collecting duct | filtration, and saturable reclaim under aldosterone, the peptide and arterial pressure | RN | slice, 17 |
| urine sodium | leaves | bladder | voiding | RN | slice |
| sweat sodium | leaves | skin bed | sweat carries sodium with its water | MT | 12 |
| feces sodium | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal sodium = urine sodium + sweat sodium + feces sodium + change in stores.

#### Potassium

Pass 1. Controllers: insulin moves a meal's potassium into cells within minutes (15), and aldosterone raises its excretion over hours (17). A meal carries a large share of the extracellular store, so the shift into cells is dynamic at rung 0 (frame follow-up section 3a).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal potassium | enters | stomach lumen | the meal's delivery | boundary | 05 |
| absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | slice |
| shift into cells | moves | interstitium to cells of the muscle and fat beds | uptake that insulin drives through the receptor layer | FL | 15 |
| pump and leak | moves | cells and interstitium of every bed | the sodium pump against a leak, so what insulin moves in can return | FL | 17 |
| filtration, reclaim and secretion | moves | glomerular bed, tubule, collecting duct | filtration, saturable reclaim, and secretion under aldosterone | RN | slice, 17 |
| urine potassium | leaves | bladder | voiding | RN | slice |
| feces potassium | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal potassium = urine potassium + feces potassium + change in stores.

#### Chloride

Pass 1. Controller: none of its own. Its reclaim follows sodium's, as in the slice, and the kidney's feedback on the strong ion difference sets how much chloride leaves beside sodium (18).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal chloride | enters | stomach lumen | the meal's delivery | boundary | 05 |
| absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | slice |
| filtration and reclaim | moves | glomerular bed, tubule, collecting duct | reclaim follows sodium's, and the acid loop's feedback on the strong ion difference | RN | slice, 18 |
| urine chloride | leaves | bladder | voiding | RN | slice |
| sweat chloride | leaves | skin bed | sweat carries chloride with its water | MT | 12 |
| feces chloride | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal chloride = urine chloride + sweat chloride + feces chloride + change in stores.

Cells hold no chloride at rung 0. A lumped anion stands for the cells' anions (see fixed acid).

#### Oxygen

Pass 1. Controllers: ventilation under the chemoreflex, whose peripheral sensor reads arterial oxygen (11), and each bed's flow under its oxygen debt (09).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| inspired oxygen | enters | conducting airway | inspiration of the room's gas | RS | 10 |
| alveolar uptake | moves | alveolar gas to pulmonary bed plasma | diffusion across the gas-exchange surface, with a shunt | RS | 10 |
| carriage | moves | blood | advection, with binding to hemoglobin as speciation | CV, BC | 08 |
| tissue delivery | moves | bed plasma, interstitium, cells | diffusion across the capillary wall and the cell membrane | MT | 08 |
| oxygen use | used | cells of every bed | the bed's oxidation reaction | MT | 09 |
| expired oxygen | leaves | conducting airway | expiration | RS | 10 |

Closes on paper: inspired oxygen = expired oxygen + oxygen use + change in stores.

Stores: alveolar gas, blood (dissolved and bound) and tissue. Until ticket 10, oxygen enters by a fixed exchange at the lung, and until ticket 09 it leaves by a fixed use at each bed (ticket 08).

#### Carbon dioxide

Pass 1. Controller: ventilation under the central chemoreflex, which reads the acidity of a brain space (11).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| carbon dioxide made | made | cells of every bed | the bed's oxidation reaction | MT | 09 |
| tissue store | moves | cells and interstitium of every bed | one lumped store a bed's spaces hold | MT | 09 |
| carriage | moves | blood | advection, with dissolved, bicarbonate and bound forms as speciation in the strong-ion frame | CV, BC | 08 |
| alveolar elimination | moves | pulmonary bed plasma to alveolar gas | diffusion across the gas-exchange surface | RS | 10 |
| inspired carbon dioxide | enters | conducting airway | the room's inspired gas | RS | 10 |
| expired carbon dioxide | leaves | conducting airway | expiration | RS | 10 |
| urine carbon dioxide | leaves | bladder | total carbon dioxide filtered and not reclaimed | RN | 18 |

Closes on paper: carbon dioxide made + inspired carbon dioxide = expired carbon dioxide + urine carbon dioxide + change in stores.

The species is total carbon dioxide in each space. Bicarbonate is a speciation of it, never its own amount, so the gas loop and the acid loop move one amount. Until ticket 18 the glomerular filter keeps carbon dioxide back.

#### Heat and energy

Pass 1. Controllers: the thermostat, whose set point follows the clock and steps down in sleep, acting through skin vessel tone, sweat and shivering (12); the metabolic rate's fall in sleep (25); and the heat a meal adds (16).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| food energy | enters | stomach lumen | the energy each fuel species carries in the meal's delivery | boundary | 05 |
| gravity's work | enters | every vessel segment | posture's work, a receipt that nets over the day | CV | slice |
| fuel energy to heat | moves | cells of every bed | the oxidation reaction gives each fuel's energy as heat, less the work it pays | MT | 09 |
| work to heat | moves | heart, vessel walls, respiratory pump | the walls' and the pump's work, paid by fuel at a bed, dissipates as heat | CV, MT | slice, 09, 10 |
| heat carried by blood | moves | core and skin heat stores | blood carries heat between the two stores | CV | 12 |
| conduction | moves | core to skin | conduction between the two stores | MT | 12 |
| dry heat loss | leaves | skin bed | radiation and convection to the room | MT | 12 |
| evaporative heat | leaves | skin bed, conducting airway | latent heat with skin evaporation, sweat and airway water | MT, RS | 12, 10 |
| warming inspired gas | leaves | conducting airway | expired gas leaves warmer than it came in | RS | 10 |
| feces energy | leaves | small intestine lumen | unabsorbed fuel in the balanced fecal exit | GI | 13 |
| urine energy | leaves | bladder | the energy urea and creatinine carry out | RN | 14, 16 |

Closes on paper: food energy + gravity's work = dry heat loss + evaporative heat + warming inspired gas + feces energy + urine energy + change in stores.

Stores: heat in the core and skin stores, chemical energy in every fuel pool (glycogen, triglyceride and body protein the largest), and the walls' elastic store. Until ticket 09 the heart's work is paid by the exterior chemical energy MT stands behind in the slice, and until ticket 12 heat leaves by the slice's single exit. The energy each fuel carries is ticket 02's choice of book shape.

#### The fuels

Pass 1. Controllers: insulin and glucagon (15). No loop holds the stores within hours. Over the day the stores close because intake equals what the body spends (record 0004), and the respiratory quotient follows the fuel burned (16).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal glucose | enters | stomach lumen | the meal's carbohydrate, delivered as glucose | boundary | 05 |
| glucose absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| liver glycogen | moves | liver bed cells | glycogen made and broken down under portal insulin and glucagon | HP | 14 |
| gluconeogenesis | made | liver bed cells | glucose from lactate, amino acids and glycerol, as reactions | HP | 14 |
| glucose uptake | moves | brain, muscle and fat beds | uptake the brain takes without insulin, and that muscle and fat take with insulin | MT | 15 |
| muscle glycogen | moves | muscle bed cells | glycogen made and used in muscle | MS | 16 |
| renal glucose reclaim | moves | glomerular bed, tubule | filtration, and reclaim up to its limit | RN | 15 |
| glucose oxidation | used | cells of every bed | the oxidation reaction, one mixed fuel until ticket 16 | MT | 09, 16 |
| glycolysis to lactate | used | cells of every bed | glycolysis when supply falls short, and at rest in some beds | MT | 09 |
| lactate made | made | cells of every bed | the product of glycolysis | MT | 09 |
| lactate use | used | liver bed cells, and cells of beds that burn it | gluconeogenesis and oxidation | HP, MT | 14, 16 |
| meal fat | enters | stomach lumen | the meal's fat | boundary | 05 |
| fat absorption | moves | small intestine to the gut bed's lymph | fat by the lymph route | GI | 13 |
| fat storage and release | moves | fat bed cells | triglyceride stored after meals, and fatty acids and glycerol released under low insulin | MT | 16 |
| fatty acid oxidation | used | cells of every bed | the oxidation reaction | MT | 16 |
| ketones made | made | liver bed cells | ketones from fatty acids, overnight | HP | 16 |
| ketogenesis | used | liver bed cells | fatty acids turned into ketones | HP | 16 |
| ketone oxidation | used | cells of the brain and muscle beds | the oxidation reaction | MT | 16 |
| glycerol to glucose | used | liver bed cells | glycerol taken into gluconeogenesis | HP | 14 |
| meal protein | enters | stomach lumen | the meal's protein, delivered as amino acids | boundary | 05 |
| amino acid absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| protein turnover | moves | muscle and liver bed cells | body protein made and broken down at its smallest | MS | 16 |
| amino acid use | used | liver bed cells | deamination into urea, with the carbon to glucose or oxidation | HP | 14 |
| feces fuel | leaves | small intestine lumen | unabsorbed fuel in the balanced fecal exit | GI | 13 |

Closes on paper (glucose and glycogen): meal glucose + gluconeogenesis = glucose oxidation + glycolysis to lactate + change in stores.

Closes on paper (lactate): lactate made = lactate use + change in stores.

Closes on paper (fat): meal fat = fatty acid oxidation + ketogenesis + glycerol to glucose + feces fuel + change in stores.

Closes on paper (ketones): ketones made = ketone oxidation + change in stores.

Closes on paper (amino acids and body protein): meal protein = amino acid use + feces fuel + change in stores.

Each fuel's carbon leaves as carbon dioxide and its hydrogen as water, which those two tables carry.

#### Nitrogen

Pass 1. Controller: none within a day. Amino acid use answers its own supply, so urea production and excretion follow intake as a regulated total (healthy-body draft section 2.8).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal nitrogen | enters | stomach lumen | the meal's amino acids | boundary | 05 |
| amino acid absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| protein turnover | moves | muscle and liver bed cells | body protein made and broken down | MS | 16 |
| urea synthesis | moves | liver bed cells | urea from amino acids, as a reaction | HP | 14 |
| ammonium made | moves | renal tubule | ammonium made from amino acids in the tubule, for acid excretion | RN | 18 |
| creatinine made | moves | muscle bed cells | creatinine made from muscle | MS | 16 |
| urine urea | leaves | bladder | filtration, with part reclaimed | RN | 14 |
| urine ammonium | leaves | bladder | ammonium excretion | RN | 18 |
| urine creatinine | leaves | bladder | filtration, with none reclaimed | RN | 16 |
| feces nitrogen | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal nitrogen = urine urea + urine ammonium + urine creatinine + feces nitrogen + change in stores.

Ammonia has no pool at rung 0: urea is made from amino acids in one reaction (14). Albumin's and hemoglobin's nitrogen stays in their own conserved quantities, whose making and loss balance (21), so they leave this table unchanged.

#### Fixed acid

Pass 1. Controllers: the kidney's feedback on the strong ion difference over hours (18), and the chemoreflex through carbon dioxide within minutes (11). The draft's "net acid excretion" becomes, in the strong-ion frame, the kidney excreting chloride and the lumped anion with ammonium instead of with sodium.

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| fixed acid made | made | cells of every bed | the oxidation reaction posts the lumped strong anion the diet's protein yields | MT | 18 |
| speciation | moves | every watery space | acidity stated from the strong ion difference, the weak acids and carbon dioxide | BC | 08, 18 |
| fixed acid excreted | leaves | bladder | the kidney's excretion of the lumped anion as a feedback on the strong ion difference | RN | 18 |

Closes on paper: fixed acid made = fixed acid excreted + change in stores.

Lactate and ketones are strong anions too. They close in the fuels table. Hydrogen ion and bicarbonate are never amounts.

#### The minerals

Pass 2, unless research 06 keeps total calcium and phosphate in the lens: then pass 1 (see what this changes in the plan). Controllers: parathyroid hormone holds calcium, and the kidney's saturable reclaim returns phosphate and magnesium after a load (19).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal calcium | enters | stomach lumen | the meal's delivery | boundary | 05 |
| meal phosphate | enters | stomach lumen | the meal's delivery | boundary | 05 |
| meal magnesium | enters | stomach lumen | the meal's delivery | boundary | 05 |
| mineral absorption | moves | small intestine lumen to portal blood | absorption of a share of each mineral | GI | 19 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | 06 |
| bone partner pool | moves | bone bed cells | fast exchange with bone's partner pool | MS | 19 |
| cell phosphate and magnesium | moves | cells of every bed | phosphate moves into cells with insulin, and magnesium with the cells' content | FL | 19 |
| filtration and reclaim | moves | glomerular bed, tubule | filtration, reclaim under parathyroid hormone for calcium, saturable reclaim for phosphate and magnesium | RN | 19 |
| urine calcium | leaves | bladder | voiding | RN | 19 |
| urine phosphate | leaves | bladder | voiding | RN | 19 |
| urine magnesium | leaves | bladder | voiding | RN | 19 |
| feces calcium | leaves | small intestine lumen | the unabsorbed share | GI | 19 |
| feces phosphate | leaves | small intestine lumen | the unabsorbed share | GI | 19 |
| feces magnesium | leaves | small intestine lumen | the unabsorbed share | GI | 19 |

Closes on paper (calcium): meal calcium = urine calcium + feces calcium + change in stores.

Closes on paper (phosphate): meal phosphate = urine phosphate + feces phosphate + change in stores.

Closes on paper (magnesium): meal magnesium = urine magnesium + feces magnesium + change in stores.

Until ticket 19 each mineral is a held store with balanced flows: absorption in and excretion out at equal synthetic rates, with bone and cell exchange held. Phosphate is a weak acid in the strong-ion frame, so ticket 18 reads its total.

#### The blood's cells and proteins

Pass 2. Controller: none. Each store loses mass in proportion to its size, with production to match, so it returns to balance by itself (21).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| hemoglobin production | made | central veins | red cell production to match the loss, entering the blood | HC | 21 |
| hemoglobin carriage | moves | blood | advection; hemoglobin crosses no capillary wall | CV | 08 |
| hemoglobin destruction | used | gut bed blood | first-order loss in proportion to the red cell mass | HC | 21 |
| iron absorption | enters | small intestine lumen to portal blood | absorption to match iron's loss | GI | 21 |
| iron recycling | moves | liver bed cells | iron from destroyed red cells returns to the store, and production draws on it | HC | 21 |
| iron loss | leaves | gut bed | first-order loss in proportion to the store | HC | 21 |
| albumin synthesis | made | liver bed plasma | constant synthesis, then production to match the loss | HP | 14, 21 |
| albumin exchange | moves | every bed | Starling exchange with albumin's reflection, and lymph | FL | slice |
| albumin loss | used | liver bed plasma | first-order loss in proportion to the store | HP | 21 |

Closes on paper (hemoglobin): hemoglobin production = hemoglobin destruction + change in stores.

Closes on paper (iron): iron absorption = iron loss + change in stores.

Closes on paper (albumin): albumin synthesis = albumin loss + change in stores.

Hemoglobin is the red cell mass at rung 0: one amount, whose partial volume stands for the red cells' volume (08, 21). Platelets, leukocytes, globulins and clotting factors are absent, as coagulation and immunity at rest have zero flux (21).
