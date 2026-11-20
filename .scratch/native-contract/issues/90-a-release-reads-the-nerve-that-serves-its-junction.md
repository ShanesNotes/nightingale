# 90: A release reads the nerve that serves its junction

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 1, `crates/ng-contract/src/scope.rs` (`NervousPorts::release`) and the coupling map's arrow 88

Type: prototype

Status: open

Blocked by: none

Blocks: none

**Why.** Cost and the body's anatomy both bound the first milestone (SRS invariant I19). Since [body 11](../../body/issues/11-breathing-is-controlled.md)'s first PR the whole body releases acetylcholine at every junction that holds it by two traffics in that junction's shares, `q_i (s_v f_v + s_m a)`: the vagal traffic `f_v` and the pump drive `a`. Each junction's shares are 1 and 0, or 0 and 1. Three things follow.

- Which nerve serves a junction is anatomy, but the shares state it in packet values. A packet can state a release that anatomy forbids, such as the vagal traffic at the respiratory pump's junction, and only tests refuse it.
- Every junction's release reads both traffics in the sparsity, whatever its shares. The vagal traffic reads the circulation's stretch through the baroreflex, the pump junction's release reads the vagal traffic, and the pump's force reaches the circulation through the pleural pressure, so a loop closes and the pump's chain joins the circulation's block: the largest block grows from 644 to 651 rows. The seven rows that joined are the vagal traffic, the pump junction's acetylcholine in vesicle, cleft and bound, its receptor effect, the pump force and the pleural space's pressure. Body 11a measured 4,048.7 M instructions a simulated second at its base and 4,049.5 M at its head, back to back on the beelink, and about 5.9 M over the base at the base's solve count (inferred).
- A fixture with no baroreflex, such as `crates/ng-validate/tests/pump_junction.rs`, states a vagal traffic that nothing uses.

**Wanted.** A release law reads the one traffic of the nerve that serves its junction, as arrow 88's relation says (release at the respiratory pump's junction). The two share parameters, `ns.vagal-share` and `ns.motor-share`, leave the packet. The largest block returns to what the wiring gives it.

**Candidates, neither ruled.**

- A release port by the nerve that serves the junction, so a law releases only where its nerve reaches.
- A junction kind of its own for the respiratory pump.

Each is a contract interface, so the ticket starts with a design round. It is weighed with [native-contract 89](89-the-eliminations-order-inside-its-largest-block.md), which works inside the same block, and it best lands with a PR that moves the body's pins anyway.

- [ ] The design round: the form, what compile refuses, and what the run's identity covers
- [ ] No share parameter in the packet
- [ ] The pinned largest block in `crates/ng-validate/tests/body.rs` is smaller by the pump's chain
- [ ] The mutant rows that hold the release's wiring still read `CAUGHT`
- [ ] `pump_junction.rs` states no vagal traffic

**Non-goals:** a second nerve at one junction, and the norepinephrine release, which reads one traffic today.
