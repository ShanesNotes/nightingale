# The packet holds what moves the body, and healthy ranges live apart

The first milestone (record 0004) needs three kinds of clinical value: anchors, the numbers of the day the patient lives, and healthy ranges. The anchors fix the start state and the day sets the run's events, so both move the body and go in the reviewed packet, which is part of the run's identity. The healthy ranges only judge the body, so they go in a healthy-range set of their own, outside the run's identity, and correcting one rechecks any replay without changing the run, as a new lens does (I7).

## Considered options

- Healthy ranges in the packet, beside the anchors: rejected. It gives one bundle to review, but correcting a range would make a new packet, so rechecking an old run would need a new run identity though no physiology changed.
- Healthy ranges on the lens's channels, beside reference intervals: rejected. The pass bar would change whenever the display does. A reference interval is a flag a learner sees, and a healthy range is what the checks hold the body to, so one home for both would tie two questions together.
- The day's numbers as plain events written by whoever starts the run: rejected. They move the body, so they are clinical values, and rule 12 lets clinical values in only through review.

## Consequences

- The packet holds more than parameters and anchors: it also holds the day (sleep, posture, meals, drinks and room conditions) that the run's boundary events are made from, since a meal's size is not a property of the body (SRS section 2).
- The healthy-range set is reviewed as a packet is. Each range rests on a source record of a stated tier, or on an inference record that adjusts a sourced range by a sourced shift, and building the set refuses anything else (I16). A synthetic set holds obviously synthetic ranges, and it is the only kind the arena's checks read.
- Rule 12 names both homes: clinical values enter only in a reviewed packet or a reviewed healthy-range set.
- One set serves every packet, because the healthy-range check picks the range that matches the patient (record 0004).
- The checks run in validation, never inside the engine, and physiology never reads the set, as it never reads a reference interval. A result of the checks names the run and the set it read, so a corrected range gives a new result for the same run.
- The within-subject variations that the return check and the certificate read live in the set too, by the same line. One for an amount no method can measure may rest on an inference record that reasons from the sourced flows into and out of it (record 0004).
