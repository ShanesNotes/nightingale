---- MODULE engine ----
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS MaxRevision, Coverage, Branches, Certifies, Overwrites

VARIABLES status, chain, kept, branch, ancestry, faulted,
          saved, report, act, source, before

vars == <<status, chain, kept, branch, ancestry, faulted, saved, report, act, source, before>>

None == [held |-> FALSE, chain |-> <<>>, branch |-> 0, ancestry |-> {}, latched |-> FALSE]

Range(s) == {s[i] : i \in 1..Len(s)}
Link(b, r) == [branch |-> b, revision |-> r]
IsPrefix(p, s) == Len(p) <= Len(s) /\ p = SubSeq(s, 1, Len(p))
AllKept(s) == Range(s) \subseteq kept
\* ng-record history.rs: the head is in memory, so the walk asks the store only for the links before it.
KeptBeforeHead(s) == Len(s) <= 1 \/ Range(SubSeq(s, 1, Len(s) - 1)) \subseteq kept

Now == [status |-> status, chain |-> chain]

Did(a) == act' = a /\ before' = Now /\ source' = "none"

Init ==
  /\ status = "running" /\ chain = <<>> /\ kept = {}
  /\ branch = 0 /\ ancestry = {0} /\ faulted = {}
  /\ saved = None /\ report = None
  /\ act = "admit" /\ source = "none" /\ before = Now

\* ng-record history.rs: History::push appends the link to those waiting and the store keeps them oldest first,
\* stopping at the first it refuses.
Publish(a) ==
  /\ Len(chain) < MaxRevision
  /\ LET grown == Append(chain, Link(branch, Len(chain) + 1))
         waits == SelectSeq(grown, LAMBDA l : l \notin kept)
     IN /\ chain' = grown
        /\ \E n \in 0..Len(waits) : kept' = kept \cup Range(SubSeq(waits, 1, n))
  /\ status' = "running"
  /\ Did(a)
  /\ UNCHANGED <<branch, ancestry, faulted, saved, report>>

\* ng-record session.rs: Session::step and Session<Retrying>::retry, through PatientRecord::attempt and publish.
Accept == status = "running" /\ Publish("accept")
RetryAccepted == status = "retrying" /\ Publish("retryAccepted")

Becomes(from, to, a) ==
  /\ status = from /\ status' = to
  /\ faulted' = IF to = "faulted" THEN faulted \cup {branch} ELSE faulted
  /\ Did(a)
  /\ UNCHANGED <<chain, kept, branch, ancestry, saved, report>>

Refuse == Becomes("running", "retrying", "refuse")
NoShorterInterval == Becomes("retrying", "faulted", "exhausted")
Fault == Becomes("running", "faulted", "fault")

Copy(latched) ==
  [held |-> TRUE, chain |-> chain, branch |-> branch, ancestry |-> ancestry,
   latched |-> latched]

\* ng-record record.rs: PatientRecord::checkpoint refuses with StoreFailed while a link the store refused waits.
TakeCheckpoint ==
  /\ status = "running" /\ (Overwrites \/ ~saved.held)
  /\ IF AllKept(chain)
     THEN saved' = Copy(FALSE) /\ Did("checkpoint")
     ELSE UNCHANGED saved /\ Did("storeFailed")
  /\ UNCHANGED <<status, chain, kept, branch, ancestry, faulted, report>>

\* ng-record session.rs: Session<Faulted>::export marks the image latched.
Export ==
  /\ status = "faulted" /\ (Overwrites \/ ~report.held)
  /\ IF AllKept(chain)
     THEN report' = Copy(TRUE) /\ Did("export")
     ELSE UNCHANGED report /\ Did("storeFailed")
  /\ UNCHANGED <<status, chain, kept, branch, ancestry, faulted, saved>>

Restore(name, ck, b, a) ==
  /\ status' = "running" /\ chain' = ck.chain
  /\ branch' = b
  /\ ancestry' = ck.ancestry \cup {b}
  /\ act' = a /\ before' = Now /\ source' = name
  /\ UNCHANGED <<kept, faulted, saved, report>>

\* ng-record checkpoint.rs: restore refuses a latched image unless it recovers, then walks the store from its seal.
Resume(name, ck) ==
  /\ ck.held /\ ~ck.latched /\ AllKept(ck.chain)
  /\ Restore(name, ck, ck.branch, "resume")

\* ng-record session.rs fork and checkpoint.rs restore: a branch id this branch or the checkpoint uses is
\* BranchInUse, a checkpoint off this branch's chain NotOnThisBranch.
Fork(name, ck, b, a) ==
  /\ ck.held /\ b \notin ancestry \cup ck.ancestry
  /\ IsPrefix(ck.chain, chain) /\ KeptBeforeHead(chain)
  /\ Restore(name, ck, b, a)

Rewind(name, ck, b) == status = "running" /\ ~ck.latched /\ Fork(name, ck, b, "rewind")
Recover(name, ck, b) == status = "faulted" /\ Fork(name, ck, b, "recover")

\* ng-validate certificate.rs: certify refuses a policy of another basis, a span shorter than the coverage and a
\* link the store does not give.
Certify(basis) ==
  /\ Certifies
  /\ IF basis = "own" /\ Len(chain) >= Coverage /\ KeptBeforeHead(chain)
     THEN Did("certified") ELSE Did("certificateRefused")
  /\ UNCHANGED <<status, chain, kept, branch, ancestry, faulted, saved, report>>

ModelRunsOutOfBranchIds ==
  /\ status = "faulted" /\ Branches \subseteq ancestry
  /\ UNCHANGED vars

Next ==
  \/ ModelRunsOutOfBranchIds
  \/ Accept \/ RetryAccepted
  \/ Refuse \/ NoShorterInterval \/ Fault
  \/ TakeCheckpoint \/ Export
  \/ Resume("saved", saved) \/ Resume("report", report)
  \/ \E b \in Branches :
       \/ Rewind("saved", saved, b) \/ Rewind("report", report, b)
       \/ Recover("saved", saved, b) \/ Recover("report", report, b)
  \/ \E basis \in {"own", "other"} : Certify(basis)

Spec == Init /\ [][Next]_vars

\* The drawn config merges states that differ only in what no step reads and no label shows: the last step's
\* record and which branches faulted. The larger config checks every rule without it.
Drawn == <<status, chain, kept, branch, ancestry, saved, report>>

Restored == {"resume", "rewind", "recover"}
Source == IF source = "saved" THEN saved ELSE report
Stepped == {"accept", "retryAccepted"}

RefusalChangesNothing ==
  act = "refuse" => chain = before.chain

AFaultTakesNoStep ==
  act \in Stepped \cup {"refuse", "checkpoint", "rewind"} => before.status # "faulted"

OnlyRecoverContinuesAFaultedBranch ==
  act \in Stepped => branch \notin faulted

RecoverOpensANewBranch ==
  act = "recover" => (before.status = "faulted" /\ branch \notin Source.ancestry)

ACheckpointHoldsEveryLink ==
  act \in {"checkpoint", "export"} => AllKept(chain)

StoreFailedOnlyWhileALinkWaits ==
  act = "storeFailed" => ~AllKept(chain)

AFaultCanBeExported ==
  status = "faulted" => AllKept(chain)

ResumeAndRewindRefuseAFaultExport ==
  act \in {"resume", "rewind"} => ~Source.latched

ForksStayOnThisBranchsChain ==
  act \in {"rewind", "recover"} => IsPrefix(chain, before.chain)

RestoreGivesTheCheckpointBack ==
  act \in Restored => (Source.held /\ chain = Source.chain)

CertificateCoversItsSpanOnItsBasis ==
  act = "certified" => (Len(chain) >= Coverage /\ KeptBeforeHead(chain))

PublishedStepsStay ==
  act \notin Restored => IsPrefix(before.chain, chain)

OnlyAnAcceptedStepMovesTheHead ==
  chain # before.chain => act \in Stepped \cup Restored
====
