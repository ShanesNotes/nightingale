const services: Record<string, string> = {
  up: "Answering",
  down: "Stopped, failing or too slow to answer",
};

const size = (s: any) => Object.keys(s ?? {}).length;

const link = (v: any) => {
  if (size(v.flight)) return "Held, a batch on its way";
  return size(v.outbox) ? "Held, the chart service not answering yet" : "Everything delivered";
};

export const state = (v: any) => ({
  "The monitor's link": link(v),
  "The chart service": services[v.service],
});

const steps: Record<string, string> = {
  SecondPasses: "a second passes and the monitor takes its reading",
  MonitorSends: "the monitor sends what it holds as one batch",
  ServiceAnswers: "the chart service takes the batch and answers each message, and the monitor lets go of each one answered",
  AnswerLost: "the chart service takes the batch, but its answer is lost on the way back",
  RequestFails: "the connection is refused, the chart service answers an error, or the request times out",
  Restarts: "the monitor restarts, loses what it held and takes this second's reading again",
};

export const step = (action: string, from: any, to: any) =>
  action === "ServiceChanges" ? (to.service === "up" ? "the chart service comes back" : "the chart service stops, fails or slows down") : steps[action];

export const title = "The bedside monitor's link to the chart service";
