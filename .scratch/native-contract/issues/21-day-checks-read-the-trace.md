# 21: The day's checks read the trace

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The validation crate runs checks drawn from the contract's day table (§6, `docs/architecture/solver-contract.md:329`) on a run's trace, as deterministic tests with no judge opinion. Each check reads one quantity around one event. It compares the mean over a window after the event with the mean over a baseline before it, and reports pass, fail or "not available yet". The rows, which of them score, and how each one is computed are in the comments below.

Blocked by: 11, 12

Status: ready-for-agent

- [ ] Each check in the table below reads a slice run's trace and reports pass, fail or not available yet, and a trace built with the wrong direction fails each scored check

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

From ticket 12: `run` counts the waveform's released values and keeps no observation, under a constant observation seed (`WAVEFORM` in `crates/ng-validate/src/run.rs`). A check that reads the trace reads the returned session or extends `run`.
