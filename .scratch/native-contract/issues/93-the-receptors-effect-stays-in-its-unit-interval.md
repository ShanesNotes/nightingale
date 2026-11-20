# 93: The receptor's effect stays in its unit interval

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4 (admission), `crates/ng-receptor/src/shared.rs` (the effect law) and [research 02](../../research/issues/02-law-forms.md)'s row for the receptor effect

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) admits the whole body from its packet, and a start the laws cannot hold should be refused at admission, not run. The receptor's effect is `E = eps B / (R - B + eps B)` for the bound amount `B` at `R` sites, written in amounts. Research 02's row says `E` runs from 0 to 1, and the respiratory pump's pressure is `PImax E`, so `PImax` is the pressure at full effect. Instant admission holds the packet's amounts, and the bound pool is checked only for nonnegativity. Once `B` passes `R`, `E` leaves its unit interval: it passes 1, and where `eps` is under 1 it has a pole at `B = R / (1 - eps)` and is negative past it. The law is main's, and it serves the sinoatrial junction as well as the pump's.

A run at main `446e29f9` shows it. A start with two bound on one receptor site at the pump's junction admits with effect 2 and pump force 14.4 at `PImax` 7.2. The test below fails there with `admitted 2 bound at respiratory pump junction with 1 receptor site: effect 2, pump force 14.4, PImax 7.2`.

**Wanted.** Admission refuses the start and names what is wrong: the bound amount, the receptor's capacity or the receptor's law. The test accepts any of the four refusals it lists.

**The slice's pins.** The receptor card is a law of the slice, so a changed declaration moves the slice's model hash. The fix either holds the slice's model hash, packet, seals and pins as they are, and says how, such as a check at admission that is no part of the declaration, or moves them once in this PR with the reason for each.

- [ ] The test below is red before the change and green after it, in `crates/ng-validate/tests/alveolar.rs`
- [ ] The same refusal at the sinoatrial junction, on a fixture of the slice
- [ ] A bound amount equal to the sites admits, with effect 1
- [ ] A mutant row that removes the refusal, CAUGHT by the test
- [ ] The slice's pins stay, or move once with the reason, as said above

The test. The fixture's `Case` gains `bound_from_vesicle`, zero in `AT_REST`, which the packet moves from the vesicle's amount to the bound one:

```rust
#[test]
fn the_pump_refuses_a_bound_amount_above_its_receptor_capacity() -> Result<(), InitializationFailure> {
    let held = Case {
        pump: 7.2,
        rest_drive: 0.5,
        ..AT_REST
    };
    let junction = site::<Junction>(JUNCTION)?;
    let bound = RowKind::Amount {
        site: junction,
        species: Species::Acetylcholine,
        space: Space::Bound,
    };
    let (bound_row, effect, force) = run(held, 0, |body, session| {
        let effect = algebraic(body, "effect acetylcholine");
        let force = algebraic(body, "pump_force");
        assert!(close(now(&session, force), rest_force(held), 1e-9));
        (row(body, bound), effect, force)
    })?;
    let overfilled = Case {
        bound_from_vesicle: 2.0 * SITES - steady(held.rest_drive)[2],
        ..held
    };
    match run(overfilled, 0, |_, session| {
        (
            now(&session, bound_row),
            now(&session, effect),
            now(&session, force),
        )
    }) {
        Ok((amount, effect, force)) => panic!(
            "admitted {amount} bound at {JUNCTION} with {SITES} receptor site: effect {effect}, pump force {force}, PImax {}",
            held.pump
        ),
        Err(InitializationFailure::Admission(AdmissionError::StartOutsideSet { row })) if row == bound => {}
        Err(InitializationFailure::Inconsistent {
            row: AdmissionRow::Body(row),
        }) if row == bound_row || row == effect => {}
        Err(InitializationFailure::Admission(AdmissionError::ParameterOutsideSet {
            key,
            binding: Binding::Site(site),
        })) if site == junction && key == ParameterKey::new("rc.ach-sites") => {}
        Err(InitializationFailure::Undefined { law })
            if law.owner == Receptor::CARD
                && matches!(law.local, "binding acetylcholine" | "effect acetylcholine") => {}
        Err(other) => return Err(other),
    }
    Ok(())
}
```

**Non-goals:** the receptor's form and its efficacy's value, which research 02 and [research 24](../../research/issues/24-the-pump-junctions-transmission.md) hold, and the pools a junction declares, which is [native-contract 92](92-a-junction-names-every-pool-its-transmitters-laws-need.md).
