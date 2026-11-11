# 14: The first experiment on the whole body

Type: research
Status: open
Blocked by: `.scratch/body/issues/28-two-synthetic-days.md`

**Question:** Once the whole body lives two synthetic days, which first experiment tests the most about a generative physiology model of the whole body? What does it need from the engine, and which parts of tickets 02 to 12 carry over to it?

The line is parked until then, and tickets 05 to 12 wait on this ticket ([the map](../map.md)). What follows was read at main `f7192b5` and is a starting point. The pass that takes this ticket reads the engine again at its own commit, because the body moves a long way before body 28.

- [ ] The experiment: its question, input, named state, readings, paired branches, outside check and falsifier, as ticket 01's section 5 gives them for posture
- [ ] Each need below asked of the engine's process, and answered there or filed there as a ticket. This line changes no engine file
- [ ] Each of tickets 04 to 12 marked as standing, rewritten for the whole body or superseded, with its reason
- [ ] The teacher audit's plan for the whole body: ticket 04's two gates on the chosen experiment's runs
- [ ] In the PR that moves the teacher, the glossary terms that name the slice, posture or one species are redefined: Teacher run, Named total, Run spec, Start history, Response feature and Teacher audit

## The model default: a meal

Open to review. The response to a meal, over the hours after it, comes first on the whole body.

- **It is the day's own input.** [Body 05](../../body/issues/05-the-days-inputs.md) makes a meal a vector of every species that body 01's tables deliver, into the gut lumen. So the experiment needs no intervention payload, which waits for scope stage 4 ([SRS section 1](../../../docs/srs.md#1-purpose-and-scope)).
- **It moves what the slice lacks.** The meal's glucose crosses the gut, the liver stores and releases it, and insulin and glucagon answer ([body tickets 13 to 16](../../body/map.md)). The tissues burn it, so the gases and heat move with it (body tickets 08 to 12). Burning a fuel is a reaction ([body 02](../../body/issues/02-reactions.md)), and the slice has none.
- **The engine already defines its paired branch.** A counterfactual run is the same start with one meal removed, and a day check reads a meal's response against that run's same window (GLOSSARY: Counterfactual run, Day check). The study's paired branches can be such runs, and its response features can be the day checks' windows.
- **Its variety has plain axes:** the meal's size, makeup and time, and the day before it.
- **Meal studies give it outside signal.** Meal studies in healthy adults follow the response for hours, as Taylor 1996 follows the liver's glycogen ([research 13](../../research/issues/13-law-forms-gut-and-fuels.md)). That kind of summary is what the planted-discrepancy step needs to test learning past the teacher (ticket 01, section 6).

Its costs:

- **Runs last hours, not minutes.** On one worker the slice runs at 7.6 times real time ([foundation map](../../foundation/map.md), "Where the route stands", item 6), so six simulated hours take about 47 minutes. The whole body's speed is unmeasured until [body 30](../../body/issues/30-step-plans-on-the-whole-body.md).
- **The named state must carry reactions.** See the table below.
- **Its outside check must come from a study that research 13 did not read.** The fuel loop's forms and model defaults rest on what research 13 read, Taylor 1996 and Dalla Man and colleagues 2009 among it. A check drawn from those sources only reads the engine's own source back.
- **No source on hand gives glucose a range after a meal** ([research 06](../../research/issues/06-healthy-ranges-and-lens.md)). That does not bind this line, which reads directions and orderings only.

The alternatives:

- **Posture on the whole body.** Runs take minutes, and the built driver, audit and study carry over almost unchanged. The volume hormones may add a slow answer to a stand ([body 17](../../body/issues/17-volume-hormones.md)). But posture moves the four systems the slice has, and no reaction, so it tests little that the slice cannot. Head-down also waits for [native-contract 33](../../native-contract/issues/33-leg-veins-empty-head-down.md).
- **A drink.** This is [ticket 13](13-fluids-line-deferred.md)'s fluid line on the whole body: water and salt, the kidney, the volume hormones and the bladder, over hours. It moves fewer loops than a meal and no fuel, so it fits as a second experiment.
- **The whole day.** It is the most lifelike, and it has the largest state. It follows once an experiment on one input earns it.

## What it needs from the engine

Each item is a question for the engine's process.

1. **The whole body, public to a host.** Body 06 admits the whole body at 06:55 on its synthetic fixture. `admit_at` already takes any compiled body, a synthetic packet, a lens and the host's own history store (`crates/ng-validate/src/initialize.rs:289`). So a teacher run needs body 06's compiled body, packet and lens to be public, as the slice's are.
2. **Meals that the host chooses.** The day comes with the packet (SRS section 2, Packet). Can a teacher run add meals of its own size, makeup and time as events, or does each varied day need a packet built with it? This is ticket 13's question about varied packet values, asked about the day.
3. **A store that keeps only what a teacher run reads.** The driver keeps every link of a run until the run completes (ticket 03). A 900 s slice run wrote about 1.8 GB of links (ticket 04's Comments), so six hours at the slice's rate of steps is about 43 GB a run. Whether a plain run, which never checkpoints or rewinds, must keep its links at all is for the store's contract to say, as record 0008's store drops the links older than its rolling day.
4. **A glucose reading.** It comes through the lens if research 06's list carries one. Otherwise the host computes it from rows, as an amount over a volume, as ticket 13 plans for sodium, with ticket 06's error convention.
5. **The whole body's cost.** [Body 26](../../body/issues/26-cost-budget.md) sets its budget and body 30 measures its speed. The plan's size waits for both.

## What carries over from tickets 02 to 12

| Part | On the whole body | Why |
|---|---|---|
| Three modules: a Rust driver outside the workspace, a Python study, Python models | Carries | Nothing in the split names the slice |
| The run spec and spec id, the run directory written whole or not at all, `status` and the pool | Carries, and the spec's grammar grows | The grammar names posture events only. The whole body's day also carries the clock, sleep, the room, meals and drinks (body 05) |
| The capture: every row, column and conserved quantity at every beat edge | Redone for long runs | An edge costs 2 + M + 4R + 4C + 6Q words (`tools/generative-physiology/driver/src/capture.rs:231`). That is about 17 KB on the slice's 241 rows and 257 columns, or about 400 MB for six hours of beats. The whole body's counts are unmeasured until body 06. A long run keeps the compiled reduction's totals and net extents at each edge, and keeps full state only at sparse edges, for R |
| The driver's history store and its file digests | Redone | See need 3. The driver also reads each finished file whole for its digest, which ticket 03's Answer already calls too large for a simulated day |
| Beat edges and beat readings | Carry for the heart's readings | The rhythm still resets at a guard's landing. A slow loop needs coarser edges. If breathing stays tidal (body map, model defaults), a breath repeats on its own period, so a gas total sampled at beat edges carries the breath's phase |
| The reduction: a schema, derived transfers and the integer identity P S = B A | Carries in form, with wider entries | A reaction is one column with a whole-number term for each pool it uses or makes (body 02). So B holds whole numbers, not only -1, 0 and 1, and one channel can move several totals |
| The sixteen named totals | Redrawn from body 06's catalogue | They are the slice's pools. On the whole body, oxidation makes water, so a water total also moves through reactions (GLOSSARY: Conserved quantity). Where the body adds metabolite pools, a junction no longer closes by release and reuptake alone (GLOSSARY: Conserved combination) |
| R, the richer reduction | Measured first | Every amount row as its own total grows with the whole body's rows. Whether R stays inside ticket 08's 100,000 parameters is known only once body 06's counts exist |
| The model forms (ticket 08) | Carry, with one new channel | A reaction channel must vanish when any total it uses empties, as a transfer channel vanishes with the total it drains |
| The audit's technical gate | Carries | Its seal-log checks name no body. Its plan is redrawn, and A0 reads the whole body's expect trace, which every body build ticket updates (body map, Notes) |
| The informativeness gate and the four response features | Redone for a meal | They measure posture changes. A meal's response is read as a day check reads it |
| Start histories | Become the day so far | The clock, sleep and the earlier meals set a meal's start, and postures alone no longer describe it |
| Schedules, the scripted rule, the freeze, the call rule and the metrics | Carry in form | Their families and thresholds are redrawn for the chosen input |
| The cost figures in tickets 04, 06 and 09 | Measured again | They are the slice's |

## Non-goals

- Training, teacher runs and any engine change.
- Starting before body 28 resolves.
