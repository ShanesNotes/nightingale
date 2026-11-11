# 22: The bladder

Type: task
Status: open
Blocked by: 05, 06, 26, `.scratch/research/issues/15-law-forms-slow-books.md`

**What to build:** Urine the kidney makes collects in the bladder and leaves at a void, which is an event of the day. Balance never needed this, but a chart of voided urine does. This is pass 2.

- [ ] Run: urine made equals urine voided plus what the bladder holds, in the ledger
- [ ] Each void is an event with a time and an id in the event log, and it replays (SRS invariant I5)
- [ ] From [research 15](../../research/issues/15-law-forms-slow-books.md): the bladder starts the day holding water, because each solute leaves by its share of the bladder's water and `divide` refuses a divisor that is not positive
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs. A void flow `voiding() * k_V` passes it, though it empties a fixed flow whatever the bladder holds

**Non-goals:** the urge to void, and bladder pressure.
