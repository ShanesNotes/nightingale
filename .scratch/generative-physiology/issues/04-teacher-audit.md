# 04: The teacher audit

Type: research
Status: open
Blocked by: 03
Runs on: the code is written on the workstation; the audit runs only on the heavy-run machine

**What it builds and answers:** `gp.py audit`, the audit plan below as `tools/generative-physiology/spec/audit-v1.txt`, and the report that turns the runs into verdicts. Then it runs the audit and records the filled template as this ticket's Answer. Before any training data exists, the audit answers one question. Is the slice on `ng_solver::Core` a usable and informative teacher for posture runs?

The program does not exist at 8e427b1. Ticket 03 builds the driver, and this ticket adds `gp.py audit`, standard-library Python that runs `gp-driver` once per run and reads the run directories, and the audit's closed-loop rule. The code PR merges first. The audit then runs from main's merge commit C of that PR, and a second, docs-only PR adds the Answer.

## Usage

On the workstation, with git only:

```text
C=<main's merge commit of this ticket's code PR>
git diff --quiet 8e427b1447e33fa9adced1fc870438f1739dc157 "$C" -- crates Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml rustfmt.toml && echo engine-unchanged
test "$(git rev-parse "$C:crates")" = 036113e6ff801dc949024ebecf7c89ff5c0275c5 && echo crates-tree-unchanged
git archive --format=tar --prefix="nightingale-$C/" -o "nightingale-$C.tar" "$C"
sha256sum "nightingale-$C.tar" > "nightingale-$C.tar.sha256"
```

On the heavy-run machine, after the archive and its digest arrive. `DATA` is a directory outside the snapshot:

```text
sha256sum -c "nightingale-$C.tar.sha256"
tar -xf "nightingale-$C.tar" && cd "nightingale-$C"
rustc --version && cargo --version && python3 --version          # 1.98.1, 1.98.1, at least 3.11
CARGO_TARGET_DIR="$PWD/../gp-target" cargo build --release --locked --manifest-path tools/generative-physiology/driver/Cargo.toml
export GP_DRIVER="$PWD/../gp-target/release/gp-driver"           # build output stays outside the snapshot
setsid nohup python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit "$C" --workers 3 \
  > DATA/audit.log 2>&1 < /dev/null &
cat DATA/audit/progress.txt                                       # after any reconnect
python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit "$C" --report > DATA/audit/report.txt
sha256sum DATA/audit/*/summary.txt > DATA/audit/summaries.sha256 && sha256sum DATA/audit/report.txt
```

`audit` is safe to repeat: a complete run directory is skipped, a `.fault` directory is reported and kept, a partial one whose process is gone is removed and redone, and once every run is present or faulted the command only reads. It starts the longest runs first, never runs G2 beside D0 or D1, and never runs more than two 30-minute runs at once, so the peak stays at about 22 GB. Before the first long run it checks free memory (at least 24 GB), free disk (at least 20 GB) and the toolchain, and refuses to start without them.

## Inputs and identities

- **Engine input.** Unchanged against 8e427b1, checked by the two workstation commands above. The crates tree is `036113e6ff801dc949024ebecf7c89ff5c0275c5` (`git rev-parse 8e427b1:crates`). For a check without git, `Cargo.lock` has SHA-256 `24b7896024353dc8a9f8d81bebd8aeee4c1a3ceb7eefdf8bb3245cf041083f07` and `rust-toolchain.toml` has `41ab86880726126e91a8f858567213dcf5332b624151ba8a6ed3b691e611f5d9`.
- **Snapshot.** The archive's SHA-256, recorded on both machines; every `summary.txt` records C.
- **Toolchain.** rustc and cargo 1.98.1 (`rust-toolchain.toml:2`). Whether rustup on the heavy-run machine honors the pin with rust-src is unconfirmed; the audit needs only rustc and cargo.
- **The engine's bits on that machine.** A0 must reproduce what `crates/ng-validate/tests/trace.rs:134-135` pins for Core at a 10 ms cap: packet digest `0ec011d81366c367549a2fcdfe72146eb4c00ac911c7610dd2572dcf6b732887` and seal `28249b2e9c8e26915452d7caa96cbae0d834f5f60837b99c9a340b6feeb8729b` at the first head at or after 1 s, which is at 1001.309992 ms (`trace.rs:146`).
- **Seeds.** None of the audit's own. The run seed is `admit_at`'s fixed `[1, 2, 3, 4]` (`crates/ng-validate/src/initialize.rs:234`), the slice has no stream, forcing or hazard, the arena lens has no measurement error (`crates/ng-validate/src/slice.rs:917-923`), every event is written out, and B7's rule is deterministic.

## Runs

Times are seconds after 06:55:00, and every stamp is at or after the fence at 1 s. Settings: flat is trunk 0 with horizontal legs; sit is trunk 1, seated; stand is trunk 1, standing; tilt is trunk 0.5, seated, which by arithmetic gives the heights of a 30 degree tilt with straight legs (`crates/ng-contract/src/boundary.rs:61-75, 640-645`); head-down is trunk -0.2, horizontal. Every run uses Core at a 10 ms cap unless its row says otherwise.

| Run | Events | End | Variant | Purpose |
|---|---|---|---|---|
| A0 | none | 1.1 | | the pinned first second |
| A1 | stand at 1.3 | 60 | checkpoint at the first head at or after 1.25 s; the stand is then enqueued on the original and on a copy resumed into a freshly compiled body and a fresh core | posture after stepping and across a checkpoint, on Core (the reference-core test `crates/ng-validate/tests/arena_body.rs:714-737`, moved to Core and compared by seal) |
| A2, A3 | none; stand at 1.2 | 6 | | the stand reaches the reflex on Core |
| A4 | stand 1, flat 120, stand 240, flat 300, stand 330, flat 390 | 420 | checkpoint at the first head at or after 290 s; later events go to the original and the resumed copy | a late checkpoint: seals, resume time, and the export's peak memory |
| A5 | A4's events | 420 | the parent runs to the end; a branch rewound to the first head at or after 290 s then takes the same later events | a rewound branch equals the parent's own continuation |
| B1 | stand 1, flat 180, stand 330, flat 400, a sit ramp from 420 to 430 (21 events: trunk 0 to 1 in steps of 0.05, horizontal legs, seated from the 11th, as `crates/ng-validate/tests/arena_body.rs:806-818`), head-down 480, stand 510, flat 700 | 900 | | the base of the prefix checks |
| B2 | B1 with sit at 510 | 900 | | one instant, another payload |
| B3 | B1 with its 510 event at 540 | 900 | | divergence at another instant |
| B4 | B1 | 900 | replica 1 | another process and another admission |
| B5 | B1 | 900 | late, lead 10 s | I5 on the slice |
| B6 | B1 | 900 | observer off | I7 on the slice |
| B7 | B1's events before 330 s, then the audit rule below, as a closed loop through `gp-driver interact` | 900 | | the online closed loop |
| B8 | B7's `realized.txt` as an open-loop spec | 900 | | B7's replay |
| C1 to C6 | start histories: none; sit 1, flat 240; stand 1, flat 240; stand 1, flat 120; sit 1, stand 120, flat 240; tilt 1, flat 240. Each then stands at 330 and lies flat at 630 | 900 | | differences across start histories; the 15-minute cost |
| C7 | C3's history, then a change every 3 s from 330 to 630 (stand, flat, sit, flat, repeating: 100 events), then a stand ramp of 21 events 0.2 s apart from 640 | 900 | | the cost of accumulated posture inputs, which never end (`crates/ng-contract/src/boundary.rs:188-198, 327-330, 450-457`) |
| D0, D1 | none; stand 300 | 1800 | | the slow component against its matched flat control; the 30-minute cost |
| F1 | tilt 300, flat 600 | 900 | | the static tilt |
| F2 | a tilt ramp from 300 to 310 (trunk 0.1 to 0.5 in steps of 0.1, horizontal legs, seated from the 3rd) and back from 600 to 610 (trunk 0.4 to 0, seated legs, horizontal from the 3rd) | 900 | | the ramped tilt |
| G1 | C3's events | 900 | Core at a 5 ms cap | numerical variation at 15 minutes |
| G2 | D1's events | 1800 | Core at a 5 ms cap | numerical variation at 30 minutes |
| G3, G4 | stand 60 | 180 | G3: the reference integrator at 1 ms, floor 0.1 ms, scales 1.0 and 1e-3 (`tools/bench/src/main.rs:53-61`); G4: Core | the two cores side by side |

**The audit rule (B7 only).** It tests the closed-loop mechanism and is no clinical rule. At each beat edge after 330 s, it compares the beat's arterial pressure with the mean of the three beats that ended last before 330 s. When the pressure is lower and the last action is at least 30 s old, it sets stand 2 s later; when it is higher under the same condition, it sets flat 2 s later. It reads nothing else, so its actions depend only on the run's beats.

## Pass and fail

The technical gate passes only if every row below holds.

| Check | Passes when |
|---|---|
| A0 | the packet digest, the seal and the head instant equal the pinned values |
| A1 | the original's and the resumed copy's seals agree from the checkpoint's revision to the end, and `RecordDigests` agree at the end |
| A2, A3 | the reference-core test's bars (`crates/ng-validate/tests/arena_body.rs:538-562`): A3's mean sympathetic traffic, one sample per accepted step from admission to 6 s as the test takes it (`arena_body.rs:86-97`), is above 1.2 times A2's; at 6 s, A3's heart rate is above A2's and its leg-vein water above A2's plus 10; and A3's aortic-arch minus carotid-sinus stretch is 0.105 within 1e-9 |
| A4 | the seals agree from the checkpoint to the end |
| A5 | the branch's seals equal the parent's from the fork to the end |
| B1, B2 | the seals agree for every revision through the one that ends at exactly 510.000 s, and differ after it |
| B1, B3 | the seals agree through the last revision that ends before 510 s, and B1's next revision ends at exactly 510 s |
| B4, B5, B6 | each seal log equals B1's from end to end |
| B7, B8 | the two seal logs are equal from end to end |
| Every run | no engine fault; every Core run at a 10 ms cap shares A0's model, packet, resolution and run-identity digests and its revision-0 seal |
| Ledger closure | between consecutive edges, each conserved quantity's inventory change equals its inflow minus its outflow within 64 times 2^-52 of the sum of the four magnitudes involved |
| Beat identity | on every beat, one over its length and its mean rate agree within 1e-3 of one over its length |

The informativeness gate decides whether the experiment can test anything. Both rows are model defaults.

| Check | Passes when |
|---|---|
| Start histories differ | for at least two of the four response features below, the range across C1 to C6 is at least three times the feature's noise floor, each feature measured against the mean of the 30 s before the stand at 330 s. A feature's noise floor is the larger of its synthetic measurement error (0.02 per s for the rate and 2.0 for the pressure, divided by the square root of the beats averaged) and its numerical variation, the feature's difference between C3 and G1 |
| The slow component | leg-interstitium water, D1 minus D0, from 360 to 1800 s, fitted as a + b exp(-(t - 360)/tau) over 161 log-spaced values of tau from 10 s to 100,000 s. Resolved when the best fit's root-mean-square residual is at most 5% of the absolute value of b, the values of tau whose residual is within twice the best span less than a factor of 2, and the absolute value of b is at least 20 times the largest absolute difference in the same quantity between D1 and G2. Otherwise it is reported as unresolved within 30 minutes, and the experiment proceeds with that limitation in every result |

**The four response features.** Defined here, and used unchanged by ticket 09's metrics and ticket 12's order test: (1) the heart-rate rise at its peak within 30 s of a posture change; (2) the arterial-pressure fall at its 3-beat-mean nadir within the same 30 s; (3) the heart-rate change averaged over 240 to 300 s after the change; (4) the same for arterial pressure. Each is measured against a baseline: here, the mean of the 30 s before the change; in ticket 09's paired branches, the run without the change over the same times.

Measurements, with no verdict: the time to 63% of the change in stroke and in heart rate after tilt-up and after tilt-back, for F1 and F2 (the teacher's column of ticket 09's outside check); the largest difference in beat-mean pressure and heart rate between G3 and G4, flagged when it exceeds 2% of G4's stand response; the ripple, as the root-mean-square difference between the lens's one-second pressure and the beats' time-weighted mean over the same windows in C1 to C6; C7's wall time per simulated second against C3's; A4's peak memory during the export, when every missing link is collected once more before the store takes it (`crates/ng-record/src/history.rs:294-302`, `crates/ng-record/src/record.rs:242-256`); and every run's admission time, step wall time, accepted steps, refusals and peak memory.

## What the audit decides

The Answer records each of these before any training data exists.

- **The teaching core.** Core at a 10 ms cap, if the technical gate passes. If it fails, the experiment stops here and the Answer records why. The reference integrator is never an automatic fallback. At 1 ms, 30 minutes means at least 1.8 million accepted steps and 28 to 35 GB of history at the reported 15.6 to 19.7 KB a step.
- **Start histories.** 300 s. If the first informativeness row fails, histories become 600 s and C1 to C6 run again once, with their stands at 630 s; every later case time is an offset from a history's end (tickets 06 and 09). A second failure removes the across-history order test from ticket 12's list of what earns the next step, and the Answer says so.
- **The encounter.** 10 minutes when tau is resolved and at most 20 minutes; 15 minutes when it is resolved and at most 40 minutes; otherwise 15 minutes, with the slow component marked unresolved.
- **Workers.** The floor of 24 GB divided by 1.15 times the projected peak memory of the longest run planned (a history plus an encounter), scaled from C1 to C7 by accepted steps, and at most 4.
- **The rule's thresholds** for ticket 06: the 20th and 80th percentiles of the 3-beat mean beat pressure over C1 to C6, after their first minute.
- **The plan's size.** If the teacher runs of tickets 06, 09 and 12 together project past 36 hours of wall time at the chosen workers, training schedules per start history drop from 8 to the largest count that fits. The validation cases, the held-out set and the replays stay whole.

## Expected time and memory

Projections, none confirmed on the heavy-run machine. Assumptions: Core takes about 212 accepted steps per simulated second on the slice (`.scratch/native-contract/issues/11-memory-stays-flat.md:23`, author-reported) and up to 1.5 times as many at a 5 ms cap; the heavy-run machine runs Core at 2.5 to 4.9 times real time, because 4.91 was measured on another machine over 35 minutes (`.scratch/native-contract/issues/12-throughput-beside-the-record.md:31`) and a Ryzen 5 2600 is taken to run at 0.5 to 1.0 of that machine's speed per thread; history costs 15.6 to 19.7 KB per accepted step plus 0.3 GB fixed; an admission takes 17 to 34 s; the reference integrator takes about 3.3 ms per step (`issues/12:15`, author-reported).

| Runs | Simulated time each | Accepted steps each | Peak memory each | Wall time each |
|---|---|---|---|---|
| A0, A2, A3 | at most 6 s | under 1,300 | 0.3 GB | 20 to 40 s |
| A1 | 60 s and a resumed tail | about 25,000 | 0.8 GB | 1 to 2 min |
| A4, A5 | 420 s and a resumed or rewound tail | about 117,000 | 3.0 to 4.5 GB, more while the export collects links | 3 to 6 min, plus an unmeasured resume |
| B1 to B8, C1 to C7, F1, F2 | 900 s | about 191,000 | 3.3 to 4.1 GB | 3.4 to 6.6 min |
| D0, D1 | 1,800 s | about 382,000 | 6.3 to 7.8 GB | 6.4 to 12.6 min |
| G1 | 900 s | up to 290,000 | 4.9 to 6.1 GB | 5 to 10 min |
| G2 | 1,800 s | up to 570,000 | 9.3 to 11.6 GB | 10 to 20 min |
| G3 | 180 s | 180,000 or more | 3.1 to 3.9 GB | 10 to 20 min |
| G4 | 180 s | about 38,000 | 0.9 GB | 1 to 1.6 min |

In all: 29 runs, about 6.5 simulated hours, 2 to 4 process-hours, about 1 to 2 hours of wall time at 3 workers, at most about 22 GB at once under the ordering rule above (G2 beside G1 and one 15-minute run, or D0 and D1 beside G1) and about 1 GB on disk. The release build takes 5 to 15 minutes more.

## Results template

The Answer copies this block and fills every field.

```text
Input: commit C <..>; archive sha256 <..>; engine unchanged against 8e427b1: <yes/no>; crates tree <..>
Toolchain: rustc <..>, cargo <..>, python3 <..>; rust-src present: <yes/no/unknown>
Machine: the heavy-run machine; logical processors <..>; memory limit <..>; swap limit <..>

| Check                 | Runs    | Measured                                             | Criterion                 | Verdict |
| A0 pinned second      | A0      | packet <..>, seal <..>, head <.. ms>                 | trace.rs:134-135, 146     |         |
| A1 across checkpoint  | A1      | equal seals from revision <r> to <r'>                | all equal                 |         |
| A2, A3 reflex         | A2, A3  | traffic <..> vs <..>; rate <..>; leg-vein water <..>; stretch gap <..> | the reference test's bars |  |
| A4 late checkpoint    | A4      | equal <..>; resume <s>; export peak <GB>             | all equal                 |         |
| A5 rewind             | A5      | equal from revision <r> <..>                         | all equal                 |         |
| B1/B2, B1/B3 prefixes |         | last common revision ends at <ms>                    | 510 000 / just before     |         |
| B4, B5, B6            |         | equal end to end <..>                                | equal                     |         |
| B7/B8 closed loop     |         | equal end to end <..>; actions <n>                   | equal                     |         |
| Faults, identities    | all     | <n>; <shared/not>                                    | 0; shared                 |         |
| Closure, beat identity| all     | max residual <..>; max beat gap <..>                 | bound; 1e-3               |         |
| Histories differ      | C1-C6   | ranges <..> against floors <..>                      | two of four               |         |
| Slow component        | D0, D1  | tau <..>, b <..>, residual over abs(b) <..>, span <..> | resolved or not         |         |
| Tilt                  | F1, F2  | stroke t63 up/back <..>; rate t63 up/back <..>       | measured                  |         |
| Cores                 | G3, G4  | max d pressure <..>, max d rate <..>                 | flag above 2%             |         |
| Numerical variation   | G1, G2  | per feature <..>; leg interstitium <..>              | measured                  |         |
| Ripple                | C1-C6   | RMS <..> against the stand response <..>             | measured                  |         |
| Accumulated postures  | C7, C3  | wall per simulated s <..> against <..>               | measured                  |         |

| Run | simulated s | accepted steps | per simulated s | refusals | admission s | wall s | times real time | peak GB |

Technical gate: <pass/fail>. Informativeness gate: <pass/fail, which rows>.
Derived: teaching core <..>; history <..> s; encounter <..> min; workers <..>; rule low <..>, high <..>; plan size <..>.
Evidence: summaries.sha256 <..>; report.txt <..>; the data stay on the heavy-run machine.
```

## Acceptance

- [ ] The code PR adds `gp.py audit`, the plan and the report, and its proof from a clean git checkout on the heavy-run machine shows `full_proof: yes`.
- [ ] The audit has run from main's merge commit C, and the Answer holds every field of the template, both gates' verdicts and every derived setting.
- [ ] If either gate fails, the Answer says which rows failed, and the next ticket that depends on this one stays blocked until a person reads it.

## Non-goals

- The reduction's balance and the reversal share per transfer (ticket 05 adds them to this report).
- Training data, models, and any change to an expectation the workspace pins.
