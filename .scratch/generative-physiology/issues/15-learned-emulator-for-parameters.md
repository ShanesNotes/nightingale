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
