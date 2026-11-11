# 31: An intake is one event, and the arena's day comes from its packet

Type: task
Status: open
Blocked by: [04](04-scopes-ports-and-census.md), [06](06-whole-body-on-holds.md) if the intake schema lands there, and [foundation 01](../../foundation/issues/01-arena-and-milestone.md) steps 2 and 3: the pick run and the 48 h protocol run on the slice

**What to build:** [Ticket 05](05-the-days-inputs.md) put the day in the packet. A packet that holds a day makes the run's boundary events from it, and its hash covers the day (records 0004 and 0005). It took two interim steps, and this ticket ends both. Each moves the arena's event digests, so they move together, once.

- **An intake is one event.** The SRS's Event names a meal as one input with an id, and the contract's sketch designs one `Intake` envelope for meals and for later interventions. Ticket 05 holds a meal in the day as one item, but the timeline holds it as one typed delivery for each species, into the boundaries named "gut" and the species.
- **The arena's day comes from its packet.** The arena's synthetic fixture holds no day. It makes its events with `DayPlan::reference` and hands them to the port, so the reference day is made two ways.

- [ ] An intake is one event: one id holds its deliveries in the timeline and the record, with a prior-shape load test for records saved before it
- [ ] An intake has a route (oral, tube or IV), a composition against the intake schema the body declares, and a bolus or continuous delivery, as the contract's sketch designs it. A packet's day keeps its hash: the day's meal item does not change
- [ ] The arena's timed run, its replay case and its day check admit one packet that holds the reference day, and their run identity covers it
- [ ] Each meal control of the day check is the same day without that meal, as its own packet and admission
- [ ] The reference day is made one way, and `DayPlan` keeps only what the day checks read
- [ ] The arena's digests and run identity move once, and the PR says why in one sentence. The slice's model hash and the first-second traces do not move

**Non-goals:** the whole body's day ([ticket 24](24-the-synthetic-day.md)), a reviewed packet's day ([research 10](../../research/issues/10-reviewed-packet-builder.md)), and interventions themselves.
