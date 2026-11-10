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

### The 13 systems at rung 0

Each row is one law or one stand-in. **Ticket** names the ticket that makes a dynamic row live, so ticket 06 holds every dynamic row until that ticket. A held row here is one that rung 0 keeps.

#### CV. Cardiovascular

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The pulsatile heart: four chambers with time-varying elastance, valves as modes, the rhythm's phase | the slice's laws (map default: pulsatile at rung 0) | slice |
| dynamic | Compliance and resistance on every vascular edge of the whole body's map, the portal circuit included | the slice's edge laws on the new beds | 06 |
| dynamic | Advection of every species the blood carries, hemoglobin included | the slice's advection, extended to each species a route carries | 06 |
| dynamic | Hydrostatic head from site positions under posture | arrow 96, an anatomy relation | slice |
| dynamic | Rate, contractility and vessel tone from receptor occupancy | the receptor layer | slice |
| dynamic | Local flow that answers each bed's oxygen debt | arrow 90 | 09 |
| dynamic | Viscosity from hematocrit, read by every vessel law | arrow 114 | 08 |
| dynamic | Skin vessel tone under the thermostat's release at the skin junction | the receptor layer at the skin bed | 12 |
| dynamic | Heat carried by blood between the core and skin heat stores | a heat advection law | 12 |
| held | Every CV law runs at a reference temperature | no rung 0 CV law reads temperature (arrow 90's temperature field) | kept |
| absent | The arrest rhythms, their hazard, and cardiac conduction | no healthy day enters them | none |
| absent | The ECG waveform as a law's output | open, and no ticket plans it (body map, Not yet specified) | none |
| absent | Coronary territories, collaterals and watershed zones | one coronary bed stands for the heart's tissue (record 0002) | none |
| cannot show | Arrhythmia, arrest, ischemia by coronary territory, tamponade, and pooling within one limb | the map holds none of the structure these need | none |

#### RS. Respiratory

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | One alveolar unit with a shunt and a dead space, filled by a tidal breath | the equation of motion for a breath (map default; ticket 10 prototypes quasi-steady alveolar gas before it chooses) | 10 |
| dynamic | Diffusion of oxygen and carbon dioxide across the gas-exchange surface | alveolar exchange, reading pulmonary capillary flow (arrow 81) | 10 |
| dynamic | The pleural space's pressure, felt by every bed it encloses | the breath's pleural pressure replaces the slice's held constant (arrow 82) | 10 |
| dynamic | The resting lung volume, which follows posture | the equation of motion's resting point | 10 |
| dynamic | Water and heat that leave in expired gas | airway heat and water loss | 10 |
| dynamic | The work of breathing charged to metabolism, and the pump's load on MS | arrows 84 and 85 | 10 |
| dynamic | Lung stretch reported to NS | arrow 83 | 11 |
| held | Lung water | no rung 0 lung law reads it (arrow 110) | kept |
| held | Intra-abdominal pressure on the diaphragm | a held constant on a healthy day (arrow 103) | kept |
| absent | More than one alveolar unit, airway resistance by generation, hypoxic vasoconstriction, the bronchial circulation, cough | ticket 10's non-goals; one unit stands for the lungs (record 0002) | none |
| absent | Pleural fluid as an amount | a left-out space: no law moves it on a healthy day | none |
| cannot show | Ventilation and perfusion mismatch, regional disease, atelectasis, pneumothorax, effusion, bronchospasm | one unit cannot hold them | none |

#### NS. Nervous

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The baroreflex: stretch afferents, and norepinephrine and acetylcholine release, reuptake and clearance at every junction | the slice's laws (arrow 77) | slice |
| dynamic | The chemoreflex: a central sensor on the brain bed's acidity and a peripheral sensor on arterial oxygen and carbon dioxide, driving the pump's junction | arrows 86 and 88. The acidity read follows carbon dioxide alone until ticket 18 | 11, 18 |
| dynamic | The thermostat: core temperature against a set point that follows the clock and steps down in sleep, driving release at the skin junction and the sweat and shiver drives | arrows 87 and 91 | 12 |
| dynamic | The osmoreceptor and the volume drive to vasopressin | arrow 89's vasopressin field | 17 |
| dynamic | Sleep's gain on sympathetic traffic | the sleep state input (05) | 25 |
| held | The circadian phase and the CRH drive to EN | the clock reaches every law as a boundary input (05), as foundation 06 says for arrow 89, so NS relays neither | kept |
| held | Brain water and intracranial pressure | no skull enclosure at rung 0 (arrow 111) | kept |
| held | Ammonia's effect on the brain, and the fever set point shift | no ammonia pool, and immunity at rest (arrows 100 and 108) | kept |
| absent | A clock oscillator, REM sleep, and sleep the body decides | the non-goals of tickets 05 and 25 | none |
| absent | Consciousness, pain, voluntary movement and breathing, thirst, cerebral autoregulation, transport delays | no conserved quantity or lens reading needs them on a healthy day | none |
| cannot show | Orthostatic syncope, raised intracranial pressure, stroke, sedation, delirium, seizure, pain, periodic breathing | the laws they need are absent | none |

#### RN. Renal

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Filtration, one lumped tubule and collecting duct, water reclaim under vasopressin, saturable sodium and potassium reclaim, and tubuloglomerular feedback | the slice's laws (arrow 97) | slice |
| dynamic | Filtration and reclaim of each new small solute: glucose up to its limit, urea in part, creatinine not at all, total carbon dioxide | tubular laws per species | 14, 15, 16, 18 |
| dynamic | Renin secretion | arrow 98 | 17 |
| dynamic | Sodium reclaim that answers aldosterone, the peptide and the renal bed's own pressure, so pressure natriuresis shows as a check | the kidney's pressure answer (curation study, need RN-L04: a check, never a law) | 17 |
| dynamic | Potassium secretion under aldosterone, and the clock's gain on filtration | the clock input (05) | 17 |
| dynamic | Ammonium made in the tubule and excreted with chloride and the lumped anion, as the feedback on the strong ion difference | | 18 |
| dynamic | Calcium reclaim under parathyroid hormone, and saturable phosphate and magnesium reclaim | | 19 |
| dynamic | The bladder: urine collects, and leaves at a void that is an event of the day | | 22 |
| held | Intra-abdominal pressure on the kidney | a held constant on a healthy day (arrow 102) | kept |
| absent | Erythropoietin and calcitriol as moving hormones | the non-goals of tickets 19 and 21 | none |
| absent | Nephrons by type, the medulla's gradient, renal nerves beyond the slice's, drug excretion, dialysis | record 0002; ticket 17's non-goals | none |
| cannot show | Acute kidney injury, obstruction, a concentrating defect from the medulla, the urge to void, bladder pressure | one lumped nephron and no bladder wall | none |

#### HP. Hepatic

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The liver bed on the portal circuit, taking portal and arterial blood | the vascular laws on the new bed (arrow 99) | 14 |
| dynamic | Glycogen made and broken down under portal insulin and glucagon | | 14 |
| dynamic | Gluconeogenesis from lactate, amino acids and glycerol, and urea from amino acids | reactions (ticket 02) | 14 |
| dynamic | Albumin synthesis: constant, then production to match a first-order loss | | 14, 21 |
| dynamic | First-pass clearance of insulin | | 15 |
| dynamic | Ketones from fatty acids overnight | | 16 |
| dynamic | Cortisol's action on the glucose output | the receptor layer | 20 |
| held | Every HP law runs at a reference temperature | arrow 93 | kept |
| held | Ammonia | urea is made with no ammonia pool, so arrow 100 carries nothing | kept |
| absent | Angiotensinogen as an amount | its supply is a parameter of angiotensin II's making (17) | none |
| absent | Bile, the enterohepatic circulation, drug metabolism, clotting factors, hepcidin, zones within the liver | ticket 14's non-goals; coagulation at rest | none |
| cannot show | Liver failure, cholestasis, encephalopathy, coagulopathy, first-pass drug metabolism | the laws they need are absent | none |

#### GI. Gastrointestinal

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Stomach and small intestine lumens on the gut network, with gastric emptying | | 13 |
| dynamic | Absorption of water, sodium, potassium, chloride, glucose and amino acids into portal blood, and of fat by the lymph route | arrow 104 | 13 |
| dynamic | The balanced fecal exit of each meal's unabsorbed share | | 13 |
| dynamic | Absorption of calcium, phosphate and magnesium | | 19 |
| dynamic | Absorption of iron to match its loss | | 21 |
| held | Gut secretion and its reabsorption | one balanced held flow (ticket 13) | kept |
| held | Abdominal pressure | GI's held compliance on the abdominal space, felt by every bed it encloses (arrows 101 to 103) | kept |
| held | Gut wall water | no rung 0 gut law reads it (arrow 112) | kept |
| absent | The colon, motility under nervous control, the enteric nervous system, gut hormones, bleeding into the lumen | ticket 13's non-goals | none |
| cannot show | Vomiting, diarrhea and their losses, ileus, gut bleeding, the incretin effect | the laws they need are absent | none |

#### EN. Endocrine

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Insulin and glucagon secreted into portal blood against glucose | | 15 |
| dynamic | Vasopressin secreted against NS's osmotic and volume drive, rising at night, with first-order clearance | arrow 89's vasopressin field | 17 |
| dynamic | Renin to angiotensin II to aldosterone, with aldosterone also answering potassium | arrow 98 | 17 |
| dynamic | The natriuretic peptide secreted against atrial wall stress | arrow 79; EN owns the secretion (fork owners) | 17 |
| dynamic | Parathyroid hormone holding calcium | | 19 |
| dynamic | Cortisol secreted on a daily profile the clock drives, with its clearance | the clock input (05) | 20 |
| held | The circadian phase and the CRH drive from NS | EN reads the clock input directly (arrow 89) | kept |
| absent | Epinephrine, thyroid hormones, growth hormone, incretins, and erythropoietin and calcitriol as moving hormones | no conserved quantity, controller or lens reading of a healthy day needs them. This answers ticket 15's question on epinephrine | none |
| cannot show | The stress response, counter-regulation of a low glucose beyond glucagon, thyroid disease, the incretin effect, pulsed secretion | the hormones are absent | none |

#### BC. Blood chemistry

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Oxygen's saturation and partial pressure from its content and hemoglobin | algebraic rows that write no amount | 08 |
| dynamic | Carbon dioxide's dissolved, bicarbonate and bound forms, in the strong-ion frame | algebraic rows. Ticket 08 states them with the lumped anion held, and ticket 18 completes them | 08 |
| dynamic | Acidity in every watery space from the strong ion difference, the weak acids and total carbon dioxide | algebraic rows (map default) | 18 |
| dynamic | Ionized calcium | an algebraic row | 19 |
| held | Every speciation runs at a reference temperature | arrow 94 | kept |
| absent | Transfers across the cell membrane | FL owns the cell membrane (fork owners), so BC writes no amount | none |
| absent | Red cell chemistry, buffering inside cells and bone, drug binding | ticket 18's non-goals; no drugs | none |
| cannot show | The oxygen curve's adaptation, intracellular acidosis, bone's buffering in chronic acidosis, the alkaline tide after a meal unless research 14 sources its size | the laws they need are absent | none |

#### HC. Hematology and coagulation

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Hemoglobin as a mass in blood, so its concentration follows dilution and hematocrit sets viscosity | arrow 114's viscosity field | 08 |
| dynamic | Red cell mass lost in proportion to its size, with production to match | | 21 |
| dynamic | Iron's store, its recycling from destroyed red cells, and its loss | | 21 |
| held | Coagulation at rest | zero flux with its cannot-show list (ticket 21); arrow 114's bleeding competence and thrombosis fields | kept |
| held | Every HC law runs at a reference temperature | arrow 92 | kept |
| absent | Platelets, leukocytes, clotting factors, red cell age, erythropoietin as a moving hormone | ticket 21's non-goals | none |
| cannot show | Bleeding, clotting, thrombosis, recovery from anemia, polycythemia, a white cell response | the laws they need are absent | none |

#### IM. Immune

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | None | immunity at rest has zero flux (map default), and IM's crate declares its ports at zero | 21 |
| held | iNOS, myocardial depression, permeability, glycocalyx, tissue factor, hepcidin, the fever set point shift, mitochondrial efficiency | each held at zero or at the healthy value (arrows 105 to 109) | kept |
| absent | Pathogens, mediators, phagocytes, antibodies | no healthy day shows them | none |
| cannot show | Infection, sepsis, fever, anaphylaxis, inflammation, wound healing | ticket 21 puts this list in the body model | none |

#### FL. Fluids and endothelium

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Starling exchange at every bed, interstitial pressure, and lymph return | the slice's laws (arrow 80) | slice |
| dynamic | Each new small solute's crossing of the capillary wall | the slice's exchange law per species, or a keep-back until the species' ticket | 06 |
| dynamic | Potassium moved into cells by insulin | the receptor layer | 15 |
| dynamic | Water across the cell membrane by tonicity, and the sodium pump against a leak, with the pump's energy charged to metabolism | | 17 |
| dynamic | Phosphate moved into cells with insulin | | 19 |
| held | The vessel wall's permeability and glycocalyx | the healthy values (arrow 106) | kept |
| held | Lung, brain, gut wall, skin and compartment water as coupling fields | no rung 0 law reads them (arrows 110 to 113) | kept |
| absent | Pleural, peritoneal and joint fluid, and the cerebrospinal fluid | left-out spaces: no law moves them on a healthy day | none |
| absent | A two-pore wall, and exchange in lymph nodes | record 0002 | none |
| cannot show | Edema inside a grouped region, capillary leak, effusion, cerebral edema, third spacing | the structure and laws they need are absent | none |

#### MT. Metabolism and thermal

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Each bed burns fuel with oxygen into carbon dioxide, water, heat and fixed acid: one mixed fuel at a fixed respiratory quotient, then each bed's own mix | a reaction (ticket 02); arrow 78 | 09, 16, 18 |
| dynamic | Gas diffusion between each bed's blood and tissue, the tissue carbon dioxide store, and oxygen debt and lactate when supply falls short | arrow 90 | 08, 09 |
| dynamic | Glucose uptake: without insulin in the brain, with insulin in muscle and fat | the receptor layer | 15 |
| dynamic | Fat stored after meals and released overnight, fatty acid and ketone oxidation, and the heat a meal adds | | 16 |
| dynamic | Two heat stores, core and skin, with conduction between them, radiation, convection, evaporation and sweat to the room, and shivering's heat in the muscle bed | arrows 87 and 91 | 12 |
| dynamic | Sleep's gain on each bed's metabolic rate | the sleep state input (05) | 25 |
| held | Mitochondrial efficiency | the healthy value (arrow 109) | kept |
| absent | More than two heat stores, temperature's effect on rate, exercise, fever | ticket 12's non-goals | none |
| cannot show | Heat illness, regional temperature differences, starvation | the structure and laws they need are absent | none |

#### MS. Musculoskeletal and skin

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The respiratory pump's force, which RS reads, and its junction, where NS's drive arrives as release | arrows 85, 88 and 95 | 10, 11 |
| dynamic | Muscle glycogen and body protein turning over at their smallest, and creatinine made from muscle | | 16 |
| dynamic | Bone's partner pool for calcium, phosphate and magnesium | | 19 |
| held | Respiratory muscle fatigue | a resting day has no fatigue law (arrow 95) | kept |
| held | Skin and compartment water | arrow 113 | kept |
| held | The chance fixture: the slice's stir, which moves no law | the replay gates need a hazard on the body (ticket 06) until a physiological law draws chance | kept |
| absent | Voluntary movement, posture as muscle work, junctions other than the pump's, bone remodeling, pressure injury | posture is a boundary input; no healthy day needs the rest | none |
| cannot show | Weakness, fatigue, rhabdomyolysis, compartment syndrome, fracture, pressure injury | the laws they need are absent | none |

### The species, spaces and sites ticket 03 builds from

Ticket 03 adds these once, so no loop ticket edits a closed list.

| Species | Measure | Charge | Conserved quantities | Spaces | First moved by |
|---|---|---|---|---|---|
| oxygen | amount | 0 | oxygen | every blood and tissue space, alveolar gas, airway gas | 08 |
| carbon dioxide, total | amount | 0 as a total; speciation gives bicarbonate's | carbon dioxide | every watery space, alveolar gas, airway gas | 08 |
| nitrogen gas | amount | 0 | none: it crosses no wall | alveolar gas and airway gas, only if ticket 10 keeps the tidal breath | 10 |
| glucose | amount | 0 | the fuels, energy | every watery space, gut and urinary lumens | 13 |
| glycogen | mass | 0 | the fuels, energy | cells of the liver and muscle beds | 14 |
| triglyceride | mass | 0 | the fuels, energy | gut lumen, the gut bed's lymph and blood, cells of the fat bed | 13 |
| fatty acids | amount | 0 at rung 0 | the fuels, energy | plasma, interstitium, cells | 16 |
| glycerol | amount | 0 | the fuels, energy | plasma, interstitium, cells | 16 |
| amino acids, one lumped species | amount | 0 | the fuels, nitrogen, energy | every watery space, gut lumen | 13 |
| body protein | mass | 0 | the fuels, nitrogen, energy | cells of the muscle and liver beds | 16 |
| lactate | amount | -1 | the fuels, energy, charge | every watery space | 09 |
| ketones, one species | amount | -1 | the fuels, energy, charge | plasma, interstitium, cells | 16 |
| urea | amount | 0 | nitrogen, energy | every watery space, urinary lumen | 14 |
| ammonium | amount | +1 | nitrogen, charge | tubule, collecting duct and bladder lumens | 18 |
| creatinine | amount | 0 | nitrogen, energy | every watery space, urinary lumen | 16 |
| other strong anion, lumped | amount | -1 | fixed acid, charge | every watery space, urinary lumen | 18 |
| calcium | amount | +2 | the minerals, charge | every watery space, bone bed cells, gut and urinary lumens | 19 |
| magnesium | amount | +2 | the minerals, charge | every watery space, gut and urinary lumens | 19 |
| phosphate | amount | a weak acid, charged by speciation | the minerals, charge | every watery space, bone bed cells, gut and urinary lumens | 19 |
| hemoglobin | mass | a weak acid in blood (18) | the blood's cells and proteins | bed plasma and vessel lumens only | 08 |
| iron | amount | 0 | the blood's cells and proteins | cells of the liver bed, gut lumen | 21 |
| insulin, glucagon | amount | 0 | each its own | blood, interstitium, bound | 15 |
| renin, angiotensin II, aldosterone, the natriuretic peptide | amount | 0 | each its own | blood, interstitium, bound (renin in blood only) | 17 |
| parathyroid hormone | amount | 0 | its own | blood, interstitium, bound | 19 |
| cortisol | amount | 0 | its own | blood, interstitium, bound | 20 |
| albumin (exists) | mass | a weak acid (18) | the blood's cells and proteins | as in the slice | 18 |

No species is added for hydrogen ion, bicarbonate, ammonia, angiotensinogen, epinephrine or thyroid hormone: the first two are speciation, ammonia has no pool, and the rest are absent at rung 0.

| Space | At | Earned by |
|---|---|---|
| alveolar gas: not watery, its volume follows the gas law | the pulmonary bed | ticket 10's exchange, and the lens's arterial and end-tidal gas readings |
| airway gas: not watery | the conducting airway | ticket 10's dead space, and the airway's water and heat loss |
| gut lumen: watery, a lumen as the renal tubule's is | the stomach and the small intestine | ticket 13's emptying and absorption |
| heat store | the skin heat site; the core's heat stays at the slice's body site | ticket 12's two heat stores |

Red cells get no space: hemoglobin is a mass in blood (ticket 08). The skull has no space at rung 0.

| Site | Class | Earned by |
|---|---|---|
| brain bed | capillary bed | the central chemoreceptor reads a brain space's acidity (11), and the brain takes glucose without insulin (15) |
| coronary bed | capillary bed, inside the pleural space | the heart's work is paid by fuel burned at a bed (09). Model default; the alternative pays it at the muscle bed |
| liver bed | capillary bed inside the abdominal space, fed by the portal vein and the systemic arteries | the glycogen store and portal concentrations (14), first-pass insulin (15), arrows 99 and 104 |
| muscle bed | capillary bed: the slice's leg bed, standing for all skeletal muscle and the tissue no other bed names | insulin-driven uptake (15), muscle glycogen, body protein and creatinine (16), shivering (12), and the posture checks that earned the leg bed |
| fat bed | capillary bed | the triglyceride store and lipolysis (16), and insulin-driven uptake (15) |
| skin bed | capillary bed | skin vessel tone, evaporation and sweat (12) |
| skin heat site | tissue site | the skin heat store (12) |
| bone bed | capillary bed | the minerals' partner pool (19) |
| stomach | vessel segment on the gut network | emptying (13) |
| small intestine | vessel segment on the gut network | absorption and the fecal exit (13) |
| conducting airway | vessel segment on the airway network | dead space, and the airway's water and heat loss (10) |
| respiratory pump junction | junction | the drive reaches the pump as release (11) |
| skin junction | junction | the thermostat's release on skin vessels and sweat glands (12) |

The gut bed and the portal vein are the slice's splanchnic bed and veins, which now drain into the liver bed.

| Landmark | At | Read by |
|---|---|---|
| central chemoreceptor | brain bed | 11 |
| peripheral chemoreceptor | systemic arteries | 11 |
| thermostat | brain bed | 12 |
| osmoreceptor | brain bed | 17 |
| volume receptor | central veins | 17 |
| islets | gut bed | 15 |
| parathyroid | systemic arteries | 19 |

Networks: the airway, from the room through the conducting airway to the alveolar gas; the gut, from the stomach through the small intestine to the fecal exit; and the portal circuit on the vascular network, from the gut bed through the portal vein to the liver bed and the central veins. Enclosures: the pleural space adds the coronary bed, and the abdominal space adds the liver bed.

### Which pass each item belongs to

Pass 1 is every loop that a monitor reading depends on or that drifts within a day. Pass 2 is the slow books. A row's ticket gives its pass.

| Pass | Tickets | Conserved quantities and controllers |
|---|---|---|
| base | 03 to 06 | the lists, the day's inputs, and the body on holds |
| 1 | 08 to 18, and 25 | water, sodium, potassium, chloride, oxygen, carbon dioxide, heat and energy, the fuels, nitrogen, fixed acid; vasopressin, the renin chain, the peptide, insulin, glucagon, the chemoreflex, the thermostat and sleep's gains |
| 2 | 19 to 22 | the minerals, cortisol, the blood's cells and proteins, the bladder |

Until its ticket lands, a pass 2 amount is a held store with balanced flows. Nitrogen is pass 1, because its exit closes the fuels (14 and 16). The minerals move to pass 1 if research 06 keeps total calcium and phosphate in the lens.
