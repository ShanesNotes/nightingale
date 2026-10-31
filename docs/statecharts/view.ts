const states: Record<string, string> = {
  mapDeclared: "Map declared",
  mapChecked: "Map checked and hashed",
  compiled: "Body compiled",
  bound: "Parameters bound",
  accepted: "Run at an accepted state",
  proposed: "Step proposed",
  refused: "Step refused",
  faulted: "Engine fault, run stopped",
  notAdmitted: "Not admitted",
};

const groups: Record<string, string> = {
  mapDeclared: "runs today",
  mapChecked: "runs today",
  compiled: "runs today",
  bound: "runs today",
  accepted: "runs today",
  proposed: "runs today",
  refused: "attention",
  faulted: "attention",
  notAdmitted: "attention",
};

const steps: Record<string, string> = {
  mapPasses: "the map passes its checks",
  compile: "organ laws compile into one body",
  bind: "a packet binds to the body",
  admit: "admission solves the start state",
  propose: "the solver proposes a step",
  accept: "accepted whole",
  refuse: "refused whole, nothing changes",
  retry: "retry over a shorter interval",
  exhausted: "no shorter interval is left",
  fault: "the solver or model cannot go on",
  recover: "recover on a new branch from an earlier checkpoint",
};

const fails: Record<string, string> = {
  mapDeclared: "a map check fails",
  mapChecked: "compilation rejects a law",
  compiled: "the packet does not bind",
  bound: "admission fails",
};

export const state = (v: any) => states[v.phase];
export const group = (v: any) => groups[v.phase];
export const colors = {
  "runs today": "teal",
  attention: "orange",
};
export const step = (action: string, from: any, to: any) => {
  if (action === "Done") return null;
  if (to.act === "fail") return fails[from.phase];
  return steps[to.act];
};
export const title = "Nightingale engine lifecycle";
