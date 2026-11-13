# 45: Rows 4 and 7 read a declared tolerance set for the step, not for the response

Spec: [native-contract 21](21-day-checks-read-the-trace.md)'s method ("of at least the solver's declared tolerance for that row") and its comment on row 7, `crates/ng-validate/src/day/judge.rs` (`Change::sign`, `clears`), `crates/ng-validate/src/day/trace.rs` (`series`), and `crates/ng-validate/src/slice.rs` (`FLOW_SIZE`, the row scales in `arena_packet`)

Type: task

Status: open

Blocked by: none

A day check counts a response only when it is at least the declared tolerance, `ErrorScales::at(value, natural size)`, which is 0.1 times the row's natural size plus 1e-3 times its value at the arena's scales. Two sit-up rows fail on that floor alone. Both fail the same way in the pick dry run at `43afdb7` and on main `c4760e8` (`engine-bench day 10 ng-solver 2100`, ryzen, identical lines).

- **Row 7, sodium excretion down within 30 min.** Excretion falls from 1.00 to 0.45 a second by 07:30, a mean change of -0.442 against 3 noise floors of 0.002 and an onset of 11.6 s. The declared tolerance is 5.01, because the packet gives the urinary flow rows no natural size: the default of 1 allows 0.1 on a flow of 0.02, and the duct's sodium concentration of about 50 carries that into the product. Ticket 21's comment saw this on the arena script before the crate existed.
- **Row 4, cardiac output's brief rise by 30 s.** Mean aortic flow rises 0.85 to 7.2 s after 07:00, by up to +2.18 a second for one beat, and stays over 3 noise floors for eight beats. The declared tolerance is 2.058: 0.1 times `FLOW_SIZE`, the valves' and circulation rows' natural size of 20, set for the step's error, plus 1e-3 times 58. With 3 noise floors as the floor, the rise passes from 0.85 s and the fall passes as it does today.

`Core::doubled` gives the same responses as `Core::new`, within 0.006 on row 4's rise and 1e-4 a second on row 7's excretion, so both responses are the body's and not either core's error. Every core on the arena's protocol declares the same scales, so no core can pass either row as the floor stands.

No ruling of the developer reviewer on ticket 21's method is recorded, so the first box below is a model default, open to review.

This ticket lands before [body 24](../../body/issues/24-the-synthetic-day.md) runs the day on the whole body.

Evidence: `~/lanes/logs/ng-wave-4/sodium/` (`REPORT.md`, and `probe/rows-p2-plan-new.txt` and `probe/rows-p2d-plan-doubled.txt`).

**What to build:** a floor that tells a response from a core's error without hiding a response the cores agree on.

- [ ] Try (a) first, on the short run: the packet states natural sizes for the urinary flow rows, and row 4's floor reads a size set for judging aortic flow rather than the step's. Take (b) only if (a) cannot tell a response from the cores' disagreement: the floor is the spread between the reference integrator's rungs on that quantity, which is what a core's error looks like on the run itself, and the PR says what the reference rungs then cost over the window. (c) is the last resort: the declared tolerance stays, and rows 4 and 7 become not available yet with this reason
- [ ] Rows 4 and 7 pass or fail on the short run from 06:55 to 07:30 under the choice, and a synthetic trace inside the old floor and outside the new one shows the change in `day/tests.rs`
- [ ] A natural size added to the packet moves the slice's run identity; the PR says so in one sentence

**Non-goals:** the step's error control; `FLOW_SIZE` for the step; the other rows.
