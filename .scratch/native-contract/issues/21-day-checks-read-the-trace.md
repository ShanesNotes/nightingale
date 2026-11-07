# 21: The day's checks read the trace

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The validation crate runs checks drawn from the contract's day table (§6, `docs/architecture/solver-contract.md:329`) on a run's trace, as deterministic tests with no judge opinion. Each check reads one quantity around one event. It compares the mean over a window after the event with the mean over a baseline before it, and reports pass, fail or "not available yet". The rows, which of them score, and how each one is computed are in the comments below.

Blocked by: 11, 12

Status: resolved

- [x] Each check in the table below reads a slice run's trace and reports pass, fail or not available yet, and a trace built with the wrong direction fails each scored check

## Answer

`ng_validate::day_checks(body, core, spec, store)` returns a `DayReport`: one check for each quantity of each row of the table below at each occurrence of the row's event, each `Pass`, `Fail(Miss)` or `NotAvailableYet(Unavailable)`, and a line per check through `Display`.

- **Each check in the table below reads a slice run's trace and reports pass, fail or not available yet, and a trace built with the wrong direction fails each scored check.** On the tower, `tools/bench`'s `day 10 ng-solver 2100` runs the slice from 06:55 to 07:30 with the day plan's sit-up at 07:00 and prints every row. Rows 1, 3, 5 and 6 pass. Row 2 fails with no dip past its band, row 4 with no rise past the declared tolerance, and row 7 with a change of -0.41 per s against a tolerance of 5.5 per s. Rows 8 and 11 to 13 are unscored, rows 9 and 10 not reached, and rows 14 to 17 have no sleep payload. These are the solver arena script's verdicts on the same core. The run took 308 s of wall time, 2859 beats and 8.3 GB of link log (measured). Fed through the same fold, the arena's own trace gives the script's numbers to three or four figures. `each_scored_row_passes_its_way_and_fails_with_the_move_reversed` (`crates/ng-validate/src/day/tests.rs`) builds, for each of the 13 scored rows, a synthetic trace that moves the row's way and passes, and the same trace reversed, which fails: each phase of rows 2 and 4 and each quantity of rows 15 to 17 alone. `the_registry_states_the_tickets_directions_and_bounds` pins the directions and bounds those traces are built from. `a_slice_run_checks_every_row_and_scores_none_before_a_window_closes` (`crates/ng-validate/tests/day.rs`) runs the slice across 07:00: the sit-up applies and its rows wait for 07:30.
- The day plan (`DayPlan::reference`) makes both the runs' boundary events and the occurrences the checks read: sit-up at 07:00 (main's 21-event ramp), meals at 07:30, 12:30 and 18:30 (synthetic amounts through the gut), lying down at 23:00, waking at 06:30. No payload carries sleep, so rows 14 to 17 are not available yet on a slice run; their logic runs on synthetic traces. An occurrence counts only once the step receipts show every one of its inputs applied.
- `day_checks` admits once with an empty port, checkpoints, and resumes that start for each run, enqueueing the run's plan at its head. A meal whose 4 h window the run covers also gets a counterfactual run from the same start with that meal's inputs withheld. The trace folds each accepted step into whole beats, from one beat edge to the next, and keeps nothing per step: the step-length-weighted mean of each series' step-end values and of its declared tolerance. Stroke volume is a beat's integral of aortic flow. The tolerance is `ErrorScales::at(value, natural size)` per row (a sum's size is its rows' sum), carried through each ratio and product. Sodium excretion is the net sodium the ledger moved along the collecting duct to bladder column over each step, per second, read from the history head, never recomputed from the flow. Its tolerance carries the flow's and the donor's concentration's through the product, and the donor is the end the sodium left by the sign of that transfer, the bladder for a reversed flow (`a_reversed_urinary_flow_excretes_what_the_ledger_moved_with_the_bladders_floor`; `the_trace_reads_the_sodium_the_engine_carried_from_the_duct_to_the_bladder` checks the read against flow times duct concentration on the arena body). A body without that column fails `day_checks` with `DayFailure::MissingTransfer`. The blood spaces are the places albumin sits in a lumen or plasma: the 10 vessel lumens and 5 bed plasma spaces the arena read.
- The method, as these comments set it. Baseline: the 10 min before the event, cut at the trace's start. Comparison: from the event to the next occurrence, and at most 4 h for a meal. A response is more than 3 baseline SDs and at least the larger of the two windows' tolerances. An onset stays out 3 beats (a bound in seconds) or through the beat that holds onset plus 5 min (minutes or hours), inside the window. Each phase of rows 2 and 4 must clear the same floors as a response, more than 3 baseline SDs and at least the baseline's tolerance, and its bound holds its onset. A meal compares the fed run's window with the counterfactual's, less their difference before the meal, and pairs beats by the instant a fed beat ends, never by count. Row 17 needs the hour before waking to lie below the hour before lying down, then a rise. Unscored rows read nothing.
- Readings where the crate differs from the arena script, each on purpose: the step that ends at a beat edge closes its beat, which moves onsets by one step; a change equal to the tolerance is a response; a slow stay has no 2 s grace; a phase's stay may run past its bound inside the comparison window (the script kept the rise's and the dip's stays inside 30 s); osmolality has a floor.
- `ErrorScales::at` and `DeclaredResolution::scales` are public (one `f64_allow` row), so a check reads the core's own declared tolerance.
- Not in this ticket: a real meal run (about two 4.6 simulated hour runs), a sleep payload, sources for rows 8 and 11 to 13, the SRS's pick definition, and every row's citation check, all still open as the comments above say.

## Comments

The developer reviewer's call for this arena (2026-11-02): the 13 rows whose Scores column reads Yes or Direction only pass or fail, and rows 15 to 17 score their direction only. Row 8, rows 11 to 13, the timing of rows 15 to 17 and any row the slice cannot produce yet show "not available yet" and never pass or fail. The scores are evidence that the engine's structure produces the right responses. They are not clinical validation.

Every row below comes from a literature draft, and each row's citation check is still open. The table gives directions and timing only, never a target value.

| # | Event | Quantity | Direction | Onset or settling | Scores | Source |
|---|---|---|---|---|---|---|
| 1 | 07:00 supine to seated | Heart rate | Up | Onset under 15 s, settles in 30 s to 20 min | Yes | Smith 1994 |
| 2 | 07:00 | Mean arterial pressure at heart level | Dips, then recovers to supine or above | Dip in 5 to 10 s, recovery under 30 s | Yes | Smith 1994 |
| 3 | 07:00 | Stroke volume | Down | Falls over the first 30 s, steady by about 5 min | Yes | Smith 1994 |
| 4 | 07:00 | Cardiac output | Brief rise, then down | Down by 5 to 20 min | Yes | Smith 1994 |
| 5 | 07:00 | Plasma volume | Down | Over about 10 to 20 min, then stable | Yes | Hagan 1978, Jacob 1998 |
| 6 | 07:00 | Plasma albumin concentration | Up | Over about 10 to 20 min | Yes | Hagan 1978 |
| 7 | 07:00 | Renal sodium excretion | Down | Within 30 min | Yes | Jacob 1998 |
| 8 | 07:00 | Filtration, or creatinine clearance | The contract says slightly down, which is disputed | Not sourced | Not available yet | Kamper 1988 found no change from supine to sitting |
| 9 | 07:30, 12:30 and 18:30 meals | Plasma osmolality | Up | Over 1 to 4 h | Yes | Gill 1985 |
| 10 | Meals | Plasma sodium concentration | Up | Over 1 to 4 h | Yes | Gill 1985 |
| 11 | Meals | Vasopressin surrogate, V2-bound | Up, as the contract says | Not sourced | Not available yet | None found for a mixed meal in healthy adults |
| 12 | Meals | Renal sodium, potassium and chloride excretion | Up | Over hours | Not available yet | Not sourced |
| 13 | Meals | Plasma potassium | As the contract says (FU §3a) | Not sourced | Not available yet | Not found. The insulin potassium shift is declared external |
| 14 | 23:00 seated to supine, with sleep onset | Plasma volume | Up | Over about 20 to 35 min | Yes | Hagan 1978 |
| 15 | 23:00 | Mean arterial pressure and heart rate | Down | As NREM sleep deepens. Timing not sourced | Direction only | Somers 1993 |
| 16 | 23:00 | Urine flow and sodium excretion | Down | Over the night. Timing not sourced | Direction only | Kamperis 2010 |
| 17 | 06:30 sleep offset | Mean arterial pressure and heart rate | Up, as the night's dip reverses | Before 07:00. Timing not sourced | Direction only | Somers 1993 |

Sources, each still owing its citation check:

- Smith JJ, Porth CM, Erickson M. Hemodynamic response to the upright posture. J Clin Pharmacol 1994. doi:10.1002/j.1552-4604.1994.tb04977.x, PMID 8089249.
- Hagan RD, Diaz FJ, Horvath SM. Plasma volume changes with movement to supine and standing positions. J Appl Physiol 1978. doi:10.1152/jappl.1978.45.3.414, PMID 701126.
- Jacob G et al. Effect of standing on neurohumoral responses and plasma volume in healthy subjects. J Appl Physiol 1998. doi:10.1152/jappl.1998.84.3.914, PMID 9480952.
- Kamper AL et al. The influence of body posture on lithium clearance. Scand J Clin Lab Invest 1988. doi:10.3109/00365518809085765, PMID 3146132.
- Gill GV, Baylis PH, Flear CT, Lawson JY. Changes in plasma solutes after food. J R Soc Med 1985. doi:10.1177/014107688507801206, PMID 4067972.
- Somers VK, Dyken ME, Mark AL, Abboud FM. Sympathetic-nerve activity during sleep in normal subjects. N Engl J Med 1993. doi:10.1056/NEJM199302043280502, PMID 8419815.
- Kamperis K et al. Excess diuresis and natriuresis during acute sleep deprivation in healthy adults. Am J Physiol Renal Physiol 2010. doi:10.1152/ajprenal.00126.2010, PMID 20519376.

The posture papers study standing or head-up tilt, not sitting, so a seated response takes the same direction and is smaller. Somers reports by sleep stage, not by clock time. The slice does not model the thermal set point, metabolism and diet-induced thermogenesis, cortisol and hepatic glucose, the respiratory chemoreflex (contract §6) or the insulin potassium shift, so none of them has a check.

How each check is computed:

- **Baseline.** The 10 min before the event. For row 17 the awake reference is 22:00 to 23:00 of the night before, seated and awake, and the asleep baseline is 05:30 to 06:30.
- **Comparison.** The row's window, measured from the event's instant.
- **Noise floor.** The standard deviation of the quantity over its own baseline window, after the beat-to-beat cycle is removed: a mean per cardiac cycle for a pulsatile quantity. A response is a change in the comparison window's mean of more than 3 times that deviation, and of at least the solver's declared tolerance for that row.
- **Direction.** The sign of the comparison mean minus the baseline mean. "Unchanged" passes when the change stays within the noise floor.
- **Onset.** The first time the per-cycle value leaves the baseline mean by 3 deviations in the stated direction and stays out. A fast check, timed in seconds, needs 3 cycles out. A slow check, timed in minutes or hours, needs 5 min out. The onset must fall inside the window.
- **Overlap.** A window ends at the next event's instant, so the 07:00 checks use 07:00 to 07:30 only. A meal check compares against the same clock window of a counterfactual run with that meal removed and the same packet, seed and timeline otherwise, so the posture change and earlier meals cancel. The 23:00 posture change and sleep onset share one instant, so their checks score together, as the contract calibrates them together.

Open before these scores enter the arena's pick:

- The SRS defines the arena's pick on conservation, accuracy, replay and speed (`docs/srs.md` section 2, and "Arena" in `GLOSSARY.md`). That definition changes before these scores decide a pick.
- AGENTS.md's behavior rule lets only checks drawn from anchors and requirements judge physiology. The directions come from the contract's day table, except row 4's brief rise. The timing windows are not yet an anchor or a requirement.
- Row 8's direction is disputed. The contract says filtration falls slightly on sitting, and `crates/ng-validate/tests/arena_body.rs:799` asserts that it falls, by under 5 percent. The one source found reports no change from supine to sitting.
- Day one starts at 06:55, so its 07:00 checks have 5 min of baseline, not 10. Day two has the full 10 min.
- Rows 14 to 17 need sleep onset and offset, and no ticket builds sleep or clock payloads yet.

What the slice reads today: `arena_lens` (`crates/ng-validate/src/slice.rs:917`) reads heart rate, arterial pressure and filtration. The checks also read stroke volume, cardiac output, plasma volume, plasma albumin and sodium concentrations, plasma osmolality, renal sodium excretion and urine flow.

The retuning lead, from ticket 10's tuning and nightingale#32: if a scored row fails on the slice, the fix is a law or the hand-set `table` (`crates/ng-validate/src/slice.rs:638`), and both cost tuning. Ticket 10 tuned `table` by hand over about 25 runs of 1 to 2 min each, about 1.6 h in all, and left 38 tuning logs outside the repo. A 20 min simulated stand took about 16 min of wall time. The stand's filtration bound (`crates/ng-validate/tests/arena_body.rs:799`) lets filtration fall by under 5 percent. Ticket 10 first failed it on sitting, at 2.00733 against 2.00725 supine, and passed after raising the kidney sites from 0.35 to 0.38 m. A throwaway arm fed renal sympathetic drive into tubular sodium reclaim at an arbitrary gain of 0.1. Once the peritubular bed had a nerve supply, the arm moved the trace and failed that bound. Row 7 is where such an arm matters, because today the renal nerve supplies only the glomerular bed (`crates/ng-validate/src/slice.rs:140`) and the peritubular bed is unsupplied (`:149`). Each new loop costs tuning, not code.

From the solver arena (in this PR): a scoring script applied the method above to a 06:55 to 07:30 trace with main's sitting ramp at 07:00, for two cores. Both cores gave the same verdicts: rows 1, 3, 5 and 6 pass; rows 2, 4 and 7 fail; the rest are not available yet.
- Row 2 fails in the slice itself: mean arterial pressure never dips past its noise floor, under the reference core too. From 30 s to 07:30 it averages 103.6 against 100.8 supine.
- Row 4's brief rise in cardiac output peaks at +1.8 to +2.4 mL/s, about the declared aortic-flow tolerance (2.06 mL/s at scales 0.1 and 1e-3), so it does not stay out for three beats. The fall passes.
- Row 7's sodium excretion starts falling 2 min after 07:00 and averages 38 percent lower to 07:30, but its declared-tolerance floor is about 5.5 per s against an excretion of about 1.1 per s: the packet gives the urinary flow rows no natural size, so the absolute scale counts 0.1 mL/s on a flow of 0.02 mL/s. Natural sizes for those rows, or tighter scales, would make it scorable.
- The floor moves with a core's declared scales. At main's test scales (1.0, 1e-3), it is about 1 per s on the heart-rate row and 20 mL/s on aortic flow, which would hide rows 1, 3 and 4.
- Readings the script had to choose, which this ticket settles: a cycle runs from one wrap of the CV phase to the next, and a cycle's time is its end; day one's baseline is the 5 min after admission; each phase of rows 2 and 4 has its own window and must clear both 3 SD and the declared tolerance; the timing bounds come from the table (row 1 under 15 s, row 2's dip by 10 s and recovery by 30 s, row 3 by 30 s, row 4's rise by 30 s and fall by 20 min, rows 5 and 6 by 20 min, and row 7's 5 min stay inside 07:30). "Settles" has no method, so it was not scored.

From ticket 11: a run long enough for the meal rows (to 11:30, about 4.6 simulated hours) can now hold its history outside memory. `tools/bench` measured the slice flat at about 4.8 MB resident from 1154 to 11578 accepted steps, with 18584 bytes a step on disk, so a meal run needs about 65 GB of disk at the arena's 212 steps a simulated second, by arithmetic.

From 17: the skin's water loss of 0.01 mL/s now varies by up to half its size through a seeded forcing with a 300 s period. That changes plasma volume by under 0.005 mL/s (arithmetic), so a trace's baseline window now holds that much seeded variation. The stir hazard moves no law in the slice.

From ticket 12: `run` counts the waveform's released values and keeps no observation, under a constant observation seed (`WAVEFORM` in `crates/ng-validate/src/run.rs`). A check that reads the trace reads the returned session or extends `run`.
