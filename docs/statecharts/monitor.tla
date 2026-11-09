---- MODULE monitor ----
EXTENDS Naturals, Sequences, FiniteSets

\* The bedside monitor's link to the chart service, for one vital read once a second until the recorded feed
\* ends at second Last. The monitor holds every message the chart service has not answered for, sends what it
\* holds as one batch with at most one batch in flight, and keeps at most Cap messages. The chart expects a
\* reading every second: the device's channel statement that makes it expect one is left out.
CONSTANTS Cap, Last, Boots

Services == {"up", "down"}

VARIABLES now, boot, seq, outbox, flight, chart, service, made, dropped, answered

vars == <<now, boot, seq, outbox, flight, chart, service, made, dropped, answered>>

Range(s) == {s[i] : i \in DOMAIN s}
Reading(b, n, s) == [boot |-> b, seq |-> n, second |-> s]
SameKey(m, c) == m.boot = c.boot /\ m.seq = c.seq

Init ==
  /\ now = 0 /\ boot = 1 /\ seq = 2
  /\ outbox = <<Reading(1, 1, 0)>> /\ made = {Reading(1, 1, 0)}
  /\ flight = <<>> /\ chart = {} /\ service = "up"
  /\ dropped = {} /\ answered = {}

\* A full outbox drops its oldest message.
Hold(m) ==
  LET held == Append(outbox, m) IN
  IF Len(held) > Cap
  THEN outbox' = Tail(held) /\ dropped' = dropped \cup {Head(held)}
  ELSE outbox' = held /\ dropped' = dropped

SecondPasses ==
  /\ now < Last
  /\ now' = now + 1 /\ seq' = seq + 1
  /\ made' = made \cup {Reading(boot, seq, now + 1)}
  /\ Hold(Reading(boot, seq, now + 1))
  /\ UNCHANGED <<boot, flight, chart, service, answered>>

MonitorSends ==
  /\ flight = <<>> /\ outbox # <<>>
  /\ flight' = <<Range(outbox)>>
  /\ UNCHANGED <<now, boot, seq, outbox, chart, service, made, dropped, answered>>

Batch == flight[1]

\* The chart takes a message whose boot and sequence it lacks. One it holds is already held, or refused when its
\* content differs, and the chart keeps nothing of a refused one.
Takes(batch) == chart \cup {m \in batch : \A c \in chart : ~SameKey(m, c)}

\* The service takes the batch in one transaction and answers each message, and the answer arrives.
ServiceAnswers ==
  /\ flight # <<>> /\ service = "up"
  /\ chart' = Takes(Batch)
  /\ outbox' = SelectSeq(outbox, LAMBDA m : m \notin Batch)
  /\ answered' = answered \cup (Batch \cap Range(outbox))
  /\ flight' = <<>>
  /\ UNCHANGED <<now, boot, seq, service, made, dropped>>

\* The service takes the batch, but its answer never arrives, so the monitor sends the same messages again.
AnswerLost ==
  /\ flight # <<>> /\ service = "up"
  /\ chart' = Takes(Batch)
  /\ flight' = <<>>
  /\ UNCHANGED <<now, boot, seq, outbox, service, made, dropped, answered>>

\* A down service is stopped (the connection is refused), failing (it answers an error) or slow (the request
\* times out first). Each leaves the outbox and the chart as they were.
RequestFails ==
  /\ flight # <<>> /\ service = "down"
  /\ flight' = <<>>
  /\ UNCHANGED <<now, boot, seq, outbox, chart, service, made, dropped, answered>>

ServiceChanges ==
  /\ service' \in Services \ {service}
  /\ UNCHANGED <<now, boot, seq, outbox, flight, chart, made, dropped, answered>>

\* The page keeps nothing. The new boot takes the reading of the second it starts in, so a reading lost within
\* its own second is held again: the chart could not yet show it missing.
Restarts ==
  /\ boot < Boots
  /\ boot' = boot + 1 /\ seq' = 2
  /\ outbox' = <<Reading(boot + 1, 1, now)>> /\ flight' = <<>>
  /\ made' = made \cup {Reading(boot + 1, 1, now)}
  /\ dropped' = {} /\ answered' = {}
  /\ UNCHANGED <<now, chart, service>>

Next ==
  \/ SecondPasses \/ MonitorSends \/ ServiceAnswers \/ AnswerLost \/ RequestFails
  \/ ServiceChanges \/ Restarts

\* The monitor keeps taking readings and sending (weak fairness). A service that stays up answers a batch now
\* and then rather than losing every answer (strong fairness, since a lost answer ends the batch it would answer).
Spec ==
  /\ Init /\ [][Next]_vars
  /\ WF_vars(SecondPasses) /\ WF_vars(MonitorSends) /\ SF_vars(ServiceAnswers)

OutboxWithinCap == Len(outbox) <= Cap /\ Len(flight) <= 1

ChartHoldsEachMessageOnceAsMade ==
  \A c \in chart : c \in made /\ \A d \in chart : SameKey(c, d) => c = d

\* A restart ends a boot's outbox, so this holds for the boot that is running.
LeavesOnlyWhenAnsweredOrDropped ==
  \A m \in made : m.boot = boot => (m \in Range(outbox) \/ m \in answered \/ m \in dropped)

Charted(s) == \E c \in chart : c.second = s
Held(s) == \E m \in Range(outbox) : m.second = s
\* The chart expects a reading up to the second before now, and the run's end stops now at Last.
ShownMissing(s) == s < now

EveryReadingChartedHeldOrMissing ==
  \A m \in made : Charted(m.second) \/ Held(m.second) \/ ShownMissing(m.second)

TakingNeverWaits == now < Last => ENABLED SecondPasses

TheOutboxEmpties == <>[](service = "up") => <>[](outbox = <<>>)
====
