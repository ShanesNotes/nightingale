# 04: A scope and ports for each system, and a census for each assembly

Type: task
Status: open
Blocked by: 03

**What to build:** Each of the nine systems gets its scope and its ports in the contract, and the census of coupling arrows reads the assembly it is asked about. Today the owner set is sealed with nine systems marked held, and the counts of mapped and unmapped arrows are constants about the slice: moving one system into the slice breaks those constants and the sketch's copy of them. Once the census reads an assembly, the slice keeps its count, the whole body has its own, and no system ticket edits a shared constant.

- [ ] A scope and a ports struct for each of the nine systems, granting only what [ticket 01](01-rung-0-specification.md) says that system owns
- [ ] Each coupling arrow with no payload yet gets one, or is recorded as an anatomy relation, so all 38 are accounted for in the whole body. Arrow 111, intracranial pressure, gets its owner or an explicit hold
- [ ] Each system crate declares one law to a module, so two tickets that add laws to one crate edit different files
- [ ] Every port signal states its time basis: instantaneous, a beat mean or a breath mean
- [ ] The census is computed for an assembly, the slice's census is unchanged, and the check that pins the slice's counts still holds them
- [ ] The rule in `AGENTS.md` that states the census, and its check, name both assemblies
- [ ] The slice's model hash and trace do not move

**Non-goals:** any law, and any emitter or receiver for an arrow. Those arrive with the loop that needs them.

This changes the contract's interface, so it runs pstack `architect` with an Opus and a Codex runner and a Codex judge.
