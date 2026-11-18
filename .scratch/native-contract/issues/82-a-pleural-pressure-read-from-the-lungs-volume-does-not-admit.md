# 82: A pleural pressure read from the lung's volume does not admit

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, and `crates/ng-respiratory/src/airway.rs` (the pleural pressure)

Type: prototype

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours with its lungs. [Body ticket 10](../../body/issues/10-lungs-breathe.md) states the pleural pressure from the pump drive by the mechanics' static solution, `P_pl = P_pl,rest - E_L / (E_L + E_W) P_mus`, because the form the mechanics give, the chest wall's recoil at the lung's own volume less the pump's pressure, `P_pl = E_W (V - V_W0) - P_mus`, did not admit on the whole body. The static form drops the airway's lag of about 0.2 s and the slow drift of the resting volume. A later rung that needs either, such as a forced breath or a stiff lung, needs the volume read.

**What the plan's two tries found.** Body ticket 10's plan prototyped both forms on the whole body (main at `2921c7f6`, the plan's evidence outside the repo). With everything else the same:

- With the pleural pressure read from the volume, admission was refused after 2,515 s with `Inconsistent` on the norepinephrine vesicle at the arteriolar junction: a stage outside its set after Newton, the first admissible-set check in admission's recheck. A second try gave no verdict in 1,800 s.
- With the pleural pressure stated from the drive, the body admitted in 689 to 760 s.
- The rest state the two tries admit differs only in the pleural form, since the breath rests and the drive is constant at admission. The volume read couples every enclosed vessel's wall to the alveolar gas's amounts, so the admission's Jacobian gains a column on each of those rows and a path from the gas to the circulation.

- [ ] A diagnosis: which of admission's Newton, its scales or the coupled system's conditioning keeps the volume read from settling, shown on the whole body or on a fixture that reproduces it
- [ ] What it would take to admit the volume read, with its cost in admission's time and in a step's
- [ ] The Answer says whether the contract's admission would change, and if so asks for a design round

**Non-goals:** the breath's rhythm and its rest mode, and any change to the static form the body states today.
