const builds: Record<string, string> = {
  declared: "Map and laws declared",
  compiled: "Body compiled",
  bound: "Packet bound",
  admitted: "Start state admitted",
  notAdmitted: "Not admitted",
};

const runs: Record<string, string> = {
  none: "No run",
  accepted: "Run at its accepted head",
  retrying: "Step refused, retry pending",
  faulted: "Engine fault, run stopped",
};

export const state = (v: any) => ({
  "Contract and admission": builds[v.build],
  "Record crate and solver core": runs[v.run],
  "Boundary port": v.waiting ? "A command's input waiting" : "No input waiting",
  Observers: v.unread ? "An accepted step not yet read" : "Observers at the head",
  "Chart (placeholder)": v.entries ? "Chart entries kept (placeholder)" : "No chart entry (placeholder)",
  Validation: v.scored ? "Arena score kept" : "No score yet",
});

const steps: Record<string, string> = {
  compiled: "the map passes its checks and the laws compile",
  bound: "a packet binds to the body",
  admit: "admission solves the start state",
  command: "an authorized command enters at the boundary port",
  accept: "the record crate accepts the step whole",
  refuse: "the record crate refuses the step whole",
  exhausted: "no shorter interval is left",
  fault: "the solver core or the model cannot go on",
  recover: "recover on a new branch from a checkpoint",
  observe: "an observer reads the accepted step",
  document: "a clinician writes a chart entry (placeholder)",
  score: "the arena scores a solver core",
};

export const step = (action: string, from: any, to: any) => {
  if (action === "Done") return null;
  if (to.act === "notAdmitted") return "a check refuses the map, a law or the packet";
  return steps[to.act];
};

export const title = "Nightingale system: who hands what to whom";
