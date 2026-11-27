# 15: Nothing in the gate sees the step path get slower

Type: task
Status: resolved
Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and how long that run takes rests on what a step costs. A change can make every step slower without changing a bit, and today no test and no gate step would see it. The speed aim of at least 10 times real time on the full 13-system body ([SRS section 6](../../../docs/srs.md#6-fidelity-aims)) is not yet met on the whole body, so a slower step path that lands unseen widens the gap.

**What happened.** The plan for part 2 of [native-contract 42](../../native-contract/issues/42-veins-collapse-branch.md) (`~/lanes/logs/ng-wave-4/nc42-pr2/PLAN.md`, the paragraph on its void first pass and its flag on ruling 3) added a helper on the refusal path, off the step path. In its first form the helper called two evaluator functions directly, so the compiler stopped inlining them into the step's own evaluation. A build with no vein marked, on main's model, ran 12.6% more instructions a step. No test and no gate step noticed. A profile found it during the plan's own cost measurement. The plan flags that a check should pin the no-vein build's instructions a step within a band of main's.

The boxes ask what such a check could read and where it would run. They do not design it.

- [x] What the check reads: user instructions a step of the slice, or another count. The Answer says how steady that count is from run to run and from load to load on one worker
- [x] On which named build worker it runs, and whether a reading on one worker means anything on another
- [x] What band around main's figure it holds, and what a change does when it moves the figure on purpose
- [x] Where it runs: the merge proof, the nightly run or a bench a PR runs by hand, with the time each would add
- [x] Whether the void first pass's form, put back, would trip the check that the Answer proposes

**Non-goals:** building the check. A cost budget for the whole body, which is [body 26](../../body/issues/26-cost-budget.md).

## Answer

### 1. What the check reads, and its steadiness on one worker

**Read `minstructions_a_simulated_s` of the slice from `python3 -I tools/bench/run.py cost slice`** (`tools/bench/run.py:218-261`, `tools/bench/src/main.rs:351-448`), which admits `slice` once on `ng_solver::Core::new(10 ms)` and counts `perf stat -e instructions:u` with `--control fifo` enabled only across the `COST_SIMULATED_S = 60` s stepping repeats (leaving out compile, admission, and checkpoint resume).

- **Why `minstructions_a_simulated_s` over wall time or raw per-step instructions:**
  - Wall time cannot gate anything: on `beelink` at `71c7549`, `slice` wall time moved `2.88x` (`62.4` to `179.8 ms/sim-s`) and `body` wall time moved `3.20x` (`396.9` to `1,271.3 ms/sim-s`, `6.4%` even quiet) across load ([body 26](../../body/issues/26-cost-budget.md), lines 29-55).
  - On a fixed slice, `steps` (`11,614` over 60 s) and `solves` (`14,749`) have zero spread across repeats (`tools/bench/src/main.rs:423-426` refuses any repeat whose `(steps, solves, seal)` differs), so `minstructions_a_simulated_s` is strictly proportional to user instructions per accepted step while also catching regressions that add solves or steps per simulated second at constant per-solve cost (as [native-contract 64](../../native-contract/issues/64-the-landing-reads-a-guard-that-did-not-cross.md)'s stalled landing did on the whole body: `+5.0%` instructions per simulated second at unchanged `24.6 M` per solve).
  - `run.py cost` already refuses build-altering `CARGO*`/`RUST*` variables, external Cargo configs, and toolchain mismatches, and marks dirty trees `-dirty` ([body 32](../../body/issues/32-the-cost-line-names-its-build.md)).
- **Measured steadiness on `beelink` (AMD Ryzen 7 6800H, 8 cores / 16 threads, `x86_64`, `perf_event_paranoid = 2`):**
  - *Run-to-run and load-to-load in one checkout* ([body 26](../../body/issues/26-cost-budget.md), lines 27-55; raw `perf` files in `~/lanes/logs/ng-wave-4/b26/runs/r{1..4,6..8}/` at `71c7549`, memory store, 1-min load `1.5` quiet to `18.8` beside 15 competing slice processes):
    - `slice` (`60 s x 5`, 14 runs across 7 jobs): raw `instructions:u` spanned `266,340,303,346` (`r2` before) to `266,341,057,420` (`r7` before, 15 extra processes), **`2.83 ppm` (`0.00028%`) across all 14 runs** and **`1.7 ppm` across the 7 job averages** (`887.8 M`).
    - `body` (`60 s x 3`, 7 jobs): raw `instructions:u` spanned `1,072,797,810,750` (`r7`, 15 extra processes) to `1,072,812,757,347` (`r3`, quiet), **`13.9 ppm` (`0.0014%`) across all 7 load levels** (`0.13 ppm` excluding the 15-process saturation job `r7`).
  - *Run-to-run in `nc42-pr2`* (`~/lanes/logs/ng-wave-4/nc42-pr2/bench/counters.md`, `e3b435e`, 3 rounds, seated 121 s minus 1 s baseline): spread of at most **`2` instructions a step out of `5,049,405` (`0.4 ppm`)**.
  - *Clone-to-clone on `beelink`* ([body 26](../../body/issues/26-cost-budget.md), line 103; [body 32](../../body/issues/32-the-cost-line-names-its-build.md), line 21, at `609f2162bcb2` / `b3a7ab66c4f7`): repeats inside one clone agreed to `1.3 ppm`; readings across separate clones (`896.5 M`, `896.6 M`, `896.65 M`) stayed within **`~100 ppm` (`0.01%`)**.
  - *Current main (`ffcd1a7a`, same Rust code as `9a7469ff`, `beelink`, load `2.6` / `2.2`):* `slice` = **`443.2 M` ins/sim-s** (`35.9 ms/sim-s` wall, `4.6 s` admission, `11,614` steps, `14,749` solves); `body` = **`2,138.0 M` ins/sim-s** (`164.4 ms/sim-s` wall, `185.7 s` admission, `11,661` steps, `14,411` solves).

### 2. Named build worker and cross-worker meaning

- **Named build worker:** **`beelink`** (`x86_64` Linux, AMD Ryzen 7 6800H, `perf_event_paranoid = 2`), matching [body 26](../../body/issues/26-cost-budget.md) and `.scratch/body/map.md:20`. Neither `ryzen` nor `asrog` has been checked for `perf stat -e instructions:u` support ([body 26](../../body/issues/26-cost-budget.md), line 93; `~/lanes/logs/ng-wave-4/b26/PLAN.md:101`). Where `perf` cannot count user instructions, `run.py cost` prints `minstructions_a_simulated_s=none` (`tools/bench/run.py:232-250`).
- **Cross-worker meaning (inferred, not measured across workers):** **A pinned reading on one worker does not carry across workers as an absolute ceiling.** Across ISAs (`x86_64` vs `aarch64`), instruction counts are unrelated. Even across two `x86_64` Linux machines running the same pinned `rustc` and `Cargo.lock`, dynamic user-space instruction counts can shift because glibc and the dynamic loader select CPU-feature-specific IFUNC paths (`memcpy`, `memmove`, `memset`, allocator paths) at runtime ([body 26](../../body/issues/26-cost-budget.md), line 93) and PMU retired-instruction accounting differs subtly across microarchitectures. A pinned threshold must therefore require `worker=beelink` (and any base-vs-head PR comparison must run both sides on one worker).

### 3. Band around main's figure and deliberate moves

- **Proposed band: `+2.0%` above the pinned `beelink` `slice` figure** (on `ffcd1a7a`/`9a7469ff`: pin `443.2 M`, ceiling `452.1 M`, `+8.9 M`).
  - Measurement noise on `beelink` is `0.4-14 ppm` in one clone and `~100 ppm` (`0.01%`) across clones (`200x` below `2.0%`), so machine load causes zero false trips.
  - Unrelated compiler codegen drift is larger than measurement noise: in `~/lanes/logs/ng-wave-4/nc42-pr2/bench/counters.md`, adding the dormant site-term machinery on main's unchanged model shifted `zero` by `-0.72%` instructions a step purely through code layout and inlining differences. A `+2.0%` band sits safely above normal `<= 1%` codegen drift while remaining **`6.3x` tighter** than the `+12.6%` regression in `nc42-pr2`'s void first pass.
- **When a change moves the figure on purpose:**
  - *Intentional increase:* update the checked-in `beelink` `slice` pin in the same PR with a one-sentence reason and base/head `run.py cost slice` lines from `beelink`, following the same discipline as `crates/ng-validate/tests/trace.rs` pins and [body 26](../../body/issues/26-cost-budget.md)'s count ceilings. Whole-body loop tickets remain governed by body 26's 12% per-loop budget and oversight exception rule.
  - *Intentional speedup (`> 2.0%` drop):* ratchet the pinned figure down in the same PR so later regressions cannot hide in freed headroom.

### 4. Where it runs and the time each adds

**Measured cost of one `slice` check on `beelink` (`ffcd1a7a`):**
- *Release build of `tools/bench` (`engine-bench`, a standalone Cargo workspace outside the repo workspace, `tools/bench/Cargo.toml:7`):* warm incremental release rebuild `6-12 s` (`5.88 s`, `10.54 s`, `11.67 s` measured in `~/lanes/logs/ng-wave-4/b26/` logs; `0.01 s` unchanged); cold release build from an empty bench target directory is `~30-60 s` (inferred, not timed on `ffcd1a7a`). By contrast, `tools/verify.d/10-rust.sh:49` runs only `cargo check` on `tools/bench` (`3 s` in `20261124T033113Z-9f5ba6ce8df0.log`).
- *Run time after build:* `tools/bench/run.py cost slice` (`60 s x 5 repeats`) takes `4.6 s` admission + `5 x 2.15 s` = **`~15.4 s` quiet** (`~30-40 s` under heavy load); a 1-repeat count-only invocation (`engine-bench cost slice 60 1`) would take **`~6.8 s` quiet** since instruction counts vary by `< 3 ppm` across repeats (`repeats = 5` exists for wall-time median and spread). For comparison, `cost body` (`60 s x 3`) takes `185.7 s` admission + `3 x 9.9 s` = **`~215 s` quiet**.

| Where | Added time on `beelink` | Fit and trade-offs |
|---|---|---|
| **Merge proof (`tools/verify` / `prove.sh`)** | `~25-30 s` warm (`~50-75 s` cold) on PRs reaching `ng-contract`, `ng-solver`, `ng-record`, or slice crates. | **Not recommended.** `tools/verify` runs across workers (`ryzen`, `asrog`, `beelink`) and developer hosts where `perf` may be absent or produce non-comparable counts; failing closed breaks `prove.sh` on `ryzen` while skipping silently makes `full_proof: yes` worker-dependent. In addition, `run.py cost` refuses `CARGO_TARGET_DIR` ([body 32](../../body/issues/32-the-cost-line-names-its-build.md)), which `prove.sh` supports (`prove.sh:52`), and body 26/32 kept `tools/bench` outside `tools/verify` (`tools/bench/run.py:35-36`). |
| **Nightly run (`tools/gate/nightly.sh` on `beelink`)** | `~25-30 s` warm (`~60 s` cold) on `slice` (`< 0.3%` of the `11,805 s` full run in `20261124T033113Z-9f5ba6ce8df0.log`), or `~230 s` (`~2%`) if `body` is added too. | **Recommended as the automated backstop.** `nightingale-nightly` runs on one fixed worker (`beelink`) from a clean detached `origin/main` clone ([foundation 10](10-nightly-job.md)), is immune to nightly CPU load (`1.7-14 ppm`), and alerts via `nightly.json` within 24 h if a PR bypasses the hand check. Implementation notes for the follow-up ticket: unset `CARGO_TARGET_DIR` for `run.py cost` (nightly sets it for `--full`) and run before `50-workspace.sh` (or whitelist `clinical-workspace/node_modules/` in `run.py`'s `ALLOWED_IGNORED`) so `npm ci` does not mark the clone `-dirty`. |
| **Hand-run bench per PR (`python3 -I tools/bench/run.py cost slice` on `beelink`)** | `~25-30 s` warm per side (`~50-60 s` for base + head). | **Recommended as the pre-merge gate.** Mirrors the existing `.scratch/body/map.md:20` rule for body loop PRs (`cost body` at base and head on `beelink`), extending it to any PR touching `crates/ng-contract/`, `crates/ng-solver/`, `crates/ng-record/`, or slice system crates (`cost slice` at base and head on `beelink`, held to `+2.0%`), catching regressions before merge without tying `tools/verify` to `beelink` or `perf`. |

**Recommended placement:** pair the **pre-merge hand-run PR rule** on `beelink` (`~50-60 s` for base + head) with the **automated nightly check** in `tools/gate/nightly.sh` on `beelink` (`~25-30 s` warm) against a checked-in `slice` pin.

### 5. Would the void first pass's form trip the proposed check?

- **At the commit where the regression occurred (`e3b435e`, `nc42-pr2` void first pass): yes, decisively (measured).** Calling `apply` and `Reads::fill` directly from `refused_at` stopped LLVM from inlining both into `forward` and raised user instructions a step on main's unchanged no-vein model by **`+12.6%`** (`+126,000 ppm`, measured on `beelink`; wall time `+21.28%`, `~/lanes/logs/ng-wave-4/nc42-pr2/PLAN.md:62,96`, `bench/first-pass/tables.md`). That exceeds the proposed **`+2.0%` band by `6.3x`** (and measurement noise by over `1,000x`), tripping both the pre-merge base-vs-head check and the nightly pin check cleanly.
- **Caveat on current main (`9a7469ff` / `ffcd1a7a`, inferred from source, not rerun):** `crates/ng-contract/src/evaluate.rs` has since been restructured (`nightingale#235` and `nightingale#239`). Step residual evaluation now runs through `#[inline(always)] apply_fast` (`evaluate.rs:1022-1068`, `nightingale#235`), `#[inline(always)] forward_fast` (`evaluate.rs:1070-1081`, `nightingale#235`), and `#[inline(always)] eval_single_fast` (`evaluate.rs:1434-1504`, `nightingale#239`), and `apply` itself carries `#[inline(always)]` (`evaluate.rs:978-1020`, `nightingale#235`) with two callers already (`forward` at line 1095 and `slope_single_fast` at line 1969, `nightingale#239`). Putting the historical `refused_at` edit back verbatim on `9a7469ff` therefore would not reproduce the same LLVM de-inlining on the step residual path (inferred). Any equivalent de-inlining or accidental step-path slowdown on `9a7469ff` costing `>= 2.0%` (`>= 8.9 M` ins/sim-s on `slice`) trips the proposed check cleanly. Building the check itself belongs in a follow-up ticket per this ticket's non-goals.
