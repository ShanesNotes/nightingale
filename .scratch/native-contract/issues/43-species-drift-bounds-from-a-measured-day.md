# 43: Each species' drift bound on the pick comes from a measured day, not a 2 s window

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (Slice and certificate), [record 0004](../../../docs/adr/0004-first-milestone.md) (each amount may change over 24 h by up to 1.96 times its within-subject variation), and `crates/ng-validate/src/slice.rs` (`SYNTHETIC_DRIFT`, `arena_certificate`)

Type: task

Status: resolved

Claimed by: nc43-bounds

Blocked by: none

The pick dry run on main at `43afdb7` (`engine-bench score 10 5 ng-solver pick`, ryzen) failed its certificate on drift: sodium changed by 18,316.79 over the window from 06:55 to 06:55, against a bound of 1e4. The ledger closed, since a closure failure would have been reported first. The certificate names only the first failure in basis order, so chloride, potassium and the later quantities were not reported.

The 1e4 began as [ticket 14](14-slice-certificate.md)'s bound for a 2 s window (`5c4110f`, `crates/ng-validate/tests/slice_certificate.rs`). [Ticket 24](24-arena-ready-to-dispatch.md) carried it into `arena_certificate` for every species and element. [Ticket 28](28-admission-energy-at-balance.md) measured energy over the day and replaced its bound with 2e5, and left every other quantity at 1e4.

The bounds are set from the day measured on main at `39bdb91`, below. [Native-contract 42](42-veins-collapse-branch.md) moves the slice's model hash, so its own PR measures the day on the slice it leaves, and moves a bound only if a figure leaves it. The day keeps its meal ([native-contract 46](46-rows-9-and-10-need-a-gut-and-the-day-keeps-its-meal.md)).

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

- [x] Measure every species' drift over the pick protocol's window on the working core, on the slice as native-contract 42 leaves it, with the day plan's meal, as ticket 28 did for energy: each figure, inflow and outflow for each, and where the window's ends fall in the posture cycle. Measured once on main at `39bdb91` (the table above). Native-contract 42's PR measures it again on the slice it leaves. The window takes about 98 minutes on the beelink, so it runs as its own job
- [x] The developer reviewer rules, with this evidence, on the fork record 0004 leaves open for the slice. Its sodium and chloride answer their own amounts slowly, and not at all below a few thousand (record 0004's "keeps moving because nothing brings it back"), and [body 17](../../body/issues/17-volume-hormones.md) builds that answer on the whole body only, since the slice does not move. Either the arena's bound admits the slice's measured day, as energy's 2e5 admitted the slice's heat, and the milestone protocol's day-two windows are read for growth; or the pick waits for a body that answers its sodium
- [x] Under the ruling, set each species' and element's bound the way §7 and record 0004 say: from a synthetic within-subject variation in the synthetic healthy-range set, or, as energy's was, as fixture data at a stated multiple of the measured figure. A test holds each bound just under and just over, and a mutant row catches a bound falling back to 1e4
- [x] §7 and `slice.rs` say where each bound comes from

**Non-goals:** energy's 2e5; native-contract 31's window statistic; any law of the slice.

## Answer

**The ruling.** The arena's bound admits the slice's measured day, as energy's 2e5 did, and the milestone protocol's day-two windows are read for growth. It was the oversight seat's recommendation, and the developer reviewer approved it.

**The bounds.** Each is fixture data in `crates/ng-validate/src/slice.rs`: twice the quantity's measured change over the pick's window, rounded up to one significant figure.

| Quantity | Measured change | Bound | Margin |
|---|---|---|---|
| Sodium | -18,316.8 | 4e4 | 2.18 times |
| Chloride | -19,234.0 | 4e4 | 2.08 times |
| Potassium | -917.2 | 2e3 | 2.18 times |
| The moving water | -232.0 | 5e2 | 2.16 times |
| Vasopressin | +395.8 | 8e2 | 2.02 times |
| Energy | +83,433.6 | 2e5, unchanged | 2.40 times |
| Charge | +1.7e-10 | the closure floor, unchanged | |
| Albumin, the transmitters | 0 | 1 | |
| The held water pools | 0 | water's 5e2 | |

On these figures the dry run's certificate would pass Drift, since every measured change sits inside its bound. The day was not run again.

**A species no boundary route carries** takes a bound of 1, a model default. Closure already holds such a species at exactly zero on the slice, since no boundary route carries it, so the bound binds only on the first body whose route moves one. At under 1% of a transmitter pool's 101, the smallest such pool, it makes that body measure its own day instead of inheriting a bound nothing measured. The certificate keys a bound by species, so the held water pools share the moving water's 5e2, and closure holds them at zero too.

**Charge** keeps its drift at the closure floor, as record 0004 says, in `crates/ng-validate/src/certificate.rs`. **Elements** keep 1e4: no test reaches that arm until [body 09](../../body/issues/09-tissues-respire.md) puts a reaction in a body.

**Tests and mutants.** `crates/ng-validate/tests/arena_certificate.rs` runs a synthetic body that each limited quantity enters at a steady rate, with an ion beside its counter-ion, so charge holds in the run that passes; in the run that fails the certificate names the species first, since charge comes last in the basis. Each bound passes a drift a thousandth under it and refuses one a thousandth over it, by name. Five mutant rows in `tools/gate/mutants.toml` put one arm back to 1e4 (sodium and chloride, potassium, vasopressin, water, the unmoved bound), and each is caught.

**Record 0004.** The record holds a reviewed packet's drift to 1.96 times a within-subject variation. Its last consequence says the arena runs only synthetic fixtures, whose checks run on obviously synthetic targets and test the checks, not the body. The slice's bounds are such targets, so they score a solver and judge no body, and the record stands as it is.
