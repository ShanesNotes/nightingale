# 02: The chart as a tree of text files

Type: task
Status: open
Blocked by: 01

**What to build:** One more view of the chart: a tree of plain text files for one patient. A reader that has only file tools (list, read, search) works in it the way it works in a repository.

Every screen is a pure function of the chart's read model, and so is this view. The flowsheet answers a clinician who looks at a grid. The tree answers a reader that lists a folder, opens a file and searches for a word, such as an agent, and it gives a person a chart that a diff can compare. It adds no fact and no way to write.

- [ ] `tree(readModel)` gives a map from path to text and reads no clock, database or network. The same read model gives the same bytes
- [ ] The root holds an index: what each folder holds, how many entries there are of each kind, what is late and what is missing
- [ ] Each entry is a file with its current version, its author, its patient, its care time and its charted time, and its earlier versions beside it
- [ ] The monitor's readings are files by vital and minute, with "no source" and missing stated in words
- [ ] The tree loses nothing. A parser in the tests reads it back into the read model's entries, readings, statements and missing spans, and the result equals the read model, on the recorded run and on charts the tests generate
- [ ] Paths are stable: an entry keeps its path when it is amended, and a new entry moves no other
- [ ] A command writes the tree of the running chart into a folder
- [ ] A cold read is kept as evidence in the PR. A fresh reader with only file tools answers a fixed list of questions from the tree alone, and each answer is checked against the chart's rows. The questions cover a value and where it came from, who amended what, what was charted late and which seconds are missing
- [ ] The `verify-clinical-workspace` skill gains the feature: after the walk's charting it writes the tree and compares it with the chart service's read model

**Non-goals:** writing to the chart through files; who may read or write; prompts, tools or a harness for any reader; more than one patient.
