# 32: The cost line names its build

Type: task
Status: resolved
Blocked by: 26

**What to build:** [Ticket 26](26-cost-budget.md) holds each loop to a budget read from the [cost line](../../../GLOSSARY.md), and a line shows a budget met only if it says what was built. The line marks a tree with changed tracked paths `-dirty`. It does not see the build's environment. In the check of nightingale#167, on a clean tree, `CARGO_PROFILE_RELEASE_OPT_LEVEL=1` printed a bare head and 1,392.3 M user instructions a simulated second for the slice, against 896.5 M (one run each on the beelink). Close that before a loop PR quotes a line.

- [x] `tools/bench/run.py cost` prints no unmarked line from a build its tree does not fix. It clears or refuses the cargo and rustc variables that change a release build, and a cargo config outside the tree or untracked inside it. The PR shows each case it covers by a run
- [x] [The map](../map.md)'s rule and ticket 26's Answer say what a marked or refused line means, in one sentence
- [x] The line's `span_s` and the bench's `span_ns` take another name. The glossary's **Span** is what a solver core commits to before a ticket, and these give a run's length
- [x] Between clones the slice's figure moved 100 parts in a million: 896.5 M through `run.py` in one clone, 896.6 M in another and 896.65 M from perf on the same binary, while repeats within one way of running agree to 1.3. Say why, or state in ticket 26's Answer what the figure reproduces to between clones

**Non-goals:** the budget's figures and the four count ceilings. A perf step in the gate, which ticket 26 ruled out.

## Answer

- **Environment and Cargo config refusal (`tools/bench/run.py`).** `cost` calls `check_clean_build_env` before building. Any `CARGO*` or `RUST*` variable outside `SAFE_BUILD_ENV` (`CARGO_HOME`, `CARGO_TARGET_DIR`, `CARGO_TERM_COLOR`, `CARGO_TERM_VERBOSE`, `CARGO_TERM_QUIET`, `CARGO_TERM_PROGRESS_WHEN`, `CARGO_NET_OFFLINE`, `CARGO_BUILD_JOBS`, `RUSTUP_HOME`, `RUST_BACKTRACE`, `RUST_LOG`) is refused with exit 1 and no cost line. Any `.cargo/config` or `.cargo/config.toml` in `tools/bench` or any ancestor directory up to `/`, or `config` / `config.toml` inside `$CARGO_HOME` (or `~/.cargo`), is refused with exit 1 and no cost line. `tree_is_dirty` checks `git status --porcelain -z --untracked-files=all` and `git ls-files -z -v` (catching staged, unstaged, untracked and `assume-unchanged`/`skip-worktree` paths) so any deviation from `HEAD` marks `head=<sha>-dirty`.
- **Cases verified by run:** `CARGO_PROFILE_RELEASE_OPT_LEVEL=1`, `RUSTFLAGS=-Ctarget-cpu=native`, `CARGO_ENCODED_RUSTFLAGS=foo`, `RUSTUP_TOOLCHAIN=stable`, `RUSTC_WRAPPER=sccache`, `CARGO_INCREMENTAL=1`, `CARGO_BUILD_RUSTFLAGS=-Copt-level=0`, `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS=-g`, `$CARGO_HOME/config.toml`, untracked `.cargo/config.toml` and untracked `tools/bench/.cargo/config.toml` each exit 1 with empty stdout and a refusal naming the variable or config path.
- **Glossary alignment (`tools/bench/run.py`, `tools/bench/src/main.rs`).** Renamed the cost line field `span_s` (and `COST_SPAN_S`) to `simulated_s` (`COST_SIMULATED_S`), and `score`'s `span_ns` to `coverage_ns` (matching `protocol.windows.coverage_ns()`).
- **Clone-to-clone reproducibility.** [Ticket 26](26-cost-budget.md)'s Answer states that a cost line reproduces to about 100 parts in a million between clones (against 14 parts in a million across jobs in one clone), about 1,200 times smaller than a loop's 12% budget.
- **Map and ticket 26 rule.** Both [the body map](../map.md) and [ticket 26](26-cost-budget.md) state that a marked (`-dirty` or `none`) or refused run cannot show a budget met.
