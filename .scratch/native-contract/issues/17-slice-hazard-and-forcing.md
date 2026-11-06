# 17: The slice carries a hazard and a forcing

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The slice gets the stochastic fixture that §7 asks for: one declared hazard law with its own stream, and one forcing, both synthetic and both model defaults. Ticket 18 runs them through the pace and resume gates, so replay with variability is proven before a core is chosen.

Blocked by: None (can start immediately)

Status: resolved

- [x] The slice declares one hazard law with its own stream and one forcing, both synthetic, and a seeded slice run fires the hazard and reads the forcing

## Answer

The slice now carries its stochastic fixture: one hazard with its own stream, and one forcing. Both are synthetic model defaults on the held MS stand-in in `crates/ng-validate/src/slice.rs`.

- **The slice declares one hazard law with its own stream and one forcing, both synthetic, and a seeded slice run fires the hazard and reads the forcing.** The forcing `skin water variability` scales the skin's water loss by `1 + a s(t)`. Its period is 300 s and its amplitude 0.5, both under `ms.skin-water-*`. The hazard `stir hazard` accrues at a constant `ms.stir-rate` of 2 per second while the variable `stirring` is `still`. It draws on the stream `stir` and moves the variable to `stirred`, where its rate is zero, so it fires once. A stir moves no law in the slice. `a_seeded_run_stirs_once_and_reads_the_skins_forcing` in `crates/ng-validate/tests/arena_body.rs` runs the slice from 06:55 with seed [1, 2, 3, 4] on the reference core at 1 ms. Measured: the stir fires once, 529 ms after admission, against a draw of 1.0560, so the integral reaches the draw over the rate within one floor. Its integral then reads zero. Each step's skin loss, divided by its length and the basal rate, gives the factor its stage read. Over the 2 s run that factor stays inside 1 ± 0.5, moves from one step to the next, and stands near 1.154.
- **A hazard is admitted under a cycle.** Before this change, `bind` refused any mode variable a hazard moves under a periodic packet, because a hazard integral only grows and so never returns around the ring. Admission now holds every hazard integral at its packet value at each ring point, as it already holds every draw and forcing: chance accrues from the anchor moment. The contract's admission paragraph says so. On main, `a_hazard_under_a_cycle_starts_where_the_packet_puts_it_and_accrues_from_the_anchor_moment` (in `tests/admission.rs`) is refused with `ModesNotYetRun`. It replaces the test that pinned that refusal.
- **The traces moved, and why.** The packet and seal digests of both first-second traces moved, because of the new laws, parameters and memory row. On the reference core the stir lands at 529 ms on its own 1 ms grid. The trace moves only in the last printed digit of wall power, from the forcing on the skin's loss. On `ng_solver` the landing is a breakpoint, so the error-controlled steps restart there (600.022833 ms became 608.059522 ms). Its earlier samples move by under 8 µs, because the new row and the varied loss enter its error control.

## Comments

From the split of ticket 11: the slice has neither today. `crates/ng-validate/src/slice.rs` declares no hazard and no forcing. Both laws move the slice's trace, so the trace update says why. A new law on the slice can mean retuning `table`. Ticket 21 carries that lead.

From 06: each rate a forcing varies records its rate class (finding 17). On `ChanceCase` an honest 1 s policy stays within 500 ms of a reference at 1/16 s.
