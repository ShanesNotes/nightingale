# 82: What a moved rest does not admit: a pleural pressure read from the lung's volume, and a rest drive at the breath's mean

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, and `crates/ng-respiratory/src/airway.rs` (the pleural pressure)

Type: prototype

Status: resolved

Claimed by: nc82-diagnose

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours with its lungs. [Body ticket 10](../../body/issues/10-lungs-breathe.md) states the pleural pressure from the pump drive by the mechanics' static solution, `P_pl = P_pl,rest - E_L / (E_L + E_W) P_mus`, because the form the mechanics give, the chest wall's recoil at the lung's own volume less the pump's pressure, `P_pl = E_W (V - V_W0) - P_mus`, did not admit on the whole body. The static form drops the airway's lag of about 0.2 s and the slow drift of the resting volume. A later rung that needs either, such as a forced breath or a stiff lung, needs the volume read.

**What the plan's two tries found.** Body ticket 10's plan prototyped both forms on the whole body (main at `2921c7f6`, the plan's evidence outside the repo). With everything else the same:

- With the pleural pressure read from the volume, admission was refused after 2,515 s with `Inconsistent` on the norepinephrine vesicle at the arteriolar junction: a stage outside its set after Newton, the first admissible-set check in admission's recheck. A second try gave no verdict in 1,800 s.
- With the pleural pressure stated from the drive, the body admitted in 689 to 760 s.
- The rest state the two tries admit differs only in the pleural form, since the breath rests and the drive is constant at admission. The volume read couples every enclosed vessel's wall to the alveolar gas's amounts, so the admission's Jacobian gains a column on each of those rows and a path from the gas to the circulation.

**What 10b's build found.** The breath rests at admission, and its first statement held the pump drive at rest at the breath's mean, 0.3684, so the admitted pleural pressure and lung volume would be the breathing means: -3.74 against -3.0, and the lung's volume up by the mean excursion, about 4,600. On the whole body at main `8a0aadb`, with everything else the same:

- At the mean rest drive with the mixing exchange at its closed form (3,460.8), admission was refused with `Inconsistent` at row 2, the norepinephrine in the cleft at the arteriolar junction.
- The same with the lung's gas guessed at the mean volume was refused at row 18.
- The mean rest drive with the mixing exchange at 10a's 3,240 was refused at row 1, the norepinephrine vesicle there.
- At a breath rate of 0.27 a second, with its own mean drive (0.3603) and mixing (4,257.3), the body admitted in 342 s, but its muscle bed owed 1.75e-4 oxygen and held 5.8e-5 lactate at admission, where the rest check allows 3e-9 and 1e-9.
- With the drive at rest at zero, as the plan had it, the body admitted in 234 s. Each row named is the first one admission's recheck tests, so a refusal there says only that Newton did not settle.

The body states the breath so that its means are what the zero-drive body admits: the pump's pressure and the chest wall's relaxed volume are fitted, so at the mean drive the lung's static volume and the pleural pressure are the zero-drive ones.

- [x] A diagnosis: which of admission's Newton, its scales or the coupled system's conditioning keeps the volume read from settling, shown on the whole body or on a fixture that reproduces it
- [x] For the mean rest drive: whether the moved body has no state admission accepts, or Newton cannot reach it from the packet's guesses. A continuation in the rest drive from 0 would tell
- [x] What it would take to admit the volume read, with its cost in admission's time and in a step's
- [x] The Answer says whether the contract's admission would change, and if so asks for a design round

**Non-goals:** the breath's rhythm, and any change to the static form or the fitted rest the body states today.

## Answer

Measured on the whole body at main `5fe4e63c`, with a probe that prints each Newton update of admission's solve, each sweep's change to the located crossings and each slot the recheck refuses. The probe, the patches and the logs stay outside the repo. Ten whole-body runs, each under 1,500 s, on one build worker beside other jobs. No smaller fixture was built: the whole body showed each refusal in 9 to 13 minutes, and the volume read's cause needs the heart's valves and the enclosure together.

Both refusals reproduce. The volume read was refused with `Inconsistent` at row 1 after 735 s and 758 s. The rest drive at the breath's mean, with the chest wall's rest at 75,750 rather than fitted, was refused at row 18 after 545 s. In each, thousands of stage residuals stood over tolerance (1,666 slots in 2 legs, and 111,263 in 230). So the row named is only the first slot the recheck tests, as the ticket says.

**The volume read: a sweep that cycles, not Newton, its scales or conditioning.** Shown.

- The state exists and Newton holds it. Started from the static form's admitted state, the volume read admitted in 55.1 s and 57.7 s, in 4 Newton updates, at the same scales. So conditioning and the scales are ruled out.
- From the packet's guess, Newton settled again and again, to a merit as low as 1.7e-26, and each time the sweep changed the located crossings. Two valve closings sit 4 microseconds apart in one interval: the aortic valve's at 0.285424 s into the beat and the leg-vein valve's at 0.285428 s. When the sweep adds the aortic closing, Newton's first update would carry it past the leg-vein closing. Admission keeps the crossings of an interval in order, so the update is clamped onto its neighbour, and Newton stops at the first clamp. The sweep then drops the crossing, Newton settles without it, and the next sweep finds it again. That cycle ran to the limit of 32 sweeps, and the run ends on a sweep with no Newton after it, so its legs stand unsolved.
- The state Newton reached without that crossing fails the recheck (`GuardSweep`), so keeping it does not help. At the admitted state the aortic closing comes first, 41 microseconds or 0.39 ms before the leg-vein closing.
- Inferred: the central veins sit inside the enclosure, so the volume read moves the leg-vein valve's closing with the lung's gas, which the static form does not.

**The rest drive at the mean: Newton cannot reach it from the packet's guesses. Whether a state exists at the mean is not shown.**

- From the packet's guess, Newton stalls. Its line search takes 1e-3 to 5e-4 of each update, and the merit stays at 6.5e-4 to 8.3e-4. With more sweeps allowed, it fell only from 8.3e-4 to 7.8e-4 in 160 more updates. The liver bed's blood pressure leads the residual in every stalled solve. Why is not shown.
- The continuation, each step from the state the last one admitted, admitted at drives 0, 0.1, 0.15, 0.2, 0.3, 0.3171, 0.3342 and 0.3513: 95% of the way to the mean, 0.368401. At each admitted step the lung's volume moved by 11,879 a unit of drive, the pump's pressure over `E_L + E_W`. The muscle bed owed no oxygen (worst debt 9e-26 against the rest check's 3e-9, lactate under 1e-18). No step diverged, so no turning point was seen.
- Each step to the mean took more updates: about 25 to 0.3342, more than 32 to 0.3513, and more than 140 from 0.3513 to 0.368401, which had not settled when the run was cut. Whether that is a turning point near the mean or a hard solve is not shown.
- Two limits of admission's solve refused steps that more work admits. When Newton uses its 32 updates without settling and the sweep then finds no change, the solve stops. The step 0.3 to 0.3342 was refused that way, and admitted once the solve was allowed one more sweep. And when no trial of an update lowers the merit, the solve takes the nearest one: in the one step from 0 to the mean the merit rose from 4.4e-5 to 5.2e-4 over three sweeps.
- The muscle bed's debt that body ticket 10's try at a breath rate of 0.27 a second admitted with was not seen on this path. That try had its own rate and mixing exchange, and it was not rerun.

**What admitting the volume read takes, with its cost.** Shown: admit the static form, then solve the volume read from its state. That adds 55.1 s and 57.7 s to admission's 239 s to 247 s, under the same load. A step costs the same: 20 simulated seconds took 7.42 wall seconds with the volume read and 7.57 with the static form (one run each, load 4.6 to 5.1, so no difference past noise). Not tried: a solve that may reorder or merge two located crossings in one interval, which would admit the volume read from the packet's guess.

**The contract's admission would change.** Section 4 has the solve march to a guess from the packet's values, solve once with Newton, and sweep until no crossing changes. It says nothing of a sweep that cycles between two sets of located crossings, and its guess cannot reach a rest moved this far. Controlled breathing ([body ticket 11](../../body/issues/11-breathing-is-controlled.md)) moves the rest first and meets both. [Native-contract 86](86-admission-reaches-a-moved-rest.md) asks for the design round.

