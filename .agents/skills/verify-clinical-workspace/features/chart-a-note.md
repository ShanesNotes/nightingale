# Chart a note

Below the flowsheet, a clinician charts a note of free text through the same chart as the vitals. The note shows its care time, author and discipline, its text, and when it was charted and how late. A refusal shows in words beside the note form, says why, and keeps the draft. **Edit** opens the editor inside the note, and saving writes a new version as the clinician chosen in **Charting as**. An amended note says which version is shown and how late that version was charted, and its earlier versions come from the chart service, inside the note, on request.

## Sub-features

- `note-late` charts a note with a care time before now and shows how late it was charted.
- `note-refused` refuses a blank note in words beside the note form and keeps the draft.
- `note-amend` writes version 2 in place, by another clinician.
- `note-versions` shows the earlier version inside the note on request.
- `note-read-back` finds the note and both versions in the chart service, beside the vitals in one read model.

## How to get to it (user POV)

- Choose a clinician in **Charting as**, type the note under **Notes**, a care time or none, and choose **Chart note**. On a note, **Edit**, change the text and **Save as a new version**. On an edited note, **Show earlier versions**.

## Driving it with npm run drive

Preconditions:

- The launch and doctor checks pass, and `#service` reads `The chart service is answering.`
- The amend recipe has amended the RR entry to 20 /min, for the read-back check.

- **Late note.** Read the patient time from `#now`. Run `npm run drive -- choose workspace '#author' synthetic-nurse`, `npm run drive -- fill workspace '#note-text' 'Synthetic walk note, first version.'`, `npm run drive -- fill workspace '#note-care' <now minus 90 s>` and `npm run drive -- click workspace '#chart-note'`. `#note-notice` reads `Charted note as Synthetic Nurse A.`, and `#note-list` shows the text and `late by 1 min 30 s`, give or take a second.
- **Refused.** Fill `#note-text` with three spaces, clear `#note-care` and click `#chart-note`. `#note-notice` reads `The note text is blank; nothing was charted.`, `#note-text` still holds the spaces, and `#notice` says nothing of it.
- **Read back.** `curl -s <service url>/api/chart` lists one entry of kind `note` at version 1 by `synthetic-nurse`, charted at least 89 s after its care time. Take its `entry` id.
- **Amend.** Choose `synthetic-physician`, click `#note-list li[data-entry="<id>"] [data-action="edit"]`, fill `#edit-text` with `Synthetic walk note, second version.` and click `#save-edit`. `#note-notice` reads `Amended note as Synthetic Physician B.`, and the note reads `Synthetic Physician B, physician · edited, version 2 of 2` and `Version 2 charted …, late by …`.
- **Versions.** `#versions` is absent until `npm run drive -- click workspace '#note-list li[data-entry="<id>"] [data-action="versions"]'`. Then the note's `#versions` lists `Version 1 · Synthetic Nurse A, nurse · care time … · charted …` and the first text.
- **Proof.** Shoot the page after the refusal and after the versions show. Keep `curl -s <service url>/api/entries/<id>/versions`, which gives both versions with their text and author, and `GET /api/chart`, which holds the note at version 2 beside the RR entry at 20 /min.

## Gotchas

- A refusal leaves the note form as typed, so clear `#note-text` and `#note-care` before the next note.
- The editor's ids are the same for a note and a vital, since one editor is open at a time. Scope a selector to the note's `li` to be sure which.
- A note never reaches the flowsheet, so the RR charted row still reads `20 /min by Synthetic Physician B` after it.
