# 15: A learned emulator to fit the body's parameters

Type: research
Status: open
Blocked by: `.scratch/research/issues/05-parameters.md`, `.scratch/body/issues/30-step-plans-on-the-whole-body.md`

**Question, for the engine's process:** Some parameters of the body's laws have no source that measures them, and only a response of the whole body pins them. Should such a parameter be fitted to sourced responses through a learned emulator of the engine? If so, what is the fitted value's provenance?

The engine's process decides this, through a decision record, and this line would supply the method. The ticket reads research 05's list first, because most parameters need no emulator. It also needs the whole body's measured speed, which body 30 gives, so it waits on body 30 and not on the line's parking.

**Which parameters.** A parameter that a source measures, or that an identity derives from sourced values, takes its value that way. One that an anchor pins is calibrated at admission (GLOSSARY: Value provenance). An emulator is only for the rest, such as a gain or a time constant that no source measures and only a closed loop's answer pins, like the controllers' answer to a meal. Research 05's list says how many parameters that leaves.

**Why an emulator.** A fit tries many values, and each try of a whole-body response is a run of hours. On one worker the slice runs at 7.6 times real time ([foundation map](../../foundation/map.md), "Where the route stands", item 6), and the whole body is unmeasured until body 30. An emulator learns the chosen produced quantities as a function of the parameters, over their ranges, from a designed set of engine runs. The search then runs on the emulator, and only its proposals run on the engine.

- **Model default:** one Gaussian process for each response summary, searched by history matching. History matching rules out the values the responses refuse and does not return one best value.
- **Alternative:** this line's generative physiology model, with the parameters among its inputs. It needs trait conditioning first (ticket 01, section 5.4, step 2).

**What an emulator may not do.**

- Its predictions are never a source or a target ([SRS rule 16](../../../docs/srs.md#3-hard-rules)). It proposes values. The engine runs each proposal, and the engine's own response must meet the sourced responses.
- A response used to fit a parameter no longer validates it. So the responses a fit uses leave research 08's validation targets, as research 05 already holds some sources out for testing.
- Where the responses leave a parameter unpinned, the answer says so and picks no value.

**What the decision record settles.**

1. **The provenance.** A packet value is specified, sampled, derived, calibrated at admission, inferred or authored (SRS section 2, Packet; GLOSSARY: Value provenance). A value fitted to sourced responses after admission is none of these. Either it enters as an inference record whose model is the fit and whose sources are the responses ([record 0003](../../../docs/adr/0003-inferred-parameters.md)), and it displays as inferred. Or it gets a kind of its own, and SRS invariant I16, which names only the identity or the anchors a value is derived or fitted from, names the responses too.
2. **The packet a fitting run uses.** Its values are candidates for the reviewed packet, so they are neither obviously synthetic nor reviewed (SRS rule 12).
3. **Where varied values come from:** a host copy of the fixture's table or a packet builder in the engine. That is [ticket 13](13-fluids-line-deferred.md)'s question, which trait draws need too.

## Acceptance

- [ ] Research 05's list marks each parameter as measured by a source, derived by an identity, pinned by an anchor at admission, or pinned only by a whole-body response, and counts the last group
- [ ] For that group: the sourced responses that could pin each parameter, and how many engine runs a direct fit and a fit through an emulator would each need, at body 30's measured speed
- [ ] A decision record settles the three points above, or the Answer says why no emulator is worth building
- [ ] If one is built, the Answer names the line that owns it

## Non-goals

- Building or training an emulator, and any engine change.
- Fitting any value.

## Comments

From [ticket 16](16-architecture-foundations.md) (sections 2, 5.3, 6 and 7):

- **Order of use:** run sensitivity ranking first (which constants move which readings and responses over their population ranges) to prioritize [research 05](../../research/issues/05-parameters.md) sourcing and guide trait conditioning, before fitting any value. Ranking produces no packet value and needs no provenance record.
- **Provenance input (section 2 rule 1):** because the engine runs every proposal, the emulator is no part of what a fitted value rests on; the decision record settles provenance for a value verified by an engine run against a sourced dynamic response (with method and window, rule 17) plus the selection rule among values not ruled out.
- **Precedent:** cardiac history matching (Coveney and Clayton 2018; Longobardi and colleagues 2020; Rodero and colleagues 2023) backs Gaussian-process emulators with implausibility waves as the baseline; switch to a neural estimator only if the count of fitted constants or summaries outgrows a Gaussian process.
- **Constant audit and Stage A ranking design ([ticket 17](17-emulator-sensitivity-ranking.md)):** Machine-verified parse of all `558` rows in [research 16](../../research/issues/16-constants-and-what-binds-them.md) yields `409` sourced, `32` derived, `2` admission-pinned (`CV-066`, `RN-016`), and `115` candidate (`52` active candidate degrees after removing `51` fixed `model default` choices and `12` collapsed `unknown` gauge/series choices). Of the 49 left after 3 deferred rows, 13 have a key a crate reads and 36 do not, and they group to 46 axes once every law is built. The draft design for Stage A uses Shape B (`N_B = 6d + 20` Sobol/LHS runs, `L9` GP feasibility surface, and conditional GP-ARD with Shapley/Saltelli decomposition). It sets no packet value, but it is not unblocked: its runs need varied packets, which is this ticket's point 3, a range for each axis, and bounded link storage. Ticket 17 is open again. Stage B fitting remains blocked on [research 05](../../research/issues/05-parameters.md) and [body 30](../../body/issues/30-step-plans-on-the-whole-body.md).

From a review of [ticket 17](17-emulator-sensitivity-ranking.md) and [ticket 18](18-teacher-library-and-start-proposals.md), read at `9a7469ff`, on point 3 of the decision this ticket asks for, where varied values come from:

- **What the engine offers now.** Four fixed builders for the whole body (`body_packet` and three variants, `crates/ng-validate/src/body/packet.rs:559-594`). The builder that takes a table, `packet_of`, is private (`body/packet.rs:602`). A second value for a bound key is refused (`DuplicateParameter`, `crates/ng-contract/src/packet.rs:301-305`), and so is a second start (`DuplicateStart`, `packet.rs:599`; measured on the whole body). The driver admits one fixed packet, and its run spec has no packet grammar (`tools/generative-physiology/driver/src/teacher.rs:739`).
- **Who waits on it.** Ticket 17's ranking, whose every run is a packet of varied values. Ticket 18's start proposals by the packet route, which is also [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md)'s first route. Every model that conditions on traits (ticket 16, section 5).
- **What it also needs.** A range for each varied constant. None exists: `data/inferences.jsonl` is empty, and research 05 has no box ticked. A clinical range enters only through review (SRS rule 12, record 0005). One way that needs no clinical value is to vary each key around the fixture's own value by a stated synthetic share. That is a model default for the record to weigh, and nothing here decides it.
- **The question, restated for the engine's process:** a host copy of the fixture's table, or a packet builder in `ng-validate` that takes values and starts?
