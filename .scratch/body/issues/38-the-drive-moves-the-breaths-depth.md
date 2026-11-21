# 38: The drive moves the breath's depth as well as its rate

Type: task
Status: open
Blocked by: 11, ../../research/issues/23-how-the-drive-splits-into-rate-and-depth.md, ../../native-contract/issues/87-a-firing-sets-a-memory-to-a-kernels-value.md

**Why.** The developer reviewer ruled the standard: the body's breathing control moves the breath's depth and its rate together. So rate alone is a step. [Ticket 11](11-breathing-is-controlled.md) closes the gas loop on the breath's rate. Ruled at planning, a model default open to the developer reviewer: depth lands before [ticket 28](28-two-synthetic-days.md)'s 48-hour run, through [ticket 24](24-the-synthetic-day.md), which waits on this ticket.

**What to build:** The nervous card's drive scale moves the breath's depth as well as its rate, in the split [research 23](../../research/issues/23-how-the-drive-splits-into-rate-and-depth.md) cites, on the whole body. Ticket 11's parts stay as they are: 11b is the rate and 11c is sleep. [Native-contract 87](../../native-contract/issues/87-a-firing-sets-a-memory-to-a-kernels-value.md)'s form, a firing at each breath edge that sets the pump drive's peak from the drive scale so the breath's depth follows, is the first candidate for the build. Ticket 11's plan saw the drive scale on depth move the admitted rest, and both valve closings with it: if the build moves the rest, [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) is the fix.

The boxes, each with what shows it:

- [ ] Tests: depth and rate each follow the drive in the split research 23 cites, on a fixture, at stated points of the drive scale
- [ ] Run: on the fixture, a 10% rise in carbon dioxide production moves carbon dioxide's pressure by what the built loop's gain predicts, with depth and rate sharing the drive. The closed form comes from the forms, as 11b's does, and the PR prints 11b's figure beside it. No box here asks for a smaller change than rate alone gives: ticket 11 raised rate's gain to give the same loop, and the size of the whole loop is research 22's
- [ ] Every link of the loop moves its reader the right way, each judged where ticket 11 judges it: on the body against the held run, and the central arm's sign on the fixture. Depth and rate are two links here, and each moves with the drive. On the body both runs are read over one span of time, each reading a mean over the whole breaths that end inside it, and the PR shows that two neighbouring spans do not flip a sign
- [ ] A body with both gains zero keeps the drive scale at its rest's bits, so neither its rate nor its depth answers the gases. It has its own admission, as in ticket 11, and it is the held run the link box judges against
- [ ] The certificate passes on the body under [native-contract 85](../../native-contract/issues/85-the-certificates-windows-end-on-breath-edges.md)'s windows between breath edges, and the body's trace is updated
- [ ] Cost: unknowns and rows before and after, and user instructions a simulated second against [ticket 26](26-cost-budget.md)'s budget, on the body's bench at the PR's base and its head. Ticket 11's plan measured depth on a continuous path as adding 238.2 M instructions a simulated second against rate's 41.2 M, as native-contract 87 quotes it, because the path from the gases to the pump's pressure joins the solver's largest dense block. Native-contract 87's form is the first candidate for paying less. After ticket 11's three PRs about 32 M of the gas loop's share is left, by ticket 26's running total. If the prototype cannot bring depth inside the budget, the budget's owner records an exception with its reason in this ticket, under ticket 26's rule. Depth is not dropped
- [ ] One body run of an hour, outside the suite, as evidence in the PR, with each breathing conserved quantity's amount kept at every breath edge as 11b's hour run keeps it. The PR says what the run shows: carbon dioxide's rise in each ten minutes, whether it has settled by the hour, and whether the drive scale swings
- [ ] The pins that move, and why for each: the model hash, the packet, the admitted seal, the first second and the first minute, as each one moves
- [ ] From [ticket 11](11-breathing-is-controlled.md): the lining's loss follows the controlled breath, its depth as well as its rate. Today the lining reads the alveolar ventilation the breath is fitted to, a packet parameter, so the loss stays at the fitted breath's when the controller moves the breath. Ticket 11's candidate was the drive's scale wired beside the mixing share on arrow 86, which a site law can read where it cannot read a body memory. 11b measured it as its own commit, kept out of its PR: with the drive scale beside the mixing share on arrow 86 and read by the lining's law, the head's 4,079.8 M rose to 4,279.1 M, +199.3 M at the same steps and solves, and the largest block grew from 651 to 659 rows, since the read joins eight rows to it, the same cause as 10d's first figure. The lining is also a rest law, so the held run's admission moved off the controlled run's. The loss needs a read that joins no block, which [native-contract 89](../../native-contract/issues/89-the-eliminations-order-inside-its-largest-block.md)'s order or a firing that sets a memory ([native-contract 87](../../native-contract/issues/87-a-firing-sets-a-memory-to-a-kernels-value.md)) may give (inferred). Once this ticket moves depth, a read of the rate alone would leave the loss off the breath, so the box sits here. The gas's water share at its vapour limit is not this box, and [ticket 12](12-heat-balance.md) owns holding it there, by its box from ticket 10: native-contract 89 records the gap and leaves the change to a later ticket
- [ ] From [ticket 11](11-breathing-is-controlled.md): the sleep law takes its better-conditioned form, each arm's gain times `1 - L s`. It is exact awake, so the awake bits stay, and the sleeping pins move here once, with depth's. Today's form, the gain less the gain times `L s`, loses the share it keeps when `1 - L` is within rounding of zero. The sleep law's test below fails at main: with a central gain of `48 / f64::EPSILON` and a gain's fall of `1 - f64::EPSILON` the drive scale is 2.47365 at 3.01 s against its form's 2.10524, with 0.10247 allowed. The body's synthetic fall is 0.4, research 22 has cited none yet, and the failure is shown at the fall and gain the test states. The test passes after the change, and the asleep scale's bits are pinned then

The sleep law's test, from the break-it pass on nightingale#214, for `crates/ng-validate/tests/chemoreflex.rs`. It copies its setup from `awake_the_wake_drive_and_the_gains_fall_leave_every_bit_of_the_drive_scale`:

```rust
/// Item 2: "With the input at one, the rest falls by `W`, and each arm's gain falls by the share `L`.
/// Each holds on its own: with no deviation at either sensor the scale is `rest - W`, and with `W` at zero
/// each arm is `(1 - L)` times its awake value."
#[test]
fn sleep_keeps_the_stated_share_of_the_central_gain() -> Result<(), InitializationFailure> {
    let unit = Case {
        gains: [48.0 / f64::EPSILON, 0.0],
        steps: Steps {
            saturation: 0.01,
            acidity: 0.01,
            ..STILL
        },
        sleep: Sleeping {
            wake: 0.0,
            fall: 1.0 - f64::EPSILON,
            ..ASLEEP
        },
        ..SENSED
    };
    let core = ng_solver::Core::new(SECOND / 100);
    let samples = trace(unit, &core)?;
    let (_, after) = either_side(&samples, unit.opens);
    let expected = scale_form(unit, &after);
    let tolerance = core.resolution().scales().at(after.scale, 1.0) + 1e-12;
    assert!(
        (after.scale - expected).abs() <= tolerance,
        "at {} s the drive scale is {}, its form {expected}, tolerance {tolerance}",
        after.at,
        after.scale
    );
    Ok(())
}
```

**Non-goals:** sleep, which is 11c, but for the form its law takes (the box from ticket 11); the gains' sourced values, which are [research 22](../../research/issues/22-the-chemoreflex-gains-awake-and-asleep.md)'s; and periodic breathing.

## Parameters with no source

Every value is synthetic. The split between depth and rate, and what bounds depth as the drive rises, are research 23's. The gains, and the cited steady change in arterial carbon dioxide that sizes the whole loop, are research 22's. This ticket judges nothing against that figure, and binding the gains waits on research 22.
