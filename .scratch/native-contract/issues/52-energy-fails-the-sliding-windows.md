# 52: Energy fails the milestone's sliding windows on the slice

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (the milestone protocol), `crates/ng-validate/src/arena.rs` (`ArenaProtocol::milestone`), `crates/ng-validate/src/slice.rs` (`SYNTHETIC_ENERGY_DRIFT`) and `crates/ng-validate/src/certificate.rs` (the drift check)

Type: task

Status: open

Blocked by: none

The milestone protocol holds every sliding 24 h window over 48 h to each conserved book's drift bound. The statistic is the change between a window's two ends. [Native-contract 43](43-species-drift-bounds-from-a-measured-day.md) set each species' bound from the slice's measured day. A two-day run of the slice on `ng_solver::Core::new` at 10 ms, under the arena's two-day plan, reached 172,800 s with no fault, and every species bound holds over every window. The closest is vasopressin, at 0.59 of its bound.

Energy does not hold. Its bound, 2e5, is about twice its change over the one window from 06:55 (8.3e4). Of 1,441 window starts on a 60 s grid, 626 fail: the starts from 07:14 to 12:32 and from 13:36 to 18:42. The worst window starts at 08:49 and changes by 1,959,469. A window whose two ends fall in the night passes: the worst start between 23:00 and 06:55 changes by 4.5e4. The pick's protocol runs the one window from 06:55, so the pick passes. The milestone protocol on this slice fails Drift on energy, whatever the core.

The cause, as far as two days show it: the two mornings are not alike, though both days hold the same events. By 08:55 energy's inflow since 06:55 is 9.26e7 on day one and 7.99e7 on day two, 14% less. Over the whole day it is 1.5% less (1.0275e9 against 1.0123e9). Day one also sheds sodium (-18,317, against +2,490 on day two) and water. So the admitted state is not the state the same man is in at 06:55 a day later, and the statistic reads that difference at every hour where energy is moving. The run, its log and the window script are in `~/lanes/logs/ng-wave-4/nc43-windows/` (`RESULT.md`, `coordinator-energy.txt`).

**What to build:** a milestone protocol the working core passes on the slice, with energy's bound or statistic chosen by runs. Options:

- Energy's bound from the measured windows, by native-contract 43's rule: about twice 1.96e6. That is 0.4% of a day's gross flow (1.0e9), where 2e5 is 0.02%, so it passes a steady leak twenty times larger than today's bound passes.
- Judge energy only on windows whose two ends fall in the night, where 2e5 holds with room. The certificate has one drift limit a quantity and no notion of where in the day a window starts.
- Admit the man in his daily balance: fixture amounts for which the state at 06:55 repeats a day later. Then every book's bound can be tight, and the check reads a leak and not the first day's settling.

- [ ] A third day confirms or corrects the cause: day three against day two, at every start.
- [ ] The milestone protocol passes on the slice with the working core, and each bound states its source as native-contract 43 states the species'.
- [ ] A bound that is raised, or a check that covers fewer windows, has its reason in the fixture's own doc line or in §7, and a mutant row holds it.
- [ ] The PR says what the chosen check still sees: the smallest steady leak in a boundary energy flow that fails it, beside the slice window's whole-beat check ([native-contract 31](31-cross-interval-drift-check.md)).

**Non-goals:** the species' bounds; the pick's one-window protocol; the whole body's bounds, which come from the healthy-range set.
