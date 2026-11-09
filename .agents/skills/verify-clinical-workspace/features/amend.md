# Amend a chart entry

Selecting a charted value shows its details: the author and discipline, the patient, the care time, the charted time, how late, and version n of m. **Edit** starts from the version shown, and saving sends an amendment of that version as the clinician chosen in the form. If someone charted a newer version meanwhile, the chart refuses it as not the current version. Every view then shows the new value, marked edited. Earlier versions stay hidden until asked for, then come from the chart service.

## Sub-features

- `amend-details` shows an entry's details on selection.
- `amend-edit` writes a new version that names its author.
- `amend-versions` shows earlier versions on request.

## How to get to it (user POV)

- Choose a value in a charted row of the flowsheet, then **Edit**, a new value, and **Save as a new version**. **Show earlier versions** lists them.

## Driving it with npm run drive

Preconditions:

- The chart-a-vital recipe has charted RR 18 as Synthetic Respiratory Therapist C.

- **Details.** Run `npm run drive -- click workspace 'tr[data-vital="RR"][data-source="charted"] button'`. `#details` reads `Version 1 of 1` and `By Synthetic Respiratory Therapist C, respiratory therapist`.
- **Edit.** Run `npm run drive -- choose workspace '#author' synthetic-physician`, `npm run drive -- click workspace '#edit'`, `npm run drive -- fill workspace '#edit-value' 20` and `npm run drive -- click workspace '#save-edit'`. `#notice` reads `Amended RR 20 /min as Synthetic Physician B.`, `#details` reads `Version 2 of 2`, and the RR charted row reads `20 /min by Synthetic Physician B` with `edited`.
- **Versions.** `#versions` is absent until `npm run drive -- click workspace '#versions-button'`. Then it lists `Version 1: 18 /min` and `Version 2: 20 /min`.
- **Proof.** Shoot the page, and keep `curl -s <service url>/api/entries/<entry id>/versions`.

## Gotchas

- The amendment's author is the clinician chosen in `#author`, not the entry's first author.
