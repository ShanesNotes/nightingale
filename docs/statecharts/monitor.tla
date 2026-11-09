---- MODULE monitor ----
EXTENDS Naturals, Sequences, FiniteSets

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

Takes(batch) == chart \cup {m \in batch : \A c \in chart : ~SameKey(m, c)}

ServiceAnswers ==
  /\ flight # <<>> /\ service = "up"
  /\ chart' = Takes(Batch)
  /\ outbox' = SelectSeq(outbox, LAMBDA m : m \notin Batch)
  /\ answered' = answered \cup (Batch \cap Range(outbox))
  /\ flight' = <<>>
  /\ UNCHANGED <<now, boot, seq, service, made, dropped>>

AnswerLost ==
  /\ flight # <<>> /\ service = "up"
  /\ chart' = Takes(Batch)
  /\ flight' = <<>>
  /\ UNCHANGED <<now, boot, seq, outbox, service, made, dropped, answered>>

RequestFails ==
  /\ flight # <<>> /\ service = "down"
  /\ flight' = <<>>
  /\ UNCHANGED <<now, boot, seq, outbox, chart, service, made, dropped, answered>>

ServiceChanges ==
  /\ service' \in Services \ {service}
  /\ UNCHANGED <<now, boot, seq, outbox, flight, chart, made, dropped, answered>>

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

\* A service that stays up answers a batch now and then rather than losing every answer (strong fairness, since
\* a lost answer ends the batch it would answer).
Spec ==
  /\ Init /\ [][Next]_vars
  /\ WF_vars(SecondPasses) /\ WF_vars(MonitorSends) /\ SF_vars(ServiceAnswers)

OutboxWithinCap == Len(outbox) <= Cap /\ Len(flight) <= 1

ChartHoldsEachMessageOnceAsMade ==
  \A c \in chart : c \in made /\ \A d \in chart : SameKey(c, d) => c = d

LeavesOnlyWhenAnsweredOrDropped ==
  \A m \in made : m.boot = boot => (m \in Range(outbox) \/ m \in answered \/ m \in dropped)

Charted(s) == \E c \in chart : c.second = s
Held(s) == \E m \in Range(outbox) : m.second = s
ShownMissing(s) == s < now

EveryReadingChartedHeldOrMissing ==
  \A m \in made : Charted(m.second) \/ Held(m.second) \/ ShownMissing(m.second)

TakingNeverWaits == now < Last => ENABLED SecondPasses

TheOutboxEmpties == <>[](service = "up") => <>[](outbox = <<>>)
====
