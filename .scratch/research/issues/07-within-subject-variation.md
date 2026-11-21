# 07: Within-subject variation

Type: research
Status: open
Blocked by: 01

**Question:** What is the within-subject variation of each anchor reading by its method and window, and of each conserved quantity's amount?

- [ ] One record per anchor reading, for the return check
- [ ] One record per conserved quantity, for the certificate
- [ ] An amount no method can measure rests on an inference record reasoning from sourced flows (record 0004)

No record in `data/` gives the within-subject variation of the salts' stores, water or arterial carbon dioxide, which [body ticket 17](../../body/issues/17-volume-hormones.md)'s day run reads. The salts are conserved quantities, which the certificate judges over sliding 24 h windows. Whole-body water is one only until cells make water by burning fuel: from then the certificate holds hydrogen and oxygen, and the healthy-range check judges water's stability. Arterial carbon dioxide is an anchor reading, which the return check judges at the anchor's time of day. [Body ticket 24](../../body/issues/24-the-synthetic-day.md) sizes its own synthetic bounds on its measured day, and this ticket's reviewed variations replace them later.

From [body ticket 12](../../body/issues/12-heat-balance.md)'s plan: the milestone's certificate also needs the within-subject variation of energy's conserved quantity for its drift (`docs/adr/0004-first-milestone.md`, line 40), and `data/` does not hold it. Main's energy bound is a synthetic solver target that judges no body. Core temperature is a reading, so the healthy-range check needs its healthy range across people (line 37), and its within-subject variation is needed only if the return check takes it as an anchor (line 39). No ticket owns core temperature's healthy range yet: [research 06](06-healthy-ranges-and-lens.md) asks which readings the lens carries and their ranges, and names no temperature, and `data/healthy-range-set.jsonl` holds no record of one.
