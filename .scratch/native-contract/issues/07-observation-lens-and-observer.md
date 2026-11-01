# 07: Observation lens and observer, apart from physiology

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A chart author compiles a lens (channels and an output grid) against the body, reads the last accepted revision through `Session::observation`, and runs an observer that applies windows, delays and seeded measurement error, one revision at a time.

Blocked by: 05

Status: resolved

- [x] The lens digest is not in the run identity; applying a new lens to a finished replay gives identical physiology digests and new observations
- [x] Physiology digests are identical with observers attached, detached and changed (the non-perturbation gate)
- [x] An observer refuses a skipped or repeated revision; each grid instant appears in exactly one step
- [x] An observer checkpoint restores its observations after a resume
- [x] Measurement error draws from the observation namespace: adding a channel leaves physiology draws unchanged

## Answer

A chart author compiles a lens against a body, `Session::observation` lends one read of the last accepted revision, and an `Observer` applies windows, delays and seeded error, one revision at a time. A lens is in no run's identity, and the session never sees an observer.

- **The lens digest is not in the run identity, and a new lens on a finished replay gives identical physiology and new observations.** `RunIdentity::new` takes four things, and a lens digest as a fifth is E0061. `a_new_lens_on_a_finished_replay_gives_the_same_physiology_and_new_observations` replays a case under two lenses and compares every seal, the last digests and every firing with a run under none.
- **Physiology is identical with observers attached, detached and changed.** Non-perturbation is `ReplayEvidence`'s seventh field. The gate runs a case detached, then under two lenses over every row, and compares every seal and the last digests. A view borrows the session and a record holds no `Cell`, so what this gate can catch is a core told about the reading, and `a_core_told_the_grid_only_in_a_watched_run_fails_the_gate` is that core. A run whose observer read no value is `MissingEvidence`, so the gate cannot pass with no one watching.
- **An observer refuses a skipped or repeated revision, and each grid instant appears in exactly one step.** One comparison, `view.previous()` against the seal last read, refuses a skip, a repeat and the next revision of another course, which a revision number cannot see. A step holds its end and not its start, and revision 0 holds the admission instant when it is on the grid. `each_grid_instant_reaches_an_observer_once_across_refusals_and_a_firing` counts every instant through a run with refusals and a firing on the grid.
- **An observer checkpoint restores its observations after a resume.** A checkpoint is bytes, bound to a lens and a seed, and holds the seal last read, each window and each held reading. One test resumes at every revision of the first 100 s and compares every observation with an unbroken run, and another holds it across a rewind. An observer keeps no log, so what it already returned is the host's to keep.
- **Measurement error draws from the observation namespace.** `Observer::new` takes an `ObservationSeed`, and a run seed is E0308. An error is a pure function of seed, channel and instant. `measurement_error_leaves_physiology_draws_unchanged` holds every seal and firing with error on, with a channel added and with another observation seed, and a channel added moves no other channel's error.

**The two open choices.**

- **One rounding.** Nothing rounds inside the engine. A channel's value stays a `Compensated` through the window, the delay and the error, and a host's one rounding is `hi`, the pair's nearest float, because a pair is normalized. A later healthy-range check can compare pairs with `at_most` and needs no float. `a_window_keeps_every_low_part` fails if a window rounds a sample, and the resume tests fail if a checkpoint rounds a sum.
- **A firing's instant.** The accepted state, which is what the firing left. The left limit is a state the record never accepted. Reading it would give one instant two values, where a snapshot has one, and once ticket 10 solves at a landing it would read one instant's rows from two states. The certificate's impulses are right-continuous too. So at the head a channel reads the accepted state, and elsewhere in a step it reads the chord. `a_channel_reads_what_a_firing_left_at_its_instant` puts a grid instant on a firing.

**Choices, each with its reason.**

- **Base.** The Opus candidate, which the judge, a Codex seat, recommended 21 to 17 against its own vendor. Its shape is one read, one order check, tumbling windows, byte checkpoints and observations streamed to the host.
- **A channel names a row**, by `RowKind`, with no expression language. The contract makes a pressure an algebraic row, so a slice row reaches a lens with no change.
- **A channel names its method.** The SRS says an observation has its site, method, time taken and time resulted. The base had dropped the method, so this came back from the Codex candidate. `Observation::channel` points to the channel, which holds the method and the row. An amount's row names its site. A memory or algebraic row is whole-body and names none, and a per-site row ticket 10 adds must carry its site in `RowKind`, because `packet.start` keys on it too. A second identical device is one more method name.
- **One read.** `ObservationView::samples`, so `value`, `grid` and `sample` went and with them `ChannelId` and `OutsideSegment`. A head value by row is still `record().accepted().value(row)`.
- **Windows tumble** on grid samples, for one pair of state per channel where a trailing window keeps a ring in every checkpoint. A window is a count of grid periods, so it always fits the grid. It closes on its grid index, with no multiplication to overflow. One that missed an instant reports `NotCollected`, never a partial mean.
- **A delay** shows a reading twice: `Pending` when collected, then its value in the first step that reaches `available`. `available` stays exact, so a step that passes it changes when a host hears and not what. A reading due past the clock's end is `InvalidSignal`, with no saturation and no extra refusal.
- **Error** is twelve uniform words less six: mean zero, variance one, inside six either way, from sums alone, because the contract has no `sin`. Its key hashes the method, row, window, delay and SD, and the draw is keyed by the instant, so a resumed observer needs no cursor.
- **Attach.** An observer's first read takes any revision, as a monitor plugged in mid-run does, and windows report `NotCollected` until one fills.
- **The checkpoint** binds a lens and a seed by digest and stores no seed. `resume` takes no view, and the next `advance` judges the seal, so one set of bytes stands beside every fork that keeps it.
- **The gate builds its lenses** from `body.rows()`, so `ReplayPlan` and `ReplayCase` do not change and the slice is covered when it lands. They hold no error, because a gate cannot name a row's dimension. Only the second is delayed, so their streams cannot match.
- **The crate table.** `seam.edge` covers dev-dependencies, so even a test in `ng-validate` that names `ng_observe` needs one cell changed. Both crates stay outside the physiology closure. `certify` still takes only a `ConservationView`, so no observation can reach the certificate.
- **All three seed domains land**, `PacketSeed` with no caller yet, because I9 names three. The run domain keeps its tag, so no draw moves.
- **Left out**, because no check reads them: an access class, a reference interval, `ObservationCatalog`, `OutputGrid`, `Transform` and `ErrorModel`. An access class or a flag comes back as one more builder call. The glossary's Channel entry drops the access class in this PR.

**Evidence**, all synthetic.

- **No physiology bit moved.** Base and head compiled from one probe, over 52 runs and 24 seeds: every identity, seal, firing and last digest compare equal, 17,352 lines and 1,358 firings. A control that changed the run domain's tag came back different, so the probe can fail.
- **Mutation sweep.** 71 mutants, one per rule or refusal, each killed by a named test on the last code. The first pass left one survivor, a forged reading caught only by the order check of the reading after it, and the last pass left one more, a forged queue holding one instant twice. Writing a working mutant for the seventh evidence field showed its test would have passed on another gate's evidence. All three tests are tightened. The mutants are hand-picked, not every line.
- **Two adversaries, one per vendor.** Each worked alone from the contract and the diff. Both proved the same panic, a resealed count near a huge window, now fixed. Between them they ran an independent oracle over 384 schedules and 23,431 observations, an oracle from the contract's prose over 443,113 observations, 400,000 edited checkpoints, and 1,494 window means in exact rationals outside the code, worst relative error 2.1e-32.
- **Comments.** Comment Sicko deleted 24 and trimmed 17, and found two that were false.
- **Two local reviews, Codex Astra at max.** The first found one claim too broad, that a channel holds its site. The second read the answer to that and found nothing.
- **Cost.** An observer on three channels of a 4 ms grid adds about 0.8 us to a 38 us step, stream digest included, in release on one thread on a machine at a load of about 5. A checkpoint with 10 held readings is 472 bytes.

**Limits.**

- Only the non-perturbation gate attaches an observer. §7 also names observer bits under pace, resume, late admission and rewind, and those are ticket 11's: equal seals give equal views, so they can only catch an impure observer.
- Windows tumble. Record 0004's sliding healthy-range check reads a bare channel and slides its own mean, and its readings carry no error, so its lens has no `erring`.
- A digest has no key. A resealed file can hold a false value, a reading from the future or a count no head could leave. It gives an observer that never panics on those, though that observer's next save can be refused and a future reading holds back later releases.
- `Compensated::between` multiplies before it divides, so a chord near 1e304 reads `InvalidSignal` between two finite ends.
- A grid's cost is its author's. `lens` cannot know a step's length, and a 1 ns grid on an 8 s step asks for 256 GB and ends the process.
- A reading can leave its row's set: an amount of 1 with an SD of 2 can read negative.
- A flow is no row, so reading one needs an algebraic row or a second `ChannelSpec` constructor (ticket 10).

**Hard-part check.** None reached. This ticket stayed contract machinery with synthetic values: no cross-site coupling, no validation against Pulse or published physiology, no stiff numerics, no disease and no outside physiology research.

## Comments

From 03: sketch probe this ticket owns: line 471 (seed domains never mix, E0308).

From 04: `Seed` is a plain struct, and this ticket brings `DomainSeed<D>` with the domains. `AcceptedStateView::last_segment` and `DenseSegment::sample` are what a lens reads. A segment is the degree 1 chord between the accepted ends, and the admission segment has no interval. `sample` returns `Compensated`, so a channel's value needs a rule for its one rounding.

From 05: `enqueue` moves no revision and no seal, so an observer keyed by revision never sees an input before it applies. A fork keeps its parent's seals up to the fork, so an `ObserverCheckpoint` bound to a seal stays valid across `rewind`, and the record's `Checkpoint` stays bytes with no observer section. `crates/ng-validate/src/replay.rs` has four gates. Non-perturbation is this ticket's fifth: add it to `ReplayGate` and to `ReplayEvidence`, whose one field per gate keeps a gate from being skipped.

From 06: `Seed::new` stores `seed_word("run", words)`, so `DomainSeed<D>` calls `seed_word` with `D`'s tag, and the run domain keeping "run" moves no draw. A dense segment ends at a reset's left limit, so `sample` at a firing's instant reads a hazard integral as it reached its draw while `value` reads zero: pick which a channel reads at that instant. `ReplayEvidence` now has six fields, and non-perturbation is the seventh. `StepReceipt::fired` says what fired, and it reaches a gate's evidence through `GateEvidence::fired`.
