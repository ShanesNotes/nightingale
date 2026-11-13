# 11: Breathing is controlled

Type: task
Status: open
Blocked by: 05, 10, `.scratch/research/issues/11-law-forms-gas-loop.md`

**What to build:** The nervous system sets the drive to breathe. A central sensor reads the acidity that blood chemistry states for a brain space from ticket 08, which follows carbon dioxide while the anions are held, and a peripheral sensor reads arterial oxygen and carbon dioxide. The drive reaches the pump as release at its junctions. Sleep shifts the set point and lowers the gain. This closes the gas loop with its own controller.

**Releases:** the held drive of ticket 10, and coupling arrows 83 (stretch from the lung), 86 (respiratory drive) and 88 (motor drive to the pump).

- [ ] Tests: each sensor's response matches its form at stated points
- [ ] Run: a 10% rise in carbon dioxide production moves arterial carbon dioxide by about what the loop's gain predicts, and ventilation follows. After a held breath, carbon dioxide returns to where it was
- [ ] Run: sleep raises arterial carbon dioxide and lowers ventilation, with no law setting either
- [ ] Every link of the loop moves its reader the way the loop needs, on the admitted start
- [ ] The certificate passes with the oxygen and carbon dioxide books held by this loop, and the body's trace is updated
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs. Sleep's shift of the set point and its lower gain are such reads

**Non-goals:** transport delays, periodic breathing, cough, and breathing under voluntary control.
