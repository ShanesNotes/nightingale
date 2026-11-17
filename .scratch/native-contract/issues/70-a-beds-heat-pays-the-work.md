# 70: A bed's heat pays the heart's and the walls' work

Spec: `docs/architecture/solver-contract.md` (section 1, the closed lists), `crates/ng-contract/src/compile.rs` (`reaches_of`)

Type: task

Status: resolved

Claimed by: nc70-build

Blocked by: none

On main the heart's stroke work and the walls' work start at the receipt `chemical energy`. Compile builds those ends whatever the body holds, so no body law can move the payer, and [body ticket 09](../../body/issues/09-tissues-respire.md)'s last PR, 09d, which wires coupling arrow 78, cannot be built. The whole body has held a heat store at every bed since the burn landed, so the payer cannot be read off the pools: the body names it. The schema names, for each work, the capillary bed whose heat pays it, as it names the site of a boundary, because composition data may name a site and a law never does. Compile then builds that work's far end at the bed's thermal pool. With no bed named the end stays the receipt, so neither the slice nor the whole body moves a pin. Which bed pays in the whole body, and the burn that rises with the work, are 09d's.

- [x] `Work` (`Stroke`, `Walls`) and `Schema::pay_work(work, bed)`. It refuses a site the map does not hold (`SiteOffTheMap`), a site that is no capillary bed (`PayerNotABed`) and a second bed for one work (`WorkPaidTwice`). The schema's seal writes the payers only when one is named, after a tag no other field writes, as it does the energy contents, so no other body's bytes move
- [x] Compile builds stroke work at each blood site, and the walls' work, from the named bed's thermal pool to the elastic store. It refuses a named bed whose thermal pool the schema does not hold (`PayerHoldsNoHeat`), because a route whose pool is missing would otherwise drop out unseen
- [x] Tests: the work columns of a schema that names a bed for each work, their ends and no receipt; each refusal above; the slice's model hash and conserved basis, and the whole body's rows, columns and digests, unchanged with no edit to either pin
- [x] Run, on a one-bed fixture with synthetic values: with the bed's burn raised by the work, heat to the core store equals the burn's energy less the work, at rest and at twice the work, energy closes over every window and the receipt takes nothing. With the burn not raised, the bed's heat settles lower by the work over the heat's speed. Work past what the bed can burn ends in `NoAdmissibleStep` whose refusal names the bed's thermal pool, never a store below zero
- [x] Section 7's must-fail table gains a row: a body that names a bed to pay its work while compile still ends the work at the receipt. Its `must-fail.toml` entry names the route test and the fixture run, through the mutant row `work-paid-from-outside` (the named work's reach returns to the receipt). A second row, `a-payer-without-heat-compiles` (the payer check passes every bed), must fail the refusal test
- [x] The contract's sentence on heat routes says stroke and wall work draw on the receipt unless the schema names a bed to pay them. The glossary gains **Paying bed**, and **Wall power** and **Bed heat** say who pays. The ports' comments on `stroke_work`, `wall_work` and `pump_work` say the same
- [x] Cost: the PR moves no pin, so its line is main's, and it says so

**Non-goals:** which bed pays in the whole body, MT's read of arrow 78 and the slice's posture checks (body 09d); the pumps' work, which keeps its receipt until [body ticket 17](../../body/issues/17-volume-hormones.md); an efficiency of muscle work, which no source on hand gives, so the work's energy is the fuel's energy it draws.

From the plan's prototypes: the work drawn as a burn of its own at the bed, its energy into the elastic store, also closed energy on the fixture. It needs a new kind of column that the reaction rules do not see (one chemistry with two owners at one bed, its energy at another site), and its oxygen escapes the bed's bend, so work past what the bed can burn was refused at the cells' oxygen and not at the paying bed's heat. The heat route is the smaller change.

## Answer

The schema names the payer for each work: `Work` (`Stroke`, `Walls`) and `Schema::pay_work(work, bed)`, which refuses `SiteOffTheMap`, `PayerNotABed` and `WorkPaidTwice`. Compile starts a named work's route at the bed's thermal pool, two terms and no receipt, and refuses a named bed with no thermal pool (`PayerHoldsNoHeat`). With none named the work starts at the receipt `chemical energy`, as before. The schema's seal writes the payers only when one is named, after the tag `u64::MAX - 1`, so the slice's and the whole body's pins pass with no edit.

Tests: `crates/ng-contract/tests/routes.rs` holds the work columns with and without payers, the four refusals, and a payer that reaches the model hash where it moves no column. The one-bed fixture `crates/ng-validate/tests/paying.rs` runs a test burn of its own, every value synthetic. With the burn raised by the work, the heat to the core store is the burn's energy less the work at 2 and at 4, and the receipt takes nothing. With the burn not raised, the bed's heat settles 40 and 80 lower, the work over the heat's speed of 0.05. Work of 40 ends in `NoAdmissibleStep` whose refusal names the bed's thermal pool, with no store below zero. The mutant rows `work-paid-from-outside`, `work-paid-from-outside-in-the-run` and `a-payer-without-heat-compiles` are caught, and section 7's must-fail table holds the row.
