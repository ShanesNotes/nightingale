# 92: A junction names every pool its transmitter's laws need

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 1, `crates/ng-contract/src/compile.rs` (the route filter) and `crates/ng-contract/src/failure.rs` (`CompileDefect::LeftOutTouched`)

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) runs a whole body whose junctions the schema declares one pool at a time, and a refusal should name what is wrong. Since [body 11](../../body/issues/11-breathing-is-controlled.md)'s first PR the respiratory pump's junction holds acetylcholine in vesicle, cleft and bound. A schema that declares the cleft and the bound pool there and leaves out the vesicle is refused, but only through the packet's unused keys. The compiler drops release and reuptake where the vesicle is missing, since the route filter keeps only the reaches whose ends are declared pools. Binding still moves acetylcholine between cleft and bound, so neither pool is an orphan and the receptor still gives the pump its force. Then the packet's release keys go unused, and binding the packet refuses the first of them: `UnknownParameter` for `ns.motor-share` at `Class(Junction)`. A packet that leaves those keys out would compile and admit with nothing that names the missing pool (inferred, not run). The sinoatrial junction is the same on main.

A run at main `446e29f9` shows it. The test below fails there with `the missing vesicle at respiratory pump junction was reported as UnknownParameter { key: ParameterKey { key: "ns.motor-share" }, binding: Class(Junction) }`.

**Wanted.** Compile refuses a junction that holds some of a transmitter's pools and not one that a law of that transmitter moves there, with `CompileDefect::LeftOutTouched` naming the law, the site and the missing space.

- [ ] The test below is red before the change and green after it, in `crates/ng-validate/tests/pump_junction.rs`
- [ ] The same refusal for the sinoatrial junction without its vesicle, on a fixture of the slice
- [ ] A junction that declares none of a transmitter's pools still compiles, so a junction with no acetylcholine stays legal
- [ ] A mutant row that puts back the silent drop, CAUGHT by the test
- [ ] No pin moves: every body that compiles on main compiles to the same model hash

The test. Its helper takes the pump's spaces, so the control keeps all three and the test leaves out the vesicle:

```rust
fn compiled() -> Result<CompiledBody, AdmissionError> {
    compiled_with_pump_spaces(&[Space::Vesicle, Space::Cleft, Space::Bound])
}

fn compiled_with_pump_spaces(spaces: &[Space]) -> Result<CompiledBody, AdmissionError> {
    // as `compiled` on main, with `for &space in spaces` in place of the three spaces
}

#[test]
fn a_pump_without_a_vesicle_names_the_missing_release_pool() -> Result<(), InitializationFailure> {
    run(HELD, |_, _| ())?;
    let result = compiled_with_pump_spaces(&[Space::Cleft, Space::Bound])
        .and_then(|body| body.bind(packet(&body, HELD)?));
    match result {
        Err(AdmissionError::Compile(CompileDefect::LeftOutTouched { law, site, space })) => {
            assert_eq!(site.key(), JUNCTION);
            assert_eq!(space, Space::Vesicle);
            assert_eq!(law.owner, Nervous::CARD);
            assert!(matches!(
                law.local,
                "release acetylcholine" | "reuptake acetylcholine"
            ));
        }
        Err(other) => panic!("the missing vesicle at {JUNCTION} was reported as {other:?}"),
        Ok(_) => panic!("bound the pump without its acetylcholine vesicle"),
    }
    Ok(())
}
```

**Non-goals:** which nerve serves a junction, which is [native-contract 90](90-a-release-reads-the-nerve-that-serves-its-junction.md), and a junction's transmitter amounts, which [native-contract 93](93-the-receptors-effect-stays-in-its-unit-interval.md) bounds.
