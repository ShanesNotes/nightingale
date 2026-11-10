# 05: The day's inputs: the clock, sleep, the room and meals

Type: task
Status: open
Blocked by: 03

**What to build:** The boundary carries everything the day does to the body. Today it carries deliveries of water, sodium, potassium and chloride, and posture. No payload carries the clock, sleep or the room, a meal goes straight into the splanchnic veins, and the day's events are written into a test fixture.

- [ ] A clock phase and a sleep state reach a law as inputs. They scale gains and set points and never set a produced quantity (SRS rule 2)
- [ ] The room reaches the body: air and radiant temperature, humidity, air speed, pressure, inspired gas, and clothing or bedding
- [ ] A meal is a vector of every species [ticket 01](01-rung-0-specification.md)'s tables deliver, iron and the diet's organic anion included, delivered into the gut lumen, and a drink is water delivered there
- [ ] A void is an input of the day, for [ticket 22](22-the-bladder.md)
- [ ] The run's boundary events are made from the day in the packet (records 0004 and 0005). This lands the day item of [foundation 07](../../foundation/issues/07-contract.md) in the crates
- [ ] Each input replays: late admission, pause and resume give the same bits as the same day known from the start (SRS invariant I5), on the existing replay gates
- [ ] The slice's model hash and trace do not move

**Non-goals:** any law that reads these inputs, thirst or appetite as behavior, and REM sleep.
