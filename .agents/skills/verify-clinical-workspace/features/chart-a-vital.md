# Chart a vital

A clinician of any discipline on the synthetic roster charts any vital of the table. The chart service stamps the charted time from the patient clock, and a blank care time means now. An entry charted after its care time shows how late it was. A refusal shows in words, and nothing is charted.

## Sub-features

- `chart-any` charts a vital as a nurse, a physician or a respiratory therapist.
- `chart-care-time` takes a typed `HH:MM:SS` care time, or now when blank.
- `chart-late` shows how late an entry was charted.
- `chart-refusals` refuses a care time after now and empty value text, in words.

## How to get to it (user POV)

- Open the workspace URL the launcher printed, choose a clinician and a vital, type a value and a care time, and choose **Chart**.

## Driving it with npm run drive

Preconditions:

- The launch and doctor checks pass, the `workspace` tab is open, and `#service` reads `The chart service is answering.`

- **Late entry.** Read the patient time from `#now`. Run `npm run drive -- choose workspace '#author' synthetic-respiratory-therapist`, `npm run drive -- choose workspace '#vital' RR`, `npm run drive -- fill workspace '#value' 18`, `npm run drive -- fill workspace '#care' <now minus 40 s>` and `npm run drive -- click workspace '#chart'`. `#notice` reads `Charted RR 18 /min as Synthetic Respiratory Therapist C.` and the flowsheet shows `18 /min by Synthetic Respiratory Therapist C, care …, charted …, late by 40 s`, give or take a second.
- **Now.** Choose `synthetic-nurse` and `HR`, fill `#value` with 64 and leave `#care` empty, then click `#chart`. `#notice` reads `Charted HR 64 /min as Synthetic Nurse A.`
- **Another discipline.** Choose `synthetic-physician` and `Temp`, fill 37.1 and click `#chart`.
- **Refusals.** Fill `#care` with a time two minutes after `#now` and click `#chart`: `#notice` reads `The care time is after now; nothing was charted.` Fill `#value` with nothing and click `#chart`: `#notice` reads `The value is not a number; nothing was charted.`
- **Proof.** Shoot the page after the late entry and after the refusals, and keep `curl -s <service url>/api/chart`. Its `entries` name three disciplines, each with the run's patient and a care time no later than its charted time.

## Gotchas

- `#care` takes the patient's clock, not the wall clock. Read `#now` first.
- A refusal leaves the form as typed, so clear `#care` before the next write.
