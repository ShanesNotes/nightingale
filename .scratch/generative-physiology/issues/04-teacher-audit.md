# 04: The teacher audit

Type: research
Status: open
Blocked by: 03, `.scratch/native-contract/issues/28-admission-energy-at-balance.md`, `.scratch/native-contract/issues/33-leg-veins-empty-head-down.md`
Runs on: the code is written on the workstation; the audit runs only on the heavy-run machine

**What it builds and answers:** `gp.py audit`, the audit plan below as `tools/generative-physiology/spec/audit-v1.txt`, the report that turns the runs into verdicts, and the driver's capture version 2, which writes what the report reads and capture version 1 did not: each run's catalogue of names, the sympathetic traffic after every accepted step, each hazard firing, the bytes of links, and on a branch its first seal and the time of the resume or rewind alone. Then it runs the audit and records the filled template as this ticket's Answer. Before any training data exists, the audit answers one question. Is the slice on `ng_solver::Core` a usable and informative teacher for posture runs?

The program does not exist at 84744af. Ticket 03 builds the driver, and this ticket adds `gp.py audit`, standard-library Python that runs `gp-driver` once per run and reads the run directories, and the audit's closed-loop rule. The code PR merges first. The audit then runs from C, a main commit that holds that PR, native-contract 28 and the core the arena's pick confirms, and a second, docs-only PR adds the Answer.

## Usage

On the workstation, with git only:

```text
C=<the main commit the audit runs from>
git rev-parse "$C:crates"                       # the teaching engine's crates tree; the Answer records it
git diff --quiet 84744af86ae9feac3a90e610ce21b741990f9cae "$C" -- crates Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml rustfmt.toml \
  && echo engine-as-designed || echo engine-moved-since-design
git show "$C:crates/ng-validate/tests/trace.rs" | sed -n '/^fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was/,/^}$/p' \
  | grep -o -e '[0-9a-f]\{64\}' -e '[0-9]\{4\}\.[0-9]\{6\} ms'   # 3 lines, A0's pins at C: the packet digest, the seal, the head
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
setsid nohup python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit "$C" --workers 3 --disk /mnt/c \
  > DATA/audit.log 2>&1 < /dev/null &
cat DATA/audit/progress.txt                                       # after any reconnect
python3 -I tools/generative-physiology/gp.py audit --data DATA --source-commit "$C" --report > DATA/audit/report.txt
(cd DATA/audit && LC_ALL=C sha256sum */summary.txt) > DATA/audit/summaries.sha256 && sha256sum DATA/audit/summaries.sha256 DATA/audit/report.txt
```

`audit` is safe to repeat: a complete run directory is skipped, a `.fault` directory is reported and kept, a partial one whose process is gone is removed and redone, and once every run is present or faulted the command only reads. It starts the longest runs first. Each run keeps its links in its own file store and deletes the file once its directory is complete (ticket 03), so memory stays flat and transient disk stays under about 25 GB at 3 workers. Before the first long run it checks free disk (at least 40 GB) and the toolchain, and refuses to start without them. Under WSL, `--disk /mnt/c` names the Windows drive that holds the disk image, whose free space is the real budget. A later command does not repeat the 40 GB check, because the image keeps what a deleted links file used; before each start it needs room for every running run's projected links and the new one's, plus 10 GB.

## Inputs and identities

- **Engine input.** The engine the audit approves is the crates tree at C, which the Answer records, and every later teacher run (tickets 06, 09 and 12) must come from a commit whose engine inputs equal C's: the same `git diff --quiet` from C to that commit exits 0, or the audit runs again. At 84744af, the commit this design last read, the crates tree is `23cbe1846b8d98cefc6f462214360b690a8843af`; `engine-moved-since-design` means the Answer lists what changed before trusting the plan. For a check without git, `Cargo.lock` has SHA-256 `24b7896024353dc8a9f8d81bebd8aeee4c1a3ceb7eefdf8bb3245cf041083f07` and `rust-toolchain.toml` has `41ab86880726126e91a8f858567213dcf5332b624151ba8a6ed3b691e611f5d9` at 84744af.
- **Snapshot.** The archive's SHA-256, recorded on both machines; every `summary.txt` records C.
- **Toolchain.** rustc and cargo 1.98.1 (`rust-toolchain.toml:2`). Whether rustup on the heavy-run machine honors the pin with rust-src is unconfirmed; the audit needs only rustc and cargo.
- **The engine's bits on that machine.** A0 must reproduce what the test `the_first_second_of_the_slice_on_ng_solver_stays_where_it_was` in `crates/ng-validate/tests/trace.rs` pins at C for Core at a 10 ms cap: the packet digest, the seal at the first head at or after 1 s, and that head's instant, the last line of the test's trace. This ticket quotes none of the three, because a change to the engine moves them. The report reads them from the test by its name (`a0_pins` in `tools/generative-physiology/audit.py`), and Usage prints them at C as three lines. Any other count means the test has changed shape, and `a0_pins` is read again before the audit runs.
- **Seeds.** None of the audit's own. The slice carries one forcing on the skin's water loss and one stir hazard (`crates/ng-validate/src/slice.rs:376-414`), and both read the run seed, which `admit_at` fixes at `[1, 2, 3, 4]` (`crates/ng-validate/src/initialize.rs:236`). So every run shares one forcing course and one stir instant, and a replica repeats them. The arena lens has no measurement error (`crates/ng-validate/src/slice.rs:954-960`), every event is written out, and B7's rule is deterministic.

## Runs

Times are seconds after 06:55:00, and every stamp is at or after the fence at 1 s. Settings: flat is trunk 0 with horizontal legs; sit is trunk 1, seated; stand is trunk 1, standing; tilt is trunk 0.5, seated, which by arithmetic gives the heights of a 30 degree tilt with straight legs (`crates/ng-contract/src/boundary.rs:61-75, 640-645`); head-down is trunk -0.2, horizontal. Every run uses Core at a 10 ms cap unless its row says otherwise.

| Run | Events | End | Variant | Purpose |
|---|---|---|---|---|
| A0 | none | 1.1 | | the pinned first second |
| A1 | stand at 1.3 | 60 | checkpoint at the first head at or after 1.25 s; the stand is then enqueued on the original and on a copy resumed into a freshly compiled body and a fresh core | posture after stepping and across a checkpoint, on Core (the reference-core test `crates/ng-validate/tests/arena_body.rs:699-725`, moved to Core and compared by seal) |
| A2, A3 | none; stand at 1.2 | 6 | | the stand reaches the reflex on Core |
| A4 | stand 1, flat 120, stand 240, flat 300, stand 330, flat 390 | 420 | checkpoint at the first head at or after 290 s; later events go to the original and the resumed copy | a late checkpoint: seals, and the resume's time through the store |
| A5 | A4's events | 420 | the parent runs to the end; a branch rewound to the first head at or after 290 s then takes the same later events | a rewound branch equals the parent's own continuation |
| B1 | stand 1, flat 180, stand 330, flat 400, a sit ramp from 420 to 430 (21 events: trunk 0 to 1 in steps of 0.05, horizontal legs, seated from the 11th, as `crates/ng-validate/tests/arena_body.rs:872-884`), head-down 480, stand 510, flat 700 | 900 | | the base of the prefix checks |
| B2 | B1 with sit at 510 | 900 | | one instant, another payload |
| B3 | B1 with its 510 event at 540 | 900 | | divergence at another instant |
| B4 | B1 | 900 | replica 1 | another process and another admission |
| B5 | B1 | 900 | late, lead 10 s | I5 on the slice |
| B6 | B1 | 900 | observer off | I7 on the slice |
| B7 | B1's events before 330 s, then the audit rule below, as a closed loop through `gp-driver interact` | 900 | | the online closed loop |
| B8 | B7's `realized.txt` as an open-loop spec | 900 | | B7's replay |
| C1 to C6 | start histories: none; sit 1, flat 240; stand 1, flat 240; stand 1, flat 120; sit 1, stand 120, flat 240; tilt 1, flat 240. Each then stands at 330 and lies flat at 630 | 900 | | differences across start histories; the 15-minute cost |
| C7 | C3's history, then a change every 3 s from 330 to 630 (stand, flat, sit, flat, repeating: 100 events), then a stand ramp of 21 events 0.2 s apart from 640 (trunk 0 to 1 in steps of 0.05, horizontal legs, standing from the 11th) | 900 | | the cost of accumulated posture inputs, which never end (`crates/ng-contract/src/boundary.rs:188-198, 327-330, 450-457`) |
| D0, D1 | none; stand 300 | 1800 | | the slow component against its matched flat control; the 30-minute cost |
| F1 | tilt 300, flat 600 | 900 | | the static tilt |
| F2 | a tilt ramp from 300 to 310 (trunk 0.1 to 0.5 in steps of 0.1, horizontal legs, seated from the 3rd) and back from 600 to 610 (trunk 0.4 to 0, seated legs, horizontal from the 3rd) | 900 | | the ramped tilt |
| G1 | C3's events | 900 | Core at a 5 ms cap | numerical variation at 15 minutes |
| G2 | D1's events | 1800 | Core at a 5 ms cap | numerical variation at 30 minutes |
| G3, G4 | stand 60 | 180 | G3: the reference integrator at 1 ms, floor 0.1 ms, scales 1.0 and 1e-3 (`tools/bench/src/main.rs:46-50`); G4: Core | the two cores side by side |

**The audit rule (B7 only).** It tests the closed-loop mechanism and is no clinical rule. At each beat edge after 330 s, it compares the beat's arterial pressure with the mean of the three beats that ended last before 330 s. When the pressure is lower and the last action is at least 30 s old, it sets stand 2 s later; when it is higher under the same condition, it sets flat 2 s later. It reads nothing else, so its actions depend only on the run's beats.

## Pass and fail

The technical gate passes only if every row below holds.

| Check | Passes when |
|---|---|
| A0 | the packet digest, the seal and the head instant equal the pinned values |
| A1 | the original's and the resumed copy's seals agree from the checkpoint's revision to the end, and `RecordDigests` agree at the end |
| A2, A3 | the reference-core test's bars (`crates/ng-validate/tests/arena_body.rs:523-547`): A3's mean sympathetic traffic, one sample per accepted step from admission to 6 s as the test takes it (`arena_body.rs:68-81`), is above 1.2 times A2's; at 6 s, A3's heart rate is above A2's and its leg-vein water above A2's plus 10; and A3's aortic-arch minus carotid-sinus stretch is 0.105 within 1e-9 |
| A4 | the seals agree from the checkpoint to the end |
| A5 | the branch's seals equal the parent's from the fork to the end |
| B1, B2 | the seals agree for every revision through the one that ends at exactly 510.000 s, and differ after it |
| B1, B3 | the seals agree through the last revision that ends before 510 s, and B1's next revision ends at exactly 510 s |
| B4, B5, B6 | each seal log equals B1's from end to end |
| B7, B8 | the two seal logs are equal from end to end |
| Every run | no engine fault; every Core run at a 10 ms cap shares A0's model, packet, resolution and run-identity digests and its revision-0 seal |
| Ledger closure | between consecutive edges, each conserved quantity's inventory change equals its inflow minus its outflow within 64 times 2^-52 of the sum of the four magnitudes involved: both inventories and the interval's inflow and outflow |
| Beat identity | on every beat, one over its length and its mean rate agree within 1e-3 of one over its length |

The informativeness gate decides whether the experiment can test anything. Both rows are model defaults.

| Check | Passes when |
|---|---|
| Start histories differ | for at least two of the four response features below, the range across C1 to C6 is at least three times the feature floor, each feature measured against the mean of the 30 s before the stand at 330 s. A feature floor is the larger of its synthetic measurement error (0.02 per s for the rate and 2.0 for the pressure, divided by the square root of the beats averaged) and its numerical variation, the feature's difference between C3 and G1 |
| The slow component | leg-interstitium water, D1 minus D0, from 360 to 1800 s, fitted as a + b exp(-(t - 360)/tau) over 161 log-spaced values of tau from 10 s to 100,000 s. Resolved when the best fit's root-mean-square residual is at most 5% of the absolute value of b, the values of tau whose residual is within twice the best span less than a factor of 2, the absolute value of b is at least 20 times the largest absolute difference in the same quantity between D1 and G2, and the best tau is not at either end of the grid, where the grid and not the data bounds it (a straight line meets the first two conditions at 100,000 s). Otherwise it is reported as unresolved within 30 minutes, and the experiment proceeds with that limitation in every result |

**The four response features.** Defined here, and used unchanged by ticket 09's metrics and ticket 12's order test: (1) the heart-rate rise at its peak within 30 s of a posture change; (2) the arterial-pressure fall at its 3-beat-mean nadir within the same 30 s; (3) the heart-rate change averaged over 240 to 300 s after the change; (4) the same for arterial pressure. Each is measured against a baseline: here, the mean of the 30 s before the change; in ticket 09's paired branches, the run without the change over the same times.

Measurements, with no verdict: the time to 63% of the change in stroke and in heart rate after tilt-up and after tilt-back, for F1 and F2 (the teacher's column of ticket 09's outside check); the largest difference in beat-mean pressure and heart rate between G3 and G4, flagged when it exceeds 2% of G4's stand response; the ripple, as the root-mean-square difference between the lens's one-second pressure and the beats' time-weighted mean over the same windows in C1 to C6; C7's wall time per simulated second against C3's; A4's resume time, because a resume walks the whole chain through the store; the stir's instant and the range of the skin's forcing factor in every run, read from the skin column's extent per beat over its basal rate; and every run's admission time, step wall time, accepted steps, refusals, peak memory and bytes of links.

## What the audit decides

The Answer records each of these before any training data exists.

- **The teaching core.** Core at a 10 ms cap, if the technical gate passes. If it fails, the experiment stops here and the Answer records why. The reference integrator is never an automatic fallback. At 1 ms, 30 minutes means at least 1.8 million accepted steps, about 33 GB of transient links and, at about 3.3 ms a step, about 1.7 hours of wall time per run.
- **Start histories.** 300 s. If the first informativeness row fails and the technical gate passes, histories become 600 s and C1 to C6 run again once as C1-600 to C6-600, each history stamp after the fence doubled, with their stands at 630 s and lying flat at 930 s, and with G1-600, C3-600 at a 5 ms cap, for their numerical variation; every later case time is an offset from a history's end (tickets 06 and 09). A second failure removes the across-history order test from ticket 12's list of what earns the next step, and the Answer says so.
- **The encounter.** 10 minutes when tau is resolved and at most 20 minutes; 15 minutes when it is resolved and at most 40 minutes; otherwise 15 minutes, with the slow component marked unresolved.
- **Workers.** At most 4, half the logical processors, and fewer if the longest planned run's peak memory or transient links, scaled from C1 to C7 by accepted steps, times the count would pass 24 GB of memory or the free disk.
- **The rule's thresholds** for ticket 06: the 20th and 80th percentiles of the 3-beat mean beat pressure over C1 to C6, after their first minute.
- **The plan's size.** If the teacher runs of tickets 06, 09 and 12 together project past 36 hours of wall time at the chosen workers, training schedules per start history drop from 8 to the largest count that fits. The validation cases, the held-out set and the replays stay whole: 32 tuning runs, 44 validation cases (ticket 09 enumerates 44, though its Usage says 48), 104 held-out cases and 160 replays, R's 40 among them.

## Expected time and memory

Projections, none confirmed on the heavy-run machine. Assumptions: Core takes about 212 accepted steps per simulated second on the slice (`.scratch/native-contract/issues/11-memory-stays-flat.md:33`, author-reported) and up to 1.5 times as many at a 5 ms cap; the heavy-run machine runs Core at 2.5 to 4.9 times real time, because 4.91 was measured on another machine over 35 minutes (`.scratch/native-contract/issues/12-throughput-beside-the-record.md:31`) and a Ryzen 5 2600 is taken to run at 0.5 to 1.0 of that machine's speed per thread; history costs 18,584 bytes per accepted step on disk, and memory stays flat with a file store (both measured: `.scratch/native-contract/issues/11-memory-stays-flat.md`, its Answer); an admission takes 17 to 34 s; the reference integrator takes about 3.3 ms per step (`issues/12:15`, author-reported).

| Runs | Simulated time each | Accepted steps each | Transient links on disk each | Wall time each |
|---|---|---|---|---|
| A0, A2, A3 | at most 6 s | under 1,300 | under 25 MB | 20 to 40 s |
| A1 | 60 s and a resumed tail | about 25,000 | 0.5 GB | 1 to 2 min |
| A4, A5 | 420 s and a resumed or rewound tail | about 117,000 | 2.2 GB | 3 to 6 min, plus an unmeasured resume |
| B1 to B8, C1 to C7, F1, F2 | 900 s | about 191,000 | 3.6 GB | 3.4 to 6.6 min |
| D0, D1 | 1,800 s | about 382,000 | 7.1 GB | 6.4 to 12.6 min |
| G1 | 900 s | up to 290,000 | up to 5.4 GB | 5 to 10 min |
| G2 | 1,800 s | up to 570,000 | up to 10.6 GB | 10 to 20 min |
| G3 | 180 s | 180,000 or more | 3.3 GB or more | 10 to 20 min |
| G4 | 180 s | about 38,000 | 0.7 GB | 1 to 1.6 min |

In all: 29 runs, about 6.5 simulated hours, 2 to 4 process-hours, about 1 to 2 hours of wall time at 3 workers, flat memory projected at tens of MB a run (the reference core peaked at 4.8 MB in ticket 11's measurement; Core is unmeasured), at most about 25 GB of transient links at once, and about 1 GB of kept captures. The release build takes 5 to 15 minutes more.

## Results template

The Answer copies this block and fills every field.

```text
Input: commit C <..>; archive sha256 <..>; crates tree <..>; engine as designed (against 84744af): <yes/no, and what moved>
Toolchain: rustc <..>, cargo <..>, python3 <..>; rust-src present: <yes/no/unknown>
Machine: the heavy-run machine; logical processors <..>; memory limit <..>; swap limit <..>

| Check                 | Runs    | Measured                                             | Criterion                 | Verdict |
| A0 pinned second      | A0      | packet <..>, seal <..>, head <.. ms>                 | the pinned values         |         |
| A1 across checkpoint  | A1      | equal seals from revision <r> to <r'>                | all equal                 |         |
| A2, A3 reflex         | A2, A3  | traffic <..> vs <..>; rate <..>; leg-vein water <..>; stretch gap <..> | the reference test's bars |  |
| A4 late checkpoint    | A4      | equal <..>; resume <s>                               | all equal                 |         |
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

| Run | simulated s | accepted steps | per simulated s | refusals | admission s | wall s | times real time | peak MB | links GB | stir s | skin factor |

Technical gate: <pass/fail>. Informativeness gate: <pass/fail, which rows>.
Derived: teaching core <..>; history <..> s; encounter <..> min; workers <..>; rule low <..>, high <..>; plan size <..>.
Evidence: summaries.sha256 <..>; report.txt's sha256 is recorded beside it; the data stay on the heavy-run machine.
```

## Acceptance

- [ ] The code PR adds `gp.py audit`, the plan and the report, and its proof from a clean git checkout on the heavy-run machine shows `full_proof: yes`.
- [ ] The audit has run from C, and the Answer holds every field of the template, both gates' verdicts and every derived setting.
- [ ] If either gate fails, the Answer says which rows failed, and the next ticket that depends on this one stays blocked until a person reads it.

## Non-goals

- The reduction's balance and the reversal share per transfer (ticket 05 adds them to this report).
- Training data, models, and any change to an expectation the workspace pins.

## Comments

The code PR merged as nightingale#85: `gp.py audit`, the plan `tools/generative-physiology/spec/audit-v1.txt`, the report and the driver's capture version 2 are on main. The first acceptance box is met. What remains is the audit itself: the long runs from main on the heavy-run machine, which wait for the developer reviewer's go-ahead because their transient links need about 100 GB of its disk, and then the Answer.

The long runs now also wait for two changes to the teacher, so that they run once. [Native-contract 28](../../native-contract/issues/28-admission-energy-at-balance.md) corrects the start state that admission gives every run. The arena's pick, which the developer reviewer confirms ([foundation map](../../foundation/map.md), "Where the route stands"), decides which core teaches. By this ticket's engine-input rule an audit run before either would run again, and a study frozen on an earlier C would learn from a start state the engine is about to correct. C is therefore the first main commit that holds both. If native-contract 28 moves the pins in `crates/ng-validate/tests/trace.rs`, A0's digests in this ticket and in the plan are read again at C. Later engine changes do not reach the study, which runs on the snapshot at C.

The audit ran once at bd38c0a5360addc7d5c2be2ffec4338a14e5c402 on the developer reviewer's go-ahead, with native-contract 28 in and the arena's pick still to come. Its technical gate fails. This run is evidence, not this ticket's Answer: the Answer still comes from a run at the C that the comment above defines. The whole audit took 24 minutes at 3 workers, and Windows C: free space went from 51.9 GB to 51.6 GB, so the next run costs little.

The technical gate fails because B1 to B6 fault with `NoAdmissibleStep` at 509.772053 s, 29.77 s after head-down (trunk -0.2, horizontal legs) at 480 s. The six courses are equal seal for seal through revision 106,937, so the replica, late and observer-off variants reach the same fault, and B2 and B3 fault before their divergence at 510 s. Because B1 faulted, the prefix row and the B4, B5 and B6 row cannot pass, and the faults row counts six. These six are the plan's only runs that go head-down, and every other run completes. Five diagnostic runs outside the audit's data find the cause. The slice's leg veins empty within 30 s of head-down, after any history, and the next step's upwind transfer from them divides by zero water. The reference integrator at 1 ms faults within 1.2 ms of Core, so the cause is in the model and no core fixes it. [Native-contract 33](../../native-contract/issues/33-leg-veins-empty-head-down.md) takes the question to the engine's process.

Every other technical row passes. A0 reproduces the pins `crates/ng-validate/tests/trace.rs` holds at C, and the checkpoint, rewind and reflex rows hold. B7 and its replay B8 share one seal log over 18 actions. Closure stays within 3.6e-12 of its bound, and the largest beat-identity gap is 8.4e-4 against the bar of 1e-3. The informativeness gate passes. The start histories differ on all four response features, and the slow component resolves with tau 631 s, which would give a 300 s history and a 10-minute encounter.

The measurements add three facts:

- Core runs the slice at 5.3 to 7.7 times real time, against the projected 2.5 to 4.9. An admission takes about 14 s, peak memory stays near 53 MB, and a 900 s run writes about 1.8 GB of links.
- The cores row is flagged. Core (G4) differs from the reference integrator at 1 ms (G3) by up to 0.95 in beat-mean pressure and 0.055 per s in heart rate, and both exceed 2% of G4's stand response, a 6.42 pressure fall and a 0.90 per s rate rise.
- The tilt times resolve only to whole beats. In F1 and F2, stroke and heart rate pass 63% of their change on the same beat, so their times are equal, and ticket 09's outside check cannot order them at this resolution.

The question for the developer reviewer is whether to wait for native-contract 33 and the arena's pick and then run the audit again, or to drop head-down from the study's settings: B1 to B6 here, and ticket 09's o4 family and its head-down branch. The model default is to wait, because head-down is the study's only setting with a negative trunk, and ticket 09 uses it to test a setting no training family reaches.

The workstation's facts for the template's first line: archive sha256 1ee726ebfbf224aacfef1c78ae63cc86c9d392842eecdb515a5bf6e7deba8b1a, the same on the heavy-run machine; crates tree e8d6f9ce7b707d1e16444ac2e263025ab56d610b; engine as designed (against 84744af): no. Seventeen commits moved it, native-contract 12 to 28, body ticket 02 and nightingale#108, and A0's pins moved with them. The report's own text, sha256 0f7ba481448e61afa391426e96c4fca070d8d3ac3ade6047eda4c8296d6c83d4, with its summaries.sha256 at 635bec7b014b0270fbedb9ae27bb327c3e9100f28e60f2d691e05e92f0377e11:

```text
Input: commit C bd38c0a5360addc7d5c2be2ffec4338a14e5c402; archive sha256, crates tree and engine as designed (against 84744af): the Answer records them from the workstation
Toolchain: rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1 (797e8a9bc 2026-08-05), python3 3.14.7; rust-src present: yes
Machine: the heavy-run machine; logical processors 12; memory limit 42061242368 bytes; swap limit 4294967296 bytes

| Check | Runs | Measured | Criterion | Verdict |
| A0 pinned second | A0 | packet 2dde20ac..045b, seal c4821276..ed7e, head 1000.229928 ms | trace.rs:135-136, 147 | pass |
| A1 across checkpoint | A1 | equal seals from revision 277 to 13398 | all equal | pass |
| A2, A3 reflex | A2, A3 | traffic 0.72067 vs 0.451838; rate 1.76645 vs 1.01558; leg-vein water 565.442 vs 470.539; stretch gap 0.105000000000 | the bars of the_body_answers_a_stand_through_its_own_baroreflex | pass |
| A4 late checkpoint | A4 | equal seals from revision 60755 to 88068; resume 3.042 s | all equal | pass |
| A5 rewind | A5 | equal seals from revision 60755 to 88068 | all equal | pass |
| B1/B2, B1/B3 prefixes | B1, B2, B3 | B1 faulted (NoAdmissibleStep) | 510 000 / just before | fail |
| B4, B5, B6 | B1, B4, B5, B6 | B1 faulted (NoAdmissibleStep) | equal | fail |
| B7/B8 closed loop | B7, B8 | equal end to end, 175740 revisions; actions 18 | equal | pass |
| Faults, identities | all | 6: B1 NoAdmissibleStep, B2 NoAdmissibleStep, B3 NoAdmissibleStep, B4 NoAdmissibleStep, B5 NoAdmissibleStep, B6 NoAdmissibleStep; shared | 0; shared | fail |
| Closure, beat identity | all | max residual 3.62716e-12 of its bound (Charge at amount/bladder/Lumen/Sodium, 35e6077a edges 6 to 7); max beat gap 0.000837558 (934d0200 beat 521) | bound; 1e-3 | pass |
| Histories differ | C1-C6 | ranges 0.162444, 6.39686, 0.0799009, 1.04083 against floors 0.0245904, 1.47056, 0.00500229, 0.204124 | two of four | pass |
| Slow component | D0, D1 | tau 630.957 s, b -87.8677, residual over abs(b) 0.0016621, span 1.05925 | resolved or not | resolved |
| Tilt | F1, F2 | F1 stroke 3.5345/3.26609 s, rate 3.5345/3.26609 s; F2 stroke 8.32229/7.55792 s, rate 8.32229/7.55792 s | measured | measured |
| Cores | G3, G4 | max d pressure 0.950034, max d rate 0.0551023; G4's stand: fall 6.42126, rise 0.896082 | flag above 2% | flagged |
| Numerical variation | G1, G2 | per feature rate rise 0.0245904, pressure fall 1.47056, late rate 0.00500229, late pressure 0.0978228; leg interstitium 0.011365 | measured | measured |
| Ripple | C1-C6 | RMS 1.04601 over 5388 windows against the stand response 6.26203 to 12.6589 | measured | measured |
| Accumulated postures | C7, C3 | wall per simulated s 0.161636 against 0.155216 | measured | measured |

| Run | simulated s | accepted steps | per simulated s | refusals | admission s | wall s | times real time | peak MB | links GB | stir s | skin factor |
| A0 | 1.1002 | 245 | 222.687 | 21 | 13.235 | 0.159 | 6.91948 | 53.3053 | 0.00229292 | 0.528060 | no beat |
| A1 | 118.748 | 26519 | 223.321 | 2760 | 14.64 | 18.663 | 6.36277 | 53.334 | 0.124892 | 0.528060 | 1.10192 to 1.15358 |
| A2 | 6.00074 | 1131 | 188.477 | 94 | 14.237 | 0.784 | 7.65401 | 53.3873 | 0.0105513 | 0.528060 | 1.15103 to 1.15357 |
| A3 | 6.00424 | 1276 | 212.517 | 124 | 13.456 | 0.863 | 6.9574 | 52.0069 | 0.0119029 | 0.528060 | 1.15108 to 1.15358 |
| A4 | 550.016 | 115381 | 209.777 | 10182 | 15.428 | 84.291 | 6.52521 | 52.5926 | 0.820891 | 0.528060 | 0.838134 to 1.1536 |
| A5 | 550.016 | 115381 | 209.777 | 10182 | 14.357 | 85.743 | 6.41471 | 52.6377 | 0.820891 | 0.528060 | 0.838134 to 1.1536 |
| B1 | 509.772 | 106937 | 209.774 | 9667 | 14.559 | 77.034 | 6.61749 | 51.0239 | 0.996769 | 0.528060 | 0.838134 to 1.1536 |
| B2 | 509.772 | 106937 | 209.774 | 9667 | 14.2 | 76.593 | 6.6556 | 51.1713 | 0.996769 | 0.528060 | 0.838134 to 1.1536 |
| B3 | 509.772 | 106937 | 209.774 | 9667 | 14.948 | 76.877 | 6.63101 | 50.9747 | 0.996769 | 0.528060 | 0.838134 to 1.1536 |
| B4 | 509.772 | 106937 | 209.774 | 9667 | 15.542 | 76.805 | 6.63722 | 50.9747 | 0.996769 | 0.528060 | 0.838134 to 1.1536 |
| B5 | 509.772 | 106937 | 209.774 | 9667 | 14.236 | 75.779 | 6.72709 | 52.609 | 0.996769 | 0.528060 | 0.838134 to 1.1536 |
| B6 | 509.772 | 106937 | 209.774 | 9667 | 14.308 | 77.211 | 6.60232 | 52.6008 | 0.996769 | 0.528060 | 0.838134 to 1.1536 |
| B7 | 900.004 | 175740 | 195.266 | 14716 | 14.409 | 122.687 | 7.33577 | 51.3229 | 1.63808 | 0.528060 | 0.838134 to 1.27989 |
| B8 | 900.004 | 175740 | 195.266 | 14716 | 14.269 | 125.943 | 7.14612 | 52.6664 | 1.63808 | 0.528060 | 0.838134 to 1.27989 |
| C1 | 900.008 | 180869 | 200.964 | 15676 | 14.386 | 133.141 | 6.75981 | 52.6664 | 1.68589 | 0.528060 | 0.838134 to 1.27989 |
| C2 | 900.009 | 189091 | 210.099 | 16983 | 14.433 | 139.833 | 6.43631 | 53.3504 | 1.76253 | 0.528060 | 0.838135 to 1.27989 |
| C3 | 900 | 192192 | 213.547 | 16582 | 14.179 | 139.694 | 6.44266 | 53.2808 | 1.79143 | 0.528060 | 0.838134 to 1.27989 |
| C4 | 900.007 | 186525 | 207.248 | 16317 | 14.119 | 131.375 | 6.85067 | 53.248 | 1.73861 | 0.528060 | 0.838134 to 1.27989 |
| C5 | 900.004 | 190711 | 211.9 | 16755 | 14.446 | 136.86 | 6.5761 | 53.2357 | 1.77763 | 0.528060 | 0.838134 to 1.27989 |
| C6 | 900.005 | 185482 | 206.09 | 16909 | 14.444 | 133.91 | 6.72097 | 53.2972 | 1.72889 | 0.528060 | 0.838134 to 1.27989 |
| C7 | 900.003 | 195100 | 216.777 | 16923 | 14.37 | 145.473 | 6.18673 | 53.2685 | 1.81854 | 0.528060 | 0.838134 to 1.27989 |
| D0 | 1800.01 | 336775 | 187.097 | 27015 | 14.33 | 235.216 | 7.65256 | 59.9368 | 3.13909 | 0.528060 | 0.838134 to 1.27989 |
| D1 | 1800 | 408412 | 226.895 | 36826 | 14.364 | 305.692 | 5.88829 | 55.0339 | 3.80682 | 0.528060 | 0.838134 to 1.27989 |
| F1 | 900.001 | 172835 | 192.039 | 15245 | 14.242 | 121.678 | 7.39658 | 56.0087 | 1.611 | 0.528060 | 0.838134 to 1.27989 |
| F2 | 900.007 | 172916 | 192.127 | 15282 | 14.152 | 121.845 | 7.38649 | 53.2644 | 1.61176 | 0.528060 | 0.838134 to 1.27989 |
| G1 | 900.004 | 241141 | 267.933 | 10382 | 14.388 | 158.893 | 5.66421 | 53.2726 | 2.24768 | 0.528060 | 0.838134 to 1.27989 |
| G2 | 1800 | 498713 | 277.062 | 23714 | 14.507 | 339.782 | 5.29753 | 55.1363 | 4.64851 | 0.528060 | 0.838134 to 1.27989 |
| G3 | 180 | 181930 | 1010.72 | 0 | 14.304 | 350.431 | 0.513655 | 53.207 | 1.69578 | 0.528052 | 0.945488 to 1.15357 |
| G4 | 180.001 | 38203 | 212.238 | 3568 | 14.821 | 27.311 | 6.59079 | 52.3346 | 0.356099 | 0.528060 | 0.945206 to 1.15357 |

Technical gate: fail. Informativeness gate: pass.
Derived: teaching core none: the technical gate failed and the experiment stops here.
Evidence: summaries.sha256 635bec7b014b0270fbedb9ae27bb327c3e9100f28e60f2d691e05e92f0377e11; report.txt's sha256 is recorded beside it; the data stay on the heavy-run machine.
```

This ticket once quoted A0's pins and their line numbers as they stood at 84744af. Native-contract 28 (nightingale#91) moved the pins, so the Usage check printed 0 at every later C, and body ticket 02 (nightingale#103) moved their lines. The ticket now names the test that holds the pins, and Usage prints them at C. The report above stays as the tool wrote it. Its A0 criterion, `trace.rs:135-136, 147`, is the label in `tools/generative-physiology/audit.py`, which still cites the lines of 84744af, while the check behind it reads the test by its name. The next code change to that tool corrects the label, and [ticket 05](05-reduction-and-balance.md)'s acceptance holds it.

The question above was decided by the oversight seat when it parked the line, under the developer reviewer's delegation: the audit waits, and head-down stays in the plan. The line is parked until the whole body lives two synthetic days ([the map](../map.md)). This audit stands outside the parking, because its first run found a defect that the engine's own gates missed. It runs again on the slice once native-contract 33 and the arena's pick land. The Blocked by line names native-contract 33. The pick has no ticket of its own ([foundation map](../../foundation/map.md), "Where the route stands", item 2), so a seat checks there that the pick has landed before it starts. The rerun's derived settings serve the slice's posture experiment, and [ticket 14](14-first-experiment-on-the-whole-body.md) decides whether that experiment stays.

Since the first run, the heavy-run machine has been rebuilt to run Linux with no disk image, and that run's directories went with the rebuild. The report's text and both digests are above, and the next run makes new data. With no WSL, the rerun leaves `--disk` out, and the audit checks the free space of its data directory (`tools/generative-physiology/audit.py:873-879`).
