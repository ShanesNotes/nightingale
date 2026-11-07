const same = (a: any, b: any) => a.branch === b.branch && a.revision === b.revision;
const links = (v: any) => Object.values(v.chain);
const kept = (v: any) => links(v).every((link: any) => v.kept.some((k: any) => same(k, link)));
const exported = (v: any) =>
  v.report.held && v.report.branch === v.branch && JSON.stringify(Object.values(v.report.chain)) === JSON.stringify(links(v));

export const state = (v: any) => {
  const waits = !kept(v);
  if (v.status === "running") return waits ? "Accepted head, a link waits for the store" : "Run at its accepted head";
  if (v.status === "retrying") return waits ? "Step refused, a link waits for the store" : "Step refused, retry pending";
  if (waits) return "Engine fault, a link waits for the store";
  return exported(v) ? "Engine fault, exported and latched" : "Engine fault, run stopped";
};

export const group = (v: any) => (v.status === "running" ? "accepted" : "attention");
export const colors = { accepted: "teal", attention: "orange" };

// Labels read the action and the drawn variables only: under the VIEW, `to` can be a representative whose
// act belongs to another step.
const becomes: Record<string, string> = {
  "running>retrying": "refused whole, nothing changes",
  "retrying>faulted": "no shorter interval is left",
  "running>faulted": "the solver core or the model cannot go on",
};

export const step = (action: string, from: any, to: any) => {
  if (action === "TakeCheckpoint") return "a checkpoint is taken";
  if (action === "Export") return "the fault is exported, latched";
  if (["Accept", "RetryAccepted", "Publish"].includes(action)) {
    if (kept(from) && !kept(to)) return "accepted, the store refuses its link";
    if (!kept(from) && kept(to)) return "accepted, the store keeps every link";
    return from.status === "running" ? "accepted whole" : "the shorter retry is accepted";
  }
  const moved = becomes[`${from.status}>${to.status}`];
  if (moved) return moved;
  if (action === "ModelRunsOutOfBranchIds" || action.startsWith("Certify")) return null;
  if (to.branch === from.branch || from.ancestry.includes(to.branch)) return "resume a checkpoint in a fresh solver core";
  return from.status === "faulted" ? "recover on a new branch" : "rewind on a new branch";
};

export const title = "Engine: steps, the history store, checkpoints and recovery";
