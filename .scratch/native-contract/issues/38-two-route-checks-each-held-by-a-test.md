# 38: Two route checks, each held by a test and a mutant row

Spec: `crates/ng-contract/src/compile.rs` (`Reaches::of` and `place`), and [body ticket 04](../../body/issues/04-scopes-ports-and-census.md)'s Answer

Type: task

Status: open

Blocked by: none

The check of nightingale#138, body ticket 04's part 3b, swept mutants of its own over the route checks. Two of them survive `cargo test -p ng-contract`. Neither is a rule part 3b added, and the code is right today, but no test fails when either check breaks. The check's rows are in `~/lanes/logs/ng-body-wave-3/04-scopes-ports-census/verify-coordinator/pr138-r2-scratch/verifier-rows-r2.toml`, and its sweep beside them (`job-sweep.log`) marks both SURVIVED.

- **A route stands where only the pool it enters is.** `Reaches::of` keeps a reach only where each of its two ends is a pool the schema holds or a receipt outside the body. The row `a-route-stands-where-both-its-pools-are` holds a missing pool at the end a route enters: HC's iron crossing at a bed whose cells hold no iron. No test holds a missing pool at the end a route leaves. The check's row `r2-a-reach-needs-only-its-far-pool`, in `crates/ng-contract/src/compile.rs`:

  ```toml
  old = '''reach.ends.iter().all(|end| match end {'''
  new = '''reach.ends.iter().skip(1).all(|end| match end {'''
  ```

- **A law on a route's own flow row loses its route.** `place` gives a law its route when it serves that route's family or a slot over that route, such as a valve's flow memory, and a law reads `edge_flow` only on a vascular route. No law on a route's own flow row reads `edge_flow` in this crate's tests. The slice's only law that reads it is gravity work, a transfer along its own route. The check's row `r2-a-law-on-a-flow-row-reads-no-route`, in `crates/ng-contract/src/compile.rs`:

  ```toml
  old = '''Serves::Family(family) | Serves::Slot(Over::Route(family)) => Some(family),'''
  new = '''Serves::Family(family) => Some(family),'''
  ```

**What to build:** a test that fails under each mutant, and each mutant as a row of `tools/gate/mutants.toml` that names its test.

- [ ] A small body has a route whose pool at the end it leaves is missing at one site and present at another. A test asserts that the route stands only where both its pools are, and the first row fails it
- [ ] A small body has a law on a vascular route's own flow row that reads `edge_flow`, such as a valve law that `memory_rate` attaches to `flow_memory`. A test asserts that it compiles and reads its own edge's flow, and the second row fails it
- [ ] Both rows are CAUGHT on the PR's head (`tools/mutate.py --only`)

**Non-goals:** any change to what compile accepts or refuses.
