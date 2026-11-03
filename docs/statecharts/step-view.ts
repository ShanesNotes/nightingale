const waiting = (v: any) => v.run.admitted.filter((i: string) => !v.run.applied.includes(i));
const last = (r: any) => r.log[Object.keys(r.log).length];
const isCut = (e: any) => e.f === e.e;

export const state = (v: any) => {
  const r = v.run;
  if (r.phase === "faulted") return "Engine fault, run stopped";
  if (r.phase === "retrying") return "Step refused, retry pending";
  if (r.fence > r.head) return "Accepted, a fence ahead";
  if (waiting(v).some((i: string) => v.stamp[i] === r.head)) return "Accepted, an input due";
  if (waiting(v).length) return "Accepted, an input waiting";
  return "Accepted, nothing waiting";
};

export const group = (v: any) => (v.run.phase === "running" ? "accepted" : "attention");
export const colors = { accepted: "teal", attention: "orange" };

export const step = (action: string, from: any, to: any) => {
  if (action === "Step") {
    const e = last(to.run);
    if (e.kind === "faulted") return "no shorter span is left";
    if (e.kind === "refused") return isCut(e) ? "the cut at the input is refused" : "refused whole, nothing changes";
    if (e.fired) return "lands on a guard and fires";
    if (isCut(e)) return "stops at the waiting input";
    return from.run.phase === "retrying" ? "the shorter retry is accepted" : "accepted whole";
  }
  if (action === "Send") return to.last.out === "admitted" ? "an input arrives" : "a back-dated input is refused";
  if (action === "TakeCheckpoint") return "a checkpoint is taken";
  if (action === "Resume") return "resume from a checkpoint in a fresh solver";
  if (action === "Rewind") return "rewind on a new branch from a checkpoint";
  if (action === "Recover") return "recover on a new branch from a checkpoint";
  return null;
};

export const title = "One step of a run";
