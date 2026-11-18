# 82: What a moved rest does not admit: a pleural pressure read from the lung's volume, and a rest drive at the breath's mean

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, and `crates/ng-respiratory/src/airway.rs` (the pleural pressure)

Type: prototype

Status: open

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

- [ ] A diagnosis: which of admission's Newton, its scales or the coupled system's conditioning keeps the volume read from settling, shown on the whole body or on a fixture that reproduces it
- [ ] For the mean rest drive: whether the moved body has no state admission accepts, or Newton cannot reach it from the packet's guesses. A continuation in the rest drive from 0 would tell
- [ ] What it would take to admit the volume read, with its cost in admission's time and in a step's
- [ ] The Answer says whether the contract's admission would change, and if so asks for a design round

**Non-goals:** the breath's rhythm, and any change to the static form or the fitted rest the body states today.
