# 31: The arena's day comes from its packet

Type: task
Status: open
Blocked by: [foundation 01](../../foundation/issues/01-arena-and-milestone.md), steps 2 and 3: the pick run and the 48 h protocol run on the slice

**What to build:** [Ticket 05](05-the-days-inputs.md) put the day in the packet. A packet that holds a day makes the run's boundary events from it, and its hash covers the day (records 0004 and 0005). The arena's synthetic fixture holds no day yet. It still makes its events with `DayPlan::reference` and hands them to the port, so the pick run and the 48 h protocol run keep the run identity they have. So the reference day is made two ways, and this ticket ends the second one once those runs are done.

- [ ] The arena's timed run, its replay case and its day check admit one packet that holds the reference day, and their run identity covers it
- [ ] Each meal control of the day check is the same day without that meal, as its own packet and admission
- [ ] The reference day is made one way: `DayPlan` keeps only what the day checks read
- [ ] The arena's run identity moves once, and the PR says why in one sentence. The slice's model hash and the first-second traces do not move

**Non-goals:** the whole body's day ([ticket 24](24-the-synthetic-day.md)) and a reviewed packet's day ([research 10](../../research/issues/10-reviewed-packet-builder.md)).
