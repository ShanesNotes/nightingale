# A guard lands before anything waiting shows

A hazard guard's crossing ends a step somewhere the core's span did not say, and record 0006 keeps every attempt blind to what waits until its span has passed. So an attempt finds the blind step first: when a span's result crosses a guard, the contract re-solves ordinary tickets until one crosses and one at most the core's floor shorter does not, and only then reads the next waiting input. This extends record 0006 and is a model default.

## Considered options

- Cut at the waiting input, then land inside the cut: rejected. A run that knows an input past the crossing would land somewhere a run that does not know it never would, and at least 110 of 120 random cases broke I5.
- A core locates the event, through `CoupledProblem::transitions` and a claim on its proposal: rejected. A core learns guards, and the record must recheck each claim with the same two tickets, so the check is the search.
- A refusal that carries a landing length: rejected. A crossing is no numerical failure, and it would spend a core's retry budget on an event.
- Fire at the end of the first step past the draw: rejected. An event's instant would be a step grid, against I8.
- Judge a landing by how close the integral came to its draw: rejected. The scales hold a dimensionless row near 1 to about 1, so a time is the only tolerance that means anything, and the check that matters asks about time.
- Retry a refused trial at the bracket's verified bottom: rejected. A shorter ticket can be refused where a longer one passed, because an error estimate over a forcing is not monotone in length, and an honest core crawled at 1 ns a step (2,980 refusals in 3,000 steps).
- Retry at half the trial, below the floor: rejected. It needs a sealed origin on every ticket and opens the same crawl, and for a trial inside a cut its length would read the input.

## Consequences

- A step that lands pays 3 to 7 extra solves on the one fixture measured, and no other step pays anything. Bisection paid 10 to 30.
- The landing tolerance is the core's floor, so the run identity already covers it and a core declares nothing new.
- A refused trial drops the bracket and retries by `StepTicket::refuse`, unchanged. A landing's own lengths come from the bracket. Its retry is a new attempt at a shorter span, which lands in turn or steps short of the crossing and loses nothing.
- A firing belongs to the blind step. Every law is read on the state it leaves, and a law with no value there refuses the trial before the timeline is read, so a run that knows an input and one that does not refuse alike.
- A cut that crosses lands in turn, short of an input a core has seen, so it keeps the cut's fence. That is a second way to stop short of an input, and I5's clause now says so.
- A search can be dropped for a cut, at about 15 cuts in a 48 h run.
- The contract integrates a hazard rate's positive part, because a crossing read only at published states must not depend on the step. A compile refusal cannot read a sign, and a refusal at run time would fault on a seed-dependent dip.
- A core still reaches an event through a row a rate reads, inside the declared scales. No bit gate can see that, since such a core replays exactly, so a step-policy gate holds each policy to the reference integrator.
- The late-admission gate reads each arrival off a run that does not hold the input. A known run's fence comes from what only it showed, so its heads hide the arrival a landing puts at risk: a planted cut-first bug was caught in 13 of 120 rounds that way, against 39.
