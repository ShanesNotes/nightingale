# Nightingale software requirements specification (SRS)

Governing principle: every artifact in this repo must justify why an agent loads it. An agent loads this SRS first, because it states what the engine must do. This SRS binds: every lower doc yields to it, and nightingale-pulse's vision, roadmap and lanes docs are history. "Model default" marks a choice the models made, open to change.

## 1. Purpose and scope

Nightingale is a high-fidelity simulation engine for one patient, built for clinicians and educators. It is for teaching and never informs the care of a real patient. One engine is one patient's body, and it moves that patient forward in time; several patients are several linked engines. The body computes every number from conserved amounts and physics, and disease, chance and interventions act on that same physics. Every feature starts from a clinician's question: "I want to see X. What do I see?"

Scope now: the solver, the core contract and the structural model, so that stage 1 can be built and finer body detail can be added later without rewrites.

Architecture layers, from the bottom: the contract (`ng-contract`: the map, the language laws are declared in, the compiler and the solver interface); on it, the system crates (laws declared once per site class over shared networks), the solver (one Rust core behind the solver interface) and the record crate (`ng-record`: the state, the ledger, the pending events and the check that accepts or refuses each step). Observers read accepted steps and never write.

Scope stages, built bottom up. Only stage 1 is built now; the later stages are designed for now and built later.

1. Body: healthy physiology from first principles, with one ledger that conserves every amount.
2. Disease: shifts in baseline parameters (chronic) or timed events (acute), applied to the body, never coded inside a law.
3. Chance: a seeded hazard model that reads current state and triggers events at cited, state-dependent odds. The solver never rolls dice; laws draw chance from seeded streams.
4. Interventions: timestamped commands (drug, fluid, oxygen, position, procedure) entering through the boundary port, like meals and posture.
5. Learning: every run keeps its event log (packet, run seed, every event), so it can be replayed and debriefed. A new attempt meets the same authored patient by default; patients drawn from a population are a later option.

## 2. Definitions

The patient and the run:

- Engine: the simulated human body of one patient. One engine runs one patient.
- Linked engines: several patients simulated together, one engine each. How engines link is not yet specified.
- Patient: the one person an engine simulates, described by a packet.
- Packet: the bundle of one patient's parameters and the day the patient lives. It holds every parameter value with where it came from (cited, drawn from a population range or chosen inside one with a reason, derived by an identity, fitted when the start state is solved, or inferred by a model), the anchors the start state must meet, the day (sleep, posture, meals, drinks and room conditions) that the run's boundary events are made from, and the source, population and inference records behind them. A packet is either reviewed (clinical values with citations, and a count of its inferred values) or a synthetic test fixture. Its hash is fixed at admission and is part of the run's identity.
- Admission: the moment a run starts. The start state is solved once here and never imposed again.
- Run: one continuous simulation of one patient from admission. It never resets.
- Step: one advance of simulated time. The solver proposes the next state, the engine checks it, and the step is accepted whole or refused whole.
- State: the patient's accepted condition at one instant: the amount in every pool, each controller's memory (such as a set point), each discrete mode (such as a valve open or shut), the position of each random stream, and what the solver needs to continue exactly. Only an accepted step changes it. Avoid: record.
- Ledger: the account of every conserved quantity over the run: how much each site holds, and every amount that entered or left the body since admission. One ledger per run; it never resets. Avoid: books, inventories, I&O ledger, record.
- Event: a timestamped input to the run, with an id: a boundary input (a meal, a drink, a posture change, room conditions, the clock, sleep), an intervention, or a timed disease event. A chance event that a law fires is not an input; replay recreates it from the seed.
- Intervention: an event a clinician applies with a real tool, such as a drug, fluid, a nasal cannula, a position change or a procedure. It changes the body's boundary conditions through a port and acts on the same physics as everything else.
- Event log: the append-only list a run is rebuilt from: the run's identity (body model, packet, run seed, solver settings) and every event in time order. Nothing in it is changed or removed. Avoid: record, recording.
- Seed: the number that fixes a run's random draws. Three seeds are kept apart: the patient seed (which patient is drawn from a population), the run seed (variation during the run) and the observation seed (measurement error).
- Stream: a named sequence of random draws inside one law. Its draws depend only on the seed, its name and which physiological event (a beat, a breath) it has reached, so adding a stream never changes another.
- Replay: rerunning a run from its event log, with the same run seed (identical bits) or a fresh one (a different course through the same events).
- Checkpoint: a saved copy of the whole accepted state at a step boundary, from which a run resumes exactly or branches.
- Scope stage: one of the five stages of scope in section 1, built bottom up. Not a solver stage, which is a numerical sub-step inside one step.
- Scenario: a case authored for a learner, made of a packet, its events and the behaviors it teaches. Scenarios belong to later scope stages.

The body's structure:

- Map: the patient's anatomy held as data: every site with its class and position, the containment tree, the networks, the enclosures and the nerve links. The solver contract calls it the anatomy. Not the atlas's coupling map, which links systems, not sites, nor a wayfinder `map.md`, which indexes an effort's tickets.
- Site: one place in the map, such as a capillary bed, a vessel segment, a nerve junction, an enclosed space, a piece of tissue or a nerve segment. Every site has one site class and a position in the body.
- Site class: a kind of site whose members share the same laws.
- Containment: the "part of" relation: cell, tissue bed, functional unit, organ, region, person. It is a tree, so an organ's totals are the sums over its sites.
- Network: the "connected to" relation: a graph of real anatomical segments that crosses organ boundaries. Section 5 lists the networks.
- Reference anatomy: the real structure of the body's networks as section 5 describes it, which each grouped region stands for and each split moves the map toward. It is not the map.
- Grouped region: a region the map holds as fewer sites than the reference anatomy it stands for. It states that part of the reference anatomy and the behaviors it cannot show because they happen only inside it.
- Tissue bed: a capillary bed with the tissue it serves (its plasma, interstitial fluid and cells), where blood and tissue exchange.
- Organ: a named part of the containment tree where several networks meet inside one enclosure, such as the kidney or the heart. It owns no pool and no law of its own; its behavior comes from the laws of its site classes and the networks that run through it. Avoid: organ compartment, compartment.
- System: one of the 13 body systems in the atlas, such as cardiovascular, fluids, renal or nervous. A system's code declares laws per site class and owns no pool.
- Enclosure: a bounded space (the skull, the pericardium, the chest, the abdomen, a limb compartment) whose pressure, set by the volume inside it and the stiffness of its wall, is felt by every tissue bed inside it.
- Membrane: a wall between two spaces, classed by what can cross it (section 5).
- Pool: the amount of one conserved quantity (water, sodium, oxygen, heat, and so on) held in one site, or in one space of a site such as a tissue bed's plasma. The body names its pools, the state holds their amounts, and no organ or system owns one.
- Port: a declared point where something crosses a boundary: an input from outside the body, a coupling from one system to another, or a reading. Each port states what it carries and its unit, and is checked when the body is built.
- Law: a declared rule that computes a flow or a rate of change from state and parameters. A law holds no time step, no solver and no disease logic.

Numbers and evidence:

- Parameter: a property of the patient's body that laws read and nothing computes, such as a vessel's stiffness, a resistance, an organ's size or a reflex gain. Each has a population range.
- Produced quantity: anything the body computes from state, laws and parameters, such as a pressure, a flow, a concentration, a heart rate or a lab value. It is never stored as a fixed number and never set.
- Target: a value or range that a check compares a produced quantity against, such as an anchor, a healthy range or a within-subject variation. Every target rests on the literature, and only a healthy range or a within-subject variation may also rest on an inference record, which adjusts a sourced one by a sourced shift or, for an amount no method can measure, reasons from the sourced flows into and out of it (rule 16).
- Anchor: a target a produced quantity must meet when the start state is solved, stated with its measurement method and averaging window. An anchor is never a parameter's value. Methods are named as current charting practice names them (an arterial-line pressure is charted as from the arterial blood pressure monitor); that naming follows practice and is not a law, since practice can improve.
- Healthy range: the target for one reading, the central 95% of healthy adults over the averaging window their study used, matched to the patient in activity, sex, age or method wherever a source shows that one of these shifts the reading. It comes from a primary study, else a compiled source or a textbook value at its tier (section 6), else an inference record that adjusts a sourced range by a sourced shift (record 0004).
- Healthy-range set: every healthy range and within-subject variation, with the source and inference records behind them, either reviewed or a synthetic test fixture. Physiology never reads it, so it is not part of the run's identity, and correcting a value in it rechecks any replay without changing the run (record 0005).
- Within-subject variation: how far one healthy adult's own reading, or the change in an amount in their body over a day, varies from one day to the next under the same conditions, as an SD from a primary study, else a compiled source or a textbook value at its tier (section 6), else an inference record that adjusts a sourced one by a sourced shift or, for an amount no method can measure, reasons from the sourced flows into and out of it. The return check allows a reading to change by up to 1.96 × √2 times it, the reference change value without its analytical part, and the certificate allows an amount to drift by up to 1.96 times it over any 24 hours (record 0004).
- Snapshot: the value of a produced quantity at one time and site, by one method.
- Source record: one located claim in clinically reviewed literature: the source, the page or table, the claim, the population measured, the method and the tier (how strong the source is for that claim, section 6).
- Inference record: a model's inferred range for a parameter or a number of the patient's day that no source covers, for a healthy range or within-subject variation it adjusts by a sourced shift, or for the within-subject variation of an amount no method can measure, with the population, the model that made it, its reasoning, the source records it reasons from and the search that found no source. It is never a source record: it has no tier, its value displays as inferred, and a source replaces it once one covers the same value.

Reading the body:

- Observer: anything that reads the body without changing it, such as the monitor, the chart's vital readings or a lab result. It reads each accepted step in order through a declared set of readings (a lens) and keeps its own state.
- Observation: a snapshot an observer takes, with its site, method, time taken and time resulted. It never writes back.
- Chart: the clinicians' documentation of the patient (assessment, vitals, medication administration, labs, progress notes). It writes chart entries, never state. Simulating the documentation is part of the simulation.

The software:

- Layer: one level of the architecture in section 1.
- Contract: the core contract (`ng-contract`): the map, the language laws are declared in, the compiler that checks them into one body, and the solver interface. It is the only meeting point of laws and solver. Detail: `docs/architecture/solver-contract.md`.
- Solver: the numerical core that advances the state over one step. It sees the compiled body only through the contract's solver interface.
- Arena: the benchmark that runs the candidate solver cores on the same compiled body and picks one, on conservation, accuracy, replay and speed.
- Certificate: the automatic check that every conserved quantity balances, that charge never drifts, and that every other quantity drifts over every sliding 24 h window no further than its within-subject variation allows (record 0004). It shows stability, not accuracy.

## 3. Hard rules

1. One patient per engine, with one state and one event log. State, chart entries and observations are different kinds of fact about that one patient and never merge: state is computed physiology, a chart entry is what a clinician documented, and an observation is a reading taken from state. The chart writes chart entries, never state (rule 13). Reason: at the bedside the chart and the monitor describe one patient, so both must come from one history; keeping the kinds of fact apart means a documented or displayed value can never become physiology.
2. No code writes a vital sign. Vitals are produced quantities, computed from state, laws and parameters.
3. Amounts are the state. Every mL, mmol and joule balances in the ledger at every step.
4. Variation is seeded. The same packet, seed and events give identical bits at any pace.
5. The engine is native Rust. Another build target may be explored later and never shapes the core.
6. Pulse is a reference to compare against, never a pass bar. Matching it would tie the engine to Pulse's modeling choices and gaps.
7. Death is modeled physiology. A numerical failure is an engine fault and never writes death. Cardiac arrest starts a code blue: a resuscitation effort, in an inpatient setting. Untreated or unsuccessful, arrest ends in death; resuscitation can succeed on a recoverable patient. Death types exist from stage 1; death behavior is built in a later scope stage.
8. The core contract is the only meeting point of laws and solver. Laws are declared as data, checked and compiled into one body; a solver core sees that compiled body only through the contract's solver interface and never runs organ code. The contract also refuses a second owner for any drug process at a site (one owner per site, substance, role and pathway). An explicit integrator (Heun) serves only as a test bench. The solver arena picks one core written in Rust, and the developer reviewer confirms the pick after a plain explanation of the result. SUNDIALS IDA is not a candidate: it would put a C dependency in a native Rust engine, its multistep method does not fit the contract's step interface, and neither candidate solver design proposed it. Reason: one interface lets different solver cores be compared on the same body, and because no organ code runs inside a solve, no organ can keep hidden state that would break replay (rule 4).
9. Organize by networks, not organ silos (see Organ in section 2).
10. Laws are declared once per site class, not per named organ.
11. Values are evidence-based ranges with a stated population, not single points.
12. Every non-test value traces to a source record or, where rule 16 allows, to an inference record. Either can be documented and displayed, and an inferred value always displays as inferred. Test values are obviously synthetic, so no one can mistake them for clinical numbers; clinical values enter only in a reviewed packet or a reviewed healthy-range set.
13. The chart and every observer are read-only on physiology. The only path from a clinician to the body is an authorized, timestamped command through the boundary port.
14. Disease logic never lives inside a law; it enters as parameter shifts or timed events.
15. No static numbers. Every quantity the body produces (pressures, flows, concentrations, temperatures, rates, lab values) is a continuous inflow and outflow; a measurement is a snapshot at a time and site. The engine stores mechanisms and parameter ranges, never a fixed value for a produced quantity.
16. Sources are clinically reviewed literature only, with a stated tier; never wikis or unreviewed web pages. The literature attests clinical content: attesting an anchor is a check of its citation against this rule, not a person's sign-off. A model's inference is never a source. It may fill a parameter or a number of the patient's day that no source covers, through an inference record that names the search that found no source and whose own source records pass that check. It never sets an anchor, and it sets a healthy range or a within-subject variation only by adjusting a sourced one by a sourced shift, or, for an amount no method can measure in a living person, by reasoning from the sourced flows into and out of it. The developer reviewer reviews the software and never attests clinical content.
17. An anchor on a produced quantity is a snapshot: it states its measurement method and averaging window, and admission matches the model output on both.

## 4. Invariants (each becomes a test)

| # | Invariant | How it is tested | Rule |
|---|---|---|---|
| I1 | Every conserved quantity balances in the ledger at every step, however finely the body is divided | The certificate over every sliding 24 h window, on a grouped body and on a finely divided one | 3 |
| I2 | The same packet, seed and events give identical bits at any pace, however the run is split into calls, and when resumed from any checkpoint | Runs at unbounded, live and irregular pace, with pauses and fast-forward, and resumes into a freshly built solver, compared bit for bit | 4 |
| I3 | A retry from a checkpoint with the same seed replays the same course; a new seed gives a different course that still meets every other invariant | A retry in the same process after the run went further equals the same branch loaded from its saved checkpoint; a run with a new seed passes the invariant tests | 4 |
| I4 | A step that fails its checks changes nothing: state, ledger, pending events and stream positions stay byte-identical | Force a refusal at a delivery instant and compare snapshots before and after; the delivery then applies exactly once | 3 |
| I5 | An event that arrives during the run gives the same bits as the same event known from the start; an event timestamped before current simulated time is refused | Every event admitted mid-run is compared with the same event known at admission; a back-dated event is refused | 4, 13 |
| I6 | No code path sets a vital sign | A program that writes a vital fails to compile | 2 |
| I7 | Physiology bits are identical with observers and the chart attached, detached or changed | Runs compared with observers attached, detached and changed; a new set of readings applied to a finished replay leaves physiology unchanged | 13 |
| I8 | Every random value depends only on the seed, its stream and the physiological event it has reached, never on solver steps or attempts | Force an extra solver evaluation before a random event, and run under two step policies; the draws do not change | 4 |
| I9 | The patient seed, the run seed and the observation seed never share a stream | Adding a measurement leaves physiology bits unchanged | 4 |
| I10 | Variability changes rates; it never adds or removes an amount | Every change in an amount comes from a transfer with two ends; no noise term reaches an amount | 3 |
| I11 | Dividing a site into finer sites keeps every regional total exact, keeps every sourced check the grouped region passed within its target's range, and passes the sourced check that earned the split: the check the grouped region failed, or the target set for the clinician's question | The children's start amounts sum to the parent's; every sourced check runs on the divided body, the one that earned the split included; manufactured cases with closed-form answers test its numerics. A comparison with the grouped body is reported for review and never decides pass or fail | 10 |
| I12 | Rebuilding a packet or a healthy-range set from its records gives the same hash | Hash check | 12 |
| I13 | Felt pain is computed from tissue signals (injury, ischemia, inflammation, swelling); no parameter sets pain or how far it differs from the signs | Check the compiled parameter list: no parameter or input sets pain. At least one early test patient cannot report pain (sedated, ventilated or delirious) | 15 |
| I14 | No produced quantity is stored as a constant or a parameter | Check the compiled parameter list, and a program that sets a produced quantity fails to compile | 15 |
| I15 | Disease logic never lives in a law: a disease enters as parameter shifts or timed events in the event log | Applying a disease leaves the body model's hash unchanged | 14 |
| I16 | Every non-test value carries a range with its population and a literature source of a stated tier, directly or through the identity or anchors it is derived or fitted from, or, where rule 16 allows, an inference record that names the search that found no source. No anchor rests on an inference record, and a healthy range or a within-subject variation rests on one only to adjust a sourced one by a sourced shift or, for an amount no method can measure in a living person, to reason from the sourced flows into and out of it | Building a reviewed packet or healthy-range set refuses a value without that trace, an authored value outside its population range and an inference record that names no search; the packet also refuses an anchor that rests on an inference record, and the set an inferred value that names neither a sourced value and sourced shift nor, for an amount no method can measure, why none can and the sourced flows it reasons from | 11, 12, 16 |
| I17 | The event log only appends, and replaying it rebuilds the run | A changed or removed event is refused, and replay from the log gives identical state and ledger bits | 1, 4 |
| I18 | The only way into the body is a timestamped event through a port | Nothing outside an accepted step can write state (a compile-fail test), and every input carries a time and an id | 13 |
| I19 | A healthy adult who lives an ordinary day with nothing done to them stays healthy and stable for 48 simulated hours from admission | A run of each reviewed packet of a healthy adult passes the certificate, the healthy-range check and the return check against a reviewed healthy-range set (record 0004) | 2, 3, 4 |

## 5. Structural model

- Two kinds of relation. Containment ("part of"): cell, tissue bed, functional unit, organ, region, person. Networks ("connected to"): real anatomical segments that branch, merge and run in series. The networks below are the reference anatomy: what each grouped region stands for, and what a split moves toward.
  - Blood: a four-chambered heart with valves driving two circuits in series, pulmonary and systemic. Arteries branch through arterioles into capillary beds, which merge through venules into veins, with collaterals, anastomoses and portal circuits (hepatic, renal, hypophyseal).
  - Lymph: blind-ended capillaries in tissue beds merge into collecting vessels, pass through nodes and drain by the thoracic and right lymphatic ducts into the great veins.
  - Airways: the trachea divides into bronchi, then bronchioles, generation by generation, down to respiratory bronchioles, alveolar ducts and sacs at the gas-exchange surface.
  - Nerves: afferent paths from receptors to the spinal cord, brainstem and cortex; efferent somatic and autonomic paths (sympathetic chain and ganglia, cranial and sacral parasympathetic outflow) back to every organ.
  - Cardiac conduction: sinoatrial node, atrial paths, atrioventricular node, His bundle, bundle branches and the Purkinje network.
  - Urinary tract: nephrons (glomerulus, proximal tubule, loop, distal tubule) drain into collecting ducts, calyces, renal pelvis, ureters, bladder and urethra.
  - Biliary tree: canaliculi merge into intrahepatic and hepatic ducts, with the gallbladder on the cystic duct, into the common bile duct and the duodenum.
  - CSF: made at the choroid plexus, flows through the ventricles and foramina into the subarachnoid space, and is absorbed into the venous sinuses.
  - Gut: segments in series (mouth, esophagus, stomach, small intestine, large intestine) separated by sphincters, each with its own wall, motility, secretion and absorption, with the liver and pancreas draining in.
- Grouping rule (record 0002). The map starts grouped. A grouped region states which structures of the reference anatomy it stands for, and the behaviors it cannot show because they happen only inside it. A scenario that depends on one of those behaviors is refused when it is built. The map splits a region only when a sourced check fails without the split, or when a clinician's question brings a sourced target for the behavior the split adds. A split meets I11 and keeps the speed aim in section 6.
- One motif at every scale. Each node is a bounded space with contents, exchange across its wall (downhill flow is free, uphill flow is paid by pumps), a supply and a drain on one or more networks, a local sensor and controller nested under a higher one, a demand, a reserve, and a damage integral when demand is unmet. No level is privileged; causes run up and down.
- Membranes. Every wall between two spaces is one membrane type with two settings: conductance, how easily each thing crosses it (water, each solute, each gas, protein, cells), and reflection, how much of each solute the wall holds back. Permeable, semi-permeable and non-permeable are ranges of these two settings, not separate kinds:
  - Permeable: reflection near zero, so water and the solutes in it cross together by bulk flow, as fast as the conductance allows.
  - Semi-permeable: some things cross and others are held back (reflection near one for those), so a held-back solute pulls water by osmosis. Most walls in the body are semi-permeable: the capillary wall holds back most protein, and the cell membrane holds back most ions except through its channels and pumps.
  - Non-permeable: conductance near zero and reflection near one, so the wall holds everything back. It bounds a space and passes on pressure, as an enclosure's wall does.
  - Routes: bulk flow through gaps, driven by pressure; diffusion, driven by a concentration or partial-pressure difference; channels and carriers, which move things downhill; and pumps, which move things uphill and draw energy from the ledger. Each route's rate is a law of its driving difference and the membrane's settings at that site, which are site parameters in the map with population ranges.
  - Membranes named so far: the capillary wall in every tissue bed (revised Starling exchange through the glycocalyx, with a permeability that inflammation changes), the cell membrane (pumps and channels set cell ion content, and water follows tonicity), the renal tubule wall (reabsorption and secretion by transporters) and the alveolar gas-exchange surface (gas diffusion).
  - Not yet specified: the full list of membranes by site class and, for each, what crosses, by which route, with its sources. Each is required before exchange at that site is physiologically and anatomically correct.
- Four universal rules. Nothing is made or lost. Flow runs downhill unless work pushes it. Every control loop has limits. Set points drift slowly with the patient's history.
- Driving pressure. Flow through a bed inside an enclosure is inflow pressure minus the higher of venous and enclosing pressure. One law covers cerebral perfusion pressure, coronary perfusion, abdominal compartment syndrome, limb compartment syndrome and tamponade.
- Perfusion territories follow real arterial anatomy. A territory is the tissue a named branch supplies, down to the bed it ends in. Once the map splits a territory out, it records which branches feed which tissue, where territories meet, and what collateral paths link them.
  - End-artery territories have no meaningful backup: renal segmental and interlobular arteries, retinal artery, coronary branches beyond collateral reach, cortical and lenticulostriate branches of the brain.
  - Collateralized territories have alternate routes: the circle of Willis, the palmar arches, the gut arcades and marginal artery, limb collaterals around the knee and elbow, coronary collaterals that grow with chronic narrowing.
  - Dual-supply organs take blood from two sources: the liver (hepatic artery and portal vein), the lungs (pulmonary and bronchial circulations), the brain stem and spinal cord (vertebrobasilar and spinal arteries).
  - Watershed zones lie at the far edges of neighboring territories and fail first when pressure falls: between the anterior, middle and posterior cerebral territories, at the splenic flexure and rectosigmoid junction of the colon, in the renal medulla, and in the subendocardium.
  - Coronary territories follow the left anterior descending, circumflex and right coronary arteries, with dominance set per patient.
  - Gut territories follow the celiac trunk, superior mesenteric and inferior mesenteric arteries.
  - Partial narrowing copes at rest and fails with demand (claudication, angina); occlusion fails at rest (stroke, MI). How much tissue is lost depends on the branch, its collaterals and the watershed it sits in. These outcomes emerge from the anatomy, flow and demand once the map holds the territories apart, never from coded rules. Until then they are on the grouped region's list of behaviors it cannot show.
- Thresholds belong to the patient and state. An autoregulation edge (for example renal) is a patient parameter that drifts, such as upward in long-standing hypertension.
- Shared body water links spaces, so pleural effusion and peripheral edema come from the same ledger.
- Nervous system: the nervous system reaches every organ and drives each organ function the way it does physiologically, through real anatomical paths, never through shortcuts that set an organ's output.
  - Sensing (afferent): receptors in every tissue (baroreceptors, chemoreceptors, stretch, thermal, osmotic, nociceptive, proprioceptive, visceral and special senses) send signals along named nerves and tracts to the spinal cord, brainstem, hypothalamus, thalamus and cortex.
  - Motor (efferent, somatic): upper and lower motor neurons drive skeletal muscle, including the diaphragm and respiratory muscles, posture, swallowing, voiding and defecation under voluntary control.
  - Autonomic (efferent): sympathetic paths (thoracolumbar outflow, chain and ganglia, adrenal medulla) and parasympathetic paths (cranial nerves including the vagus, and sacral outflow) set heart rate and contractility, vessel tone, airway tone, gut motility and secretion, kidney blood flow and renin release, bladder function, pupils, sweating and metabolism.
  - Enteric nervous system: the gut's own network, coupled to autonomic input, runs motility and secretion segment by segment.
  - Reflex arcs: spinal and brainstem reflexes (baroreflex, chemoreflex, cough, gag, swallow, micturition, withdrawal, stretch) close loops locally, nested under higher control.
  - Higher control: the brainstem sets breathing rhythm and cardiovascular tone; the hypothalamus sets temperature, thirst, hunger, sleep and wake, and links to the endocrine system (pituitary and adrenal); the cortex and limbic system add consciousness, voluntary action, perception, emotion and stress responses.
  - Level of consciousness, sedation and delirium emerge from brain perfusion, oxygen, carbon dioxide, glucose, temperature, drugs and injury, never from a set score.
  - Pain: a site signal from injury, ischemia, inflammation and swelling, carried by nociceptive paths and turned into felt pain that raises heart rate, changes breathing and releases stress hormones. Report and signs diverge for real causes (adaptation, beta blockade, nerve block, sensitization, or a patient who cannot report).
  - Injury anywhere along a path (nerve, cord level, brainstem or cortex) removes or changes exactly the functions that path carries.
- Schema data, not code: topology, enclosure, site heights for gravity and sensor sites are data in the map. Ports are site-indexed. Splitting a site changes map and packet, not laws.
- Pieces needed before the solver arena runs, so later stages need no rewrite: topology and enclosures in the map, site-indexed ports, laws per site class, the rule that an event arriving late gives the same result as the same event known early, seeded draws inside laws, the test that a retry in the same process equals a loaded checkpoint, and the slowest steps' wall time in arena results.

## 6. Fidelity aims

- Variability. Most beat-to-beat variation comes from modeled loops (breathing, baroreflex); a small seeded term covers only what is not modeled, tied to the simulated clock, never to solver steps.
- Evidence standard. Sources are clinically reviewed literature and textbook figures backed by the latest evidence from all disciplines. Evidence ranks in four steps. Primary data measured in the target population comes first. A compiled source comes next: a reference compilation, a review or a consensus standard that pools located primary data, under the compiled tier, with a capped certainty and a locator that names the table or section. When neither is available, a textbook value comes next: a textbook collates reviewed knowledge, so it ranks above inference and above a weaker or mismatched single study. It may support a magnitude, a range or a shape, under the textbook tier, with a capped certainty and a locator that names the edition and page, and coverage shows its tier. Every anchor or range resting on a compiled source or a textbook records whether a primary source would upgrade it. An inference record ranks below every tier. It fills a parameter or a number of the patient's day, adjusts a healthy range or a within-subject variation for a shift no study measured, or sets the within-subject variation of an amount no method can measure, only while no primary, compiled or textbook source covers it, and the first source that does replaces it.
- Continuity. One continuous run over a shift, a stay or days, starting at admission and never reset, with one cumulative ledger.
- Speed and clock. At least 100 times real time with full waveforms, so a simulated day takes about 15 minutes, plus a live mode at real clock speed. The clock can be stopped and paused, and a run can fast-forward through quiet stretches to the points where interventions happen. Every pace, fast-forward included, gives identical bits (rule 4).
- Three claims. A Pulse diff, a match to published data and clinical validity are separate claims. A clinical validity claim needs at least a reviewed packet; a reviewed packet alone does not prove it.
- Live vital stream: the clinician-facing monitor is a required part of the system. It shows the simulated patient's vital signs in real time and behaves like a bedside Philips IntelliVue-style monitor: numerics, continuous waveforms (ECG, arterial pressure, plethysmography, capnography and others), trends and alarms. It is the visible counterpart of the body: an observer that reads each value as a snapshot at its sensor site and method, never writes physiology, and never shows a value the body did not produce.
  - Measurement follows current clinical practice and is anatomically correct: every parameter is read by its real physical tool at its real placement on the body, and the reading carries that tool's behavior. Examples, not exhaustive: ECG from five leads placed on the chest and limbs; non-invasive blood pressure from a cuff on a limb; arterial blood pressure from an arterial line; oxygen saturation from a pulse oximeter probe; respiratory rate from chest rise sensed through the ECG leads (lead II).
  - Alarms. The patient's thresholds are not hidden: they reach the learner as on a real monitor, through alarm limits on the monitored vitals, set to tip the learner off. Not every deterioration sets off an alarm. Alarm limits belong to the monitor's specification, not to the patient's packet.

## 7. Questions to decide

| # | Question | Model default | Alternative | Who decides |
|---|---|---|---|---|
| Q5 | How evidence is displayed (UI/UX) | Keep the full source record (source, range, population, method, page or table) off the main display, which it would crowd; a developer console is a possible later home | The citation only on the main display, with the full record on request | Models recommend; developer reviewer approves |

Q5 asks only how evidence is shown. Every value still traces to a source record (rule 12).

## References

Records in this repo form one series from 0001. A record from nightingale-pulse is cited as "pulse 000N" with a permalink at the import commit; none is imported. A pulse record that still matters is written fresh as a new record in this series, never copied.

This SRS carries forward the rules of these nightingale-pulse records. Each rule here states its own reason, so none depends on them.

- [pulse 0001](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0001-native-rust-first.md): native Rust first (rule 5)
- [pulse 0002](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0002-pulse-as-reference.md): Pulse as a reference (rule 6, section 6)
- [pulse 0003](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0003-one-patient-record.md): one patient (rule 1)
- [pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md): demise is modeled physiology (rule 7)
- [pulse 0006](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0006-first-milestone-stable-healthy-body.md): the first milestone (rules 2 to 4)
- [pulse 0007](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0007-solver-picked-by-arena-ida-dropped.md): the solver arena (rule 8)
- [pulse north star](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/north-star.md): the vision the rules came from (rule 2)
