# 97: Two PRs that add mutant rows merge without a conflict

Spec: `tools/gate/mutants.toml`, `tools/mutate.py` and `tools/gate/gate.py` (the mutant sweep and `seam.must-fail`), and `tools/gate/must-fail.toml`

Type: task

Status: resolved

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) is held by its gate, and the gate's mutant sweep is how a test that once missed a mistake keeps seeing it. Each code PR adds the rows for its own rules, and the gate's throughput limits how many tickets merge in a day.

**What the merges of nightingale#210 to nightingale#214 showed.** Every row lives in one file, `tools/gate/mutants.toml`, 659 rows at main, and code PRs add theirs at its end, or beside the rows of the code they touch. Two PRs in flight conflict there when both append at the end of the table: nightingale#213 did against nightingale#211. Nightingale#214 did not, because its kickoff placed its rows above an earlier ticket's first row. Each conflict costs a merge of main and a new proof, and a full `tools/verify` took 2,158 to 4,135 s on the beelink in [body ticket 26](../../body/issues/26-cost-budget.md)'s records.

Candidates, not picked: one row file for each crate, or a folder of row files that the gate reads in a stated order.

- [x] Two scratch branches from one base that each append rows at the end of the table conflict today and merge with no conflict after the change
- [x] `tools/mutate.py --check` and the sweep read the same rows in the same order as today
- [x] `seam.must-fail` still finds each row `tools/gate/must-fail.toml` names
- [x] The repo's `AGENTS.md` and the `verify-nightingale` skill name the new place, and `tools/verify.d/40-mutants.sh` replays a changed row as it does today

**Non-goals:** which rows the gate keeps, a row's budget ([native-contract 84](84-a-mutant-rows-budget-follows-its-tests-own-time.md)), and the order the sweep runs its parts in.

## Answer

- **Layout:** Moved `tools/gate/mutants.toml` byte-for-byte to `tools/gate/mutants.d/00-base.toml` (all 707 rows at `9f5ba6ce`, exact order and git history preserved). New PRs add their own `tools/gate/mutants.d/<ticket-slug>.toml` file, so two PRs in flight never touch the same file even when mutating the same crate (86.6% of existing rows touch `ng-contract` or `ng-validate`, so per-crate files would still conflict).
- **Reader and gate (`tools/mutate.py`, `tools/gate/gate.py`):** `load_rows` accepts either a single `.toml` file (preserving hand-run ticket tables) or a directory of `*.toml` files read in sorted filename order, refusing empty directories, non-`.toml` entries, empty files, and duplicate row names across files. `gate.py`'s `MUTANTS` points to `tools/gate/mutants.d`, `mutant_tests` and `_copy` read all `*.toml` files under it, and `rows_at(rev)` reads `tools/gate/mutants.d/*.toml` with fallback to legacy `tools/gate/mutants.toml` so diffs against pre-move bases compare exact row parity (0 rows falsely marked new).
- **Verification:** `mutants_dir_case` in `gate.py` (`DRIVER_CASES`, run by `05-seam.sh`) merges two sibling branches that each add a file under `tools/gate/mutants.d/` with zero conflicts and checks order, `--check`, duplicate-name refusal, non-`.toml` refusal and empty-directory refusal. `tools/mutate.py tools/gate/mutants.d --check` verifies all 707 rows apply in exact order; `seam.must-fail` pairs all 55 rows; `AGENTS.md` (60 lines), `.agents/skills/verify-nightingale/SKILL.md`, `tools/gate/must-fail.toml` and `docs/adr/0011` name `tools/gate/mutants.d/`.
