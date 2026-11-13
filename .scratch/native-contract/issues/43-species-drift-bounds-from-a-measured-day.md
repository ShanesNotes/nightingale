# 43: Each species' drift bound on the pick comes from a measured day, not a 2 s window

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (Slice and certificate: "drift within its bound from the synthetic healthy-range set"), [record 0004](../../../docs/adr/0004-first-milestone.md) (each amount may change over 24 h by up to 1.96 times its within-subject variation), and `crates/ng-validate/src/slice.rs` (`SYNTHETIC_DRIFT`, `arena_certificate`)

Type: task

Status: open

Blocked by: 42

The pick dry run on main at `43afdb7` (`engine-bench score 10 5 ng-solver pick`, ryzen) failed its certificate on drift: sodium changed by 18,316.79 over the window from 06:55 to 06:55, against a bound of 1e4. The ledger closed, since a closure failure would have been reported first. The certificate names only the first failure in basis order, so chloride, potassium and the later quantities were not reported.

The 1e4 began as [ticket 14](14-slice-certificate.md)'s bound for a 2 s window (`5c4110f`, `crates/ng-validate/tests/slice_certificate.rs`). [Ticket 24](24-arena-ready-to-dispatch.md) carried it into `arena_certificate` for every species and element. [Ticket 28](28-admission-energy-at-balance.md) measured energy over the day and replaced its bound with 2e5, and left every other quantity at 1e4.

[Native-contract 42](42-veins-collapse-branch.md) moves the slice's model hash, so the day is measured again on the slice as 42 leaves it, and the bounds are set from that measurement. The day keeps its meal ([native-contract 46](46-rows-9-and-10-need-a-gut-and-the-day-keeps-its-meal.md)).

**The window, measured once on main at `39bdb91`.** `Core::new` at 10 ms, the day plan from 06:55 to 06:55, on the beelink alone in its slot: 5,881 s and 18,181,188 accepted steps, the dry run's count. Change is the window's end less its start, and every closure is within 1e-7 for the species and 5e-4 for energy.

| Quantity | Change | Inflow | Outflow | Bound | Passes |
|---|---|---|---|---|---|
| Sodium | -18,316.792 | 86,550 | 104,866.792 | 1e4 | no |
| Chloride | -19,234.020 | 90,876 | 110,110.021 | 1e4 | no |
| Potassium | -917.229 | 4,326 | 5,243.229 | 1e4 | yes |
| The moving water | -231.970 | 4,092 | 4,323.970 | 1e4 | yes |
| Energy | +83,433.6 | 1.027488e9 | 1.027405e9 | 2e5 | yes |
| Vasopressin | +395.839 | 28,976.38 | 28,580.54 | 1e4 | yes |
| Charge | +1.7e-10 | 200,986.02 | 200,986.02 | closure floor | yes |
| The held water pools, the transmitters and albumin | 0 | 0 | 0 | 1e4 | yes |

Sodium's figure is the dry run's, and its sign is now measured: a loss. Chloride fails the 1e4 bound too, and would be the certificate's next line. Energy's figure equals ticket 28's. Sodium bottoms at -32,529 at 15:55, and the night regains about 10,000 and is still regaining at 06:55.

What the slice does, measured on main `c4760e8` with `Core::new` at 10 ms:

- With no inputs, sodium wanders within 14 of admission over 2 h, and its rate changes sign. Admission starts at balance.
- On sitting up at 07:00, sodium and water move from the blood into the leg and splanchnic interstitium (blood -7,950 in 30 min). Sodium excretion along the duct falls from 1.00 to 0.45 a second within 30 min, while the gut still delivers 1.0 a second. Sodium gains 1,266 in the first seated hour.
- Lying down returns little of it. After an hour seated, sodium keeps gaining for an hour after the 08:00 lie-down, to 2,024 above admission, and then holds between 1,982 and 2,012 for 2.5 h: excretion returns to the gut's delivery with the excess still in the body. After 3.9 h seated (+6,593), it peaks at +7,085 an hour after lying down and gives back 906 over the next 3 h, at 0.05 to 0.11 a second. The slice answers its own sodium slowly, and not at all below a few thousand, so a day keeps most of what its inputs leave.
- A meal's 500 water expands plasma, since the slice holds cell water, and drives a natriuresis: by 10:55 the fed morning is 25,427 below the same morning seated without the meal. A meal at plasma's own concentration instead, 49,200 sodium, leaves the body 46,780 above admission at 08:55 and 43,765 above the no-meal control. The slice's sodium follows whatever the day delivers.
- `Core::doubled` agrees with `Core::new` on these figures, so every honest core shares them.

Evidence: `~/lanes/logs/ng-wave-4/sodium/`, with `REPORT.md`, `NOTES.md`, and the probes, logs and tables under `probe/` (the window is `probe/p10-day-plan-new.log` and `probe/day-tables.txt`).

**What to build:**

- [ ] Measure every species' drift over the pick protocol's window on the working core, on the slice as native-contract 42 leaves it, with the day plan's meal, as ticket 28 did for energy: each figure, inflow and outflow for each, and where the window's ends fall in the posture cycle. Measured once on main at `39bdb91` (the table above). The box stays open for the measurement after 42. The window takes about 98 minutes on the beelink, so it runs as its own job
- [ ] The developer reviewer rules, with this evidence, on the fork record 0004 leaves open for the slice. Its sodium and chloride answer their own amounts slowly, and not at all below a few thousand (record 0004's "keeps moving because nothing brings it back"), and [body 17](../../body/issues/17-volume-hormones.md) builds that answer on the whole body only, since the slice does not move. Either the arena's bound admits the slice's measured day, as energy's 2e5 admitted the slice's heat, and the milestone protocol's day-two windows are read for growth; or the pick waits for a body that answers its sodium
- [ ] Under the ruling, set each species' and element's bound the way §7 and record 0004 say: from a synthetic within-subject variation in the synthetic healthy-range set, or, as energy's was, as fixture data at a stated multiple of the measured figure. A test holds each bound just under and just over, and a mutant row catches a bound falling back to 1e4
- [ ] §7 and `slice.rs` say where each bound comes from

**Non-goals:** energy's 2e5; native-contract 31's window statistic; any law of the slice.
