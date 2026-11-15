# 32: The cost line names its build

Type: task
Status: open
Blocked by: 26

**What to build:** [Ticket 26](26-cost-budget.md) holds each loop to a budget read from the [cost line](../../../GLOSSARY.md), and a line shows a budget met only if it says what was built. The line marks a tree with changed tracked paths `-dirty`. It does not see the build's environment. In the check of nightingale#167, on a clean tree, `CARGO_PROFILE_RELEASE_OPT_LEVEL=1` printed a bare head and 1,392.3 M user instructions a simulated second for the slice, against 896.5 M (one run each on the beelink). Close that before a loop PR quotes a line.

- [ ] `tools/bench/run.py cost` prints no unmarked line from a build its tree does not fix. It clears or refuses the cargo and rustc variables that change a release build, and a cargo config outside the tree or untracked inside it. The PR shows each case it covers by a run
- [ ] [The map](../map.md)'s rule and ticket 26's Answer say what a marked or refused line means, in one sentence
- [ ] The line's `span_s` and the bench's `span_ns` take another name. The glossary's **Span** is what a solver core commits to before a ticket, and these give a run's length
- [ ] Between clones the slice's figure moved 100 parts in a million: 896.5 M through `run.py` in one clone, 896.6 M in another and 896.65 M from perf on the same binary, while repeats within one way of running agree to 1.3. Say why, or state in ticket 26's Answer what the figure reproduces to between clones

**Non-goals:** the budget's figures and the four count ceilings. A perf step in the gate, which ticket 26 ruled out.
