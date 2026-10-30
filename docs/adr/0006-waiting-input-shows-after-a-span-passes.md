# A step shows a waiting input only after its span has passed

A core commits to a span, and each attempt of a step runs at that span with the head's drives, as if nothing waited. When a span passes and a waiting input lies inside it, the result is dropped and the same ticket is solved again, ended at that input. Every accepted step still stops at the next breakpoint; what changes is when the cut shows. This is a model default.

The obvious rule, cutting a ticket when it is issued, cannot give I5. A cut ticket can pass where the uncut ticket of the same head is refused, so a run that knows an input and one that does not land on different heads, with no dishonest core involved. It also leaves finding 12 open: a core that asks for a huge span and refuses the ticket reads the next breakpoint. Refusing every stamp a refused attempt reached would restore I5, at the price of refusing 10% of stamps drawn just after a head on an honest corpus, where this rule refused none in 14,500 steps.

## Consequences

Each cut costs one more solve and a dropped pass, about 15 in a 48 h run. A run can fault on an attempt past a waiting input where a cut might have passed, and the same run with that input unknown faults there too. A cut refused after its span passed leaves a fence, and `enqueue` refuses a stamp behind it, because two late schedules of one input set then reach different heads and no run can equal both. The fence is why I5 names a second refusal. A core that commits to a span the record would accept does see the next waiting input inside it; that step would have crossed the input in a run that did not know it, so no late input can still arrive before it.
