# A step reads its error from its two half stages

The doubled step solved a stage over the whole ticket at weight zero only to estimate its error. The solver core now declares `backward_euler_halved`: its two half stages carry the increment, and its error is their second difference from the start, which has the doubled difference's leading term. A ticket after a breakpoint shows the doubled set, because a firing restarts stiff transients the second difference over-reads, and a ticket under 2 ns shows one whole stage, as under the doubled scheme. This is a model default.

## Considered options

- Two-stage Radau IIA: rejected, 15% slower on the slice.
- One stage whose error is read against the last segment by Milne's device: rejected. It was 30% faster, but it left one refusal of margin above the floor, a worst accuracy of 0.989 and fewer valve reopenings.
- Committing, after a firing, the length the last restart passed at: rejected, 3.95% slower.
- Choosing the set from the last segment as well, so the admission head keeps the doubled set: rejected. It reproduces the prototype's bits, but the second difference reads no segment.

## Consequences

- On the slice the halved core takes 21% less wall time a simulated second than the doubled core (asrog, five alternating rounds of 25 repeats each from one binary, 16.8% to 23.1% by round), with 3.2% more steps and 23% fewer instructions a step. Its health is the doubled core's but for fewer accuracy refusals on one valve row, and the score's certificate, accuracy and replay gates pass.
- `Core::new` is a new run identity. `Core::doubled` keeps the old one, so a run it wrote still resumes and replays, and the reference integrator stays doubled.
- On a decaying mode the second difference is (1 + |λ| h) times the doubled difference, so the halved set never reads a step as more accurate. A body whose admission leaves a fast transient pays refusals on its first steps.
- A core reads its stage set and its estimate from its problem, which runs the record's own functions, so a core that follows them is never refused for accuracy by the record.
