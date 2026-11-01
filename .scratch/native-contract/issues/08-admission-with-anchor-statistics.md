# 08: Admission solve with anchor statistics

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The admission system solves a manufactured pulsatile case once, fitting a calibrated parameter so that a cycle-mean anchor holds under the periodic goal, and records provenance for every value.

Blocked by: 04

Status: resolved

Claimed by: claude

- [x] A cycle-mean anchor on a pulsatile output is met as a cycle mean, not an instant
- [x] An initializer that returns a guess violating a row or anchor is refused with the row named
- [x] Provenance is recorded per value (Derived, Calibrated, Sampled, Authored, Fixture); an Authored value lies inside its population record's range (I16), and no parameter is pinned to an anchor
- [x] The fixture packet type is the only one the validation crate accepts

## Answer

Admission can now solve a start state, not only take the packet's numbers. It fits a parameter so that an anchor holds as a mean over one beat, checks the result itself, and records where every value came from. All values are synthetic.

**How it works.** A packet can state one beat: how long it lasts, and how many equal steps to cut it into. That is a `Cycle`. Admission sets those steps out as a ring of points in time, so the last joins back to the first. Each point is one backward Euler stage, the same equation a solver core evaluates. All the points and the fitted value are solved together, once, by plain Newton, down to rounding. Then `admit` reads every equation again by itself. It uses no solver and trusts nothing the solver said. The first point becomes the start state, and the others are dropped.

**The test body.** Two vessels joined by one law. Each beat a pulse of water enters the first vessel, and the same amount drains evenly from the second. The amount in the first vessel rises and falls. That amount is the pulsatile output, because no pressure row can read an amount until ticket 10. The fitted value is the vessels' stiffness. Its closed form is at the top of `crates/ng-validate/tests/admission.rs`, where every test below lives unless marked.

**The four checks.**

- **Met as a cycle mean, not an instant.** `a_cycle_mean_anchor_is_met_as_a_cycle_mean_not_an_instant`. The start value is 2.7 from the anchor. A real run's mean over the first beat, read through the anchor's own lens channel, is within 1e-12 of it. The fit matches its closed form to 1e-12 on grids of 4 to 256 steps. A core whose step is off the grid starts a little away from its own orbit: its first beat's mean is off by 3e-3 to 4e-2, and that shrinks about sevenfold each beat. On a law that is not linear, or a coarse core with an uneven step, the orbit's own mean sits near the anchor and not on it: 8e-3 off in the worst case measured.
- **A guess that breaks a row or anchor is refused with the row named.** `an_initializer_that_returns_its_guess_is_refused_with_the_row_named`, and tests for one broken stage at one point, a missed anchor and a moved total. `a_guess_that_already_solves_the_system_is_admitted` is the other side. A refusal names a body row, a conserved total or an anchor.
- **Provenance per value.** `provenance_is_recorded_per_value_in_all_five_kinds_and_a_solved_start`. `InitialRevision::provenance` gives one record per packet entry and one per row's start, with the value the run starts from. A start value admission moved reads `Solved`, a sixth kind, because calling it `Fixture` would be false. An authored value outside its population range is refused. A derived value is one other entry times a stated scale, and `bind` computes it, so it moves with its source. No parameter can be pinned: no path copies an anchor's value (E0616). A fit no law behind its anchor reads is refused at `bind`. One that moves no row behind its anchor past rounding on the solved start is refused at `admit`. Either could only stamp its guess. A fit on a forcing's period is refused too, because a solved value cannot stay on the clock. An anchor on a row that reads no amount and no drive is refused too, because that row is only a parameter in disguise. These checks are partial, as finding 15's is: a law can read a drive only to dodge the last, two fits that reach the body only as a product are not told apart, and a fit stands at its guess when a closed edge cuts it off from its anchor or the body's symmetry fixes that anchor. In each of these the anchor still holds.
- **The fixture packet is the only one validation accepts.** Two compile-fail doctests in `crates/ng-validate/src/lib.rs`, E0308, with a legal twin. `Reviewed`, `Citation` and `Specified` are built only as far as those and the two sketch probes need. Both probes now live in `crates/ng-contract/src/lib.rs`.

**What the brief settled.**

- **The tolerance.** It is an `ErrorScales`, the same reading a core's scales have, and the packet states it with `within`. `ng_validate::admission_tolerance` is the model default, 1e-9 plus 1e-12 of the value. The packet's hash covers it, so an initializer cannot widen it under a run's identity. A packet that states none is held exactly, which is the old rule and leaves every existing fixture as it was. It is reported with the row nearest it. At an instant, an amount or memory row reads its value less the packet's, like any other row, so a value rounding moved inside the tolerance stands.
- **Finding 15.** `bind` now compares what two sides compute, not the nodes they are, so `(p + p) / (p + p)` from separate reads is refused, and so are `(p + q) / (q + p)`, `(p q) / (q p)`, swapped smooth limits and a value over its own `as_dimension`. It stays partial: a regrouped sum, a swapped `scale` and `p / (p + 0)` still pass.
- **Method and window.** An anchor is a `Snapshot`: row, method, statistic and value. `bind` matches the window to the row and the packet. `initialization` refuses an anchor no channel of the request's lens reads by that method over that window, reading exactly the ring's points, with the run's start on its grid, because a lens is the one place a method meets a row.
- **The fence.** The solve reads the drives of `[t0, t0 + T)`, so the opening fence is `t0 + T`, the first instant it did not read. An instant's window is one nanosecond, so one rule gives the old value. `a_late_input_inside_the_window_is_refused_and_one_at_its_end_is_taken`, and all seven replay gates pass on this case. A delivery inside the window shaped the start, so every run of a case holds it.

**Choices, each with its reason.**

- **The beat lives in the packet.** It shapes the start, so the packet's hash must cover it, and a run's identity stays four things. `admit` closes the hash a run names over the moment the run starts, the drives the window read, the start it reached and the core's history there. So one run identity means one start, whatever made it, and `bind`'s hash and the hash a run names differ. Later events stay out of the hash, as before.
- **A ring, not shooting through a core.** Shooting makes the start depend on the core, and `admit` would need a core to check it. Run-in would take about a million beats on a slow total, where Newton took three updates in every run, from guesses 0.2 to 20 times the answer.
- **One conserved total replaces one row.** Around the ring a combination's rows add up to its net boundary flow, so one row says nothing new and the total is free. The total takes that row's place in the solve. `admit` still reads the replaced row at its own scale, and it adds each total up around the whole ring, so a net flow over the beat is refused as `NetBoundaryFlow`, however thinly it was spread.
- **A solve that stops is not a failure of its own.** It hands `admit` what it reached, and `admit` is the one judge. The solve stops when no unknown moved by more than rounding on itself, or after 32 updates. It reads no tolerance, so a tolerance only refuses and shapes no bit, and a row or a fit too small for the tolerance to see is solved all the same. An unknown that solves to zero never settles, and a weak fit may not. Each costs the 32 updates and nothing else.
- **`admit_fixture` still returns the packet's guess.** That is §7's must-fail row, and 32 sites read it. `admit_solved` is the solve.

**Left out, and what brings each back.** A window mean returns with a check that reads one. A mean and spread, quantiles and correlation groups return with the first source that gives one. `Reading`, `Tier` and an inferred kind return with the first reviewed packet. A mode under a cycle stays refused, a heart that sets its own rate needs a free period, and a body past a few thousand unknowns needs a solve that follows the ring: ticket 10 has all three.

**Limits that refuse, stated plainly.** Each fails closed.

- A very stiff law: past about `h k` of 1e5 to 1e6 no value passes, because rounding alone is above the tolerance. That is a time constant near a microsecond here, and it is the one place the plain solve runs out. No outside physiology was needed. A row that swings far above a small start meets this limit sooner, near an `h k` of 1e3, because the row a total replaced carries the rounding of the whole ring.
- A row with no restoring term under a cycle, such as `m' = 0`: the ring has no level for it, so the matrix is singular, the solve stops and `admit` names the first row it finds, not the cause.
- A Newton trial outside a law's domain stops the solve. There is no step halving, and `admit` gives the verdict.
- `admit` holds each stage alone, so a guess or a hand-made start can miss closing the ring by the tolerance times the number of steps. The solve's start closes, because it runs to rounding.
- On a body of amounts far below one, the default's floor of 1e-9 lets `admit` pass almost any guess, until rows have their own scales. The solve is not affected. Such a packet states its own floor.
- A packet that states no tolerance is held exactly, so rounding alone can refuse a solved start. A packet meant for the solve states one.
- A net flow inside a wide tolerance can pass on a guess that spreads it and be refused after the solve, which puts it all on one row.
- Two coupled rows 1e15 apart are past what a float can hold, so the small one wanders and the solve can stop anywhere inside the tolerance.
- A huge interval count asks for that much memory and ends the process. It is the packet author's number, and no learner or run can reach it.
- A forged checkpoint can still add a waiting delivery, or name another start, because a seal covers nothing that waits and the digest has no key. That design is older than this ticket, and nothing here has an adversary.
- `bind`'s anchor and finding 15 checks are partial. A law can read an idle drive to dodge the first, and the second folds nothing.
- A fit is read step by step, so one whose whole effect on each step is below rounding on its rows is refused, though the sum over the beat might show it. On a ring of 64 that takes a flow near 1e-12 per beat on amounts of 50. A fit that solves to zero from a guess of zero is refused the same way.

**Not proven here.** The test body is driven from outside, so it says nothing about a heart that sets its own rate. No reader from another vendor has seen this design or code: Codex ran out of credits and Grok stopped at its free limit, so the base was picked alone and the adversaries were two Opus seats. See the PR.

## Comments

From 03: a mode variable declares its values but no starting value, so initialization picks one consistent with the guards. Sketch probes this ticket owns: lines 46 (an anchor needs a citation, E0061) and 201 (a synthetic value cannot be an anchor, E0308).

From 04: `body.bind(packet)`, `body.initialization(parameters, boundary, request)` and `system.admit(solved, history)` exist, with `InitializationRequest::consistent_instant`. `packet.start(row, measure, note)` carries start values into the packet's hash until anchors replace it, and no `Initializer` trait exists, so `ng_validate::admit_fixture` is the one caller. `admit` refuses a moved amount or memory value and an algebraic residual that is not exactly zero, because no tolerance is declared yet; this ticket owns that tolerance. `bind` refuses a body with a mode variable.

From the deep review of 02 to 04: `admit` now evaluates every law at the start in every body, so a law with no value there is `InitializationFailure::Undefined` whether or not the body has an algebraic row.

Decided after the deep review of 02 to 04 (nightingale#13, findings 12 to 17): finding 15 gets a partial admission check. Admission refuses a constant built only from literals, and a parameter divided by itself, when the constant has no packet entry. This is not a complete check: other ways to build a constant stay possible, and no compile-time check is claimed.

From 05: `ImageParts` replaces `InitialParts`, with `canonical` and `parse` for its bytes, so a new image part lands in that one place and in the checkpoint at once. `PendingInputs::opening` sets the fence one nanosecond past the anchor, because initialization read that instant's drive. A windowed initialization reads a span of drives, so it must raise the opening fence past every instant it read, or a late input at one of them breaks I5.

From 06: `ModesNotYetRun` now refuses a mode variable a plain guard fires or no hazard guard moves, and any mode variable in a body with an algebraic row, because a firing leaves algebraic values as they were. `ParameterPacket::start_mode` carries a first mode into the packet's hash, and `BoundParameters::modes` is the slot initialization fills when it picks one consistent with the guards. Finding 15's partial check landed here: `bind` refuses `UnbackedConstant` for `exp` or `logistic` over literals and for a division of two equal nodes, which `(p + p) / (p + p)` built from separate reads is not, so this ticket extends it and does not rebuild it. Admission takes no seed: a forcing reads 1 and a draw 1/2, so a start state and any calibration are a function of the packet alone. A seeded start would need a seed in the request and a check against the identity.

From 07: `PacketSeed` exists for `ParameterPacket::sampled` and has no caller yet. Its word is `seed_word("packet", words)`, so `sampled` takes it with no change to any draw.
