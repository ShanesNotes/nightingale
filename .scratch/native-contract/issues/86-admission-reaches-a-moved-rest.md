# 86: Admission reaches a moved rest, and settles near-coincident crossings

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, `crates/ng-contract/src/admission.rs` and `crates/ng-validate/src/initialize.rs`

Type: prototype

Status: open

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and every rung that moves the breathing body's rest meets admission first, controlled breathing first. [Native-contract 82](82-what-a-moved-rest-does-not-admit.md) found two limits of the admission section 4 states. Neither is shown to be the physiology, and the first is not Newton's conditioning.

- **A sweep that cycles.** With the pleural pressure read from the lung's volume, the body has a state admission accepts: solved from the static form's state, it admits in 55 to 58 s. From the packet's guess it never does. Two valve closings sit 4 microseconds apart in one interval. Newton's update would carry one past the other, admission keeps the crossings of an interval in order, so the update is clamped, Newton stops at the clamp, and the sweep drops the crossing and finds it again, until the sweep limit.
- **A guess that cannot reach a moved rest.** With the rest drive at the breath's mean and the chest wall's rest unfitted, Newton from the packet's guess stalls. A continuation in the rest drive admits each step to 95% of the mean, with more updates at each step, and the last step had not settled after 140 updates. Two rules of the solve refused steps that more work admits: it stops when Newton has used its 32 updates unsettled and the sweep finds no change, and it takes a trial that raises the merit when none lowers it.

**What body 11's plan measured.** [Body ticket 11](../../body/issues/11-breathing-is-controlled.md) does not wait on this ticket: its controller scales no rest, so both valve closings are main's to every digit, and the body admitted from the packet's guess with the controller in it in every form tried. Under a controller that did scale the rest, the closings came close or crossed, and each run admitted. Main has the leg vein's closing 41.5 microseconds after the aortic valve's. With the drive scale on the breath's depth the leg vein's closed 28.1 microseconds before it, and 276 before at four times the gain. With depth through the pump's junction it sat 3.2 microseconds after it, inside the 4 at which native-contract 82's sweep cycled. The cause was the drive scale's beat ripple reaching the rest drive, which moved 0.033%. This ticket stays the fix for any later move of the admitted rest.

**What body ticket 10's last PR found.** The lung's water slowed admission with no shown cause in its own rows. An independent check counted, back to back at the base and with the water: ring Newton 24 to 39 iterations, 2,280.6 to 3,130.1 G instructions, 155.5 to 231.8 s, the march unchanged (10 calls, 58 beats, leg Newton 90,683 and 91,286), and ruled out by run the gas water's guess and the kidney's rebalance. A probe printed, for each ring iteration, the slot whose relative move sets the step and the merit. Both reach a merit of about 2e-26 in one Newton pass of 23 iterations. The base stops there. With the water, the sweeps then relocate valve crossings (the aortic valve at points 80 and 106, the mitral at 112, the tricuspid at 4 and 106), and three more short passes follow, each led by a valve memory or the right ventricle's pressure. No slot the water moved sets a step. So the time goes to settling crossings that the water's small shift of the circulation moved near a grid point, which this ticket's first box asks about.

**What the round decides.** Each is settled by two or three prototypes on the whole body, with the volume read and the moved rest as its cases.

- [ ] How the solve settles two located crossings in one interval whose order Newton would swap: reorder them, merge them into one knot, or relocate them from the new curves rather than clamp. The volume read admits from the packet's guess
- [ ] How admission reaches a rest the packet's guess cannot: a continuation in a packet value from a solved body, or a guess solved from another form. What the start state's identity covers when its guess came from another solve, given that `admit` stays the only judge
- [ ] When the solve stops: whether an unsettled Newton with no change in the sweep ends the solve, and whether a trial that raises the merit may stand
- [ ] Whether the rest drive at the breath's mean has a state at all: the continuation from 0.3513 to 0.368401 settles, or a turning point is shown
- [ ] The cost of each choice in admission's time, against today's 240 s from the packet's guess

**Non-goals:** the breath's rhythm, the static pleural form and the fitted rest the body states today.
