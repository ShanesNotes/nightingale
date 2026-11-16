# 52: Energy fails the milestone's sliding windows on the slice

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (the milestone protocol), `crates/ng-validate/src/arena.rs` (`ArenaProtocol::milestone`), `crates/ng-validate/src/slice.rs` (`SYNTHETIC_ENERGY_DRIFT`) and `crates/ng-validate/src/certificate.rs` (the drift check)

Type: task

Status: claimed

Claimed by: nc52-build

Blocked by: none

The milestone protocol holds every sliding 24 h window over 48 h to each conserved book's drift bound. The statistic is the change between a window's two ends. [Native-contract 43](43-species-drift-bounds-from-a-measured-day.md) set each species' bound from the slice's measured day. A two-day run of the slice on `ng_solver::Core::new` at 10 ms, under the arena's two-day plan, reached 172,800 s with no fault, and every species bound holds over every window. The closest is vasopressin, at 0.59 of its bound.

Energy does not hold. Its bound, 2e5, is about twice its change over the one window from 06:55 (8.3e4). Of 1,441 window starts on a 60 s grid, 626 fail: the starts from 07:14 to 12:32 and from 13:36 to 18:42. The worst window starts at 08:49 and changes by 1,959,469. A window whose two ends fall in the night passes: the worst start between 23:00 and 06:55 changes by 4.5e4. The pick's protocol runs the one window from 06:55, so the pick passes. The milestone protocol on this slice fails Drift on energy, whatever the core.

The cause, as far as two days show it: the two mornings are not alike, though both days hold the same events. By 08:55 energy's inflow since 06:55 is 9.26e7 on day one and 7.99e7 on day two, 14% less. Over the whole day it is 1.5% less (1.0275e9 against 1.0123e9). Day one also sheds sodium (-18,317, against +2,490 on day two) and water. So the admitted state is not the state the same man is in at 06:55 a day later, and the statistic reads that difference at every hour where energy is moving. The run, its log and the window script are in `~/lanes/logs/ng-wave-4/nc43-windows/` (`RESULT.md`, `coordinator-energy.txt`). These window figures were measured before [native-contract 57](57-the-store-below-a-veins-unstressed-volume.md) made a marked vein's store below its unstressed volume the work of its pressure.

**The third day.** A three-day run of the same plan on the same core splits the starts by the day they fall in (`~/lanes/logs/ng-wave-4/nc52/`: `by-day.txt`, `energy-phase.txt` and the scripts that made them). The first day's settling is most of the failure and not all of it. Of day one's 1,440 starts the same 626 fail. Of day two's 1,441 starts, which set day three against day two, 151 fail: 78 from 07:17 to 08:36, 22 from 14:15 to 15:03 and 51 from 17:45 to 18:35. The worst starts at 07:37 and changes by 639,446, 3.2 times the bound. A window whose two ends fall in the night still passes: the worst of day two's changes by 5.7e4. Each day's change from 06:55 to 06:55 passes too: +83,434, -35,609 and -44,983. Every species holds at every start of both days.

The man is still settling on day three, more slowly. Sodium sheds 18,317 on day one, then gains 2,490 on day two and 2,063 on day three. Energy's inflow since 06:55 differs between days two and three by up to 3.19e6, at 11:41. Its outflow follows, and the gap between the two is what the statistic reads at the hours where energy moves fastest.

This bears on the options below. A bound from the measured windows would be set by day one alone. A run cannot say how many days the man needs to settle: three are not enough. The night windows hold on both days. A man admitted in his daily balance needs fixture amounts that sodium takes more than three days to find by running.

**The collapse branch.** [Native-contract 42](42-veins-collapse-branch.md)'s part 3 moved the slice's day: energy's change from 06:55 to 06:55 is -164,356, 0.82 of its 2e5 bound with its sign flipped, where it was +83,434. No run splits the cause.

**What to build:** a milestone protocol the working core passes on the slice, with energy's bound or statistic chosen by runs. Options:

- Energy's bound from the measured windows, by native-contract 43's rule: about twice 1.96e6. That is 0.4% of a day's gross flow (1.0e9), where 2e5 is 0.02%, so it passes a steady leak twenty times larger than today's bound passes.
- Judge energy only on windows whose two ends fall in the night, where 2e5 holds with room. The certificate has one drift limit a quantity and no notion of where in the day a window starts.
- Admit the man in his daily balance: fixture amounts for which the state at 06:55 repeats a day later. Then every book's bound can be tight, and the check reads a leak and not the first day's settling.

- [x] A third day confirms or corrects the cause: day three against day two, at every start. It corrects it in part: "The third day" above, with `~/lanes/logs/ng-wave-4/nc52/by-day.txt`
- [ ] The milestone protocol passes on the slice with the working core, with [native-contract 68](68-the-ledgers-sum-reads-one-last-place-off.md), and each bound states its source as native-contract 43 states the species'. Each bound now states its source. The pass is the certificate's own 48 h run, which the coordinator starts from `~/lanes/logs/ng-wave-4/nc52-build/milestone-certificate.sh`; this box stays open until that run is read.
- [x] A bound that is raised, or a check that covers fewer windows, has its reason in the fixture's own doc line or in §7, and a mutant row holds it.
- [x] The PR says what the chosen check still sees: the smallest steady leak in a boundary energy flow that fails it, beside the slice window's whole-beat check ([native-contract 31](31-cross-interval-drift-check.md)).

**Non-goals:** the species' bounds; the pick's one-window protocol; the whole body's bounds, which come from the healthy-range set.

## Answer

**Fixture amounts cannot place the man in his day.** Contract §4: admission "cannot move a conserved total, unless a boundary law reads the body: then the total answers its own flows, admission solves it, and the packet's value is a guess". On the slice a boundary law reads water, sodium, potassium, chloride, energy and vasopressin. A packet start of -5,000 sodium in the splanchnic interstitium, with -250 potassium and -5,250 chloride, was admitted with the same sodium total, 866,616.6, to 1e-9. So admission always solves the man at one beat's balance at 06:55, and the ticket's third option has nothing to set. [Native-contract 62](62-admission-places-the-man-at-one-beats-balance.md) holds that question. Evidence: `~/lanes/logs/ng-wave-4/nc52-plan/` (`PLAN.md` §1 and §3, `r0.log`, `r0-windows.txt`).

**Energy's windows by start day.** A four-day run of the arena's plan on `ng_solver::Core::new` at 10 ms, at model hash `93c12728`, on a 60 s grid of starts, against 2e5. A start in day d sets day d+1 against day d.

| Starts in | Failing of all | Worst | Its start |
|---|---|---|---|
| Day 1, the milestone's | 625 of 1,440 | 1,828,020 (9.14 of 2e5) | 08:51 |
| Day 2 | 191 of 1,440 | 520,803 (2.60) | 07:36 |
| Day 3 | 59 of 1,441 | 248,374 (1.24) | 17:46 |

With the baroreflex set point held, the counts are 622, 142 and 128, so the set point is not what fails energy. No warm-up of a few days passes at 2e5.

**The bound.** The ticket's first option, as fixture data: twice energy's worst sliding window, 1.83e6, rounded up to one significant figure, 4e6 (`SYNTHETIC_MILESTONE_ENERGY_DRIFT` in `crates/ng-validate/src/slice.rs`). `arena_certificate` takes it when the windows' coverage is longer than one window, so the pick's one window keeps 2e5 and the pick's policy is unchanged. Every protocol whose coverage is longer than its window takes 4e6 for energy where it took 2e5, not the milestone protocol alone: the whole body's run test (`crates/ng-validate/tests/body_run.rs`, 10 s windows over its first minute), `crates/ng-validate/tests/arena.rs` (1 s windows over 2 s, and 155.5 s windows over 311 s) and a bench `score` with a span past its window. Over windows of seconds that changes no verdict, since 2e5 bound nothing there. Over 120 s windows across the sit-up energy reads 300,206, which 2e5 would refuse, and no gate runs such a protocol. A test holds each bound just under and just over, and three mutant rows hold the choice. [Native-contract 63](63-the-sliding-energy-bound-holds-more-protocols.md) holds what those tests leave open. §7's milestone bullet says where the bound comes from and why.

**Will the certificate pass?** On the same run, for every conserved quantity with a bound: the grid's worst day-1 window plus twice its largest one-minute change, as a fraction of its bound. The three worst are vasopressin 0.800, energy 0.534 of 4e6 and water 0.476. Sodium, chloride and potassium are 0.42 to 0.44, and every other quantity is 0. None is over 0.9 (`~/lanes/logs/ng-wave-4/nc52-build/ruling5.txt`).

**What the check still sees.** Heat leaves by `relax(held, 0, bd.heat-speed)` with k = 1e-3 a second, so energy answers its own flows within about 1,000 s.

- A steady leak in a boundary energy law is part of the model: admission solves the store at its balance and the store absorbs it. No drift bound sees it.
- A core's steady bias b, from admission, moves the store by b/k (1 - e^(-kt)). A window from start s reads b/k (e^(-ks) - e^(-k(s+W))): at most 1,000 b, and only in windows that start within about an hour of admission.
- With the honest change of -164,356 in that window, b fails at 4e6 from about 3.84e3 to 4.16e3 a second for a bias from admission, depending on its sign. A bias that starts later is read whole by a window that holds its start, so over all starts the range is 2.2e3 to 4.2e3 a second, about a fifth to a third of the heart's mean work, 1.17e4 a second. At 2e5 it failed from 36 to 364 a second.
- A ramping bias of a a second squared fails when a·W/k passes the bound: a growth of 4e3 a second over a day at 4e6, and 200 at 2e5.
- Native-contract 31's whole-beat window passes a stroke-work bias up to 2.7e3 a second. That is its plan's figure, measured at main `7f0cbfe`, for a check that reads the first 6 s. So for a bias from admission energy's drift at 4e6 adds nothing below it, and a bias that starts after those 6 s is drift's alone, read from 2.2e3 a second as a loss. Native-contract 31's plan gave energy's bound as a rate of 2.3 a second, assuming the store adds up the bias. On this slice the store relaxes, so the figures above are what it catches. The repo carries that rate nowhere.

**The 48 h gate.** The whole gate over 48 h is the milestone route's own run, after the developer reviewer confirms the pick. The certificate's own 48 h pass comes first, from `milestone-certificate.sh`, and box 1 waits on it.

**The pick's record.** The pick ran at model hash `42c60687`. The merge of nightingale#168 moved the slice's solution at rounding only (item 6 of `~/lanes/logs/ng-wave-4/nc57/verify-coordinator/VERIFY-pr168.md`), and this ticket's PR leaves the pick's policy as it was.
