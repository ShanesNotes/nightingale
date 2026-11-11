# 05: The day's inputs: the clock, sleep, the room and meals

Type: task
Status: resolved
Claimed by: 05-days-inputs
Blocked by: 03

**What to build:** The boundary carries everything the day does to the body. Today it carries deliveries of water, sodium, potassium and chloride, and posture. No payload carries the clock, sleep or the room, a meal goes straight into the splanchnic veins, and the day's events are written into a test fixture.

- [x] A clock phase and a sleep state reach a law as inputs. They scale gains and set points and never set a produced quantity (SRS rule 2)
- [x] From [research 12](../../research/issues/12-law-forms-heat.md), [research 14](../../research/issues/14-law-forms-volume-and-acid.md) and [research 15](../../research/issues/15-law-forms-slow-books.md): the clock arrives as the cosine and the sine of its phase, as posture arrives as a sine, because no operator takes a phase's cosine. This ticket states the phase's one zero, since research 14 counts the clock from the hour of the day and research 15 from the core temperature minimum, and each law maps its source's zero to it
- [x] The room reaches the body: air and radiant temperature, humidity, air speed, pressure, inspired gas, and clothing or bedding
- [x] A meal is a vector of every species [ticket 01](01-rung-0-specification.md)'s tables deliver, iron and the diet's organic anion included, delivered into the gut lumen, and a drink is water delivered there
- [x] A void is an input of the day, for [ticket 22](22-the-bladder.md)
- [x] From [research 15](../../research/issues/15-law-forms-slow-books.md): the void reaches the bladder's law as a gate held from one event to the next, as posture is, because a delivery only carries an amount in and the volume voided is a produced quantity
- [x] The run's boundary events are made from the day in the packet (records 0004 and 0005). This lands the day item of [foundation 07](../../foundation/issues/07-contract.md) in the crates
- [x] Each input replays: late admission, pause and resume give the same bits as the same day known from the start (SRS invariant I5), on the existing replay gates
- [x] The slice's model hash and trace do not move

**Non-goals:** any law that reads these inputs, thirst or appetite as behavior, and REM sleep.

From [research 12](../../research/issues/12-law-forms-heat.md): air speed's form needs a speed over a reference speed, and the identity table has no speed. Either this ticket delivers air speed as its ratio to 0.1 m/s, or the table gains a speed. [Ticket 03](03-whole-body-lists.md) added no speed to the table, so this ticket delivers the ratio. Clothing or bedding arrives as a count of clo, because insulation has no dimension in the table.

## Answer

The boundary carries the whole day, and a packet holds it. Every value in the tests is synthetic. The day's clinical numbers are research 04's.

**What a law reads.** Each new input is a held input: it holds from its event until the next of its kind and carries no amount. A body that reads one with no value at the anchor moment is refused (`InputUnstated`).

- The clock. A clock event states the time of day at its instant (`Clock::since_midnight_ns`). The phase is zero at midnight and makes one turn a day. A law reads its cosine and sine (`Kernel::clock`), which the contract's own `pinned::turn` gives for each whole minute, because no operator takes a cosine. Research 14 counts from the hour of the day, so its laws need no shift. Research 15 counts from the core temperature minimum, so ticket 20 maps Scheer's zero to midnight through its own phase parameters. In a body that reads the clock, each minute's edge ends a step. The slice reads none and never sees one.
- Sleep: `Sleep::Awake` and `Sleep::Nrem`, read as 0 and 1 (`asleep`). Rung 0 has no REM sleep.
- The room: `Room` gives air and radiant temperature, relative humidity, air speed as its ratio to 0.1 m/s, barometric pressure, the inspired dry fractions of oxygen and carbon dioxide, and clothing or bedding in clo, each read in its own dimension (`room_air` and the rest).
- The void: `Void(Gate::Open)` and `Void(Gate::Shut)`, held from one event to the next and read as 1 and 0 (`voiding`). It carries no amount, because the volume voided is a produced quantity.

**A meal.** A meal is one event with one id (`BoundaryInput::Meal`) over the twelve species body 01's tables deliver: water, sodium, potassium, chloride, glucose, triglyceride, amino acids, the diet base, calcium, phosphate, magnesium and iron (`Meal::SPECIES`, pinned by a test). It parses into one typed delivery for each species with an amount, into the boundary named "gut" and the species (`Meal::BOUNDARIES`), with an id derived from the meal's. It lands whole: parsing and admission refuse the whole meal when any part is refused. It lands once: the same meal sent twice collapses. A drink is water delivered there. On the SRS: the SRS's meal event is the day's meal item, and until [ticket 31](31-one-intake-event-and-the-arenas-day.md) the timeline holds it as its typed deliveries.

**What holds "an input scales a gain or a set point and never sets a produced quantity".** Three things hold it:

- Compile refuses a law that reads a held input and no parameter or memory slot (`UnscaledInput`). It has a test and a mutant row.
- A void carries no amount, by type: a `compile_fail,E0308` probe and its legal twin.
- No delivery carries a negative amount (`NegativeDelivery`), for every species of a meal.

**The day in the packet.**

- `Day` holds the day's events. A meal stays one item: its id, its instant, its end and its twelve amounts. The day's seal writes the meal as that item, so ticket 31 can change how a meal reaches the timeline without changing a packet's day or its hash.
- A synthetic packet may take one day (`ParameterPacket::day`). A reviewed packet is built only with one (`with_day`, `periodic_with_day`), and a `compile_fail,E0599` probe shows that a reviewed packet with no day does not compile.
- The packet's hash seals the day only when the packet holds one, so every day-less packet hashes as on main.
- `bind` checks the day against the body, and admission merges it into the port:
  - an event at or after the anchor moment enters, a meal as its parts;
  - the latest held input of each kind before the moment is restamped at it, unless one of its kind is stated there, and a clock moves on by the time between;
  - a delivery that ended before the moment is dropped, and one that runs across it is refused (`DeliveryAcrossAdmission`).

**Reading record 0005.** Record 0005 holds for every packet that holds a day: the run's events are made from the day, and the run's identity covers it. The arena's synthetic fixture holds no day until ticket 31, and its events are named by the event log and the day checks' digests. A test shows that a packet holding the reference day admits the same timeline digest as the arena's port, and that the two run identities differ. A mutant row that drops the merge's last event fails that test.

**Replay.** On the replay gates, late admission, pause and resume give the same bits as the same day known from the start. That holds for a body that reads every held input and takes a meal. It also holds with the day in the packet, against the same events admitted late on a day-less packet.

**What did not move.** The slice's model hash, its conserved basis, the packet named in `trace.rs` and the three first-second traces. Every record `prior_shape` loads. The reference day's event digest, pinned before any change. No pinned value moved.

**Follow-ups.** [Ticket 31](31-one-intake-event-and-the-arenas-day.md) makes an intake one event and moves the arena's day into its packet. [Research 10](../../research/issues/10-reviewed-packet-builder.md) gains a box: its builder refuses a reviewed packet with no day.
