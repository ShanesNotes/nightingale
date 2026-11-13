# 46: Rows 9 and 10 need a gut the slice does not have, and the slice's day keeps its synthetic meal

Spec: [native-contract 21](21-day-checks-read-the-trace.md)'s rows 9 and 10 (Gill 1985), `crates/ng-validate/src/day.rs` (`Nutrient::delivered`, "synthetic fixture amounts"), `crates/ng-validate/src/slice.rs` (`gut`, `absorption`: the stand-in that puts a meal straight into the splanchnic veins), [body 13](../../body/issues/13-gut-absorbs-a-meal.md) and [body 31](../../body/issues/31-one-intake-event-and-the-arenas-day.md)

Type: task

Status: open

Blocked by: none

Rows 9 and 10 want plasma osmolality and plasma sodium to rise over 1 to 4 h after each meal, against the same run without that meal. In the pick dry run at `43afdb7` both fall at all three meals: sodium by 3.3 to 5.0 and osmolality by 7.0 to 10.6. Main `c4760e8` gives the same figures for the 07:30 meal (`engine-bench day 10 ng-solver 16500`, ryzen).

Two things make the fall, and neither is the core:

- **The slice has no gut.** The gastrointestinal card is a held stand-in: a meal's water and salt enter the splanchnic veins at a steady rate over 1 h, with no lumen, no secretion and no nutrient. So plasma can only move toward what the meal brings. A probe on main feeding the same sit-up and one meal at 07:30 measured it: over 07:30 to 08:55, sodium at the plasma's own concentration (98.4 a unit of water) leaves plasma sodium nearly where its control is (-0.29), and sodium at 1.5 times it raises plasma sodium by 1.62 and osmolality by 2.85. A healthy man's plasma sodium and osmolality rise even after a low-sodium meal (Migdal 2020, doi 10.14814/phy2.14585, read to its results paragraph: +0.7 mmol/L at 60 min after a 138 mg meal), so the source's rise needs a gut that takes a meal into its lumen first. [Body 13](../../body/issues/13-gut-absorbs-a-meal.md) builds that gut on the whole body. The slice does not move, so it never gets one.
- **The day plan's meal is water on the slice's scale.** Each meal is 500 water with 50 sodium, 2 potassium and 52 chloride. The slice's plasma holds about 98 sodium a unit of water and the gut's steady supply about 33, so the meal is about a thousandth as salty as plasma. On main it dilutes plasma sodium to 92.7 by 08:55 against 98.7 without it, raises the mean arterial pressure by 9 (the slice holds cell water, so the meal's water stays outside cells) and drives sodium excretion to 3.8 a second, eight times the seated rate. That natriuresis also moves the day's sodium, which the certificate judges ([native-contract 43](43-species-drift-bounds-from-a-measured-day.md)): by 10:55 the fed morning is 25,427 below the same morning without the meal, and the day from 06:55 to 18:30 ends 26,021 below admission with a seated mean arterial pressure of 86 and a heart rate of 2.17 a second.

The slice's day keeps its synthetic meal, because no meal leaves the slice near balance. A meal at the gut's own mix (33.3 sodium a unit of water) ends 7,983 sodium below the no-meal control at 10:55 and is still falling. A meal at plasma's own concentration, 49,200 sodium, leaves the body 46,780 above admission at 08:55 and 43,765 above the no-meal control, so it keeps most of what it brings. The slice answers volume and pressure and not its sodium amount, so a meal tuned to cancel its own natriuresis would be a number fitted to pass. [Body 31](../../body/issues/31-one-intake-event-and-the-arenas-day.md) states the meal on the whole body's scale when it moves the day into the packet.

Evidence: `~/lanes/logs/ng-wave-4/sodium/` (`REPORT.md`, `NOTES.md` from "Follow-up from the coordinator", and `probe/compare-p12-against-p11.txt`).

**What to build:**

- [ ] Rows 9 and 10 report not available yet on a body whose gut is a held stand-in, with that reason in the table, the report line and ticket 21's registry test, and they score on the whole body once body 13 lands

**Non-goals:** a gut on the slice; a change to the slice's meal; vasopressin and excretion rows 11 and 12; the drift bound itself (native-contract 43).
