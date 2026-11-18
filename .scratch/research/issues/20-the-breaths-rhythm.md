# 20: The breath's rhythm at rest: rate, inspiration's share and the drive's rise and fall

Type: research
Status: open
Blocked by: 11

**Question:** Which cited values and forms set a healthy adult's breath at rest: its rate, the share of each breath spent in inspiration, and how fast the inspiratory drive rises and falls within a breath? [Body ticket 10](../../body/issues/10-lungs-breathe.md)'s second PR states each as a synthetic value, and [research 11](11-law-forms-gas-loop.md) found no form ("Breathing rhythm" under No form on hand).

Today's forms, in `crates/ng-respiratory/src/breath.rs`:

- The breath's phase runs at the rate `f`, 0.2 a second, through inspiration's length, 0.4, and then expiration's, 0.6, so inspiration takes 2 s of a 5 s breath.
- The pump drive `a` relaxes toward its peak, 1, at 1.5 a second through inspiration, and toward zero at 2.0 a second through expiration. The pump's pressure is `P_mus = PImax a`, with `PImax` 3.8013 in the packet, fitted so the breath ventilates the alveolar unit as 10a's mixing exchange does. No source gives `PImax` a value here: research 11's pump row holds it, and the plan's prototype used 4.
- The body admits with the breath at rest, the drive held at its mean over a breath by closed form from these values, and a hazard at 1,000 a second starts the first breath.

- [ ] The resting rate and inspiration's share of a breath (the duty cycle `T_I / T_TOT`), awake and supine or seated, with the population and the method, and how each moves asleep. Douglas 1982 and Gothe 1981 report both awake and asleep and were not read for a form
- [ ] The shape of the inspiratory drive or of the inspiratory flow within a breath, as a time course or as the speeds of an exponential rise and fall, or the evidence that sources go no further than a duty cycle and a mean flow
- [ ] The tidal volume at rest in the same population, so the drive's swing and `PImax` bind the breath's depth
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it

**Non-goals:** the controller that sets the drive from the chemoreceptors (body ticket 11), the pump's force against its drive and its fatigue, and the start hazard's rate, a model default that only starts the first breath.
