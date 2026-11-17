# 75: An edge's law reads a bed's field at the edge's end

Spec: `crates/ng-contract/src/law.rs` (`Kernel::downstream_if_in`), `crates/ng-contract/src/compile.rs` (`addresses`), [body 09](../../body/issues/09-tissues-respire.md)'s third PR

Type: task

Status: claimed

Claimed by: nc75

Blocked by: none

Body 09's third PR makes each bed's resistance fall as its oxygen debt rises. The debt is arrow 90's field, stated on capillary beds. The resistance is the circulation's law on each vascular edge, and an edge's law reads only fields of its ends' set, which is every blood site. So the circulation cannot read a bed's debt at the bed's inflow edge, and arrow 90 cannot be wired. The same wall stands before any signal a bed sends its vessels.

**What to build:** an edge's law reads a field stated on a narrower set at the edge's downstream end. Where that end is in the set the read is the field's row there, and elsewhere it is zero with no slope. One law owns each edge's resistance, so a refusal names one law. A law that uses the read states its form so that zero off the set is what it means: for a debt zero is no debt, but for a temperature zero would be wrong.

A term written into the edge law at the edges that end in the set was built and left: it changes the placing of every law, and on a body with no bed it reads zero silently. A narrower edge set was left too, because it splits one edge's resistance across two laws on two routes.

- [ ] `Kernel::downstream_if_in` on an edge kernel reads a port over a set the edge's downstream end includes. At an end in the set it reads that site's row, and at any other end it reads zero with no slope.
- [ ] A read of a set the edge's end can never include does not compile. A law whose edges have no downstream end in the set is refused at compile by name (`EndOffSet`), never read as a constant zero.
- [ ] Only the card an arrow names as its receiving end reads that arrow's field at an edge: a compile-fail test with its declared code.
- [ ] On a fixture of two beds with different values, each fed by one edge, and edges that feed no bed: the read's values and slopes equal a handwritten twin bit for bit, with each slope in its bed's column.
- [ ] No pin moves: the slice's and the whole body's pins pass with the read in and nothing using it.
- [ ] Mutant rows, each CAUGHT: the read returns the upstream end's value; the read returns zero at a bed; the `EndOffSet` refusal is dropped. Section 7's must-fail table gains the refusal's row.
- [ ] The contract's line on what an edge's law reads says so.
