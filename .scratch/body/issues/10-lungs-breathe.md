# 10: The lungs breathe

Type: task
Status: open
Blocked by: 08, 09, `.scratch/research/issues/11-law-forms-gas-loop.md`

**What to build:** A tidal breath moves gas through the airway into one alveolar unit with a shunt and a dead space, and alveolar gas exchanges with pulmonary capillary blood. The breath's pleural pressure replaces the constant the slice holds, so venous return feels each breath. The resting lung volume follows posture. Water and heat leave with expired gas, and the work of breathing is charged to metabolism (arrow 84). The pump's force is musculoskeletal state that the lungs read, driven by a held drive until [ticket 11](11-breathing-is-controlled.md).

Breathing is tidal unless ticket 26's budget forbids it. Prototype tidal breathing against quasi-steady alveolar gas with a moving pleural pressure on the body before choosing, and say which in the Answer. The prototype also shows admission with a tidal breath beside the pulsatile heart, which no other ticket covers.

**Releases:** the respiratory card's hold on pleural pressure (coupling arrow 82), arrow 81 (pulmonary capillary flow), arrow 84 (the work of breathing), arrows 85 and 95 (the pump's load and force), and the lung's share of water and heat loss.

- [ ] Tests: the alveolar gas equation holds at steady state, and doubling alveolar ventilation at a fixed production halves alveolar carbon dioxide as the form says
- [ ] Run: for each gas, inspired, alveolar, expired and diffused amounts close. Venous return rises and falls with each breath. Sitting up changes the resting volume in the direction its source gives
- [ ] The hold table no longer lists pleural pressure, and the certificate passes on the whole body
- [ ] The body's trace is updated with one sentence on why it moved
- [ ] Cost: the firings and landings a breath adds, unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs. Dalton's dry inspired gas, `inspired_oxygen() * room_pressure()`, reads only inputs and is refused as it stands; it passes once the water vapour pressure is a parameter in it

**Non-goals:** more than one alveolar unit, airway resistance by generation, hypoxic vasoconstriction, and lung water (arrow 110).

From [research 12](../../research/issues/12-law-forms-heat.md): Kingma's supplement, now on hand, gives respiratory heat at rest, evaporative and convective together, as a fixed 8% of the metabolic rate (its Eq A2). Research 11 had no form for the dry heat of warming inspired air.

From [ticket 03](03-whole-body-lists.md)'s Answer: the conducting airway is a tissue site with a gas space on the airway network, where ticket 01 said a vessel segment. A vessel segment's lumen gets a water pool by rule, and nothing moves water in an airway, so compile would refuse that pool as an orphan. The site has a thermal and an elastic store, so heat can reach it from the core and leave there. The water that humidifies inspired gas comes from the pulmonary bed, as ticket 01's row says: the lining wall reaches beds only, so vapor enters at the lung bed's gas and moves out along the airway.

From [ticket 04](04-scopes-ports-and-census.md):

- The units of arrows 84 (`work_of_breathing: Rate<Energy>`), 85 (`pump_load: Pressure`) and 95 (`pump_force: Pressure`) are picks that no source states. This ticket settles each unit when it wires the arrow, in that arrow's row of `crates/ng-contract/src/catalog.rs`.
- Every port is read at the instant at rung 0, and a law that needs an average holds it in its own memory (`docs/architecture/solver-contract.md`, section 2, the paragraph "The coupling catalog is generated"). If this ticket picks quasi-steady alveolar gas, its quantities per breath are that law's own state, not a mean that a port carries (research 11:40, 64).
- Body 01 has no inspired water. It lists airway water only as leaving (01:63, 356), so RS has `Expired<Water>` and no `Inspired<Water>`, and the room's humidity has no way in. This ticket adds `Inspired<Water>` if its form needs one.
- RS has one airway heat process, `AirwayHeat`, for body 01's two rows on warming inspired gas and on airway water's latent heat (01:171, 172), because Kingma's form gives them together. This ticket adds a second process if its form needs them apart.
- The pleural fluid cannot be left out while GI's abdominal enclosure is held, because `leave_out` leaves out a kind of space at every site and a held enclosure's fluid must exist (`crates/ng-contract/tests/held.rs:717`). A fresh built RS that places it with no law to move it has an orphan pool. So RS keeps an empty `HeldDeclaration` whose validity names the pleural fluid, as [ticket 06](06-whole-body-on-holds.md) does, or the contract gains a leave-out for one site.
- MT's conduction feeds the conducting airway's heat store, and this ticket's airway heat exit draws on that store. [Ticket 06](06-whole-body-on-holds.md)'s MT stand-in serves conduction at the airway until [ticket 12](12-heat-balance.md) builds it.
