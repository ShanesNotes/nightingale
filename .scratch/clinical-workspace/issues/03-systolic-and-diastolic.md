# 03: The monitor shows systolic and diastolic pressure

Type: task
Status: open
Blocked by: `.scratch/native-contract/issues/30-streaming-host.md`

**What to build:** The monitor finds systolic and diastolic arterial pressure for each beat from the engine's own observations, and shows them as sourced.

Five of the monitor's seven vitals read "no source". Two of them need no new physiology. The body already computes arterial pressure through the beat, and the recorded run's lens averages it over one second (`arena_lens` in `crates/ng-validate/src/slice.rs`), which leaves only the mean. [Native-contract 30](../../native-contract/issues/30-streaming-host.md) decides a bedside lens that reads arterial pressure at the output grid without averaging. A recorded run read through that lens is enough here, so this ticket does not need the live stream.

- [ ] A recorded run through the bedside lens is committed under `clinical-workspace/capture/`, and its hashes match its `summary.txt`, as the first capture's do
- [ ] The monitor finds each beat's highest and lowest arterial pressure from that channel alone, as a bedside monitor does, and gives them out as systolic and diastolic beside the mean. It reads nothing but the observer's rows
- [ ] A test holds the values to the channel: on a fixture with a known wave, the monitor's systolic, diastolic and mean are the wave's own
- [ ] Before the first whole beat, and across a gap in the channel, the monitor shows no number and says why
- [ ] The monitor's channel statement marks both vitals sourced, the flowsheet shows them, and SpO2, respiratory rate and temperature still read "no source"
- [ ] The pressure unit is the one [native-contract 29](../../native-contract/issues/29-pressure-unit.md) states. Until it lands, the monitor says it assumes mmHg
- [ ] The `verify-clinical-workspace` walk checks both tiles against the capture

**Non-goals:** a waveform on the screen; a cuff; any other vital; the live stream.
