# 02: A reaction, and the books it closes

Type: prototype
Status: resolved
Claimed by: body-02-reactions
Blocked by: none

**What to build:** The law language gains a reaction: at one site, several amounts are used and made in fixed proportion. Today a binding is a paired transfer and there is no reaction, so no law can burn a fuel, make urea or buffer an acid, and the books are one for each species plus charge. A reaction turns one species into another, so the ticket must also say what stays conserved across it.

Settle the shape with two or three prototypes on the bench before the contract changes (the team contract asks for prototypes, not prose, where an experiment can answer). The candidates to compare:

- books by element, with each species declaring what it is made of
- books by declared group, where species that share a conserved part belong to one group
- books by species, with each reaction posting to a paired account at its site

- [ ] A bench body burns a synthetic fuel with oxygen into carbon dioxide, water and heat, and the certificate closes every book over a run
- [ ] A reaction whose proportions leave a declared book unbalanced is refused when the body compiles, with a must-fail row
- [ ] Every change in an amount still comes from a transfer with named ends (SRS invariant I10), and the compile-fail tests that hold it still pass
- [ ] The slice's model hash and trace do not move
- [ ] The contract and its sketch state the reaction and its books, and the glossary gains the term
- [ ] Cost: the evaluator's time a step on the bench with and without one reaction a site

**Non-goals:** any physiological reaction, rate laws beyond what the bench needs, and equilibria, which stay algebraic rows.

This changes the contract's interface, so it runs pstack `architect` with an Opus and a Codex runner and a Codex judge.

## Answer

A reaction is one column at each site where its pools exist: one rate, a whole-number term for each pool it uses or makes, and no receipt. Each species states what one unit is made of (`Species::makeup`), each body's schema states a fuel's energy content (`Schema::energy_content`), and `Assembly::compile` refuses a reaction unless its counts balance every element, charge and energy. Where a reaction joins species, the conserved quantities are carbon, hydrogen, oxygen and energy; everywhere else they are today's species, so a body with no reaction compiles to the same bytes. Contract: `docs/architecture/solver-contract.md` (section 1, section 2, the basis paragraph in section 5, the must-fail row, tradeoffs, revision r7).

- **The three candidates, on a throwaway bench** (`~/lanes/logs/ng-body-wave-1/02-reactions/bench/`, a model of the column, basis and ledger, not the engine; synthetic). Ten reactions, four right and six wrong:

  | | By element | By declared group | By species with a paired account |
  |---|---|---|---|
  | Wrong reactions that pass compile | 0 | 0 with a full list | 6: the account takes whatever the proportions leave |
  | Right reactions refused | 1: a receptor complex has no formula | 3 with a lean list that leaves oxygen and water out | 0 |
  | Compile checks it needs | each species' atoms; every element, charge and energy balances | each species' groups; every group balances; every reacting species is in a group | none it can make without a composition |
  | What a law author writes | the reaction's counts; each species' atoms once (26 numbers for the bench's species) | the counts; each group's members (27 with a full list) | the counts only |
  | Evaluator a call at 128 beds, against 17.5 us with no reaction | 25.2 us rates and residual, 48.4 us Jacobian | as element | 34.2 us and 82.7 us: five columns a reaction |
  | A carbon-making burn with the check bypassed | the carbon book opens by 0.34 of its turnover | as element | every book still closes |

- **The pick**, by pstack `architect` (Opus and Codex runners, a Codex judge, all at max; `~/lanes/logs/ng-body-wave-1/02-reactions/architect/`): books by element and energy where a reaction joins species, species elsewhere. The judge scored the Codex candidate 25 to 16. Grafts from the Opus candidate: `Proportions` with whole counts and a derived energy, the content in the schema, glucose as the fuel, one basis derivation.
- **A bench body burns a synthetic fuel and the certificate closes every book over a run.** `BurningBeds` (`crates/ng-validate/src/burning.rs`) is three capillary beds, each burning glucose with six oxygen into six carbon dioxide and six water in its interstitium, the energy into its own heat store, with boundaries for glucose and oxygen in and carbon dioxide, water and heat out. Glucose's content of 1,000 is synthetic. `every_bed_closes_carbon_hydrogen_oxygen_and_energy_while_it_burns` (`crates/ng-validate/tests/burning.rs`) admits it at an instant, runs 600 s on the reference integrator, and certifies carbon, hydrogen, oxygen and energy for each bed over every 60 s window. Each burn column moved forward.
- **A reaction that leaves a book open is refused at compile.** A burn that makes seven carbon dioxide is `UnbalancedReaction { law: MT burn, book: Element(Carbon) }` (`a_burn_that_makes_carbon_is_refused_naming_carbon`). The must-fail row is "A reaction whose proportions leave an element, charge or energy open, or written from raw stoichiometry", held by the mutant `reaction-balanced`, caught, and the hand-made `Proportions` E0451 doctest. Five more mutants are caught: the energy check skipped, the span check skipped, an admission total put on its least row, a quantity kept whole, and the heat store found off the tissue site.
- **I10.** A reaction's two ends are what it uses and what it makes, each a named pool at its site, holding the same count of every element, the same charge and the same energy. A forcing still only scales a rate. The compile-fail tests that hold I10 pass, and new ones hold the reaction: hand-made `Proportions` or `Reaction` (E0451), energy written as a term and an ungranted card (E0277), a rate in another species (E0308), a count of zero (E0080).
- **The slice's model hash and trace do not move.** `tests/trace.rs` now pins the slice's model hash `8e461394...e635ea` and conserved basis `1e5235d9...189b25`, recorded on main before any change. Both pins and both first-second traces pass with no `UPDATE_EXPECT`.
- **The contract, its sketch and the glossary state the reaction.** The sketch replaces `BalancedReaction` with `Proportions` and `Reaction`, and binds by transfer as the crate does. The glossary gains Reaction and Energy content.
- **Cost on the engine** (`~/lanes/logs/ng-body-wave-1/02-reactions/cost/run-2.md`; build machine, release; sides alternate, nine rounds each, median and range). The three beds and their twin with no reaction hold the same 15 rows. With 15 columns and no reaction, one stage (every rate, residual and the Jacobian) takes 1,962 ns (1,944 to 1,982). With one burn a bed, 18 columns, it takes 2,401 ns (2,379 to 2,462), about 146 ns a reaction. An accepted step on the reference integrator takes 59.7 us (58.9 to 63.4) and 66.3 us (65.0 to 68.5). The limiter is inferred, not profiled: each burn adds a nine-operation kernel and ten Jacobian entries.
- **Not in this ticket, and flagged.** The history counts any column's imbalance as exchange, so only the compile check guards an unbalanced reaction; counting exchange on receipts alone needs a change in `crates/ng-record`. A held route on a reacting pool names only the first quantity that holds it, which matters when a held card first supplies glucose (ticket 13). No cycle admission of a reacting body has run; the totals plan is tested on its rows alone. The teacher driver's catalogue gained two names for the new quantities.
- **For ticket 03:** each new species needs its makeup (atoms, or carried) and the elements it needs, nitrogen first; a fuel's content is a body's schema value in whole energy units per amount unit; a lump needs a fixed formula. Water's stability moves to the healthy-range check once a reaction makes water.
