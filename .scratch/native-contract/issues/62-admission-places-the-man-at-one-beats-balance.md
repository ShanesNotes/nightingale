# 62: Admission places the man at one beat's balance, not in his day

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §4 (admission) and §7 (the milestone protocol), `crates/ng-contract/src/admission.rs`, and [native-contract 52](52-energy-fails-the-sliding-windows.md)

Type: task

Status: open

Blocked by: none

**What the slice shows.** Contract §4 says admission "cannot move a conserved total, unless a boundary law reads the body: then the total answers its own flows, admission solves it, and the packet's value is a guess". On the slice that covers water (urine), sodium, potassium and chloride (the kidney), energy (heat) and vasopressin (clearance). Native-contract 52's plan started the slice's packet at -5,000 sodium in the splanchnic interstitium, with -250 potassium and -5,250 chloride. Admission gave the same sodium total, 866,616.6, to 1e-9. So a fixture amount cannot place the man in his day: admission always solves him at the balance of one beat at 06:55, and only the totals no boundary law reads (albumin, the held waters and the transmitters) keep the packet's amounts. None of those moves. Evidence: `~/lanes/logs/ng-wave-4/nc52-plan/` (`PLAN.md` §1, `smoke.sh`, `smoke-tuned.log` against `smoke-10ms.log`).

**What it costs.** A four-day run of the arena's plan on `ng_solver::Core::new` at 10 ms, at model hash `93c12728` (`r0.log` there, `r0-windows.txt`). Each conserved quantity's worst sliding 24 h window, on a 60 s grid of starts. A start in day 1 is the milestone's, from admission. A start in day 3 sets day 4 against day 3, when the man has mostly settled.

| Conserved quantity | Its bound | Worst window, day-1 starts | Worst window, day-3 starts |
|---|---|---|---|
| Sodium | 4e4 | 16,439 | 2,156 |
| Chloride | 4e4 | 17,262 | 2,263 |
| Potassium | 2e3 | 823 | 106.7 |
| Water | 5e2 | 221 | 35.7 |
| Vasopressin | 8e2 | 590 | 253 |
| Energy | 4e6 on the milestone's windows, 2e5 on the pick's | 1,828,020 | 248,374 |

The first day reads between 2.3 and 8 times what a settled day reads. A bound by native-contract 43's rule from the settled day would be about 5e3 for sodium and chloride, 3e2 for potassium, 8e1 for water, 6e2 for vasopressin and 5e5 for energy. A protocol that judges the first day after admission cannot get those.

**Why it matters beyond the slice.** I19 reads 48 h "from admission". A reviewed packet's bounds come from the healthy-range set, not from a measured day (record 0004). On the slice the bounds were set from the slice's own first day, so they admit it, and that hides this. A reviewed man admitted out of his day would fail I19 by settling, not by a leak.

**The routes.** None is chosen.

- Admission solves a day, not a beat: one pass of the day's cycle in the admission solve.
- The packet states the totals a boundary law reads, so admission keeps them.
- A settling stretch before the judged 48 h. This changes I19's "from admission", so it is the developer reviewer's call.

- [ ] A four-day run of the slice with `ms.skin-water-amplitude` at 0 says whether the skin-water wander is what makes each settled day differ (native-contract 52's plan, §2: about 7.7 h on the beelink)
- [ ] A plan picks a route by prototypes on the slice
- [ ] The route is chosen before the first reviewed packet's run

**Non-goals:** the slice's bounds, which native-contract 43 and 52 set; the pick's protocol.
