# 102: The skin's reads, a design round

From: [body ticket 12](../../body/issues/12-heat-balance.md)'s plan, its first design question

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 12](../../body/issues/12-heat-balance.md)'s PR 12b, the thermostat's commands and the skin's vessels, and not its first two PRs, 12a and 12e

**Why.** Body ticket 12's thermostat must narrow and widen the skin's vessels, carry heat to the skin at the skin bed's actual flow, and make sweat where the skin junction is. In the plan's prototype no command reached a vessel: its carriage coefficient sat inside MT's conduction law, so the warm room showed no order of flow before sweat. The question is which anatomical relations and typed reads give each owner the skin bed's actual flow, both temperatures and the junction's effect. Each need below is refused on main at the line named, as the plan found and a second read confirmed.

- **Blood carriage to the skin.** Carriage pairs the core store with the heat store of the bed's parent and drops the pair when the two are the same (`crates/ng-contract/src/compile.rs:1192-1197`), and the skin bed and the skin heat site are both contained directly by the body (`crates/ng-validate/src/body/anatomy.rs:86,91,98`). A flow read outside an edge context compiles to `NoSuchRead` (`crates/ng-contract/src/compile.rs:2296,2351-2358`), and `inflow()` is the urinary edge's (`crates/ng-contract/src/law.rs:1407-1409`). The skin bed has one arterial inflow (`crates/ng-validate/src/body/anatomy.rs:149-151`).
- **Temperature readers.** NS reads skin temperature at its thermoreceptor, and CV reads both ends' temperatures. Arrow 91 is wired with `CoreTemperature { core_temperature }` alone, and arrow 90's `temperature` is held because "CV's laws run at a reference temperature" (`crates/ng-contract/src/catalog.rs:482-486`). An amount read resolves at the kernel's context (`crates/ng-contract/src/compile.rs:2211`), although the map places the skin thermoreceptor at the skin heat site (`crates/ng-validate/src/body/anatomy.rs:210`).
- **Effectors.** Sweat is computed where the junction is, and MT reads a receptor's effect. A boundary law that does not drain is placed at the body's context (`crates/ng-contract/src/compile.rs:1436-1439`), and a junction field read demands a site or edge context (`crates/ng-contract/src/compile.rs:2179-2182`). The bridge is `only_at` (`crates/ng-contract/src/law.rs:2494-2496`), and only CV has `effect` (`crates/ng-contract/src/scope.rs:651-653`).

**The options the plan lists, each with its cost.**

- The relation. (A) Reparent the skin bed under the skin heat site, which changes one line of the validation anatomy and gives a skin carriage column with no compiler change, at the cost of rechecking every site set and every read down the body's children. The glossary already says carriage is the heat blood carries into the store a bed sits in, which this makes true. (B) Declare an explicit relation from bed to store and leave containment alone, at the cost of a new relation in `ng-contract` and a census row.
- The flow. A new site kernel read of the bed's one arterial inflow, checked as `inflow()` is. Or CV computes the carriage inside its own edge kernel, where `edge_flow()` already resolves, which puts a heat law in CV and needs both temperatures there.
- The temperatures. Wire arrow 90's held `temperature` for the sites CV needs, which moves the census and gives CV the skin's temperature for the local factor on the arteriole's resistance (research 12 line 37). Or CV derives them from its own carriage ends with one capacity and reference binding, which puts two cards on one packet key. For NS, a fix of the landmark lookup, then either a skin field on arrow 91 or a field sampled at a landmark at each site.
- The junction's effect: MT's twin of CV's `effect`, the only option found.

**Method.** pstack architect, with a prototype where a run can settle a fork, such as whether the reparented skin bed leaves every site set and read as it was.

- [ ] A design is accepted, with its record
- [ ] The contract PR is merged alone, with its census rows and a compile-fail test for each type rule it adds
- [ ] Body ticket 12's 12b is unblocked

**Non-goals:** evaporation's form and the thermal zero, both ruled in body ticket 12, and any law.
