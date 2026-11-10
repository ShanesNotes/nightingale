# 01: Rung 0, system by system

Type: research
Status: resolved
Claimed by: body-01-rung-0
Blocked by: none

**Question:** What is the smallest thing each of the 13 systems must do for the body to live a day? No doc in this repo says. The glossary defines rung 0 as a system's simplest working version, and the only system-by-system account is in two drafts outside the repo that predate the contract.

Write the answer as tables a builder can work from:

- [x] The day's books. For each amount the day moves (water, sodium, potassium, chloride, oxygen, carbon dioxide, heat and energy, the fuels, nitrogen, fixed acid, the minerals, the blood's cells and proteins): where it enters, where it is made or used, where it leaves, and which law holds it. Each book closes on paper for the reference man's day before any law is written
- [x] For each system: what is dynamic at rung 0, what is a held store with explicit balanced flows, what is absent, and the behaviors the system cannot show at rung 0 (record 0002 asks a grouped region for the same list)
- [x] The species, spaces and sites those laws need, as the list ticket 03 builds from. Each site the body adds beyond the slice's names the check or the reading that earns it (record 0002)
- [x] Which pass each item belongs to. Pass 1 is every loop that a monitor reading depends on or that drifts within a day. Pass 2 is the slow books
- [x] The holds rung 0 keeps, such as abdominal pressure, each with its reason
- [x] For every coupling arrow and each of its fields: the ticket that wires it, an anatomy relation, or a hold with its reason
- [x] An owner for each fork the docs leave open: the nitrogen, acid and carbon dioxide ledgers, the potassium shift into cells, the bone pools, the natriuretic peptide's secretion, cell water, the pleural space, the bladder
- [x] The atlas card trims this implies, listed for [foundation 06](../../foundation/issues/06-atlas.md) to apply as each system is built
- [x] For each loop, the links across systems that the evidence library holds, set beside the coupling map's arrows: wired, held, or a lead the coupling map lacks. A lead goes to the evidence library's process and adds no arrow here

**Start from:** the evidence on hand, which the [body map](../map.md)'s Notes locate: the curation study's `needs.md` for what each system needs, and the evidence library's links for each loop. Then read `~/lanes/logs/ng-q1-healthy-body-draft.md` (sections 2 to 8: the balance sheets, the systems derived from them, a model named for each rung, and the rules that contain growth) and `~/lanes/logs/ng-q1-frame-followup.md` (sections 3 and 4: what must be dynamic once the day has meals and sleep, and the smallest clock and sleep). They are model proposals, never sources (SRS rule 16), and they predate the contract, so restate them in the contract's and the glossary's words. Numbers stay out: this ticket fixes shape, and the research tickets fix forms and values.

**Done when** each book closes on paper with only the laws the tables list, and a builder can tell from the Answer alone which holds each later ticket releases.

## Answer

Rung 0 is the whole body at the body map's destination. Each system has the smallest set of laws without which an amount the day moves cannot close, has no controller, or cannot produce a reading in the milestone's lens. The tables below are written for the builders of tickets 03 to 06 and for each loop ticket. This Answer fixes shape only. The research tickets fix forms and values, so no clinical number appears here.

### How to read the tables

- The ticket says "books". The glossary's word is conserved quantity, which this Answer uses.
- Each row names a law and the system whose laws hold it. No system owns a pool, because the state holds every amount (SRS section 2).
- Ticket 06 builds the whole body with every row on a held stand-in. The **Ticket** column names the ticket that replaces that stand-in with the row's law. "slice" means the slice already has the law and ticket 06 carries it over. "05" means a boundary input. "kept" means rung 0 keeps the hold, for the reason given.
- In a system table, **dynamic** rows are laws the system declares at rung 0. **Held** rows are stores or values a held stand-in keeps at rung 0, with their reason. **Absent** rows are not in the body at all. **Cannot show** lists the behaviors record 0002 asks a grouped region to state.
- Sites are the slice's (`crates/ng-validate/src/slice.rs`), on the site classes of the [solver contract](../../../docs/architecture/solver-contract.md) section 1, plus the sites listed for ticket 03 below. The slice's splanchnic bed is the gut bed, its splanchnic veins are the portal vein, and its leg bed is the muscle bed.
- "Every watery space" means the plasma, interstitium and cells of each bed and the lumen of each vessel on the blood network. The gut and urinary lumens are named apart.
- Each bed has two walls, and each route family across them has one owner. FL owns every crossing of the capillary wall and the cell membrane by water, ions and other solutes, and the fuels' crossings of the capillary wall. MT owns the crossings of both walls by oxygen and carbon dioxide, and the crossings of the cell membrane by fuels, except at the liver bed, where HP's glucose hub owns them. Two cargoes have another owner at the cell membrane. MS owns the minerals' crossings at the bone bed (19), and HC owns iron's crossings at the liver and gut beds (21).
- Where a row states acidity from ticket 08, it follows the default for a choice that waits on a ruling (see what this changes in the plan).

### The day's conserved quantities

Each table closes on paper for the reference man's day. Its closure line sums every way in against every way out and the change in what the body stores. Over 24 hours the change in stores is held to the certificate's drift bound, not to zero (record 0004). The two drafts give each quantity a balance with numbers (healthy-body draft section 2), which these tables restate in the contract's words without the numbers.

#### Water

Pass 1. Vasopressin on the collecting duct controls it, answering osmolality, volume and the night (17), and so does the kidney's pressure answer (17). Thirst is absent, because the day's drinks are scheduled (record 0004).

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
| airway evaporation | leaves | pulmonary bed interstitium, out through the airway gas | water drawn from the pulmonary bed's interstitium humidifies inspired gas as vapor, which leaves in expired gas | RS | 10 |
| feces water | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: drinks + food water + metabolic water = urine + skin evaporation + sweat + airway evaporation + feces water + change in stores.

Water is stored in every watery space, cells included. Metabolic water needs ticket 02's reaction, because a transfer never changes a species today (`crates/ng-contract/src/conservation.rs`).

#### Sodium

Pass 1. Aldosterone through renin and angiotensin II, the natriuretic peptide, and pressure natriuresis control it (17). The store integrates intake less excretion, which gives the loop its integral action (a model proposal in the healthy-body draft, section 2.2, for research 14 to source).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal sodium | enters | stomach lumen | the meal's delivery | boundary | 05 |
| absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | slice |
| cell sodium | moves | interstitium and cells | the sodium pump against a leak | FL | 17 |
| filtration and reclaim | moves | glomerular bed, tubule, collecting duct | filtration, and saturable reclaim under aldosterone, the peptide and arterial pressure | RN | slice, 17 |
| urine sodium | leaves | bladder | voiding | RN | slice, 22 |
| sweat sodium | leaves | skin bed | sweat carries sodium with its water | MT | 12 |
| feces sodium | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal sodium = urine sodium + sweat sodium + feces sodium + change in stores.

#### Potassium

Pass 1. Insulin moves a meal's potassium into cells within minutes (15), and aldosterone raises its excretion over hours (17). A meal carries a large share of the extracellular store, so the shift into cells is dynamic at rung 0 (a model proposal in the frame follow-up, section 3a, for research 13 to source).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal potassium | enters | stomach lumen | the meal's delivery | boundary | 05 |
| absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | slice |
| shift into cells | moves | interstitium to cells of the muscle and fat beds | uptake that insulin drives through the receptor layer | FL | 15 |
| pump and leak | moves | cells and interstitium of every bed | the sodium pump against a leak, so what insulin moves in can return | FL | 17 |
| filtration, reclaim and secretion | moves | glomerular bed, tubule, collecting duct | filtration, saturable reclaim, and secretion under aldosterone | RN | slice, 17 |
| urine potassium | leaves | bladder | voiding | RN | slice, 22 |
| feces potassium | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal potassium = urine potassium + feces potassium + change in stores.

#### Chloride

Pass 1. Chloride has no controller of its own. Its reclaim follows sodium's, as in the slice, and the kidney's feedback on the strong ion difference sets how much chloride leaves beside sodium (18).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal chloride | enters | stomach lumen | the meal's delivery | boundary | 05 |
| absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| circulation and capillary exchange | moves | blood and interstitium | advection, Starling exchange and lymph | CV, FL | slice |
| filtration and reclaim | moves | glomerular bed, tubule, collecting duct | reclaim follows sodium's, and the acid loop's feedback on the strong ion difference | RN | slice, 18 |
| urine chloride | leaves | bladder | voiding | RN | slice, 22 |
| sweat chloride | leaves | skin bed | sweat carries chloride with its water | MT | 12 |
| feces chloride | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal chloride = urine chloride + sweat chloride + feces chloride + change in stores.

Cells hold no chloride at rung 0. The cell anion, a species that never crosses the cell membrane, stands for the cells' anions (see the species list).

#### Oxygen

Pass 1. Ventilation under the chemoreflex controls it, through a peripheral sensor on arterial oxygen (11), and so does each bed's flow under its oxygen debt (09).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| inspired oxygen | enters | conducting airway | inspiration of the room's gas | RS | 10 |
| alveolar uptake | moves | alveolar gas to pulmonary bed plasma | diffusion across the gas-exchange surface, with a shunt | RS | 10 |
| carriage | moves | blood | advection, with binding to hemoglobin as speciation | CV, BC | 08 |
| tissue delivery | moves | bed plasma, interstitium, cells | diffusion across the capillary wall and the cell membrane | MT | 08 |
| oxygen use | used | cells of every bed | the bed's oxidation reaction | MT | 09 |
| expired oxygen | leaves | conducting airway | expiration | RS | 10 |

Closes on paper: inspired oxygen = expired oxygen + oxygen use + change in stores.

Oxygen is stored in alveolar gas, in blood (dissolved and bound) and in tissue. Until ticket 10, oxygen enters by a fixed exchange at the lung, and until ticket 09 it leaves by a fixed use at each bed (ticket 08).

#### Carbon dioxide

Pass 1. Ventilation under the central chemoreflex controls it, reading the acidity of a brain space (11).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| carbon dioxide made | made | cells of every bed | the bed's oxidation reaction | MT | 09 |
| tissue store | moves | cells and interstitium of every bed | the carbon dioxide a bed makes fills the bed's tissue water, and diffusion across both walls empties it into blood | MT | 09 |
| carriage | moves | blood | advection, with dissolved, bicarbonate and bound forms as speciation in the strong-ion frame | CV, BC | 08 |
| alveolar elimination | moves | pulmonary bed plasma to alveolar gas | diffusion across the gas-exchange surface | RS | 10 |
| inspired carbon dioxide | enters | conducting airway | the room's inspired gas | RS | 10 |
| expired carbon dioxide | leaves | conducting airway | expiration | RS | 10 |
| urine carbon dioxide | leaves | bladder | total carbon dioxide filtered and not reclaimed | RN | 18 |

Closes on paper: carbon dioxide made + inspired carbon dioxide = expired carbon dioxide + urine carbon dioxide + change in stores.

The species is total carbon dioxide in each space. Bicarbonate is a speciation of it, never its own amount, so the gas loop and the acid loop move one amount. Until ticket 18 the glomerular filter keeps carbon dioxide back.

#### Heat and energy

Pass 1. The thermostat controls it, with a set point that follows the clock and steps down in sleep, acting through skin vessel tone, sweat and shivering (12). The metabolic rate's fall in sleep (25) and the heat a meal adds (16) move it too.

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| food energy | enters | stomach lumen | the energy each fuel species carries in the meal's delivery | boundary | 05 |
| gravity's work | enters | every vessel segment | posture's work, a receipt that nets over the day | CV | slice |
| fuel energy to heat | moves | cells of every bed | the oxidation reaction gives each fuel's energy as heat, less the work it pays | MT | 09 |
| heat of conversion | moves | liver bed cells | each conversion (glycogen, gluconeogenesis, urea, ketones) gives its energy difference as heat | HP | 14, 16 |
| work to heat | moves | heart, vessel walls, respiratory pump | the walls' and the pump's work, paid by fuel at a bed, dissipates as heat | CV, MT | slice, 09, 10 |
| heat carried by blood | moves | core and skin heat stores | blood carries heat between the two stores | CV | 12 |
| conduction | moves | core to skin | conduction between the two stores | MT | 12 |
| dry heat loss | leaves | skin heat store | radiation and convection to the room | MT | 12 |
| evaporative heat | leaves | skin heat store, and the core heat store through the airway gas | latent heat with skin evaporation and sweat (MT), and with airway water (RS) | MT, RS | 12, 10 |
| warming inspired gas | leaves | core heat store, out through the airway gas | heat drawn from the core heat store warms inspired gas, which leaves warmer than it came in | RS | 10 |
| feces energy | leaves | small intestine lumen | unabsorbed fuel in the balanced fecal exit | GI | 13 |
| urine energy | leaves | bladder | the energy urea and creatinine carry out | RN | 14, 16 |

Closes on paper: food energy + gravity's work = dry heat loss + evaporative heat + warming inspired gas + feces energy + urine energy + change in stores.

Energy is stored as heat in the core and skin stores, as chemical energy in every fuel pool (glycogen, triglyceride and body protein the largest), and the walls' elastic store. Until ticket 09 the heart's work is paid by the exterior chemical energy MT stands behind in the slice, and until tickets 10 and 12 heat leaves by held exits at the skin and the airway. How a reaction carries each fuel's energy is ticket 02's choice.

#### The fuels

Pass 1. Insulin and glucagon control them (15). No loop holds the stores within hours. Over the day the stores close because intake equals what the body spends (record 0004), and the respiratory quotient follows the fuel burned (16).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal glucose | enters | stomach lumen | the meal's carbohydrate, delivered as glucose | boundary | 05 |
| glucose absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| liver glycogen | moves | liver bed cells | glycogen made and broken down under portal insulin and glucagon | HP | 14 |
| gluconeogenesis | made | liver bed cells | glucose from lactate, amino acids and glycerol, as reactions | HP | 14 |
| glucose uptake | moves | cells of every bed | uptake without insulin in every bed, and more with insulin in muscle and fat | MT | 09, 15 |
| muscle glycogen | moves | muscle bed cells | glycogen made and used in muscle | MS | 16 |
| renal glucose reclaim | moves | glomerular bed, tubule | filtration, and reclaim up to its limit | RN | 15 |
| glucose oxidation | used | cells of every bed | the oxidation reaction, with fatty acids in a fixed proportion until ticket 16 | MT | 09, 16 |
| glycolysis to lactate | used | cells of every bed | glycolysis wherever supply falls short, as research 11's form makes it | MT | 09 |
| lactate made | made | cells of every bed | the product of glycolysis | MT | 09 |
| lactate use | used | cells of the beds that burn it, and liver bed cells | oxidation (09), and gluconeogenesis (14) | MT, HP | 09, 14 |
| meal fat | enters | stomach lumen | the meal's fat | boundary | 05 |
| fat absorption | moves | small intestine lumen to the gut bed's interstitium, then by lymph | fat by the lymph route, since the gut bed's capillary wall keeps it back | GI | 13 |
| fat storage and release | moves | fat bed cells | triglyceride stored after meals, and fatty acids and glycerol released under low insulin | MT | 16 |
| fatty acid oxidation | used | cells of every bed but the brain's | the oxidation reaction | MT | 09, 16 |
| ketones made | made | liver bed cells | ketones from fatty acids, overnight | HP | 16 |
| ketogenesis | used | liver bed cells | fatty acids turned into ketones | HP | 16 |
| ketone oxidation | used | cells of every bed | the oxidation reaction | MT | 16 |
| glycerol to glucose | used | liver bed cells | glycerol taken into gluconeogenesis | HP | 14 |
| meal protein | enters | stomach lumen | the meal's protein, delivered as amino acids | boundary | 05 |
| amino acid absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| protein turnover | moves | muscle and liver bed cells | body protein made and broken down at its smallest | MS | 16 |
| amino acid use | used | liver bed cells | deamination into urea, with the sulfur posted as fixed acid and the carbon made into glucose | HP | 14, 18 |
| amino acids to ammonium | used | peritubular bed cells | ammonium made for acid excretion | RN | 18 |
| protein to creatinine | used | muscle bed cells | creatinine made from muscle | MS | 16 |
| feces fuel | leaves | small intestine lumen | unabsorbed fuel in the balanced fecal exit | GI | 13 |

Closes on paper (glucose and glycogen): meal glucose + gluconeogenesis = glucose oxidation + glycolysis to lactate + feces fuel + change in stores.

Closes on paper (lactate): lactate made = lactate use + change in stores.

Closes on paper (fat): meal fat = fatty acid oxidation + ketogenesis + glycerol to glucose + feces fuel + change in stores.

Closes on paper (ketones): ketones made = ketone oxidation + change in stores.

Closes on paper (amino acids and body protein): meal protein = amino acid use + amino acids to ammonium + protein to creatinine + feces fuel + change in stores.

Ticket 09 burns glucose and fatty acids in a fixed proportion, which fixes the respiratory quotient, and it oxidizes lactate. Each is supplied by its held stand-in until tickets 13 to 16 build its sources. Each fuel's carbon leaves as carbon dioxide, except the carbon urea and creatinine carry out, and its hydrogen leaves as water. The carbon dioxide and water tables carry both.

#### Nitrogen

Pass 1. No controller acts within a day, and enzyme induction acts over days (a model proposal in the healthy-body draft, section 2.8). Amino acid use rises with supply, so urea's making and excretion follow intake.

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| meal nitrogen | enters | stomach lumen | the meal's amino acids | boundary | 05 |
| amino acid absorption | moves | small intestine lumen to portal blood | absorption | GI | 13 |
| protein turnover | moves | muscle and liver bed cells | body protein made and broken down | MS | 16 |
| urea synthesis | moves | liver bed cells | urea from amino acids, as a reaction | HP | 14 |
| ammonium made | moves | peritubular bed cells, then the tubule lumen | ammonium made from amino acids in the kidney's cells and secreted, for acid excretion | RN | 18 |
| creatinine made | moves | muscle bed cells | creatinine made from muscle | MS | 16 |
| urine urea | leaves | bladder | filtration, with part reclaimed | RN | 14 |
| urine ammonium | leaves | bladder | ammonium excretion | RN | 18 |
| urine creatinine | leaves | bladder | filtration, with none reclaimed | RN | 16 |
| feces nitrogen | leaves | small intestine lumen | the balanced fecal exit | GI | 13 |

Closes on paper: meal nitrogen = urine urea + urine ammonium + urine creatinine + feces nitrogen + change in stores.

Ammonia has no pool at rung 0, because urea is made from amino acids in one reaction (14). The peritubular bed's cells stand for the tubule's cells, where ammonium is made. Albumin's and hemoglobin's nitrogen stays in their own conserved quantities, whose making and loss balance (21), so they leave this table unchanged.

#### Fixed acid

Pass 1. The kidney's feedback on the strong ion difference controls it over hours (18), and the chemoreflex answers through carbon dioxide within minutes (11). The draft's "net acid excretion" becomes, in the strong-ion frame, the kidney excreting chloride and the lumped anion with ammonium instead of with sodium. The diet's organic anions are its base: tissues burn them to total carbon dioxide.

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| fixed acid made | made | liver bed cells | the liver's amino acid use posts the sulfur of the diet's protein as the lumped strong anion | HP | 18 |
| fixed acid out of the liver's cells | moves | liver bed cells to liver bed interstitium | crossing of the cell membrane | FL | 18 |
| diet base | enters | stomach lumen | the meal's organic anions, delivered with its cations | boundary | 05 |
| diet base absorption | moves | small intestine lumen to portal blood | absorption | GI | 18 |
| diet base burned | used | cells of every bed | the oxidation reaction burns the organic anion to total carbon dioxide | MT | 18 |
| speciation | moves | every watery space | acidity stated from the strong ion difference, the weak acids and carbon dioxide | BC | 08 |
| fixed acid excreted | leaves | bladder | the kidney's excretion of the lumped anion with ammonium, as a feedback on the strong ion difference | RN | 18 |

Closes on paper (fixed acid): fixed acid made = fixed acid excreted + change in stores.

Closes on paper (diet base): diet base = diet base burned + change in stores.

The lumped anion is made in the liver bed's cells, crosses out of them, and moves through every watery space outside cells. The cells' own anions are a separate species that never crosses the cell membrane. Lactate and ketones are strong anions after speciation too, and they close in the fuels table. Hydrogen ion and bicarbonate are never amounts. Until ticket 18 the synthetic meal pairs its cations with chloride, as the slice's does.

#### The minerals

Pass 2. If research 06 keeps total calcium and phosphate in the lens, they are pass 1 (see what this changes in the plan). Parathyroid hormone holds calcium, and the kidney's saturable reclaim returns phosphate and magnesium after a load (19).

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

Until ticket 19 each mineral is a held store with balanced flows. Absorption in and excretion out run at equal synthetic rates, and bone and cell exchange are held. Phosphate is a weak acid in the strong-ion frame, so ticket 18 reads its total.

#### The blood's cells and proteins

Pass 2. No controller acts. Each store loses mass in proportion to its size, with production to match, so it returns to balance by itself (21).

| Term | Kind | Where | Law | Owner | Ticket |
|---|---|---|---|---|---|
| hemoglobin production | made | central veins | red cell production to match the loss, entering the blood | HC | 21 |
| hemoglobin carriage | moves | blood | advection; hemoglobin crosses no capillary wall | CV | 06 |
| hemoglobin destruction | used | gut bed blood | first-order loss in proportion to the red cell mass | HC | 21 |
| meal iron | enters | stomach lumen | the meal's delivery | boundary | 05 |
| iron absorption | moves | small intestine lumen to portal blood | absorption, to match the loss | GI | 21 |
| iron carriage | moves | blood and interstitium | advection, and exchange across the capillary wall | CV, FL | 21 |
| iron into the store | moves | liver bed interstitium to liver bed cells | uptake into the store | HC | 21 |
| iron out of the store | moves | liver bed cells to liver bed interstitium | release in proportion to the store | HC | 21 |
| iron into the gut's cells | moves | gut bed interstitium to gut bed cells | uptake by the cells that line the gut | HC | 21 |
| iron shed | moves | gut bed cells to the small intestine lumen | the gut's cells shed, carrying their iron in proportion to what they hold | GI | 21 |
| feces iron | leaves | small intestine lumen | the balanced fecal exit, carrying the unabsorbed share and the shed iron | GI | 21 |
| albumin synthesis | made | liver bed plasma | constant synthesis, then production to match a first-order loss | HP | 14, 21 |
| albumin exchange | moves | every bed | Starling exchange with albumin's reflection, and lymph | FL | slice |
| albumin loss | used | liver bed plasma | a held loss equal to the constant synthesis, then first-order loss in proportion to the store | HP | 14, 21 |

Closes on paper (hemoglobin): hemoglobin production = hemoglobin destruction + change in stores.

Closes on paper (iron): meal iron = feces iron + change in stores.

Closes on paper (albumin): albumin synthesis = albumin loss + change in stores.

Hemoglobin is the red cell mass at rung 0, one amount whose partial volume stands for the red cells' volume (08, 21). Its making and loss are balanced flows that draw no iron from the store and return none. Platelets, leukocytes, globulins and clotting factors are absent, as coagulation and immunity at rest have zero flux (21).

### The 13 systems at rung 0

Each row is one law or one stand-in. **Ticket** names the ticket that makes a dynamic row live, so ticket 06 holds every dynamic row until that ticket. A held row here is one that rung 0 keeps.

#### CV. Cardiovascular

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The pulsatile heart: four chambers with time-varying elastance, valves as modes, the rhythm's phase | the slice's laws (map default: pulsatile at rung 0) | slice |
| dynamic | Compliance and resistance on every vascular edge of the whole body's map, except the liver bed's, which HP's sinusoidal law holds | the slice's edge laws on the new beds | 06 |
| dynamic | Advection of every species the blood carries, hemoglobin included | the slice's advection, extended to each species a route carries | 06 |
| dynamic | Hydrostatic head from site positions under posture | arrow 96, an anatomy relation | slice |
| dynamic | Rate, contractility and vessel tone from receptor occupancy | the receptor layer | slice |
| dynamic | Local flow that answers each bed's oxygen debt | arrow 90 | 09 |
| dynamic | Viscosity from hematocrit, read by every vessel law | arrow 114 | 08 |
| dynamic | Skin vessel tone under the thermostat's release at the skin junction | the receptor layer at the skin bed | 12 |
| dynamic | Heat carried by blood between the core and skin heat stores | a heat advection law | 12 |
| held | Every CV law runs at a reference temperature | no rung 0 CV law reads temperature (arrow 90's temperature field) | kept |
| held | Ventricular wall stress as a field | one natriuretic peptide is secreted, on atrial stretch (arrow 79) | kept |
| absent | The arrest rhythms, their hazard, and cardiac conduction | no healthy day enters them | none |
| absent | The ECG waveform | open: whether a law or an observer makes it, and no ticket plans it (body map, Not yet specified) | none |
| absent | Coronary territories, collaterals and watershed zones | one coronary bed stands for the heart's tissue (record 0002) | none |
| cannot show | Arrhythmia, arrest, ischemia by coronary territory, tamponade, and pooling within one limb | the map holds none of the structure these need | none |

#### RS. Respiratory

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | One alveolar unit with a shunt and a dead space, filled by a tidal breath | the equation of motion for a breath (map default; ticket 10 prototypes quasi-steady alveolar gas before it chooses) | 10 |
| dynamic | Diffusion of oxygen and carbon dioxide across the gas-exchange surface | alveolar exchange, reading pulmonary capillary flow (arrow 81) | 10 |
| dynamic | The pleural space's pressure, felt by every bed it encloses | the breath's pleural pressure replaces the slice's held constant (arrow 82) | 10 |
| dynamic | The resting lung volume, which follows posture | the equation of motion's resting point | 10 |
| dynamic | Water vapor and heat that leave in expired gas, drawn from the pulmonary bed's interstitium and the core heat store | airway heat and water loss | 10 |
| dynamic | The work of breathing charged to metabolism, and the pump's load on MS | arrows 84 and 85 | 10 |
| dynamic | Lung stretch reported to NS | arrow 83 | 11 |
| held | Lung water | no rung 0 lung law reads it (arrow 110) | kept |
| held | Intra-abdominal pressure on the diaphragm | a held constant on a healthy day (arrow 103) | kept |
| absent | More than one alveolar unit, airway resistance by generation, hypoxic vasoconstriction, the bronchial circulation, cough | the first three are ticket 10's non-goals and cough is ticket 11's. The bronchial circulation is absent at rung 0, as one unit stands for the lungs (record 0002) | none |
| absent | Pleural fluid as an amount | a left-out space: no law moves it on a healthy day | none |
| cannot show | Ventilation and perfusion mismatch, regional disease, atelectasis, pneumothorax, effusion, bronchospasm | one unit cannot hold them | none |

#### NS. Nervous

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The baroreflex: stretch afferents, and norepinephrine and acetylcholine release, reuptake and clearance at every junction | the slice's laws (arrow 77) | slice |
| dynamic | The chemoreflex: a central sensor on the brain bed's acidity, which BC states from ticket 08, and a peripheral sensor on arterial oxygen and carbon dioxide, driving the pump's junction. Sleep shifts its set point and lowers its gain | arrows 86 and 88 | 11 |
| dynamic | The thermostat: core temperature (arrow 91) and skin temperature against a set point that follows the clock and steps down in sleep, driving release on skin vessels and sweat glands at the skin junction, and the shiver drive | arrows 87 and 91 | 12 |
| dynamic | The osmoreceptor and the volume drive to vasopressin | arrow 89's vasopressin field | 17 |
| dynamic | Sleep's gain on sympathetic traffic | the sleep state input (05) | 25 |
| held | The circadian phase and the CRH drive to EN | the clock reaches every law as a boundary input (05), as foundation 06 says for arrow 89, so NS relays neither | kept |
| held | The thermostat's set point and its sweat drive as fields of arrow 87 | the set point is NS memory, and sweat answers release at the skin junction | kept |
| held | Brain water and intracranial pressure | no skull enclosure at rung 0 (arrow 111) | kept |
| held | Ammonia's effect on the brain, and the fever set point shift | no ammonia pool, and immunity at rest (arrows 100 and 108) | kept |
| absent | A clock oscillator, REM sleep, and sleep the body decides | the non-goals of tickets 05 and 25 | none |
| absent | Consciousness, pain, voluntary movement and breathing, thirst, cerebral autoregulation, transport delays | no conserved quantity or lens reading needs them on a healthy day | none |
| cannot show | Orthostatic syncope, raised intracranial pressure, stroke, sedation, delirium, seizure, pain, periodic breathing | the laws they need are absent | none |

#### RN. Renal

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Filtration, one grouped tubule and collecting duct, water reclaim under vasopressin, saturable sodium and potassium reclaim, and tubuloglomerular feedback | the slice's laws (arrow 97) | slice |
| dynamic | Filtration and reclaim of each new small solute: glucose up to its limit, urea in part, creatinine not at all, total carbon dioxide | tubular laws per species | 14, 15, 16, 18 |
| dynamic | Renin secretion | arrow 98 | 17 |
| dynamic | Sodium reclaim that answers aldosterone, the peptide and the renal bed's own pressure, so pressure natriuresis shows as a check | the kidney's pressure answer (curation study, need RN-L04: a check, never a law) | 17 |
| dynamic | Potassium secretion under aldosterone, and the clock's gain on filtration | the clock input (05) | 17 |
| dynamic | Ammonium made from amino acids in the peritubular bed's cells, which stand for the tubule's cells, and secreted with chloride and the lumped anion, as the feedback on the strong ion difference | | 18 |
| dynamic | Calcium reclaim under parathyroid hormone, and saturable phosphate and magnesium reclaim | | 19 |
| dynamic | The bladder: urine collects, and leaves at a void that is an event of the day | | 22 |
| held | Intra-abdominal pressure on the kidney | a held constant on a healthy day (arrow 102) | kept |
| absent | Erythropoietin and calcitriol as moving hormones | the non-goals of tickets 19 and 21 | none |
| absent | Nephrons by type, the medulla's gradient, renal nerves beyond the slice's, drug excretion, dialysis | one grouped nephron stands for all (record 0002), and renal nerves beyond the slice's are ticket 17's non-goal. Drugs and dialysis are absent at rung 0 | none |
| cannot show | Acute kidney injury, obstruction, a concentrating defect from the medulla, the urge to void, bladder pressure | one grouped nephron and no bladder wall | none |

#### HP. Hepatic

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The liver bed on the portal circuit, taking portal and arterial blood through its sinusoidal edge law | arrow 99, an anatomy relation | 14 |
| dynamic | Glycogen made and broken down under portal insulin and glucagon | | 14 |
| dynamic | Gluconeogenesis from lactate, amino acids and glycerol, and urea from amino acids, each reaction giving its energy difference as heat | reactions (ticket 02) | 14 |
| dynamic | The sulfur of the diet's protein posted as fixed acid by the amino acid reaction | | 18 |
| dynamic | Albumin synthesis: constant with a matching held loss, then production to match a first-order loss | | 14, 21 |
| dynamic | First-pass clearance of insulin | | 15 |
| dynamic | Ketones from fatty acids overnight | | 16 |
| dynamic | Cortisol's action on the glucose output | the receptor layer | 20 |
| held | Every HP law runs at a reference temperature | arrow 93 | kept |
| held | Ammonia | urea is made with no ammonia pool, so arrow 100 carries nothing | kept |
| absent | Angiotensinogen as an amount | its supply is a parameter of angiotensin II's making (17) | none |
| absent | Bile, the enterohepatic circulation, drug metabolism, clotting factors, hepcidin, zones within the liver | bile, drug clearance and zones are ticket 14's non-goals, and clotting factors go with coagulation at rest (21). The enterohepatic circulation and hepcidin are absent at rung 0 | none |
| cannot show | Liver failure, cholestasis, encephalopathy, coagulopathy, first-pass drug metabolism | the laws they need are absent | none |

#### GI. Gastrointestinal

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Stomach and small intestine lumens on the gut network, with gastric emptying | | 13 |
| dynamic | Absorption of water, sodium, potassium, chloride, glucose and amino acids into portal blood, and of fat by the lymph route | arrow 104, an anatomy relation | 13 |
| dynamic | The balanced fecal exit of each meal's unabsorbed share | | 13 |
| dynamic | Absorption of the diet's organic anion | | 18 |
| dynamic | Absorption of calcium, phosphate and magnesium | | 19 |
| dynamic | Absorption of iron to match its loss, and the shedding that carries the gut's cells and their iron into the lumen | | 21 |
| held | Gut secretion and its reabsorption | one balanced held flow (ticket 13) | kept |
| held | Abdominal pressure | GI's held compliance on the abdominal space, felt by every bed it encloses (arrows 101 to 103) | kept |
| held | Gut wall water | no rung 0 gut law reads it (arrow 112) | kept |
| absent | The colon, motility under nervous control, the enteric nervous system, gut hormones, bleeding into the lumen | the colon and motility under nervous control are ticket 13's non-goals, and gut hormones ticket 15's. The enteric nervous system and bleeding into the lumen are absent at rung 0 | none |
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
| dynamic | Acidity in every watery space from the strong ion difference, the weak acids and total carbon dioxide, with carbon dioxide's dissolved, bicarbonate and bound forms | algebraic rows (map default). By default ticket 08 states them with the lumped anion and the diet's organic anion held, and ticket 18 adds their making and excretion. Which ticket states them waits on a ruling (what this changes in the plan) | 08 |
| dynamic | Ionized calcium | an algebraic row | 19 |
| held | Every speciation runs at a reference temperature | arrow 94 | kept |
| absent | Transfers across the cell membrane | FL and MT own the membrane's routes, so BC writes no amount | none |
| absent | Red cell chemistry, buffering inside cells and bone, drug binding | buffering inside cells and bone are ticket 18's non-goals. Red cells have no space, because hemoglobin is a mass in blood (08), and there are no drugs | none |
| cannot show | The oxygen curve's adaptation, intracellular acidosis, bone's buffering in chronic acidosis, the alkaline tide after a meal unless research 14 sources its size | the laws they need are absent | none |

#### HC. Hematology and coagulation

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Hemoglobin as a mass in blood, so its concentration follows dilution and hematocrit sets viscosity | arrow 114's viscosity field | 08 |
| dynamic | Red cell mass lost in proportion to its size, with production to match | | 21 |
| dynamic | Iron's store in the liver bed's cells, filled from blood and released to it in proportion to its size, and iron's uptake by the gut's cells, which shed it into the lumen | | 21 |
| held | Coagulation's fields | held at zero, and ticket 21 declares coagulation at zero flux with its cannot-show list (arrow 114's bleeding competence and thrombosis fields) | kept |
| held | Every HC law runs at a reference temperature | arrow 92 | kept |
| absent | Platelets, leukocytes, clotting factors, red cell age, erythropoietin as a moving hormone | ticket 21's non-goals | none |
| cannot show | Bleeding, clotting, thrombosis, erythropoietin's answer to hypoxia or bleeding, iron limiting red cell production, polycythemia, a white cell response | the laws they need are absent | none |

#### IM. Immune

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | None: ticket 21 replaces the held card with a declaration of zero flux and its cannot-show list | immunity at rest (map default) | 21 |
| held | iNOS, myocardial depression, permeability, glycocalyx, tissue factor, hepcidin, the fever set point shift, mitochondrial efficiency | each held at zero or at the healthy value (arrows 105 to 109) | kept |
| absent | Pathogens, mediators, phagocytes, antibodies | no healthy day shows them | none |
| cannot show | Infection, sepsis, fever, anaphylaxis, inflammation, wound healing | ticket 21 puts this list in the body model | none |

#### FL. Fluids and endothelium

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | Starling exchange at every bed, interstitial pressure, and lymph return | the slice's laws (arrow 80) | slice |
| dynamic | Each new solute's crossing of the capillary wall, gases excepted | the slice's exchange law per species, or a keep-back until the species' ticket | 06 |
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
| dynamic | Oxygen and carbon dioxide across the capillary wall and the cell membrane, and the tissue carbon dioxide store | diffusion | 08, 09 |
| dynamic | Each bed burns fuel with oxygen into carbon dioxide, water and heat: glucose and fatty acids in a fixed proportion with lactate's oxidation, then each bed's own mix | a reaction (ticket 02); arrow 78 | 09, 16 |
| dynamic | Oxygen debt and lactate when supply falls short | arrow 90 | 09 |
| dynamic | Fuels across the cell membrane: glucose without insulin in every bed and with insulin in muscle and fat, and the other fuels | the receptor layer | 09, 15, 16 |
| dynamic | Fat stored after meals and released overnight, fatty acid and ketone oxidation, and the heat a meal adds | | 16 |
| dynamic | The diet's organic anion burned to total carbon dioxide | | 18 |
| dynamic | Two heat stores, core and skin, with conduction between them, radiation, convection, evaporation and sweat to the room, sweat answering release at the skin junction, and shivering's heat in the muscle bed | arrows 87 and 91 | 12 |
| dynamic | Sleep's gain on each bed's metabolic rate | the sleep state input (05) | 25 |
| held | Mitochondrial efficiency | the healthy value (arrow 109) | kept |
| absent | More than two heat stores, exercise and fever (ticket 12's non-goals), and temperature's effect on rate (ticket 09's) | | none |
| cannot show | Heat illness, regional temperature differences, starvation | the structure and laws they need are absent | none |

#### MS. Musculoskeletal and skin

| Kind | What | Law or reason | Ticket |
|---|---|---|---|
| dynamic | The respiratory pump's force, which RS reads, and its junction, where NS's drive arrives as release | arrows 85, 88 and 95 | 10, 11 |
| dynamic | Muscle glycogen and body protein turning over at their smallest, and creatinine made from muscle | | 16 |
| dynamic | Bone's partner pool for calcium, phosphate and magnesium | | 19 |
| held | Respiratory muscle fatigue | a resting day has no fatigue law (arrow 95) | kept |
| held | Skin and compartment water | arrow 113 | kept |
| absent | Voluntary movement, posture as muscle work, junctions other than the pump's, bone remodeling, pressure injury | posture is a boundary input; no healthy day needs the rest | none |
| cannot show | Weakness, fatigue, rhabdomyolysis, compartment syndrome, fracture, pressure injury | the laws they need are absent | none |

### The species, spaces and sites ticket 03 builds from

Ticket 03 adds these once, so no loop ticket edits a closed list. "Blood" means each vessel lumen and each bed's plasma.

| Species | Measure | Charge | Conserved quantities | Spaces | First moved by |
|---|---|---|---|---|---|
| oxygen | amount | 0 | oxygen | every watery space, alveolar gas, airway gas | 08 |
| carbon dioxide, total | amount | a neutral moiety; speciation gives bicarbonate's charge | carbon dioxide | every watery space, urinary lumens, alveolar gas, airway gas | 08 |
| nitrogen gas | amount | 0 | its own: it enters and leaves with each breath and crosses no wall | alveolar gas, airway gas | 10 |
| water (exists) | volume | 0 | water | adds vapor in the alveolar and airway gas | 10 |
| glucose | amount | 0 | the fuels, energy | every watery space, gut and urinary lumens | 09 |
| glycogen | mass | 0 | the fuels, energy | cells of the liver and muscle beds | 14 |
| triglyceride | mass | 0 | the fuels, energy | gut lumen, the gut bed's interstitium, blood, cells of the fat bed | 13 |
| fatty acids | amount | 0 at rung 0 | the fuels, energy | every watery space | 09 |
| glycerol | amount | 0 | the fuels, energy | every watery space | 14 |
| amino acids, one lumped species | amount | 0 | the fuels, nitrogen, energy | every watery space, gut and urinary lumens | 13 |
| body protein | mass | 0 | the fuels, nitrogen, energy | cells of the muscle and liver beds | 16 |
| lactate | amount | a neutral moiety; -1 after speciation | the fuels, energy | every watery space | 09 |
| ketones, one species | amount | a neutral moiety; -1 after speciation | the fuels, energy | every watery space | 16 |
| urea | amount | 0 | nitrogen, energy | every watery space, urinary lumens | 14 |
| ammonium | amount | a neutral moiety; +1 after speciation | nitrogen | peritubular bed cells, urinary lumens | 18 |
| creatinine | amount | 0 | nitrogen, energy | every watery space, urinary lumens | 16 |
| other strong anion, lumped: the fixed acid | amount | a neutral moiety; -1 after speciation | fixed acid | liver bed cells, every watery space outside cells, urinary lumens | 18 |
| diet organic anion, lumped: the diet's base | amount | a neutral moiety; -1 after speciation | fixed acid, energy | gut lumen, every watery space | 18 |
| cell anion | amount | -1 | charge | cells of every bed; it crosses no membrane, held or a site parameter as ticket 03 picks | 17 reads it |
| calcium | amount | +2 | the minerals, charge | every watery space, gut and urinary lumens, bone bed cells | 19 |
| magnesium | amount | +2 | the minerals, charge | every watery space, gut and urinary lumens | 19 |
| phosphate | amount | a weak acid, charged by speciation | the minerals | every watery space, gut and urinary lumens, bone bed cells | 19 |
| hemoglobin | mass | a weak acid, charged by speciation | the blood's cells and proteins | blood only | 06 |
| iron | amount | 0 | the blood's cells and proteins | gut lumen, blood, the interstitium and cells of the liver and gut beds | 21 |
| insulin, glucagon | amount | 0 | each its own | blood, interstitium, bound | 15 |
| renin, angiotensin II, aldosterone, the natriuretic peptide | amount | 0 | each its own | blood, interstitium, bound; renin in blood only | 17 |
| parathyroid hormone | amount | 0 | its own | blood, interstitium, bound | 19 |
| cortisol | amount | 0 | its own | blood, interstitium, bound | 20 |
| albumin (exists) | mass | a weak acid, charged by speciation | the blood's cells and proteins | as in the slice | 08 |

No species is added for hydrogen ion, bicarbonate, ammonia, angiotensinogen, epinephrine or thyroid hormone. The first two are speciation, ammonia has no pool, and the rest are absent at rung 0.

Charge is a default for tickets 02 and 03 (see hard parts). A reaction that makes lactate, ketones, the fixed acid or ammonium would change the sum of fixed charges, and hydrogen ion is never an amount in the strong-ion frame. So each acid or base is a neutral moiety whose charge BC's speciation gives, and every reaction balances charge. Each space's charge after speciation is zero, which electroneutrality holds. A meal delivers its cations with chloride, phosphate and the diet's organic anion, so each delivery is neutral after speciation only. In fixed charges a potassium meal paired with the organic anion adds positive charge, while the certificate holds charge's drift at zero (SRS section 2, and the solver contract's certificate section). Ticket 02's prototype tests which way charge closes. By default the certificate reads charge after speciation, so the sum of fixed charges may move when a cation enters or leaves with an anion counted as a neutral moiety. The alternative gives each strong anion a fixed charge and pairs each reaction that makes or burns one with a counter-charge.

| Space | At | Earned by |
|---|---|---|
| alveolar gas: not watery, its volume follows the gas law, and it holds oxygen, carbon dioxide, nitrogen gas and water vapor | the pulmonary bed | ticket 10's exchange, and the lens's arterial and end-tidal gas readings |
| airway gas: not watery, holding the same gases | the conducting airway | ticket 10's dead space, and the airway's water and heat loss |
| gut lumen: watery, a lumen as the renal tubule's is | the stomach and the small intestine | ticket 13's emptying and absorption |
| heat store | the skin heat site; the core's heat stays at the slice's body site | ticket 12's two heat stores |

Red cells get no space, because hemoglobin is a mass in blood (ticket 08). The skull has no space at rung 0.

| Site | Class | Earned by |
|---|---|---|
| brain bed | capillary bed | the central chemoreceptor reads a brain space's acidity (11), and the brain takes glucose without insulin (15) |
| coronary bed | capillary bed, inside the pleural space | Model default, as no reading names it. The check that would earn it is the slice's posture checks once ticket 09 wires arrow 90, if paying the heart's work at the muscle bed lets the heart's demand open the leg's vessels as the body stands. If they pass without it, the heart's work is paid at the muscle bed |
| liver bed | capillary bed inside the abdominal space, fed by the portal vein and the systemic arteries | the glycogen store and portal concentrations (14), first-pass insulin (15), arrows 99 and 104 |
| muscle bed | capillary bed: the slice's leg bed, standing for all skeletal muscle and the tissue no other bed names | insulin-driven uptake (15), muscle glycogen, body protein and creatinine (16), shivering (12), and the posture checks that earned the leg bed |
| fat bed | capillary bed | Model default, as no reading names fat. The check that would earn it is ticket 16's run in which the respiratory quotient rises after meals and falls overnight, if it fails with the triglyceride store in the muscle bed's cells. If it passes, the store and its laws move to the muscle bed |
| skin bed | capillary bed | skin vessel tone, evaporation and sweat (12) |
| skin heat site | tissue site | the skin heat store and the skin thermoreceptor (12) |
| bone bed | capillary bed | Model default. The readings that would earn it are total calcium and phosphate, which the healthy-range set holds (`data/healthy-range-set.jsonl`), if ticket 19 finds them leaving their ranges after a meal with the kidney's reclaim alone. If they stay in range, bone's partner pool stays a held store |
| stomach | vessel segment on the gut network | emptying (13) |
| small intestine | vessel segment on the gut network | absorption and the fecal exit (13) |
| conducting airway | vessel segment on the airway network | dead space, and the airway's water and heat loss (10) |
| respiratory pump junction | junction | the drive reaches the pump as release (11) |
| skin junction | junction | the thermostat's release on skin vessels and sweat glands (12) |

The gut bed and the portal vein are the slice's splanchnic bed and veins, which now drain into the liver bed. The peritubular bed's cells stand for the tubule's cells.

| Landmark | At | Read by |
|---|---|---|
| central chemoreceptor | brain bed | 11 |
| peripheral chemoreceptor | systemic arteries | 11 |
| skin thermoreceptor | skin heat site | 12 |
| osmoreceptor | brain bed | 17 |
| volume receptor | central veins | 17 |
| adrenal | systemic arteries, reading potassium, with angiotensin II through the receptor layer | 17 |
| islets | systemic arteries, reading glucose | 15 |
| parathyroid | systemic arteries | 19 |

The thermostat reads core temperature through arrow 91, so it needs no landmark of its own.

| Hormone | Enters at | Leaves at | Ticket |
|---|---|---|---|
| insulin, glucagon | the portal vein, from the islets | the liver bed for insulin's first pass, and the central veins | 15 |
| renin | the glomerular bed's plasma | the central veins | 17 |
| angiotensin II | the pulmonary bed's plasma, made at a rate renin sets | the central veins | 17 |
| aldosterone, the natriuretic peptide | the central veins | the central veins | 17 |
| parathyroid hormone | the central veins | the central veins | 19 |
| cortisol | the central veins | the central veins | 20 |
| vasopressin (exists) | the central veins | the central veins | slice, 17 |

Each hormone leaves by a first-order clearance, as vasopressin does in the slice.

Three networks are new. The airway runs from the room through the conducting airway to the alveolar gas. The gut runs from the stomach through the small intestine to the fecal exit. The portal circuit on the vascular network runs from the gut bed through the portal vein to the liver bed and the central veins. The pleural space also encloses the coronary bed, and the abdominal space the liver bed.

### Which pass each item belongs to

Pass 1 is every loop that a monitor reading depends on or that drifts within a day. Pass 2 is the slow conserved quantities. A row's ticket gives its pass.

| Pass | Tickets | Conserved quantities and controllers |
|---|---|---|
| base | 03 to 06 | the lists, the day's inputs, and the body on holds |
| 1 | 08 to 18, and 25 | water, sodium, potassium, chloride, oxygen, carbon dioxide, heat and energy, the fuels, nitrogen, fixed acid; vasopressin, the renin chain, the peptide, insulin, glucagon, the chemoreflex, the thermostat and sleep's gains |
| 2 | 19 to 22 | the minerals, cortisol, the blood's cells and proteins, the bladder |

Until its ticket lands, a pass 2 amount is a held store with balanced flows. Nitrogen is pass 1, because its exit closes the fuels (14 and 16). The minerals move to pass 1 if research 06 keeps total calcium and phosphate in the lens.

### The holds rung 0 keeps

Every other hold of ticket 06's table has a releasing ticket. These stay at the destination, so ticket 28's hold table lists exactly these. Some belong to cards that have a crate by then, which needs the contract change listed under hard parts.

| Hold | Card | Reason | Where it shows |
|---|---|---|---|
| Abdominal pressure | GI | A healthy day barely moves it, no rung 0 check reads it, and a compliance law would need the volume of the abdominal contents, which rung 0 does not hold | the abdominal space; arrows 101 to 103 |
| Gut secretion and its reabsorption | GI | They balance over the day, and the losses they would show are disease (ticket 13) | the gut lumen |
| Laws at a reference temperature | CV, HP, HC, BC | The healthy day's swing in temperature is small, and ticket 12 keeps these readers at a reference temperature | arrows 90 (temperature), 92, 93, 94 |
| Ammonia | HP | Urea is made in one reaction, and ammonia matters for the brain only in liver failure (ticket 14) | arrow 100 |
| Lung water | FL | No rung 0 lung law reads it, and a healthy lung's water barely moves (ticket 10's non-goal) | arrow 110 |
| Brain water and intracranial pressure | FL, NS | No skull enclosure at rung 0, and no check of a healthy day reads either | arrow 111 |
| Gut wall, skin and compartment water | FL | No rung 0 law reads them | arrows 112 and 113 |
| Intra-abdominal pressure on the kidney and the diaphragm | GI | The same held constant as the abdominal space's pressure | arrows 102 and 103 |
| Ventricular wall stress | CV | One natriuretic peptide is secreted, on atrial stretch | arrow 79 |
| The circadian phase and the CRH drive | NS | The clock is a boundary input every law reads (ticket 05), and cortisol follows it directly (ticket 20) | arrow 89 |
| The thermostat's set point and its sweat drive as fields | NS | The set point is NS memory, compared inside NS, and sweat answers release at the skin junction | arrow 87 |
| Respiratory muscle fatigue | MS | A resting day has no fatigue | arrow 95 |
| IM's coupling fields | IM | Held at zero or at the healthy value (map default). Ticket 21 replaces IM's held card with a declaration of zero flux and its cannot-show list, and the fields stay held | arrows 105 to 109 |
| Coagulation's fields | HC | Held at zero (map default), as ticket 21 declares coagulation at zero flux | arrow 107, arrow 114's bleeding and thrombosis fields |
| The vessel wall's permeability and glycocalyx | FL | Their healthy values, since immunity is at rest | arrow 106 |
| Mitochondrial efficiency | MT | Its healthy value, since immunity is at rest | arrow 109 |
| The chance fixture | none: the composition root | The replay gates need a hazard on the body until a physiological law draws chance. The slice's stir moves no law. The slice's forcing on skin water loss moves with that loss to MT's evaporation (12) | the stir's stream and hazard |

### The 38 arrows

Each row is one line of the [coupling map](../../../docs/atlas/coupling-map.md), lines 77 to 114, with each field of the census payload in `crates/ng-contract/src/catalog.rs`. An arrow with no payload is split by the words of its label. "Wired by" names the ticket whose laws define and read the field.

| Line | Arrow | Fields at rung 0 |
|---|---|---|
| 77 | CV to NS, baroreceptor afferents | `carotid_sinus_stretch`: wired by 06, live in the slice; `aortic_arch_stretch`: wired by 06, live in the slice |
| 78 | CV to MT, myocardial work, flow per bed | `myocardial_work`: wired by 09, the heart's work paid by fuel burned at the coronary bed; `flow_per_bed`: wired by 09, the delivery each bed's extraction and debt read |
| 79 | CV to EN, wall stress | `atrial_wall_stress`: wired by 17, the natriuretic peptide's secretion; `ventricular_wall_stress`: held at rung 0, one peptide is secreted, on atrial stretch |
| 80 | CV to FL, capillary and venous pressure | `capillary_pressure`: wired by 06, live in the slice; `venous_pressure`: wired by 06, live in the slice |
| 81 | CV to RS, pulmonary capillary flow | `pulmonary_capillary_flow`: wired by 10, alveolar exchange |
| 82 | RS to CV, pleural pressure | `pleural pressure`: anatomy relation, the pleural space's pressure felt by every bed it encloses, defined by the breath from ticket 10 |
| 83 | RS to NS, lung afferents | `pulmonary_stretch`: wired by 11 |
| 84 | RS to MT, work of breathing | `work of breathing`: wired by 10 |
| 85 | RS to MS, pump load | `pump load`: wired by 10 |
| 86 | NS to RS, respiratory drive | `respiratory_drive`: wired by 11 |
| 87 | NS to MT, thermostat, shiver and sweat | `thermostat_set_point`: held at rung 0, the set point is NS memory and no MT law reads it; `shiver_drive`: wired by 12, read by the muscle bed's heat, since shivering is a somatic motor command; `sweat_drive`: held at rung 0, sweat answers release at the skin junction (S10) |
| 88 | NS to MS, motor drive | `motor_drive`: wired by 11, release at the respiratory pump's junction |
| 89 | NS to EN, CRH, vasopressin, circadian phase | `crh_drive`: held at rung 0, cortisol's profile follows the clock input directly (ticket 20); `vasopressin_drive`: wired by 17; `circadian_phase`: held at rung 0, the clock is a boundary input every law reads (ticket 05) |
| 90 | MT to CV, O2 debt per bed, temperature | `oxygen_debt_per_bed`: wired by 09; `temperature`: held at rung 0, CV's laws run at a reference temperature |
| 91 | MT to NS, temperature | `core_temperature`: wired by 12 |
| 92 | MT to HC, temperature | `temperature`: held at rung 0, HC's laws run at a reference temperature (ticket 12) |
| 93 | MT to HP, temperature | `temperature`: held at rung 0, HP's laws run at a reference temperature (ticket 12) |
| 94 | MT to BC, temperature | `temperature`: held at rung 0, BC's speciation runs at a reference temperature (ticket 12) |
| 95 | MS to RS, pump force and fatigue | `pump force`: wired by 10; `fatigue`: held at rung 0, a resting day has no fatigue law |
| 96 | MS to CV, position and gravity | `position`: anatomy relation, site positions under the boundary's posture; `gravity`: anatomy relation, the hydrostatic head those positions give |
| 97 | RN to CV, tubuloglomerular feedback | `afferent_tone`: wired by 06, live in the slice |
| 98 | RN to EN, renin | `renin_secretion`: wired by 17 |
| 99 | HP to CV, portal pressure | `portal_pressure`: anatomy relation, the liver bed on the portal circuit, so CV's one solve gives portal pressure through HP's sinusoidal edge law (14) |
| 100 | HP to NS, ammonia | `plasma_ammonia`: held at rung 0, urea is made with no ammonia pool and no rung 0 law reads ammonia (ticket 14) |
| 101 | GI to CV, intra-abdominal pressure | `intra-abdominal pressure`: anatomy relation, the abdominal space's pressure felt by every bed it encloses, whose compliance GI holds |
| 102 | GI to RN, intra-abdominal pressure | `intra_abdominal_pressure`: held at rung 0, a held constant on a healthy day (ticket 13) |
| 103 | GI to RS, intra-abdominal pressure | `intra-abdominal pressure`: held at rung 0, the same held constant, and the resting lung volume follows posture directly (ticket 10) |
| 104 | GI to HP, portal delivery | `portal delivery`: anatomy relation, the portal vein carries what the gut absorbs to the liver bed by CV's advection (13) |
| 105 | IM to CV, iNOS, myocardial depression | `inos_activity`: held at rung 0, immunity at rest has zero flux (ticket 21); `myocardial_depression`: held at rung 0, zero flux |
| 106 | IM to FL, permeability, glycocalyx | `permeability`: held at rung 0, the healthy wall (ticket 21); `glycocalyx`: held at rung 0, the healthy wall |
| 107 | IM to HC, tissue factor, hepcidin | `tissue factor`: held at rung 0, zero flux (ticket 21); `hepcidin`: held at rung 0, zero flux, and iron's store follows its own loss |
| 108 | IM to NS, set point shift | `set_point_shift`: held at rung 0, no fever (ticket 21) |
| 109 | IM to MT, mitochondrial efficiency | `mitochondrial efficiency`: held at rung 0, the healthy value (ticket 21) |
| 110 | FL to RS, lung water | `lung_water`: held at rung 0, no rung 0 lung law reads it (ticket 10's non-goal) |
| 111 | FL to NS, brain water and ICP | `brain water`: held at rung 0, no skull enclosure; `ICP`: held at rung 0, no skull enclosure, and NS owns it when one is earned |
| 112 | FL to GI, gut wall water | `gut_wall_water`: held at rung 0, no rung 0 gut law reads it |
| 113 | FL to MS, skin and compartment water | `skin_water`: held at rung 0, pressure injury is absent; `compartment_water`: held at rung 0, limb compartments are absent |
| 114 | HC to CV, bleeding competence, viscosity, thrombosis | `bleeding_competence`: held at rung 0, no bleeding edge (ticket 21); `viscosity`: wired by 08; `thrombosis`: held at rung 0, zero flux (ticket 21) |

Totals at rung 0: 18 arrows wire at least one field, 5 are anatomy relations, and 15 hold every field. Arrows 79, 87, 89, 90, 95 and 114 wire some fields and hold others.

### Fork owners

Owning a fork means writing the laws that fill the gap it names. The ledger stays the body's, and no system owns a pool.

| Fork | Owner | Why | Built by |
|---|---|---|---|
| nitrogen ledger | HP | The gap is urea's making, which closes nitrogen's table (ticket 14). MS turns protein over and makes creatinine (16), and RN makes ammonium and excretes urea, ammonium and creatinine, each by its own laws | 03, 14 |
| acid ledger | HP | The gap is the fixed acid's making, which no card posted. HP's amino acid use posts the sulfur of the diet's protein as the lumped anion. MT burns the diet's organic anion, RN's feedback excretes the fixed acid, and BC states acidity, each by its own laws | 03, 14, 18 |
| carbon dioxide ledger | MT | The gap is the tissue store, which no card named. It is each bed's spaces, filled by MT's reaction and emptied by MT's diffusion to blood. Total carbon dioxide is one species, so the gas and acid loops move one amount | 03, 09 |
| potassium shift into cells | FL | FL owns the water and ion routes across the cell membrane. Insulin's uptake (15) and the pump and leak (17) are its laws, read through the receptor layer. The atlas gave the shift to BC, whose speciation writes no amount | 03, 15, 17 |
| natriuretic peptide's secretion | EN | EN secretes every hormone. Arrow 79 carries the atrial wall stress CV computes to EN's law, ticket 17 already releases that arrow, and the packet anchors the peptide in plasma | 17 |
| cell water | FL | FL owns water and osmoles in every space. One membrane type (SRS section 5) puts the cell membrane beside the capillary wall, and FL owns water's routes across both | 03, 17 |
| bone pools | MS | Bone is MS's tissue. Its partner pool is the bone bed's cells, moved by MS's exchange law (19). Until then the minerals are held stores | 03, 19 |
| pleural space | RS | RS defines the pleural space's pressure from the breath (10). The pleural fluid is a left-out space at rung 0 | 03, 04, 10 |
| bladder | RN | The urinary tract is RN's (atlas renal card). The slice's drain is a boundary law, and ticket 22's void moves it to RN | 22 |
| intracranial pressure | NS | NS owns the intracranial pressure laws (atlas nervous card) once a skull enclosure is earned. Arrow 111 then becomes that enclosure's pressure, an anatomy relation. At rung 0 there is no skull enclosure, and arrow 111 is held | 04 |

The ticket lists the bladder and the body map lists intracranial pressure, so both are answered.

### Atlas card trims for foundation 06

[Foundation 06](../../foundation/issues/06-atlas.md) lands each trim with its system's crate (line 91). Line numbers below are foundation 06's.

| Card | Trim | Lands with |
|---|---|---|
| CV | Natriuretic peptide secretion leaves the register for EN (line 17 left it open). Heat carried between the core and skin stores is a CV law. The liver bed's edges are HP's | 12, 14, 17 |
| RS | One alveolar unit with a shunt, not two per lung (line 87). Pleural pressure is the pleural space's pressure. Ventilation and perfusion spread and hypoxic vasoconstriction are later rungs | 10 |
| NS | NS owns intracranial pressure once a skull enclosure exists, so line 27's open ownership closes, and arrow 111 becomes that enclosure's pressure. Its sensors are landmarks, and the thermostat reads core temperature through arrow 91. It sends no circadian phase at rung 0 | 11 |
| RN | RN owns the bladder and its void, and makes ammonium. Rung 0's tubular detail is one tubule and one collecting duct (line 28). Erythropoietin and calcitriol are not moving hormones at rung 0 | 17 |
| HP | Zones within the liver are absent (line 63). HP's conversions post their own heat, and HP posts the fixed acid. Urea is an uncharged osmole carrying nitrogen, not an S4 ion. Albumin synthesis is constant with a matching held loss, then matches a first-order loss. HP owns the liver bed's edges and its cells' fuel routes | 14 |
| GI | Enteric control is absent at rung 0 (line 55). Stomach and small intestine only, with secretion as one balanced held flow and a balanced fecal exit | 13 |
| EN | The axes at rung 0 (line 47): insulin and glucagon, the renin chain, vasopressin and the natriuretic peptide in pass 1, then parathyroid hormone and cortisol in pass 2. Epinephrine, thyroid hormones, growth hormone, incretins, erythropoietin and calcitriol are absent. Insulin's portal-to-peripheral gradient is the liver's first-pass clearance (15) | 17 or 15, whichever lands first |
| BC | Speciation only, writing no amount: the cell's pumps and shifts, and tonicity-driven cell water, go to FL | 08 |
| HC | The reduced coagulation model is cut (line 22): coagulation at rest has zero flux. Red cell mass is the hemoglobin amount | 08, 21 |
| IM | A quiescent card with its ports at zero flux (line 24) | 21 |
| FL | FL owns the crossings of both walls by water, ions and other solutes, except those How to read the tables gives to MT, HP, MS and HC. Its laws there are water by tonicity, the sodium pump and leak, and insulin's uptake of potassium and phosphate at the cell membrane. The intake and output ledger goes (line 20) | 17 |
| MT | Two heat stores, not eight (lines 25 and 71). MT posts metabolic water, skin evaporation and sweat to S8, owns the fat bed's laws, and owns the gases' crossings of both walls and the fuels' crossings of the cell membrane. "Oxygen debt to every card" goes (line 18) | 08, 09, 12 |
| MS | Creatinine is a solute, not S9 (line 26). Bone is a reservoir only until ticket 19 (line 75). The respiratory pump and its junction are rung 0. Its heat goes through MT's reaction | 10, 16, 19 |
| Coupling map | Arrow 89's circadian phase comes from the boundary clock (line 18). Arrow 87's set point stays NS memory, its sweat drive gives way to release at the skin junction, and its shiver drive is read by the muscle bed's heat. Arrows 99 and 104 are realized by the portal circuit, as anatomy relations. Arrow 111 waits for a skull enclosure | 12, 13, 14, 17 |

Each card's crate is first built in these tickets: BC, HC and MT in 08, RS and MS in 10, GI in 13, HP in 14, EN in 17 or 15, whichever lands first, and IM in 21. CV, FL, NS and RN have crates in the slice.

### Evidence library links by loop

The [evidence library](../../evidence-library/map.md) stores each link as a directed claim that one term raises or lowers another, pooled over the passages where its classifier judged the pair so with confidence of at least 0.7 (`data/graph/edges.parquet` in the evidence store). Works are counted per work, and editions count apart until evidence library ticket 05 merges them. The graph covers only the tilt neighborhood (the store's `scripts/graph/terms.py`), so the water and salt loop is dense, the other loops are thin, and the bladder has no link. **Arrow** names a coupling map line, a shared substrate (S1 to S10, coupling map lines 11 to 20), or none. **Status** is wired or held at rung 0, or a lead the coupling map lacks. A lead goes to the evidence library's process and adds no arrow here. Of the 38 arrows, 24 have at least one link; 84, 85, 88, 93, 94, 95, 100, 102, 103, 107, 109, 110, 112 and 113 have none.

#### Gases

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Pleural and intrathoracic pressure change venous return, filling and cardiac output | 19 works, 33 passages (Nunn 1969; Berne and Levy 7e; Ganong 26e) | 82 | wired | 10 |
| Metabolic rate and oxygen use dilate arterioles and raise local flow | 9 works, 14 passages (Berne and Levy; Klabunde 2e) | 90 | wired | 09 |
| Heart rate, filling and afterload raise the heart's oxygen use | 6 works, 9 passages (Klabunde; Goodman and Gilman 13e) | 78 | wired | 09 |
| Hypoxia narrows pulmonary vessels | 4 works, 10 passages (West 11e; Boron and Boulpaep) | S1 | held | ticket 10's non-goal; an RS law on the pulmonary bed's own edges would carry it |
| Hydrogen ion, carbon dioxide and saturation set arteriolar tone | 4 works, 9 passages (Guyton; Klabunde; West) | S3 | held | rung 0's local signal is oxygen debt alone (09) |
| Chemoreceptor activity drives breathing, heart rate and pressure | 3 works, 3 passages (Ganong; Guyton; Klabunde) | 86 | wired | 11 |
| A fall in arterial oxygen raises chemoreceptor discharge | 1 work, 1 passage | S2 | wired | 11 |
| Lung volume changes heart rate | 1 work (Berne and Levy) | 83 | wired | 11 |
| Pulmonary blood flow sets oxygen uptake | 1 work | 81 | wired | 10 |

No edge covers ventilation acting on arterial carbon dioxide, or carbon dioxide acting on breathing, though both are terms in the graph.

#### Heat

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Body temperature raises heart rate | 6 works, 10 passages (Guyton; two models) | 90 | held | CV's laws run at a reference temperature |
| A rise in core temperature relaxes skin vessels and raises skin flow | 8 works and 6 works (a Stolwijk model; Berne and Levy; Levick) | 91 | wired | 12 |
| Skin vessel tone and skin flow set skin temperature | 7 works, 8 passages (Berne and Levy; Boron and Boulpaep) | S6 | wired | 12 |
| Core temperature sets sympathetic activity to the skin | 2 works (Boron and Boulpaep; Levick) | 91 | wired | 12 |
| Sympathetic activity narrows skin vessels | 4 works (Berne and Levy; Katzung; Klabunde) | S10 | wired | 12 |
| Muscle activity raises body temperature | 4 works (Katzung 14e; Rang and Dale 8e) | 87 | wired | 12, through the shiver drive |
| Thyroid hormone raises heat production | 1 work (Berne and Levy) | S7 | held | thyroid hormones are absent at rung 0 |
| Cytokines raise body temperature | 1 work, a model | 108 | held | 21, zero flux |
| Body temperature changes viscosity | 1 work (Klabunde) | 92 | held | a reference temperature |

No edge covers sweating, shivering, the set point or the night's fall in temperature.

#### Fuels

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Glucose drives insulin release | 3 works (Berne and Levy; Katzung; Modern Nutrition 11e) | S5 | wired | 15 |
| Insulin resistance lowers glucose uptake | 2 works, 4 passages (a forearm study; Modern Nutrition) | S7 | wired | 15 |
| Sympathetic activity narrows splanchnic vessels | 2 works (Guyton 12e and 13e) | S10 | wired | slice |
| Sympathetic activity changes glucose uptake | 1 work, 3 passages | S10 | held | sympathetic effects on metabolism, and epinephrine, are absent at rung 0 |
| Splanchnic vasodilation raises portal pressure | 1 work, a dose model | 104 | wired | 13 |
| Portal and hepatic venous pressure change venous flow and cardiac output | 2 works (Guyton; a flow MRI study) | 99 | wired | 14 |

No edge covers emptying, absorption, the liver's glucose output, glycogen or lactate.

#### Water and salt

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Arterial pressure raises sodium excretion and urine flow | 11 works, 62 passages (Guyton 12e; Clinical Physiology of Acid-Base and Electrolyte Disorders 5e) | S1 | wired | 17: the kidney's reclaim reads its own bed's pressure, the check evidence library ticket 03 asks for |
| Arterial pressure and vessel tone set filtration and renal flow | 21 works, 51 passages (Guyton; Ganong) | S1 | wired | slice |
| Arterial pressure lowers renin | 9 works, 18 passages (Berne and Levy; Goodman and Gilman; Guyton) | S1 | wired | 17 |
| Extracellular volume raises sodium excretion and filtration, and lowers renin and aldosterone | 10, 11, 10 and 8 works (August, Nelson and Thorn 1958; Berne and Levy; Boron and Boulpaep) | none | lead | At rung 0 volume reaches the kidney and EN only as pressure (S1) and atrial stretch (79). Evidence library tickets 01 and 04 name it the largest gap |
| Urine output and sodium excretion lower extracellular volume | 19 works, 38 passages (Guyton; Levick) | S8 | wired | slice |
| Blood volume raises arterial pressure, filling and cardiac output | 22 works, 108 passages (Guyton 13e; Klabunde; Boron and Boulpaep) | S1 | wired | slice |
| Arterial pressure drives the baroreflex | 29 works, 70 passages (Levick; Katzung) | 77 | wired | slice |
| A fall in blood volume or pressure raises vasopressin | 13 works, 39 passages (Guyton 12e; Boron and Boulpaep; Berne and Levy) | 89 | wired | 17 |
| Plasma osmolality drives vasopressin | 10 works, 22 passages (Berne and Levy; Boron and Boulpaep; Goodman and Gilman) | 89 | wired | 17 |
| Atrial pressure and volume drive the natriuretic peptide | 4 works (Ganong; Boron and Boulpaep; Berne and Levy) | 79 | wired | 17 |
| Chloride at the macula densa sets afferent tone and renin | 4 works, 6 passages (Guyton 12e and 13e) | 97 | wired | slice |
| Renin drives angiotensin II and aldosterone | 9 works and 7 works (Tietz 7e; Berne and Levy) | 98 | wired | 17 |
| Renal sympathetic activity raises renin and reclaim and lowers filtration | 11 works, 23 passages (renal nerve studies) | S10 | wired | slice, 17 |
| Vasopressin, aldosterone and the peptide act on urine and sodium excretion | 18 works and 8 works (Berne and Levy; Guyton; a renal model) | S7 | wired | 17 |
| Angiotensin II, aldosterone and renin raise arterial pressure | 7 to 8 works each (Guyton; OpenStax Anatomy and Physiology) | S7 | wired | 17 |
| Plasma potassium raises aldosterone | 4 works, 8 passages (Guyton; Tietz) | S4 | wired | 17 |
| Venous pressure and arteriolar resistance set capillary pressure | 12 works and 10 works (Levick; Klabunde) | 80 | wired | slice |

No edge covers thirst or drinking, which rung 0 schedules.

#### Acid

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Hydrogen ion and carbon dioxide change arteriolar tone | 2 works (Guyton) | S4 | held | rung 0's local signal is oxygen debt alone |
| Carbonic anhydrase drives bicarbonate reclaim | 1 work (Katzung) | S4 | wired | 18 |
| Hydrogen ion lowers potassium secretion | 1 work (Guyton) | S4 | held | ticket 17's potassium secretion reads aldosterone, not acidity |
| Volume contraction and sympathetic activity raise acid secretion | 1 work each (Boron and Boulpaep) | none | lead | the same volume gap as the water and salt loop |

No edge covers ventilation acting on arterial carbon dioxide, or net acid excretion.

#### Minerals and bone

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Calcitriol lowers phosphate excretion and raises calcium reclaim | 3 works (Guyton; Rang and Dale 9e) | S7 | held | calcitriol is not a moving hormone at rung 0 (19) |
| Calcitriol and plasma phosphate act on each other | 1 work each way | S7 | held | the same |
| Plasma calcium narrows vessels and raises contractility | 2 works (Guyton; OpenStax Anatomy and Physiology) | S4 | held | no CV law reads calcium at rung 0 |
| Calcitriol and calcium lower renin | 1 work, a model | S7 | held | calcitriol is absent |

No edge covers parathyroid hormone, magnesium or bone.

#### Cortisol

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| CRH drives ACTH | 1 work | 89 | held | cortisol follows the clock directly (20) |
| Cortisol raises heart rate, and ACTH raises pressure | 1 work each (a model; Ganong) | S7 | held | cortisol acts only on the liver's glucose output at rung 0 (20) |

No edge covers the daily rhythm.

#### Blood cells and proteins

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| Erythropoietin raises red cell production | 12 works, 15 passages (Guyton; OpenStax Anatomy and Physiology; Berne and Levy) | S9 | held | erythropoietin is not a moving hormone at rung 0 (21) |
| Erythropoietin raises hematocrit, viscosity and blood volume | 5 works each (OpenStax Anatomy and Physiology; Clinical Physiology 5e) | S9 | held | the same |
| Plasma volume dilutes hemoglobin and hematocrit | 11 works, 15 passages | S9 | wired | 08: concentration follows dilution |
| Viscosity and hematocrit change flow, resistance and pressure | 12 works, 33 passages (Guyton; Levick) | 114 | wired | 08 |
| Filtration fraction raises peritubular oncotic pressure and reclaim | 4 works, 7 passages (Guyton; Clinical Physiology 5e) | S1 | wired | slice: FL's Starling exchange at the peritubular bed |
| Renal flow and venous pressure change erythropoietin | 1 work each | S7 | held | erythropoietin is absent |

No edge covers albumin synthesis, red cell lifespan or iron.

#### Bladder

| Link | Evidence | Arrow | Status | Ticket |
|---|---|---|---|---|
| No link: no term names the bladder, voiding, micturition or the detrusor | 0 works, from the graph's term list | none | none | 22 builds the void with no library lead |

Posture is the graph's densest neighborhood. Venous pooling, hydrostatic pressure and the muscle pump act on venous return and pressure in 22 works (arrow 96, an anatomy relation), and intra-abdominal pressure acts on venous return in 5 works (arrow 101). Sleep-tagged passages hold 56 edges from 17 works, none of them on the night's fall in pressure, the chemoreflex's gain or vasopressin. Three arrows have links outside these loops, and all three are held at rung 0: 105 (cytokines on nitric oxide, one textbook), 106 (one to three works) and 111 (intracranial pressure on arterial pressure, the Cushing response).

### Which holds each later ticket releases

| Ticket | Releases |
|---|---|
| 08 | The held gas in blood and tissue, the carrier's capacity, and arrow 114's viscosity. It adds a fixed exchange at the lung and a fixed use at each bed |
| 09 | Ticket 08's fixed use at each bed, MT's held chemical energy that pays the heart's work in the slice, and arrows 78 and 90. It burns glucose and fatty acids in a fixed proportion with lactate's oxidation, held until ticket 16 |
| 10 | RS's held pleural pressure, ticket 08's fixed exchange at the lung, ticket 06's held airway exit for water and heat, and arrows 81, 82, 84, 85 and 95's pump force. It adds a held drive |
| 11 | Ticket 10's held drive, and arrows 83, 86 and 88 |
| 12 | MS's held skin water loss, the slice's single heat exit, and arrows 87 (shiver and sweat) and 91 |
| 13 | GI's absorption stand-in, which puts a meal straight into the veins, and arrow 104, which the portal circuit realizes |
| 14 | HP's held card: glycogen, gluconeogenesis and urea, the liver bed's held edges, and arrow 99, which the portal circuit realizes. It adds held insulin and glucagon values, and a constant albumin synthesis with a matching held loss |
| 15 | EN's held insulin and glucagon with ticket 14's held values, the slice's external insulin-driven potassium shift, and the filter's keep-back of glucose |
| 16 | MS's held stores (muscle glycogen, body protein and creatinine's source), MT's one mixed fuel, the fat store and ketones |
| 17 | EN's held vasopressin logistic, its held renin chain and peptide, BC's held cell water, and arrows 79 (atrial stress), 89 (vasopressin drive) and 98 |
| 18 | BC's held lumped anion and the diet's held organic anion, the filter's keep-back of total carbon dioxide, and the synthetic meal's pairing of its cations with chloride |
| 19 | The minerals' held stores with balanced flows: absorption, bone and cell exchange, and excretion |
| 20 | EN's held cortisol |
| 21 | HC's held red cell mass and iron, ticket 14's constant albumin synthesis and its held loss, and IM's held card, replaced by a declaration of zero flux with its cannot-show list |
| 22 | The slice's bladder drain, which has no void |
| 23 | None. It builds the healthy-range and return checks in validation and no law, so no stand-in changes |
| 24 | None. It runs the day on the body the earlier tickets built, and its check is that only pass 2 holds and the holds rung 0 keeps remain |
| 25 | The constant gains that stand in for sleep: sympathetic traffic and each bed's metabolic rate keep their waking values through the night until it lands |

Rows 08 and 18 follow the default for when acidity is stated, which waits on a ruling (what this changes in the plan). Under the alternative, ticket 08 also holds a sourced buffer line, and ticket 18 releases it with NS's read of carbon dioxide alone.

### Hard parts

#### Conserved quantities rung 0's laws cannot close today

- Charge cannot close as the contract counts it today. Its charge is a sum of fixed charges, and the compiler asks each reaction for exact charge stoichiometry. Rung 0's reactions make lactate, ketones, the fixed acid and ammonium, and the strong-ion frame keeps hydrogen ion out of the amounts. A meal's calcium, magnesium and potassium also come with phosphate and organic anions that no species named. The default above counts each acid and base as a neutral moiety whose charge speciation gives, and adds the diet's organic anion and the cell anion. That balances every reaction, but a delivery is neutral only after speciation. The sum of fixed charges moves when a cation enters or leaves with an anion counted as a neutral moiety, while the certificate holds charge's drift at zero. The default therefore also has the certificate read charge after speciation. Tickets 02 and 03 decide it, and ticket 02's reaction prototype is the place to test it against the alternative, a fixed charge on each strong anion with a counter-charge for each reaction that makes or burns one.
- Every made or used row (metabolic water, oxygen use, carbon dioxide made, the fuels, urea, ammonium, creatinine, fixed acid) needs ticket 02's reaction. Today a transfer never changes a species, and the contract has no reaction.
- The energy table needs each fuel species to carry its energy, so a reaction's heat is the fuels' energy less the products'. MT's oxidation and HP's conversions both post heat this way.
- Iron cannot close as ticket 21 words it. "Production to match" gives iron no source, because iron enters only from the meal, by absorption. The table above closes it with the meal's iron, absorption, the store's uptake and release, and the gut's cells, whose shed iron leaves through the fecal exit.
- The minerals cannot close as laws, because no ticket or research ticket absorbs calcium, phosphate or magnesium from the gut (13, research 13 and research 15 leave it out).
- Sodium and chloride lose their sweat exit if ticket 12's sweat carries only water. A thermoneutral day barely sweats, but ticket 12's warm room and ticket 28's warm hour do.
- No ticket's certificate line names potassium or chloride. Ticket 17's laws close both, and its check names only sodium and water.
- Albumin climbs from ticket 14 to ticket 21 unless ticket 14's constant synthesis comes with a matching held loss, which the tables above give it.
- Albumin's and hemoglobin's nitrogen stays in their own conserved quantities, because their making and loss are balanced flows, not reactions with amino acids. Counting them in the nitrogen table would need those reactions.

#### Systems whose rung 0 needs what the contract lacks

Read against the contract at main `85d1e6e`.

| Need | Systems | What the contract lacks | Where it lands |
|---|---|---|---|
| A gas space whose volume follows the gas law, an airway network and its edge kind, and an identity from a gas amount to a partial pressure | RS | Every watery space takes its volume from water, each site class has a fixed set of spaces, there is no airway network, and the identity table has no gas law | 03. A non-watery space on the pulmonary bed and the conducting airway is the smaller change; a new site class is the alternative, a contract version |
| A route family across the cell membrane, with a pump that draws energy | FL, MT, MS | No route family reaches the cells space | 03, before tickets 08, 15 and 17 |
| Heat carried by blood, conduction between two heat stores, and temperature read from heat | CV, MT | Energy is not carried by flow, the energy families take one thermal pool, and no identity gives temperature | 03 lists temperature. Heat advection and conduction are unticketed and belong in 03, so ticket 12 edits no shared list |
| An osmotic identity for small solutes | FL | Only albumin has one | 03, before ticket 17's cell water |
| A reaction | MT, HP, MS, RN | None | 02 |
| The clock phase and the sleep state as inputs a law reads | NS, EN, RN, MT | No kernel input reads time, and a clock kept as a law's memory cannot pass periodic admission | 05 |
| A pressure law for a released enclosure | RS, and GI if its compliance ever becomes a law | Only a held card owes an enclosure's pressure, and the compiler refuses a rigid or elastic one | unticketed: 04 or 10 |
| Admission with a tidal breath beside a pulsatile heart | RS, CV | An admission cycle has at most one pacing clock | unticketed: ticket 10's prototype covers it |
| Synthesis and loss across the boundary for a system that is not held | EN, HP, HC, GI | Only the boundary scope and held cards get boundary routes | 04 |
| A card with a crate that keeps named holds | GI (the abdominal space's compliance), and MS, BC and HC until their releasing tickets | A held compliance needs a named stand-in, and only a card outside the built set has a held declaration | 04 |
| A hold of one species in a space | the body on holds | A hold names a whole space, so the slice's hold on cells cannot be released species by species, though tickets 08, 15, 17 and 19 each move a different species there | 04 or 06 |
| A census disposition for a field held with its reason | every system | The census has live, input, sink, anatomy relation, outside the slice and unresolved | 04 |

No system needs a new site class if the gas space lands as a kind of space. Every new bed fits the capillary bed class, and the portal circuit is legal on the vascular network because each bed drains to one site.

#### Where the drafts and the atlas disagree on physiology

| Topic | Drafts | Atlas | Resolution here |
|---|---|---|---|
| The thermostat | release at skin vessels and sweat glands, and motor drive for shivering | one NS to MT arrow for the set point, shivering and sweat | skin vessels and sweat answer release at the skin junction (12), as the coupling map's rule that controllers act only by releasing ligands asks (line 26). Shivering is a somatic motor command, a posted demand like arrow 88's motor drive, which the map's direct couplings admit (line 73), so arrow 87's shiver drive carries it. The set point stays NS memory |
| The natriuretic peptide | EN secretes it | the CV card secretes it, while arrow 79 sends wall stress to EN | EN |
| The potassium shift and cell water | FL moves cell water, and the shift's owner is open | BC owns the cell's pumps and shifts, and shares cell water with FL | FL owns the cell membrane |
| Metabolic water, fixed acid and skin evaporation | MT posts all three | MT posts lactate and sweat only, and no card posts skin evaporation | MT posts metabolic water and skin evaporation (09, 12); HP's amino acid use posts the fixed acid (18) |
| Heat production | through MT's reaction at each bed | HP and MS also post heat | MT's oxidation makes most heat, and HP's conversions post their own; MS posts none |
| Urea and creatinine | uncharged solutes, urea an osmole carrying nitrogen | urea on S4 with the ions, creatinine on S9 with the formed elements | uncharged solutes |
| Hepcidin | made by the liver | an immune signal (arrow 107) | held at zero at rung 0, and iron's store follows its own loss |
| The tissue carbon dioxide store | blood plus one tissue store | no card names a tissue carbon dioxide space | each bed's spaces, moved by MT |
| The clock | one forced phase that many laws read | one NS to EN arrow | a boundary input, so arrow 89's phase is held |
| Acid-base | Henderson-Hasselbalch on plasma first | Stewart's frame decided (blood chemistry card) | the strong-ion frame from ticket 08 (map default) |
| The first rungs of RS, HC, IM, MS and GI | one alveolar unit; quiescent coagulation and immunity; the junction later; one lumen for the whole gut | ventilation and perfusion spread; reduced coagulation and inflammation models; the junction first; emptying and segmental absorption | one unit; zero flux; only the pump's junction; stomach and small intestine |
| Ownership | each store has an owner card | no system owns a pool | no system owns a pool, and forks name law owners |

### What this changes in the plan

Each item names a ticket this Answer leaves unedited. The first list holds changes the tables imply. The second holds choices that need a ruling, each with its options, the default the tables use, and the evidence for that default.

- **Ticket 02.** Its prototype gains a box that tests the charge default, in which acids and bases are neutral moieties that speciation charges and the certificate reads charge after speciation. The alternative it tests is a fixed charge on each strong anion, with a counter-charge for each reaction that makes or burns one.
- **Ticket 03.** The alveolar and airway gas needs a non-watery space and a gas-law identity, with nitrogen gas and water vapor. The cell membrane's route families, heat advection, conduction and the small-solute osmotic identity belong here, so tickets 12, 15 and 17 edit no shared list. No red cell site or space is needed, because hemoglobin is a mass in blood, as ticket 08 says. Its line 12 names landmarks for the chemoreceptors, the thermostat's sensors and the osmoreceptor. The tables add four more: the volume receptor, the adrenal, the islets and the parathyroid. Of the thermostat's sensors only the skin's is a landmark, because core temperature reaches NS by arrow 91. It also needs the hormones' secretion and clearance sites. Iron, ammonium, glycerol, body protein, the cell anion and the diet's organic anion join the species.
- **Ticket 04.** The census needs a disposition for a field held with its reason. Arrow 111 is held with NS as its owner, and arrows 99 and 104 are anatomy relations, as 82, 96 and 101 are. A card with a crate must be able to keep named holds, and an unheld card to own an enclosure's pressure. EN, HP, HC and GI need boundary routes for synthesis and loss.
- **Ticket 05.** The meal's vector carries every species the tables deliver, iron and the diet's organic anion included. Ticket 22 needs a void as an input of the day, and ticket 05 lists none.
- **Ticket 06.** Holds name one species in a space. The airway's exit for water and heat is an RS held route beside the slice's skin and heat exits, so ticket 10 has a hold to release. The liver bed's edges are held under HP's card until ticket 14. Its hold table takes this Answer's rows.
- **Ticket 09.** It burns glucose and fatty acids in a fixed proportion and oxidizes lactate, so lactate closes from ticket 09. Research 11 makes lactate only when supply falls short and gives no form for burning it, so that form is still to find. "The bed's heat" is the core heat store, or the skin heat store for the skin bed, because only tissue sites hold heat.
- **Tickets 09, 16 and 19.** Each decides whether a bed the tables add as a model default is earned (record 0002), by running its check once without the bed. Ticket 09 decides the coronary bed through the slice's posture checks with arrow 90 wired, ticket 16 the fat bed through its respiratory quotient run, and ticket 19 the bone bed through calcium and phosphate after a meal.
- **Ticket 10.** Admission with two rhythms is unticketed, so its prototype covers it. Its Releases line should name arrow 84, which its text wires.
- **Ticket 12.** Sweat carries sodium and chloride with its water. Its text (line 7) has the thermostat act only through autonomic release, shivering included. Skin vessels and sweat answer release at the skin junction, and shivering is a somatic motor command whose drive the muscle bed's heat reads. Its Releases line (line 9) names arrow 87 whole, and the tables hold 87's set point and sweat drive, so it releases 87's shiver drive. Skin temperature joins core temperature as the thermostat's input, through a skin thermoreceptor, as research 12 asks.
- **Ticket 14.** Its constant albumin synthesis needs a matching held loss. Its amino acid reaction carries the sulfur that ticket 18 posts as fixed acid.
- **Ticket 15.** Epinephrine is not in rung 0. Its potassium check is a choice below.
- **Ticket 17.** Its certificate line should also name potassium and chloride, and its Releases line the held renin chain and peptide.
- **Ticket 18.** It waits on ticket 14, whose amino acid reaction makes the fixed acid. Its check bounds the lumped anion's amount, or the total carbon dioxide reading's return, because bicarbonate is speciation and no amount. Its Releases line is a choice below.
- **Ticket 19.** It adds absorption of calcium, phosphate and magnesium, and releases the minerals' held stores. The healthy-range set already holds total calcium and phosphate (`data/healthy-range-set.jsonl`), and the packet anchors plasma phosphate. If research 06 keeps both readings, ticket 19 is pass 1 and joins ticket 24's blockers.
- **Ticket 21.** Iron enters from the meal by absorption, not production, and leaves with the gut's shed cells through the fecal exit. Hemoglobin and red cell mass are one amount. IM's held card becomes a declaration of zero flux, and its fields stay held.
- **Ticket 22.** It needs a void input and research 15's bladder item among its blockers.
- **Ticket 24.** Its hold check should allow the holds rung 0 keeps, as ticket 28's does.
- **Ticket 27.** Its box on ticket 06's body (line 14) waits on ticket 06, which its blockers do not name. If ticket 27 must not close before ticket 06, ticket 06 joins them.
- **Research 06.** It leaves the postprandial creatinine reading open, because creatinine rises with meat in the meal (its comments). Rung 0 makes creatinine from muscle only (16), so that reading needs the meal's creatinine in ticket 05's delivery, or it leaves the postprandial set.
- **Research 12.** Its thermostat box (line 12) has skin blood flow, sweating and shivering each act through autonomic release. Shivering is a somatic motor command, arrow 87's shiver drive, so the box asks for shivering's form against that drive.
- **Research 13 and 15.** Neither gives a form for mineral absorption.
- **The evidence library.** Two leads go to its process. One is the volume signal to the kidney and its hormones, which its tickets 01 and 04 already hold. The other is loop neighborhoods with the quantity nouns the graph lacks (ventilation, pH, absorption, emptying, synthesis, storage, rhythm, thirst, voiding).

#### Choices that need a ruling

- **When acidity is stated (tickets 08, 11 and 18).** In option A, the default, ticket 08 states acidity in every watery space from the strong ion difference, the weak acids and total carbon dioxide, with the lumped anion and the diet's organic anion held, and ticket 18 adds their making and excretion. In option B, ticket 08 carries carbon dioxide on a sourced buffer line, and ticket 18 replaces that line with the full balance, as tickets 08, 11 and 18 read today. The evidence for the default is research 11's section "The interface with ticket 14". It finds that ticket 08 cannot hold pH while it waits for ticket 18, because with pH held the body would store and release about three times too much carbon dioxide for each step in partial pressure. So ticket 08 needs the charge balance or a sourced buffer line. The body map's model default states acid-base in the strong-ion frame in every space, and its alternative is one a later ticket would have to replace. Option A changes three lines:
  - Ticket 08's non-goal "acid-base beyond what carriage needs (ticket 18)" (line 17) narrows to the anions' making and excretion, which widens ticket 08.
  - Ticket 11's central sensor (line 7) reads the acidity BC states from ticket 08. That acidity follows carbon dioxide only while the anions are held.
  - Ticket 18's Releases line (line 9) drops the move of NS's read from carbon dioxide alone to the full balance.

  Under option B those lines stand, and the rows of this Answer that state acidity from ticket 08 move it to ticket 18.
- **Ticket 15's potassium check.** Its run says a meal's potassium "moves into cells first and is excreted after" (line 13). Potassium that insulin moves into cells comes back out only through the leak that ticket 17 builds with the pump, and aldosterone's potassium secretion is ticket 17's too. In option A, the default, ticket 15 waits on ticket 17 (its line 5). In option B, the run drops "and is excreted after", and whichever of tickets 15 and 17 lands second carries that check. The evidence for the default is that the check stays whole, and ticket 16 already waits on ticket 15, so waiting delays tickets 15 and 16 only if ticket 17 lands after ticket 14.
- **The source for ticket 25's sympathetic gain.** Ticket 25's line 5 has it wait on research 12. Research 12 asks for heat production by state (its line 14), and none of its boxes asks for sleep's gain on sympathetic traffic. Research 11, now resolved, sources sleep's effect on the chemoreflex only, and research 14 has no box on sleep. In option A, the default, research 12 gains a box for sleep's gain on sympathetic traffic, and ticket 25's blockers stand. In option B, research 14 or a new research ticket gains the box, and ticket 25's blockers name it. The evidence for the default is that ticket 25 already waits on research 12, which already asks for the sleep state's other gain, so one research ticket would give both.

### Sources and how far they were read

- In the repo, in full: `docs/srs.md`, the solver contract's sections 1, 2, 4, 5 and 6, the 13 atlas cards, the atlas README and coupling map, the glossary's sections on the body's structure and the software, records 0002 and 0004, `crates/ng-contract/src/catalog.rs`, `crates/ng-validate/src/slice.rs` to line 490, body tickets 02 to 28, research tickets 06 and 11 to 15, foundation 06, and the keys in `data/`.
- In the repo, in part: the contract's `quantity.rs`, `scope.rs`, `anatomy.rs`, `law.rs`, `conservation.rs`, `compile.rs`, `boundary.rs` and `packet.rs`, for their closed lists and operators.
- Leads, never sources (SRS rule 16): the healthy-body draft (sections 2 to 8, 10c, 10d, and Part II's minimum stores and fidelity ladders), the frame follow-up (all), the curation study's `needs.md` (all), the evidence store's link graph (queried read-only, with nothing restricted opened), and the earlier readiness logs (all). No number from them enters this Answer.
