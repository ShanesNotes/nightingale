# 02: Architecture of the first build

Type: research
Status: resolved
Blocked by: 01

**Read set and outputs** (docs/governance.md section 5), declared before the pass began, at commit `8e427b1447e33fa9adced1fc870438f1739dc157`, which was origin/main when the pass started:

1. `docs/srs.md`.
2. Records 0001 (the engine is native Rust, and nothing outside the workspace builds into it), 0003 (a model's output is never a source or a target), 0004 (the first milestone, and the arena runs only synthetic fixtures), 0005 (checks run in validation, never inside the engine, and a result names the run and the set it read), 0006 and 0007 (when a waiting input shows, and the fence that `enqueue` refuses behind). Not 0002: this pass neither groups nor splits the map.
3. `GLOSSARY.md`.
4. `docs/architecture/solver-contract.md`. Not the sketch, because this pass edits no types.
5. No atlas card. The teacher is the slice as built, so the code says what it does, and the cards are frozen and rank lowest.

origin/main moved twice during the pass, to `c3f9783bde2ac63b9bcfeb2a846d715ac8b3d7eb` (native-contract tickets 11 and 19) and then to `84744af86ae9feac3a90e610ce21b741990f9cae` (ticket 17), and this branch merges both. Every fact below that those tickets changed was checked again at 84744af, and every path:line in tickets 02 to 13 holds there.

The pass also read `map.md` and ticket 01 (they steer and never bind), `docs/governance.md`, `docs/agents/issue-tracker.md`, the code named in ticket 01's section 7.1 plus `tools/bench`, `crates/ng-solver`, `crates/ng-observe` and every public item a driver needs, and tickets 11, 12, 17 and 18 in `.scratch/native-contract/issues/`. Its outputs are tickets 02 to 13 in this folder, `map.md` and four new terms in `GLOSSARY.md`. Nothing else changes.

**Question:** What is the first build of the generative physiology model? What does the teacher audit check, how does a driver outside the workspace make teacher runs, what does the named model carry, what is frozen before tuning and how is the freeze checked, and when is a fourth model added?

## Answer

### The decisions this design starts from

These six were made before the pass and are not reopened here.

1. The first experiment starts with three models: C1 linear, C2 readings-only and C4a named. Another model is added only if their results are too close to call, and the design defines that phrase, the one model added and the reason before any tuning (ticket 11).
2. The engine is the teacher and stays unchanged. The driver lives outside the workspace members, as `tools/bench` does, and uses only public interfaces.
3. The evaluation harness is frozen before any model is tuned, and the design says how the freeze is recorded and checked (ticket 09).
4. Heavy runs, the teacher audit included, run only on the heavy-run machine.
5. No grading, lessons or product features.
6. The repo is public, so nothing personal enters it.

### Problem

The first experiment asks whether a learned model whose state is a few named totals carries the slice's response to posture through a free-running encounter, out of policy and in a closed loop, better than a model that learns from readings alone. The engine has to teach it without changing, and five facts at 84744af shape everything else. An admission costs about 17 s and belongs to one core, because the packet hash a run names folds in the core's method history (`crates/ng-contract/src/packet.rs:54-73`, implemented; `.scratch/native-contract/issues/12-throughput-beside-the-record.md:15`, author-reported). No input can be enqueued before the fence at the anchor plus one second (`crates/ng-contract/src/admission.rs:162`, implemented). Every accepted step writes a history link of about 18.6 KB, at about 212 steps a simulated second. The session hands each link to its history store as the step is accepted and keeps only the chain's head, so a store outside the process keeps memory flat, while the links take about 14 GB of disk a simulated hour (`crates/ng-record/src/record.rs:200`, implemented; `.scratch/native-contract/issues/11-memory-stays-flat.md`, its Answer, measured). The heart rate moves with posture, while a lens window closes on fixed multiples of its period (`crates/ng-contract/src/lens.rs:71-73`, implemented), so fixed windows alias the beat. And the ledger splits each column's gross forward and reverse parts by sign at stage points inside steps of at most 10 ms (`crates/ng-contract/src/quadrature.rs:58-65`, implemented), so gross parts depend on the core's steps while net extents do not.

### Usage (caller's view)

The work lives in `tools/generative-physiology/`, outside the workspace members. It has two parts: a Rust driver, `driver/`, with its own `[workspace]` and lock as `tools/bench/Cargo.toml:7` has, and a Python study, `gp.py` and the modules beside it. Every command below runs on the heavy-run machine, from a snapshot of one commit.

```text
gp-driver smoke                      # ticket 03: a 20 s run that reproduces the pinned Core seal
gp-driver run SPEC --out DIR         # ticket 03: one teacher run, written whole or not at all
gp-driver interact SPEC --out DIR    # ticket 03: a closed-loop teacher run; the policy answers on stdin
gp-driver catalogue --schema FILE --out FILE   # ticket 05: rows, columns and the compiled reduction
python3 -I tools/generative-physiology/gp.py audit --data DATA       # ticket 04
python3 -I tools/generative-physiology/gp.py run train --data DATA   # ticket 06; ticket 09 adds cases and heldout
python3 -I tools/generative-physiology/gp.py screen --data DATA      # ticket 07
python3 -I tools/generative-physiology/gp.py freeze --data DATA      # ticket 09: prints FROZEN
python3 -I tools/generative-physiology/gp.py fit C4a --training P-rand --seed 3 --data DATA --out MODELS/c4a-prand-3   # ticket 10
python3 -I tools/generative-physiology/gp.py call --data DATA --models MODELS            # ticket 11
python3 -I tools/generative-physiology/gp.py evaluate --data DATA --models MODELS        # ticket 12
```

Three call sites show the shape:

```python
study = Study.frozen(tool_dir)                 # refuses a tree whose digest differs from FROZEN
run = study.teacher.run(study.case("heldout", history=3, family="stand"))
print(run.beats.heart_rate[:5], run.summary.last_seal)
```

```python
model = models.load(models_dir / "c4a-prand-3", study.reduction)  # refuses another reduction digest
run = model.start(case.named_start)            # a free run from the encounter's first beat edge
for event in case.events:
    run.enqueue(event)
run.advance(until_ns=case.end_ns)              # no reset to a teacher state
score = study.score(run.beats(), case)         # beat readings against the teacher's, a reference and never a target
```

```python
decision = study.call(validation_scores)       # "C4a wins", "C2 wins", "C1 kept" or "too close: add R"
```

### Shape

**Three modules, one owner per invariant.** This follows the separation the chosen candidate made, and per minimize-reader-load no trace crosses more than three files.

| Module | Owns | Never does |
|---|---|---|
| `driver/` (Rust) | Admission, the step loop with refusals and faults, enqueue against the fence, beat detection, the capture of every row and column at beat edges, the teacher's beat readings, checkpoint and resume variants for the audit, the compiled reduction | Choose a schedule, apply measurement error or judge anything |
| `study.py` with `gp.py` (Python) | Schedules from seeds, partitions, the scripted rule, measurement error from the driver's table, metrics, the call rule, the freeze, the audit report | Step the engine or define a model's evolution |
| `models.py`, `train.py` (Python) | The model forms, their integrator and their readings; fitting within a budget | Read the teacher during a scored run |

**Data first.** A teacher run is one canonical text, a run spec, that holds the core, the end, and the posture events in whole ms after the anchor, at or after the fence and strictly increasing. Its digest, with the driver's capture version, names its directory, so a directory cannot disagree with what made it and a corrected driver cannot reuse an old run. A present run is reused only when its identity digests equal the audit's first run's (ticket 03). A run writes the full accepted state, every column's cumulative forward and reverse extents and every row's chord integral at each beat edge, plus every accepted step's seal. That keeps every later reduction offline, so the richer state R needs no new teacher runs.

**Time resolution: beat edges.** The rhythm's phase memory resets at a guard's landing (`crates/ng-cardiovascular/src/rhythm.rs:84-89`, implemented), so a beat edge is an exact accepted step end, where amounts and the ledger are exact. Named totals are sampled at beat edges, the same phase of every beat. Within the first second on Core, arterial pressure runs from 125.6 to 76.3 (`crates/ng-validate/tests/trace.rs:138-147`, synthetic test), a swing a fixed-instant sample would carry as phase noise. A beat reading is the heart rate as one over the beat's length and the arterial pressure as the beat mean, both computed once, by the driver.

**What the named model carries.** Sixteen named totals (ticket 05 holds the table): ten water totals that merge pools which relax within seconds and keep apart pools whose walls or heights differ, such as leg and splanchnic veins (`crates/ng-validate/src/slice.rs:30-31, 776-777`, implemented); and six transmitter totals, stored and released norepinephrine at the four junctions whose parameters agree, the same two at the renal junction, whose `rc.ne-off` is 40 against 10 (`crates/ng-validate/src/slice.rs:902-904`, implemented), and stored and released acetylcholine. Transfers are never listed. The driver derives them from the body's columns by their two ends and origin, and an exact integer check, P S = B A, proves the grouping.

**What a model learns.** Every transfer has two nonnegative channels, each proportional to the amount at the end it drains, so an empty total stops supplying by construction. C1's channel coefficients depend only on the drive, so C1 is linear in its state. C4a's are learned functions of the totals and the drive. C2 is a recurrent model over beats that reads its own previous noiseless reading and the drive, and never sees a named total. The primary target is each transfer's net extent over each beat; gross parts stay in the capture, because the ledger splits them at stage points. R, the only fourth model, is C4a's form on a reduction where every amount row is its own total and every memory row is carried; it is defined and frozen now, and trained only under ticket 11's rule.

**Interface depth.** A caller of the driver names a run spec and gets a run directory; it never coordinates retries, the fence, beat detection, chord integrals or atomic writes. A caller of the study names a case and gets a score. A caller of a model gives a start and events and gets beat readings. Per boundary-discipline, the run spec, the schema and FROZEN are parsed once at the edge.

**Invariants and where they are held.**

| Invariant | Held by |
|---|---|
| No event before the fence, and no two postures at one instant | The run spec's parser refuses both (ticket 03) |
| A shared prefix gives shared ids and seals | Event ids digest the instant and the setting; the audit compares seal logs (tickets 03, 04) |
| A reduction loses no amount | The catalogue assigns every water and transmitter row exactly once, and P S = B A holds in integers (ticket 05) |
| A named model balances | Its state moves only through paired channels (ticket 08) |
| Readings never feed a named model's evolution, and no measurement error enters any model | The run API gives evolution no reading; noise is added after, by the study (tickets 06, 08) |
| Held-out data never tunes | The held-out cases are made only after FROZEN merges, and `fit` refuses a held-out path (ticket 09) |
| Nothing is judged under an unfrozen tree | `fit`, `call` and `evaluate` refuse a tree whose digest differs from FROZEN, and a gate check refuses a frozen file changed without FROZEN (ticket 09) |

### Synthesis decision

Two runners on two model families each produced one candidate, and a cross-family judge scored both against a six-criterion rubric. The split candidate divided the work into a Rust driver, a Python study and Python models. The single-tool candidate put everything that drives the teacher or judges a model into one Rust tool and kept only training in Python. The judge recommended the split candidate as the base, 23 points to 17. The orchestrator first leaned toward the single-tool candidate and changed for the judge's main reason. That candidate writes each model's forward pass and integrator twice, once in Rust to judge and once in PyTorch to train, and a pointwise parity probe cannot prove two rollout integrators agree. That duplication would sit on the module that changes most often. The split candidate keeps one owner for each invariant.

Grafted from the single-tool candidate, each folded in by hand:
- Beat readings in place of the split candidate's fixed-clock observation windows, which alias the beat once the heart rate leaves one per second.
- Paired branches as fresh runs of one canonical prefix, in place of rewinds, so every run rebuilds from its own event log. Checkpoint, resume and rewind remain audit cases.
- The audit's first check, which reproduces the pinned first second on Core, and the prefix checks that compare seal logs between runs.
- The partition of named totals by timescale and wall, corrected by the judge's check that the renal junction binds at another rate.
- Transfers derived from columns, and net extents per beat as the primary target.
- A sufficiency screen on teacher data before the freeze, as a diagnostic and never a model.
- FROZEN committed in git, with a gate check.
- The seed spread as the basis of the call rule, and the per-beat scripted rule.

Grafted from the judge's review: the call rule reads validation cases only, and adds a bootstrap over start histories to the seed spread; R stays the fourth model; the audit costs the accumulated posture inputs, a late checkpoint's export and the numerical variation at 15 and 30 minutes; the audit includes a true closed-loop case; the reference integrator is never an automatic fallback.

Rejected from the split candidate: its fixed 5 percent tie margin, which no measurement grounds; its 32-episode training set; gross extents as the only target; and a freeze record kept outside git. Rejected from the single-tool candidate: the second forward pass; C4b as the fourth model, because its latent part starts from the named totals and so cannot recover start information they discard; a trigger read on held-out cases; an outflow gate that made its linear model nonlinear; and the claim that summing all five junctions pools no differing rates.

### Tradeoffs accepted

- We accept two languages in exchange for one implementation of each model, each rule and each noise draw. The driver's interactive mode costs a line protocol per beat.
- We accept repeating each start history in every run, a third of a 15-minute run, in exchange for runs that need no stored checkpoint and replay from their spec alone.
- We accept beat readings that bypass the engine's lens, which stays attached only as a diagnostic, in exchange for readings that do not alias the beat. The audit measures the ripple this avoids.
- We accept sixteen named totals instead of the brief's dozen, in exchange for transmitter totals that close and no summing of pools whose rates differ.
- We accept a screen that sees one beat ahead and cannot prove sufficiency. Ticket 11's rule covers a tie that a longer horizon would explain.
- We accept net targets that leave each transfer's split between its two channels unidentified, in exchange for targets that do not depend on the core's steps.
- We accept about 555 teacher runs, 595 if R is added, against the brief's 420, in exchange for a validation set that can trigger the fourth model without opening the held-out set, and a closed-loop call that replays every seed.

### Alternatives considered

- **One Rust tool that also judges the models.** It gives one definition of every engine-facing piece and bitwise replay on any machine, but it needs a second forward pass and integrator for every model. It lost on the duplication.
- **A Python driver over a foreign-function binding.** No public binding exists, and a binding would be a new crate beside the engine for no capability the experiment needs.
- **Branches by rewind.** It saves the prefix's compute, but every branch then depends on a stored checkpoint and on its links in a store, and a fork walks the parent's whole chain through that store.

### Repository facts behind section 7, checked at 8e427b1 and again at 84744af

No code changed between b8990a5 and 8e427b1. `git diff --stat b8990a5 8e427b1` lists only `.scratch/generative-physiology/issues/01-concept-brief.md` and `.scratch/generative-physiology/map.md`, so every difference below is one the brief got wrong.

| Fact in section 7 | What the code says | Where, at 84744af |
|---|---|---|
| Posture and deliveries enter a run in progress through public interfaces | Holds. Admission-time events go through `BoundaryPort::new`; later ones through `Session::enqueue`, at or after `PatientRecord::earliest_stamp`, which is the fence or the head | `crates/ng-record/src/session.rs:145-147`, `crates/ng-record/src/record.rs:107-109`, `crates/ng-contract/src/boundary.rs:420-448` (implemented) |
| Full rows, modes and per-column extents are readable | Holds. Rows and modes at the head through `AcceptedStateView`; the newest segment's cumulative extents through `Session::conservation().head()`, and older segments newest first through the store. Inside a step a value is a chord | `crates/ng-contract/src/state.rs:244-262`, `crates/ng-record/src/session.rs:75-77`, `crates/ng-record/src/history.rs:575-586, 624-648` (implemented) |
| Checkpoints resume with identical bits | Partly. One slice test checks a resume against a continuation, by row bits on the reference integrator; no test does it on Core or compares seals | `crates/ng-validate/tests/arena_body.rs:699-725` (synthetic test) |
| No public interface sets a state | Holds for typed interfaces; a checkpoint's bytes enter only through digest checks | `crates/ng-record/src/record.rs:26`, `crates/ng-record/src/checkpoint.rs:9-19, 278-324` (implemented) |
| Explicitly bound traits cannot vary without a host-built packet or an engine builder | Holds. The table is private and a second value for one key and binding is refused | `crates/ng-validate/src/slice.rs:672`, `crates/ng-contract/src/packet.rs:252-258` (implemented) |
| History memory bounds run length | Held at 8e427b1; changed at c3f9783. Memory stays flat with a store outside the process, and history costs 18,584 bytes a step on disk | `.scratch/native-contract/issues/11-memory-stays-flat.md` (its Answer, measured), `tools/bench/src/link_log.rs:10-37` (implemented) |
| The core picked for the slice has not run it with an input | Holds. Core runs the slice only for one second with an empty port | `crates/ng-validate/tests/trace.rs:133-148` (synthetic test) |
| A host outside the workspace can drive the slice and read its ledger (7.2, 7.4) | Holds. Nothing private blocks it; the driver adds `ng-observe` to the four path dependencies `tools/bench` has | `tools/bench/Cargo.toml:9-13`, `crates/ng-validate/src/lib.rs:104-122` (implemented) |
| The reference integrator runs at 1.05 to 1.3 times real time, and Core at 4.91 over 35 minutes with no input | Partly. 1.309 is over 5 s at 5 ms, 1.05 only to 07:00:30, and the 35-minute run states no input; its driver is not in the repo | `.scratch/native-contract/issues/12-throughput-beside-the-record.md:15, 31` (author-reported) |
| The heavy-run machine's GPU | The brief says RTX 4060 in one place and 4060 Ti in another; the machine's notes say 4060 Ti, 8 GB, with no CUDA workload run | notes kept outside the repo |

Facts that changed after b8990a5, on main at c3f9783 and 84744af (native-contract tickets 11, 19 and 17). None changes the design's shape, and tickets 03 to 06 use the new interfaces:
- `admit_at` and `Session::admit` take a history store, held for the whole run (`crates/ng-validate/src/initialize.rs:219-227`).
- Each accepted step hands its link to that store, and the record keeps only the head (`crates/ng-record/src/record.rs:200`), so memory stays flat with a store outside the process, as the bench's link log is (`tools/bench/src/link_log.rs:10-37`).
- `Session::checkpoint` and `export` take no store and refuse until the store has kept every link (`crates/ng-record/src/session.rs:159-163`).
- History is read through `Session::conservation()`, whose `head()` and `newest_first()` replace `segments()`; a resume, rewind or recover walks the chain through the store (`crates/ng-record/src/history.rs:575-586`).
- `MemoryStore` is public (`crates/ng-validate/src/lib.rs:114-117`).
- The replay gates compare observer bits, on manufactured cases (`.scratch/native-contract/issues/19-observer-bits-under-replay.md`).
- The slice now carries one forcing, which scales the skin's water loss by 1 plus or minus 0.5 over a 300 s period, and one hazard, a stir that fires once and moves no law in the slice (`crates/ng-validate/src/slice.rs:376-414`). Both read the run seed, which `admit_at` fixes, so every teacher run shares one forcing course and one stir instant. The memory rows rose from 9 to 10, and the pinned first-second digests moved (`crates/ng-validate/tests/arena_body.rs:163-166`, `crates/ng-validate/tests/trace.rs:135-136`). No model gets a clock, so the skin loss's wander is variation that every model leaves unexplained alike; the audit measures its size.

Other claims of the brief that the code contradicts or only partly supports:
- A 30 degree tilt as trunk sine 0.5 with seated legs holds by arithmetic only. The code names `SeatedDependent` as seated, and its share of 0.5 is private (`crates/ng-contract/src/boundary.rs:61-75`, implemented).
- Posture reaches only the cardiovascular laws that read height, the blood edges and the baroreceptors, not every site's physics (`crates/ng-cardiovascular/src/lib.rs:57-72, 189-215`, implemented).
- A host can draw the measurement error, because `Channel::error` is public (`crates/ng-contract/src/lens.rs:79-85`, implemented).
- The arena packet never draws a trait with the patient seed; only a test does (`crates/ng-contract/src/packet.rs:262-279`, implemented).
- The state also holds algebraic values and a fired flag (`crates/ng-contract/src/state.rs:8-15`, implemented).

### Where the brief is wrong or too heavy

- **Gross extents as the transfer target (5.4).** The ledger splits by sign at stage points inside each step (`crates/ng-contract/src/quadrature.rs:58-65`), so a beat's gross reverse part includes back-flow inside the beat and depends on the core's steps. The net extent per transfer per beat is exact and step-independent.
- **Cycle-mean state (7.2).** A mean of amounts over a cycle does not balance against the extents over the same cycle. Values at beat edges do. "Cycle" is also admission's word (`GLOSSARY.md:323`), and a heartbeat here is a beat.
- **Readings through the lens.** Lens windows close on fixed instants, so they alias the beat once the rate moves.
- **Bound transmitter totals (5.4).** Binding settles in about 0.1 s while reuptake takes 4 s (`crates/ng-validate/src/slice.rs:894, 901-902`), and each junction is closed (`crates/ng-nervous/src/lib.rs:13-46`). A bound-only total has an uncarried other end. Stored and released totals close.
- **One dependent-vein total.** Leg and splanchnic veins differ in wall and height (`crates/ng-validate/src/slice.rs:30-31, 776-777`); summing them makes the closure error the brief's section 2.1 warns of.
- **Eight arms, 64 closed-loop replays and "R does not beat the named arm" as a gate (5.6, 5.8, 5.9).** Too heavy under the three-model decision. R is the conditional fourth model.
- **"Across-patient" order (5.8).** The packet is fixed, so the experiment varies start histories, not patients (`docs/srs.md:27`).
- **"Refinement test" and "split test" (3).** Refinement is a map term (`GLOSSARY.md:301`). Here they are the step-halving test and the piecewise-input test.
- **Paired branches are separate runs "because a rewind restores the whole history into memory" (5.9).** A rewind saves compute, and since c3f9783 it walks the chain through the store instead. Separate runs are chosen so that each run replays from its spec alone.

### Open questions and risks

- For the engine's process: do trait variation and the fluids experiment take a host copy of the synthetic table or an engine-side packet builder (ticket 13)?
- For the engine's process, not blocking: what provenance would fitted weights need under SRS rule 12 if a learned law ever entered the engine?
- For the developer reviewer: should `tools/gate/paths.toml` send `tools/generative-physiology/**` to fewer steps than "everything", so a research PR does not run the engine's mutation step (ticket 03)?
- Does Core pass the audit's technical gate with posture mid-run, across a checkpoint and at 30 minutes? If not, the experiment stops at ticket 04, because the reference integrator at 1 ms would take about 1.8 million steps, about 33 GB of transient history and hours of wall time per 30-minute run.
- Is the posture response informative enough? Ticket 04's informativeness gate decides before any training data exists.
- Can the heavy-run machine install pinned CPU PyTorch wheels? Unconfirmed (ticket 06).

### Next implementation step

Build ticket 03's driver and prove, on the heavy-run machine, that `gp-driver smoke` reproduces the pinned Core seal and survives being killed and repeated.
