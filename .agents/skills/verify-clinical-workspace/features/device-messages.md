# Device messages

The chart service answers each device message: taken, already held, or refused with its kind. A message the chart already holds changes nothing when sent again. A gap in a device's sequence leaves a second the chart expected and lacks, which shows as missing. Postgres itself refuses any update, delete or truncate of chart rows, and a version that changes which vital an entry is about.

## Sub-features

- `device-held` answers a resent batch as already held, and the chart holds each message once.
- `device-gap` shows the second a sequence gap skipped as missing.
- `device-append-only` shows Postgres refusing a change to chart rows.
- `device-fixed-field` shows Postgres refusing a third version of the RR entry about HR.

## How to get to it (user POV)

- A device posts batches to `POST /api/device/messages`. There is no page for this, so the recipe posts as a device would.

## Driving it with npm run drive

Preconditions:

- The launch and doctor checks pass. Read `now` from `GET /api/chart`.

- **Resend.** Post one batch from the synthetic device `verify-walk-device`, boot `verify-walk-boot`: a statement at now minus 4 s calling HR sourced, and HR readings with sequence 2 at now minus 4 s and sequence 4 at now minus 2 s. Post the same batch again. The first answer has `taken` for each message, the second `already held` for each.
- **Gap.** `GET /api/chart` lists a `missing` span for `verify-walk-device` HR that covers now minus 3 s, the second sequence 3 would have carried, and none covering now minus 4 s or now minus 2 s. It holds two readings from that device.
- **Append-only.** Run `docker exec <container> psql -U postgres -c "UPDATE content_vital SET vital = 'HR'"`, and the same with `DELETE FROM device_message` and `TRUNCATE entry_version`. Each fails with `the chart is append-only`.
- **Fixed field.** In one psql command, insert version 3 of the RR entry into `entry_version`, copied from its version 2, and its content into `content_vital` with the vital `HR`. It fails with `violates foreign key constraint`, so Postgres keeps the vital without the chart service. `walk.sh` holds the exact SQL.
- **Proof.** Keep both answers and the psql output, in `postgres-refusals.txt` and `postgres-fixed-field.txt`.

## Gotchas

- The fixed-field probe needs the amend recipe's RR entry at version 2.
- Use a device id of the walk's own, so its missing seconds are told apart from the monitor's.
