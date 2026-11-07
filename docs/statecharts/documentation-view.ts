const phases: Record<string, string> = {
  open: "Chart open (placeholder)",
  entry: "Chart entry written (placeholder)",
  order: "Intervention awaiting authorization (placeholder)",
  command: "Command waiting at the boundary port (placeholder)",
};

const steps: Record<string, string> = {
  note: "a clinician documents an assessment or a note (placeholder)",
  reading: "a clinician charts an observer's reading (placeholder)",
  kept: "the entry is kept, never state (placeholder)",
  order: "a clinician chooses an intervention (placeholder)",
  unauthorized: "an intervention not authorized never reaches the body (placeholder)",
  command: "the authorized intervention enters as a timestamped command",
  applied: "an accepted step applies it",
};

export const state = (v: any) => phases[v.phase];
export const step = (action: string, from: any, to: any) => (action === "Done" ? null : steps[to.act]);
export const title = "The chart, clinicians' documentation (placeholder)";
