# 30: A host that streams a run's observations as they are made

Spec: SRS section 2 (Observer, Observation, Monitor, Pace) and rule 13; [solver contract](../../../docs/architecture/solver-contract.md)

Type: task

Status: open

Blocked by: none

A bedside monitor reads the patient as each step is accepted. Nothing today hands an observer's output to a client while a run goes:

- `gp-driver run` writes the lens rows (`tools/generative-physiology/driver/src/capture.rs:513-532`) to `lens.npy`, and a client can read them only after the run ends.
- `gp-driver interact` writes a line per beat while it runs (`tools/generative-physiology/driver/src/teacher.rs:197`). Each line holds the driver's means over accepted state (`capture.rs:265-329`), not observations, and the driver waits for a posture answer after each beat.
- A run steps as fast as it can, with no pace: 20 s of patient time took 2.7 s on the tower, and 10 minutes took 84 s.
- The arena lens reads three one-second means (`crates/ng-validate/src/slice.rs:998-1004`). Nothing reads arterial pressure finely enough for a monitor to find systolic and diastolic itself, and nothing reads a temperature, though `Temperature` is a sensed dimension (`crates/ng-contract/src/quantity.rs:323`).

So the clinical workspace prototype's monitor replays a recorded `lens.npy`.

**What to build:** a host program outside the physiology closure, as `gp-driver` is, that admits a run, steps it at a pace, passes each accepted step through an `Observer` (`crates/ng-observe/src/observer.rs`), and writes each observation as soon as it is made, in the lens row format the driver already writes. Decide:

- the transport: a line stream on stdout, a local socket, or server-sent events;
- pace: 1 by default, with pause and faster pace as the SRS's Pace allows;
- a bedside lens: its channels, including arterial pressure at the output grid without averaging, so a monitor can find systolic, diastolic and mean for each beat, and a temperature if the body produces one;
- how a client learns the patient's clock: the run's time with each observation, and a heartbeat when no observation is due.

- [ ] The host streams a synthetic run's observations at pace 1, and the rows it streamed equal the `lens.npy` that `gp-driver run` writes for the same spec and lens.
- [ ] A client that connects late gets observations from then on, with the patient's clock.
- [ ] The host reads the body only through an observer, and reaches a session only through the boundary port (SRS rule 13).

**Non-goals:** interventions; clinical values; the workspace side, whose capture decoder already reads lens rows.
