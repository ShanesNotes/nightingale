# Clinical workspace wayfinder map

## Destination

One chart holds every kind of entry a care team writes about one patient, in one shape. Every screen is a view of that chart, a tree of plain text files among them, and the bedside monitor shows every vital the engine can source. All of it runs on synthetic patients.

## Notes

- The package is `clinical-workspace/`: a chart service on Postgres, a standalone bedside monitor and a chart screen. It reads the engine's output and never builds or runs the engine. The `verify-clinical-workspace` skill drives it.
- The workspace's vision stays outside the repo ([governance section 2](../../docs/governance.md#2-what-belongs-in-the-repo)). This map holds build tickets only, and each ticket states its own reason.
- State, chart entries and observations are different kinds of fact and never merge ([SRS rule 1](../../docs/srs.md#3-hard-rules)). A clinician writes chart entries. A device's readings reach the chart as device messages and stay apart from entries.
- The chart only appends. An edit is a new version that names the version it replaces, and the database refuses an update, a delete and a stale version.
- Every screen is a pure function of the chart's read model and keeps no facts of its own.
- Values are synthetic. A vital the engine does not produce shows "no source", never a number.
- Two engine tickets feed this line: [native-contract 29](../native-contract/issues/29-pressure-unit.md), the pressure unit, and [native-contract 30](../native-contract/issues/30-streaming-host.md), a host that streams observations through a bedside lens.
- A change under `clinical-workspace/` alone runs only the gate's `50-workspace` step (`tools/gate/paths.toml`).
- Which ticket is built next is the developer reviewer's call.

## Decisions so far

- [The prototype](../../clinical-workspace/src/chart.ts): one append-only chart in Postgres behind a small chart service, a monitor that replays the recorded run `307caf19` through an outbox that survives an outage, and a flowsheet that reads only the chart. The monitor's link is model-checked in `docs/statecharts/monitor.tla`.

## Not yet specified

- **Which vitals have a source.** Heart rate and mean arterial pressure do. Systolic and diastolic wait for [ticket 03](issues/03-systolic-and-diastolic.md). SpO2 and respiratory rate wait for the body's gas loop, and temperature for its heat loop ([body map](../body/map.md)).
- **The ECG.** [SRS section 6](../../docs/srs.md#6-fidelity-aims) requires the waveform on the monitor, and no ticket plans one ([body map](../body/map.md), "Not yet specified").
- **Waveforms.** The monitor shows numbers only. A waveform needs the stream of native-contract 30 and a place to keep strips.
- **A live feed.** The monitor replays a recorded run until native-contract 30 lands.
- **Kinds of entry past the second.** [Ticket 01](issues/01-one-entry-shape.md) proves the shape on two kinds. Orders, medication administration and assessments each need a ticket of their own, and their clinical content needs review (SRS rule 16).
- **A second device.** One device is only a guess at what devices share.

## Out of scope

- Who may write what. An entry names its author. What an author is permitted to do belongs to whatever runs that author, not to the chart.
- Policy, grading, scenarios and lessons.
- Clinical values, and any claim about the care of a real patient.
- Anything that builds or runs the engine from this package.
- Styling beyond what keeps a screen readable.
