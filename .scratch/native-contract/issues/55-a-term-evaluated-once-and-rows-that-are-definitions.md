# 55: A term evaluated once for all the laws that add it, and the algebraic rows that are plain definitions

Spec: the evaluator in `crates/ng-contract` and the site term of [native-contract 42](42-veins-collapse-branch.md), part 2

Type: prototype

Status: open

Blocked by: 42

**Why.** The first milestone (SRS invariant I19) runs the healthy reference man for 48 hours, and how long that run takes rests on what a step costs. Native-contract 42 places a site term by writing it into each law that adds it, so a term three laws add is computed three times. Evaluating it once and feeding each law would cost less, but it changes every evaluation path in the crate that every run's bits depend on, so it was not built.

**Where it comes from.** The synthesis of native-contract 42's two design candidates (`~/lanes/logs/ng-wave-4/nc42-judge/SYNTHESIS.md`, the part on shared evaluation) rejected shared evaluation for now: it was not measured, and it changes every evaluation path. It made it this ticket, behind the same surface a law's author uses for a term, with the prototype that one candidate and the judge named. The written term measured +2.4% of seated step time with three veins. The synthesis keeps 2% of seated step time as the bar for this ticket.

The same synthesis asks for a count. In one candidate's measurement, three added algebraic rows cost 2% of a step and 37% of admission's instructions. An algebraic row that only defines a value from others may leave the solved system, and that may be worth far more than one term.

- [ ] A prototype of shared evaluation: four private producer instances feeding the three host laws, with derivatives in the unknowns and the parameters, and the rows and sparsity unchanged. Its derivatives equal the written term's, bit for bit
- [ ] Its cost against the written term, as seated step time on one named build worker under the same other load, held to the 2% bar
- [ ] A count of how many of the slice's and the whole body's algebraic rows are plain definitions
- [ ] The Answer says whether shared evaluation is built, and whether the definitions leave the solved system, each as its own ticket if so

**Non-goals:** building shared evaluation in this ticket. Changing a term's surface for a law's author. The three-halves power, which is [native-contract 54](54-three-halves-power-as-ratio-times-square-root.md).
