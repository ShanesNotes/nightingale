# Nightingale

The words of the native engine for one simulated patient. An agent loads this file before it writes prose, a ticket, a test name or a type name. Section and rule numbers refer to `docs/srs.md`. An entry that repeats an SRS section 2 definition matches it word for word, and `tools/verify.d/10-docs.sh` checks that.

## The patient and the run

**Engine**:
The simulated human body of one patient. One engine runs one patient.

**Linked engines**:
Several patients simulated together, one engine each. How engines link is not yet specified.

**Patient**:
The one person an engine simulates, described by a packet.

**Packet**:
The bundle of one patient's parameters and the day the patient lives. It holds every parameter value with where it came from (cited, drawn from a population range or chosen inside one with a reason, derived by an identity, fitted when the start state is solved, or inferred by a model), the anchors the start state must meet, the day (sleep, posture, meals, drinks and room conditions) that the run's boundary events are made from, and the source, population and inference records behind them. A packet is either reviewed (clinical values with citations, and a count of its inferred values) or a synthetic test fixture. Its hash is fixed at admission and is part of the run's identity.

**Admission**:
The moment a run starts. The start state is solved once here and never imposed again.

**Run**:
One continuous simulation of one patient from admission. It never resets.
_Avoid_: session (the code type that holds a run), simulation

**Step**:
One advance of simulated time. The solver proposes the next state, the engine checks it, and the step is accepted whole or refused whole.

**State**:
The patient's accepted condition at one instant: the amount in every pool, each controller's memory (such as a set point), each discrete mode (such as a valve open or shut), the position of each random stream, and what the solver needs to continue exactly. Only an accepted step changes it.
_Avoid_: record, physiology truth

**Ledger**:
The account of every conserved quantity over the run: how much each site holds, and every amount that entered or left the body since admission. One ledger per run; it never resets.
_Avoid_: books, inventories, I&O ledger, record

**Event**:
A timestamped input to the run, with an id: a boundary input (a meal, a drink, a posture change, room conditions, the clock, sleep), an intervention, or a timed disease event. A chance event that a law fires is not an input; replay recreates it from the seed.

**Intervention**:
An event a clinician applies with a real tool, such as a drug, fluid, a nasal cannula, a position change or a procedure. It changes the body's boundary conditions through a port and acts on the same physics as everything else.

**Event log**:
The append-only list a run is rebuilt from: the run's identity (body model, packet, run seed, solver settings) and every event in time order. Nothing in it is changed or removed.
_Avoid_: record, recording

**Seed**:
The number that fixes a run's random draws. Three seeds are kept apart: the patient seed (which patient is drawn from a population), the run seed (variation during the run) and the observation seed (measurement error).

**Stream**:
A named sequence of random draws inside one law. Its draws depend only on the seed, its name and which physiological event (a beat, a breath) it has reached, so adding a stream never changes another.

**Replay**:
Rerunning a run from its event log, with the same run seed (identical bits) or a fresh one (a different course through the same events).

**Checkpoint**:
A saved copy of the whole accepted state at a step boundary, from which a run resumes exactly or branches.
_Avoid_: save (in prose)

**Seal**:
A digest that chains the seal before it with one accepted step's image and ledger chunk, so one seal stands for the whole accepted course up to that step. It covers nothing that waits.

**History store**:
Host storage for the links of the seal's chain, addressed by seal and trusted for nothing: a load checks each link against the seal that asked for it.

**Scope stage**:
One of the five stages of scope in section 1, built bottom up. Not a solver stage, which is a numerical sub-step inside one step.

**Scenario**:
A case authored for a learner, made of a packet, its events and the behaviors it teaches. Scenarios belong to later scope stages.

**Start state**:
The first accepted state of a run, solved at admission from the packet's anchors.
_Avoid_: initial conditions

**Run identity**:
What makes two runs the same run: the body model's hash, the packet's hash as admission fixes it with the moment the run starts and the drives it read, the run seed and the solver core's declared resolution. Pace is not part of it.

**Pace**:
How fast simulated time runs against the wall clock: unbounded, live, paused or fast-forwarded. Every pace gives identical bits (rule 4).

**Branch**:
A run continued from a checkpoint under its own id, with its ancestry kept. The run it came from is never reset.

**Rewind**:
Continuing from an earlier checkpoint on a new branch in the same process. It must give the same bits as that branch loaded from its saved checkpoint (I3).

**Boundary port**:
The one port for events from outside the body: intake, posture, environment, clock and sleep, and later interventions. Each event is timestamped, has an id and is applied once.

**Breakpoint**:
An instant that ends a step: an event or a guard crossing. A display refresh, a pace change or a checkpoint request is never one.

**Delivery**:
An event's amount entering the body through a boundary at a steady rate from the event's instant until its end. Overlapping deliveries add.

**Waiting input**:
An admitted event not yet due. No seal or ticket digest covers it, so nothing a core reads carries it until a cut shows it.

**Span**:
The length of simulated time a solver core commits to for its next step, before the session issues a ticket.

**Cut**:
A ticket ended at a waiting input, solved only after the same ticket at the core's span has passed.

**Fence**:
The earliest stamp a late input may take: the instant of a cut that did not publish at its own end, because it was refused or landed short, or the first instant admission did not read.

**Blind step**:
The step the record would accept as if nothing waited, landing included. An attempt reads the timeline only after it has one.

**Landing**:
The end of a step at a guard's crossing: a ticket that crosses, and one at most the core's floor shorter that does not. The contract finds it by re-solving ordinary tickets, so no core locates an event.

**Due input**:
An event whose instant is the start of a step. It is applied when that step is accepted, so a refused step leaves it pending.

**Drive frame**:
What the boundary supplies over one step, frozen when the step's interval is issued, so a stage at the step's end never reads a delivery that starts there.

**Causal step rule**:
A step is the solver's own interval, cut short only at the next breakpoint and blind to anything later, so an event that arrives during the run gives the same bits as the same event known from the start (I5). A waiting input shows only in a cut.

**Refusal**:
A step the record crate rejects, leaving state, ledger, pending events and stream positions unchanged (I4). The only way on is the solver's retry over a shorter interval.

**Engine fault**:
A step the solver or the model cannot complete under its declared settings, including a refusal below the minimum interval. It stops the run for diagnosis and is never death (rule 7).
_Avoid_: crash, death (for a numerical failure)

**Death**:
When the patient's physiology stops, the patient is dead. A numerical failure is never death (rule 7).
_Avoid_: fatal latch

## Chance

**Forcing**:
Smooth seeded variability tied to simulated time, through seeded knots, with a period and an amplitude from the packet. A law reads it as a positive factor with mean one, so it changes rates and never adds or removes an amount (I10).

**Hazard**:
A risk rate a law integrates until it crosses its stream's draw, which fires an event. The contract integrates the rate's positive part, so the integral never falls. Better care lowers the rate, so the same draw gives a later event or none.

## The body's structure

**Map**:
The patient's anatomy held as data: every site with its class and position, the containment tree, the networks, the enclosures and the nerve links. The solver contract calls it the anatomy. Not the atlas's coupling map, which links systems, not sites, nor a wayfinder `map.md`, which indexes an effort's tickets.
_Avoid_: topology

**Site**:
One place in the map, such as a capillary bed, a vessel segment, a nerve junction, an enclosed space, a piece of tissue or a nerve segment. Every site has one site class and a position in the body.
_Avoid_: node, location

**Site class**:
A kind of site whose members share the same laws.

**Site classes in the contract**:
A closed set of six: capillary bed (a tissue bed), vessel segment, junction, enclosed space (an enclosure), tissue site and nerve segment. A new class is a contract version.

**Vessel segment**:
A conducting segment of a flow network between two branch points.

**Junction**:
A neuroeffector junction, where a nerve releases a transmitter that is then cleared. A branch point of a network is not a junction.

**Tissue site**:
A piece of tissue that reports afferent signals, such as stretch or nociception, to a nerve segment.

**Nerve segment**:
A spinal or cranial nerve segment where afferent signals from many sites converge.

**Served-by edge**:
The link from a site to the nerve segment it reports to, with the kind of afferent signal it carries: stretch, nociceptive, chemical or thermal.

**Containment**:
The "part of" relation: cell, tissue bed, functional unit, organ, region, person. It is a tree, so an organ's totals are the sums over its sites.

**Region**:
A named node of the containment tree, such as an organ, a perfusion territory or the whole body. Its totals are exact sums over its sites.
_Avoid_: organ module

**Reference anatomy**:
The real structure of the body's networks as section 5 describes it, which each grouped region stands for and each split moves the map toward. It is not the map.
_Avoid_: real structure, full map

**Grouped region**:
A region the map holds as fewer sites than the reference anatomy it stands for. It states that part of the reference anatomy and the behaviors it cannot show because they happen only inside it.
_Avoid_: lump, lumped compartment

**Perfusion territory**:
The tissue a named arterial branch supplies, down to the beds it ends in (section 5).

**Network**:
The "connected to" relation: a graph of real anatomical segments that crosses organ boundaries. Section 5 lists the networks.

**Circulation**:
The largest part of the vascular network in which every site reaches every other and that holds a vessel segment. Every capillary bed lies on it, so blood reaches each bed and can return.

**Tissue bed**:
A capillary bed with the tissue it serves (its plasma, interstitial fluid and cells), where blood and tissue exchange.

**Organ**:
A named part of the containment tree where several networks meet inside one enclosure, such as the kidney or the heart. It owns no pool and no law of its own; its behavior comes from the laws of its site classes and the networks that run through it.
_Avoid_: organ compartment, compartment

**System**:
One of the 13 body systems in the atlas, such as cardiovascular, fluids, renal or nervous. A system's code declares laws per site class and owns no pool.

**Enclosure**:
A bounded space (the skull, the pericardium, the chest, the abdomen, a limb compartment) whose pressure, set by the volume inside it and the stiffness of its wall, is felt by every tissue bed inside it.
_Avoid_: pressure wire

**Compliance**:
How an enclosure's pressure answers the volume inside it: rigid, elastic, or held by a named stand-in.

**Position**:
A site's place in the body frame. Posture rotates the frame, so every site's hydrostatic head follows from one place and no law carries gravity.

**Membrane**:
A wall between two spaces, classed by what can cross it (section 5).

**Pool**:
The amount of one conserved quantity (water, sodium, oxygen, heat, and so on) held in one site, or in one space of a site such as a tissue bed's plasma. The body names its pools, the state holds their amounts, and no organ or system owns one.

**Space**:
One part of a site that holds pools: a tissue bed's plasma, interstitial fluid or cells, a vessel segment's lumen, or an enclosed space's own fluid. A junction, a tissue site and a nerve segment have none.

**Conserved quantity**:
A quantity the ledger balances: water, an element (sodium, potassium, chloride, calcium, magnesium, phosphorus, nitrogen), charge, energy, a moiety such as glucose or catecholamine, or carrier mass. The contract's `Book` type names one.
_Avoid_: book

**Conserved combination**:
A sum of pools that no transfer or reaction inside the body changes, such as all catecholamine across its free, bound and metabolite pools. The certificate checks these, not each free species.

**Transfer**:
A flow with two ends: what leaves one pool enters another pool or a receipt pool, so no amount appears or vanishes (I10).

**Receipt pool**:
A named pool outside the body, or behind a held stand-in, that takes what crosses the boundary, so a boundary transfer still has two ends.

**Route family**:
Every transfer of one kind the map implies: one per site of a class, such as plasma to interstitium at every capillary bed, or one per vascular edge. A system requests it whole through its scope, so the body owes a law for every one, and no law names a site.

**Left-out space**:
A space the schema states this body does not model, with its reason, such as cell water in a bench with no cell-water law. Its pools are not amounts, and no law may move or read them. Every other pool is an amount some transfer must move; one that none moves is an orphan pool, which compilation refuses.

**Partial volume**:
The volume one unit amount of a species takes up: a fixed coefficient of the schema, not a parameter. A space's volume derives from its amounts through it.

**Shared substrate**:
One of the ten resources the systems share (S1 to S10 in the coupling map), such as the vascular registry, or water and osmoles per space.

**Port**:
A declared point where something crosses a boundary: an input from outside the body, a coupling from one system to another, or a reading. Each port states what it carries and its unit, and is checked when the body is built.

**Law**:
A declared rule that computes a flow or a rate of change from state and parameters. A law holds no time step, no solver and no disease logic.

**Memory**:
Non-material state a law keeps, such as a filter's value or a set point. It moves only by a declared rate that the contract integrates, or by the reset its hazard guard fires, and it never holds an amount.
_Avoid_: organ state

**Set point**:
Slow memory in the unit of the variable it guards, compared with that variable only as a deviation. History and disease move it; nothing writes the guarded variable.

**Mode**:
A discrete state of a law, such as a valve open or shut. Only a guard changes it.

**Guard**:
A crossing of a computed value, such as a valve's pressure head passing zero, that ends a step and fires a declared change: a mode transition, a stream's next draw or a hazard's event.

**Rate class**:
How fast a process moves: wave, beat, reflex, minutes or hours. It constrains scheduling and never makes an organ lane.

**Rung**:
One level of detail of a system. Rung 0 is its simplest working version.

**Refinement**:
Replacing a grouped region of the map with its finer subtree, earned by a failing sourced check or by a clinician's question with a sourced target, and keeping every regional total exact and every sourced check passing (I11). A refined map is a new body model, never a mid-run zoom.

## Numbers and evidence

**Parameter**:
A property of the patient's body that laws read and nothing computes, such as a vessel's stiffness, a resistance, an organ's size or a reflex gain. Each has a population range.

**Produced quantity**:
Anything the body computes from state, laws and parameters, such as a pressure, a flow, a concentration, a heart rate or a lab value. It is never stored as a fixed number and never set.
_Avoid_: output, computed value

**Target**:
A value or range that a check compares a produced quantity against, such as an anchor, a healthy range or a within-subject variation. Every target rests on the literature, and only a healthy range or a within-subject variation may also rest on an inference record, which adjusts a sourced one by a sourced shift or, for an amount no method can measure, reasons from the sourced flows into and out of it (rule 16).
_Avoid_: golden value, goal

**Anchor**:
A target a produced quantity must meet when the start state is solved, stated with its measurement method and averaging window. An anchor is never a parameter's value. Methods are named as current charting practice names them (an arterial-line pressure is charted as from the arterial blood pressure monitor); that naming follows practice and is not a law, since practice can improve.

**Anchor statistic**:
What an anchor's value summarizes: an instant, a cycle mean or a window mean. An anchor on a pulsatile quantity is a cycle mean.

**Cycle**:
One pass of a periodic goal from the anchor moment, in equal intervals. The packet states it, so the packet's hash covers it, and a cycle mean reads it.

**Healthy range**:
The target for one reading, the central 95% of healthy adults over the averaging window their study used, matched to the patient in activity, sex, age or method wherever a source shows that one of these shifts the reading. It comes from a primary study, else a compiled source or a textbook value at its tier (section 6), else an inference record that adjusts a sourced range by a sourced shift (record 0004).
_Avoid_: normal range, normal limits, reference range

**Healthy-range set**:
Every healthy range and within-subject variation, with the source and inference records behind them, either reviewed or a synthetic test fixture. Physiology never reads it, so it is not part of the run's identity, and correcting a value in it rechecks any replay without changing the run (record 0005).
_Avoid_: normals, reference table

**Within-subject variation**:
How far one healthy adult's own reading, or the change in an amount in their body over a day, varies from one day to the next under the same conditions, as an SD from a primary study, else a compiled source or a textbook value at its tier (section 6), else an inference record that adjusts a sourced one by a sourced shift or, for an amount no method can measure, reasons from the sourced flows into and out of it. The return check allows a reading to change by up to 1.96 × √2 times it, the reference change value without its analytical part, and the certificate allows an amount to drift by up to 1.96 times it over any 24 hours (record 0004).
_Avoid_: intra-individual variation, day-to-day noise

**Snapshot**:
The value of a produced quantity at one time and site, by one method.

**Source record**:
One located claim in clinically reviewed literature: the source, the page or table, the claim, the population measured, the method and the tier (how strong the source is for that claim, section 6).
_Avoid_: evidence record

**Tier**:
How strong a source is for one claim (section 6). Primary data from the target population ranks first, then a compiled source (a reference compilation, review or consensus standard), then a textbook value, then inference. A compiled source or a textbook may support a magnitude, a range or a shape, with a capped certainty and a locator (table or page; edition and page for a textbook).

**Inference record**:
A model's inferred range for a parameter or a number of the patient's day that no source covers, for a healthy range or within-subject variation it adjusts by a sourced shift, or for the within-subject variation of an amount no method can measure, with the population, the model that made it, its reasoning, the source records it reasons from and the search that found no source. It is never a source record: it has no tier, its value displays as inferred, and a source replaces it once one covers the same value.
_Avoid_: estimate, assumed value

**Population record**:
A parameter's range in a stated population (a mean and spread, quantiles or an interval), with the source records behind it and the parameters it varies with.

**Value provenance**:
How a packet value was set: specified (cited), sampled (drawn from a population record), derived (by an identity), calibrated (fitted at admission), inferred (by a model, through an inference record) or authored (chosen inside a population record's range, with a reason). In a synthetic fixture a note stands where a source would, and a value set by hand is a fixture value. A start value admission moves is solved.

**Synthetic fixture**:
A packet or healthy-range set of obviously synthetic test values with no citations. The arena accepts only this kind.

**Validity range**:
Where a law's form is trusted, with its basis. A state outside it still publishes, and the step records the excursion.

**Developer reviewer**:
The person who reviews the software and confirms the arena's pick.

## Reading the body

**Observer**:
Anything that reads the body without changing it, such as the monitor, the chart's vital readings or a lab result. It reads each accepted step in order through a declared set of readings (a lens) and keeps its own state.

**Observation**:
A snapshot an observer takes, with its site, method, time taken and time resulted. It never writes back.
_Avoid_: finding

**Lens**:
A compiled set of readings (channels and an output grid), hashed apart from the body, so a new lens applies to any replay without changing physiology (I7).

**Channel**:
One reading in a lens: the method that takes it, the produced quantity and site it reads, its window and delay, and its measurement error.

**Output grid**:
The instants a lens reads: every whole multiple of one period. No law reads it and it never cuts a step.

**Reference interval**:
A range an observer may flag a reading against, with its evidence. Physiology never reads it.

**Monitor**:
The clinician-facing observer that shows the patient's vital signs in real time as a bedside monitor does: numerics, waveforms, trends and alarms (section 6).

**Alarm limit**:
A threshold on a monitored vital that alarms when crossed. It is a monitor setting, not part of the packet.

**Chart**:
The clinicians' documentation of the patient (assessment, vitals, medication administration, labs, progress notes). It writes chart entries, never state. Simulating the documentation is part of the simulation.

**Chart entry**:
What a clinician documented about the patient. It is never state, and a note is never parsed into physiology.
_Avoid_: chart truth

## The software

**Layer**:
One level of the architecture in section 1.

**Contract**:
The core contract (`ng-contract`): the map, the language laws are declared in, the compiler that checks them into one body, and the solver interface. It is the only meeting point of laws and solver. Detail: `docs/architecture/solver-contract.md`.

**System crate**:
The code of one system, such as `ng-renal`: laws declared once per site class, owning no pool.
_Avoid_: organ crate

**Record crate**:
`ng-record`: the state, the ledger, the pending events and the check that accepts or refuses each step. It is the only code that changes state.

**Physiology closure**:
The crates a step's arithmetic runs through: the rows the crate table in `docs/architecture/solver-contract.md` marks yes, which are the contract, the system crates, the solver and the record crate. It admits no crate from outside the workspace.

**Fragment**:
What a system crate declares: its laws, ports, parameter slots, modes and streams, as data the contract compiles. No fragment runs during a solve.

**Kernel**:
One law's closed expression graph, built once at declaration from the contract's own operators. It reads no time step, solver, state store or host math.
_Avoid_: callback

**Compiled body**:
The checked, immutable result of compiling the map and every fragment. A solver core sees only this, through the solver interface.

**Model hash**:
The hash of the compiled body, map included. A finer map or a changed law gives a new hash; parameter values are not in it.

**Solver**:
The numerical core that advances the state over one step. It sees the compiled body only through the contract's solver interface.

**Solver interface**:
The one seam between the compiled body and a solver core. Every core implements all of it, and no adapter sits between cores.
_Avoid_: adapter

**Solver core**:
One implementation behind the solver interface. The arena compares solver-A, the compiled conservation graph, and solver-B, the beat-synchronous ledger stepper.
_Avoid_: candidate A, candidate B

**Resolution**:
A solver core's declared settings: method, version, arithmetic profile, event rules, tolerances and partition policy. It is part of the run identity.

**Arithmetic profile**:
The floating-point operations a run may use: correctly rounded add, subtract, multiply, divide and square root, plus the contract's own versioned routines. It is why a replay gives identical bits.

**Extent**:
The amount a transfer moved over one step, kept as gross forward and reverse parts and produced only by summing evaluated rates.

**Stage witness**:
The record of each numerical stage a step used (its time, point and weight), which the contract re-evaluates on the record crate's own values before the step can be accepted.

**Reference integrator**:
The fully coupled backward Euler core in `ng-validate`, run at three refinements, against which the accuracy gate measures each solver core.

**Bench integrator**:
An explicit Heun integrator that serves only as a test bench, never as a solver core (rule 8).

**Arena**:
The benchmark that runs the candidate solver cores on the same compiled body and picks one, on conservation, accuracy, replay and speed.

**Slice**:
The systems the arena runs for real: pulsatile cardiovascular, fluids, renal, a minimal nervous baroreflex, the receptor layer and the boundary port, for 48 simulated hours with synthetic values.

**Held stand-in**:
A system outside the slice, held by a declaration that names every route on which it supplies or removes an amount, so stability bought with held support shows in the certificate.
_Avoid_: stub, mock

**Coverage manifest**:
The compiled list of which conserved combinations close inside the slice and which rest on held stand-ins.

**Certificate**:
The automatic check that every conserved quantity balances, that charge never drifts, and that every other quantity drifts over every sliding 24 h window no further than its within-subject variation allows (record 0004). It shows stability, not accuracy.

**Healthy-range check**:
The check that each reading in the lens stays inside its healthy range for what the patient is doing at that moment. With the certificate and the return check, it decides the first milestone (record 0004).
_Avoid_: health check, normal-range check

**Return check**:
The check that the patient meets the anchors again, within the change their within-subject variation allows, at the same time of day on each later day of the run, with nothing reimposed since admission. With the certificate and the healthy-range check, it decides the first milestone (record 0004).
_Avoid_: re-anchoring, daily reset

**Accuracy gate**:
The arena check that runs a solver core on manufactured cases with known answers and against the reference integrator, because a step can close every ledger and still be wrong.

**Replay gates**:
The arena checks that pace, resuming, late events, rewinding, extra evaluations and attached observers never change physiology bits, and that a step policy moves no event further from the reference integrator than a declared tolerance.

**Pulse**:
The Pulse Physiology Engine, a reference to compare against and never a pass bar (rule 6). A Pulse diff, a match to published data and clinical validity are three separate claims.
_Avoid_: oracle

**nightingale-pulse**:
The repo that holds the Pulse port, the playground, the kernel, the chart and the UX work. This repo reads it only at the commit `docs/adr/0001-native-engine-repo.md` names, and cites its records as pulse 000N.

**Atlas**:
`docs/atlas/`: one card per system and the coupling map that joins them. It is frozen, and the SRS and the contract win where it disagrees with them.

**Coupling map**:
The atlas's map of how systems affect each other: shared substrates and 38 direct couplings. Not the map of sites.

## Systems

The 13 atlas systems by id: CV cardiovascular, RS respiratory, NS nervous, RN renal, HP hepatic, GI gastrointestinal, EN endocrine, BC blood and acid-base chemistry, HC hematology and coagulation, IM immune and inflammation, FL fluid compartments and the endothelium, MT metabolism and thermoregulation, MS musculoskeletal and skin. Each has one card in `docs/atlas/`.

**Receptor layer**:
`ng-receptor`: the binding of transmitters and hormones to receptors at each site, and the effect that binding has on the laws that read it. It owns laws, like a system, but is not one of the 13.
