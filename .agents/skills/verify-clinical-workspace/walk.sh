#!/usr/bin/env bash
# Walks every mapped feature of the clinical workspace in headless Chromium against a fresh local Postgres, keeps
# screenshots and the chart's rows in the evidence folder, and stops what it started. Exit 0 only when every
# check passed. Usage, from anywhere:
#   bash .agents/skills/verify-clinical-workspace/walk.sh <evidence folder> [feature ...]
# Features: monitor chart-a-vital flowsheet amend outage device-messages (all, in that order, by default).
set -uo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)
pkg=$repo/clinical-workspace
evidence=${1:?usage: walk.sh <evidence folder> [feature ...]}
shift
features=("${@:-monitor chart-a-vital flowsheet amend outage device-messages}")
read -r -a features <<<"${features[*]}"
mkdir -p "$evidence"
evidence=$(cd "$evidence" && pwd)
export CW_DB_CONTAINER=cw-verify-$$
report=$evidence/walk.txt
: >"$report"
failures=0
cd "$pkg" || exit 9

note() { echo "$*" | tee -a "$report"; }
pass() { note "PASS $*"; }
fail() {
  note "FAIL $*"
  failures=$((failures + 1))
}
drive() { timeout 60 node scripts/drive.ts "$@"; }
state() { node -p "JSON.parse(require('fs').readFileSync('.dev/state.json', 'utf8'))$1" 2>/dev/null; }
shot() { drive shot "$1" "$evidence/$2.png" >/dev/null && note "shot $evidence/$2.png"; }
chart() { curl -sf "$service/api/chart" >"$evidence/chart-$1.json" && note "rows $evidence/chart-$1.json"; }
# Polls a tab until the element's text, its lines joined by spaces, matches the extended regex, or fails the check.
expect() {
  local tab=$1 selector=$2 pattern=$3 check=$4 wait=${5:-15} text=''
  for _ in $(seq 1 $((wait * 4))); do
    text=$(drive text "$tab" "$selector" 2>/dev/null | tr '\n' ' ' | sed 's/ *$//') && grep -Eq -- "$pattern" <<<"$text" && {
      pass "$check"
      return 0
    }
    sleep 0.25
  done
  fail "$check: $selector reads: $(head -c 300 <<<"$text" | tr '\n' ' ')"
  return 1
}
clock() { drive text "$1" "$2" | grep -Eo '[0-9]{2}:[0-9]{2}:[0-9]{2}' | head -1; }
shift_clock() { node -e "const [h,m,s]=process.argv[1].split(':').map(Number);const t=h*3600+m*60+s+Number(process.argv[2]);console.log([t/3600|0,(t/60|0)%60,t%60].map(n=>String(n).padStart(2,'0')).join(':'))" "$1" "$2"; }

cleanup() {
  note "cleanup"
  [ -f .dev/drive.json ] && drive stop >>"$report" 2>&1
  [ -n "${offline:-}" ] && kill "$offline" 2>/dev/null
  if [ -s "$evidence/service-again.pid" ]; then kill -TERM "$(cat "$evidence/service-again.pid")" 2>/dev/null && note "stopped the restarted chart service"; fi
  if [ -s "$evidence/launcher.pid" ]; then kill -TERM -- "-$(cat "$evidence/launcher.pid")" 2>/dev/null && note "stopped the launcher, its chart service and Vite"; fi
  sleep 2
  docker rm -f "$CW_DB_CONTAINER" >/dev/null 2>&1 && note "removed container $CW_DB_CONTAINER"
  docker volume rm "$CW_DB_CONTAINER" >/dev/null 2>&1 && note "removed volume $CW_DB_CONTAINER"
  note "evidence kept in $evidence"
}
trap cleanup EXIT

note "walk at $(git -C "$repo" rev-parse HEAD) on $(date -u +%FT%TZ), features: ${features[*]}"
npm run reset >>"$report" 2>&1
(
  exec 8>&-
  setsid node scripts/dev.ts >"$evidence/dev.log" 2>&1 &
  echo $! >"$evidence/launcher.pid"
)
for _ in $(seq 1 180); do
  grep -q '^dev: monitor' "$evidence/dev.log" && break
  kill -0 "$(cat "$evidence/launcher.pid")" 2>/dev/null || break
  sleep 1
done
service=$(state .service.url)
workspace=$(state .pages.workspace)
monitor=$(state .pages.monitor)
spec=$(sed -n 's/^spec_id //p' capture/307caf19/summary.txt)
packet=$(sed -n 's/^packet //p' capture/307caf19/summary.txt)
if [ -z "$service" ] || ! curl -sf "$service/api/run" | grep -q "\"specId\":\"$spec\""; then
  fail "launch: the chart service never answered with its run row (see $evidence/dev.log)"
  exit 1
fi
pass "launch: the chart service holds the run row for $spec"
for page in "$workspace" "$monitor"; do
  curl -sf "$page" >/dev/null && pass "launch: $page answers" || fail "launch: $page does not answer"
done
for pid in "$(state .launcher)" "$(state .service.pid)" "$(state .vite.pid)"; do
  kill -0 "$pid" 2>/dev/null && pass "doctor: pid $pid is alive" || fail "doctor: pid $pid is not alive"
done
(
  exec 8>&-
  drive start >>"$report"
)
drive open monitor "$monitor" >/dev/null
drive open workspace "$workspace" >/dev/null

for feature in "${features[@]}"; do
  note "== $feature"
  case $feature in
    monitor)
      expect monitor 'td[data-tile="HR"]' '^[0-9]+ /min$' 'monitor: heart rate shows a whole number per minute' 30
      expect monitor 'td[data-tile="ABP mean"]' '^[0-9]+ mmHg$' 'monitor: mean arterial pressure shows a whole number in mmHg'
      for vital in 'ABP sys' 'ABP dia' SpO2 RR Temp; do expect monitor "td[data-tile=\"$vital\"]" '^no source$' "monitor: $vital says no source"; done
      expect monitor '#feed' "recorded synthetic run, patient $packet" 'monitor: the feed is a recorded synthetic run of this patient'
      [ "$(drive eval monitor '/filtration/i.test(document.body.innerText)')" = false ] && pass 'monitor: filtration never appears' || fail 'monitor: filtration appears'
      expect monitor '#link' '^Everything delivered to the chart\.$' 'monitor: everything delivered on a healthy link'
      seen=$(drive eval monitor "[document.querySelector('#clock').innerText, document.querySelector('td[data-tile=\"HR\"]').innerText]")
      expected=$(node --input-type=module -e "
        import { readFileSync } from 'node:fs'; import { decodeRun } from './src/run.ts'; import { parseClock } from './src/time.ts'
        const [clock] = JSON.parse(process.argv[1]); const at = parseClock(clock.replace('Patient time ', ''))
        const d = decodeRun({ lens: readFileSync('capture/307caf19/lens.npy'), catalogue: readFileSync('capture/307caf19/catalogue.txt', 'utf8'), summary: readFileSync('capture/307caf19/summary.txt', 'utf8') })
        const o = d.run.observations.find((x) => x.channel === d.run.channels.indexOf('algebraic/CV/rate/body') && x.collected === at)
        console.log(o === undefined ? 'no observation for the clock ' + clock : o.value.kind === 'present' ? Math.round(o.value.value * 60) + ' /min' : o.value.kind)" "$seen" 2>&1)
      [ "$(node -p "JSON.parse(process.argv[1])[1]" "$seen")" = "$expected" ] && pass "monitor: heart rate $expected at $(node -p "JSON.parse(process.argv[1])[0]" "$seen") is the observer's value" || fail "monitor: shown $seen, the observer gives $expected"
      shot monitor 01-monitor
      ;;
    chart-a-vital)
      expect workspace '#service' '^The chart service is answering\.$' 'workspace: the chart service answers' 30
      now=$(clock workspace '#now')
      care=$(shift_clock "$now" -40)
      drive choose workspace '#author' synthetic-respiratory-therapist && drive choose workspace '#vital' RR
      drive fill workspace '#value' 18 && drive fill workspace '#care' "$care" && drive click workspace '#chart'
      expect workspace '#notice' '^Charted RR 18 /min as Synthetic Respiratory Therapist C\.$' "workspace: a respiratory therapist charts RR with care time $care"
      drive choose workspace '#author' synthetic-nurse && drive choose workspace '#vital' HR
      drive fill workspace '#value' 64 && drive fill workspace '#care' '' && drive click workspace '#chart'
      expect workspace '#notice' '^Charted HR 64 /min as Synthetic Nurse A\.$' 'workspace: a nurse charts HR with a blank care time, which is now'
      drive choose workspace '#author' synthetic-physician && drive choose workspace '#vital' Temp
      drive fill workspace '#value' 37.1 && drive click workspace '#chart'
      expect workspace '#notice' '^Charted Temp 37.1 °C as Synthetic Physician B\.$' 'workspace: a physician charts temperature'
      expect workspace '#flowsheet' '18 /min by Synthetic Respiratory Therapist C, care [0-9:]+, charted [0-9:]+, late by [0-9]+ s' 'workspace: the late RR entry shows who, both times and how late'
      shot workspace 02-charted
      drive fill workspace '#value' 61 && drive fill workspace '#care' "$(shift_clock "$(clock workspace '#now')" 120)" && drive click workspace '#chart'
      expect workspace '#notice' '^The care time is after now; nothing was charted\.$' 'workspace: a care time after now is refused in words'
      drive fill workspace '#value' '' && drive fill workspace '#care' '' && drive click workspace '#chart'
      expect workspace '#notice' '^The value is not a number; nothing was charted\.$' 'workspace: empty value text is refused, never charted as zero'
      shot workspace 03-refusals
      chart charted
      node -e "const m=JSON.parse(require('fs').readFileSync(process.argv[1]));const d=m.entries.map(e=>e.author.discipline).sort().join(',');if(d!=='nurse,physician,respiratory therapist'||!m.entries.every(e=>e.patient===m.run.patient&&e.care<=e.charted))process.exit(1)" "$evidence/chart-charted.json" && pass 'chart: three entries by three disciplines, each with its patient, care time and charted time' || fail 'chart: the entries do not name who, the patient and both times'
      ;;
    flowsheet)
      expect workspace 'tr[data-vital="HR"][data-source="monitor"]' '[0-9]+ /min' 'flowsheet: the HR monitor row shows readings'
      expect workspace 'tr[data-vital="SpO2"][data-source="monitor"]' 'no source' 'flowsheet: the SpO2 monitor row says no source'
      expect workspace 'tr[data-vital="HR"][data-source="charted"]' '64 /min by Synthetic Nurse A' 'flowsheet: the HR charted row holds the nurse entry apart from the monitor row'
      shot workspace 04-flowsheet
      ;;
    amend)
      drive click workspace 'tr[data-vital="RR"][data-source="charted"] button'
      expect workspace '#details' 'Version 1 of 1' 'amend: the RR entry shows version 1 of 1'
      expect workspace '#details' 'By Synthetic Respiratory Therapist C, respiratory therapist' 'amend: the details name the author and discipline'
      drive choose workspace '#author' synthetic-physician && drive click workspace '#edit'
      drive fill workspace '#edit-value' 20 && drive click workspace '#save-edit'
      expect workspace '#notice' '^Amended RR 20 /min as Synthetic Physician B\.$' 'amend: a physician amends the RR entry'
      expect workspace '#details' 'Version 2 of 2' 'amend: the details show version 2 of 2'
      expect workspace 'tr[data-vital="RR"][data-source="charted"]' '20 /min by Synthetic Physician B.*edited' 'amend: the flowsheet shows the new value, marked edited'
      [ "$(drive eval workspace "document.querySelector('#versions') === null")" = true ] && pass 'amend: earlier versions stay hidden until asked for' || fail 'amend: earlier versions show unasked'
      drive click workspace '#versions-button'
      expect workspace '#versions' 'Version 1: 18 /min.*Version 2: 20 /min' 'amend: the earlier version comes from the chart service on request'
      shot workspace 05-amended
      ;;
    outage)
      drive click workspace '#close'
      expect workspace '#closed' 'The chart is closed\.' 'outage: the workspace is closed'
      kill -TERM "$(state .service.pid)" && pass 'outage: the chart service is stopped'
      sleep 6
      expect monitor '#link' '^Holding [0-9]+ messages\. The chart service has not answered since [0-9:]+\.$' 'outage: the monitor holds its messages and says since when'
      first=$(clock monitor '#clock')
      sleep 2
      [ "$(clock monitor '#clock')" != "$first" ] && expect monitor 'td[data-tile="HR"]' '^[0-9]+ /min$' 'outage: the monitor keeps taking readings with the service stopped and the workspace closed'
      shot monitor 06-outage-monitor
      drive offline monitor 14 >>"$report" 2>&1 &
      offline=$!
      sleep 1
      (
        exec 8>&-
        CHART_DATABASE_URL=$(state .database.url) PORT=${service##*:} setsid node scripts/service.ts >"$evidence/service-again.log" 2>&1 &
        echo $! >"$evidence/service-again.pid"
      )
      for _ in $(seq 1 60); do curl -sf "$service/api/run" >/dev/null && break; sleep 0.5; done
      pass 'outage: the chart service is started again on its port, the monitor still cut off'
      drive click workspace '#reopen'
      expect workspace '#service' '^The chart service is answering\.$' 'outage: the reopened workspace reads the chart as it is now' 20
      expect workspace 'tr[data-vital="HR"][data-source="monitor"]' '[0-9]+ missing' 'outage: the seconds the monitor still holds show as missing'
      chart while-held
      shot workspace 07-missing
      wait "$offline"
      offline=''
      expect monitor '#link' '^Everything delivered to the chart\.$' 'outage: the monitor delivers what it held once it reaches the service' 20
      sleep 2
      chart after-delivery
      node -e "
        const [held, after] = process.argv.slice(1).map((f) => JSON.parse(require('fs').readFileSync(f)))
        const seconds = (m) => new Set(m.missing.filter((s) => s.vital === 'HR').flatMap((s) => { const out = []; for (let t = s.from; t <= s.through; t += 1e9) out.push(t); return out }))
        const before = seconds(held), now = seconds(after)
        const turned = [...before].filter((t) => !now.has(t))
        const late = after.readings.filter((r) => r.vital === 'HR' && Math.floor(r.arrived / 1e9) > Math.floor(r.available / 1e9))
        const lateAt = new Set(late.map((r) => r.collected))
        if (turned.length === 0 || !turned.every((t) => lateAt.has(t))) { console.error('turned', turned.length, 'late', late.length); process.exit(1) }
        console.log(turned.length + ' missing seconds of HR turned late')" "$evidence/chart-while-held.json" "$evidence/chart-after-delivery.json" >>"$report" 2>&1 \
        && pass 'outage: every missing second the monitor held turned into a late reading' || fail 'outage: missing seconds did not turn late'
      expect workspace 'tr[data-vital="HR"][data-source="monitor"]' '[0-9]+ late' 'outage: the flowsheet marks the held readings late' 20
      shot workspace 08-late
      shot monitor 09-monitor-recovered
      ;;
    device-messages)
      now=$(node -p "JSON.parse(require('fs').readFileSync('$evidence/chart-after-delivery.json')).now" 2>/dev/null || curl -sf "$service/api/chart" | node -p "JSON.parse(require('fs').readFileSync(0)).now")
      batch=$(node -e "
        const now = Number(process.argv[1]), s = 1e9, header = { device: 'verify-walk-device', boot: 'verify-walk-boot', patient: process.argv[2] }
        const channels = { HR: 'sourced', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'no source', SpO2: 'no source', RR: 'no source', Temp: 'no source' }
        const reading = (sequence, at) => ({ kind: 'reading', ...header, sequence, vital: 'HR', collected: at, available: at, value: { kind: 'present', value: 60 } })
        console.log(JSON.stringify([{ kind: 'statement', ...header, sequence: 1, at: now - 4 * s, channels }, reading(2, now - 4 * s), reading(4, now - 2 * s)]))" "$now" "$packet")
      curl -sf -X POST -H 'content-type: application/json' --data "$batch" "$service/api/device/messages" >"$evidence/device-first.json"
      curl -sf -X POST -H 'content-type: application/json' --data "$batch" "$service/api/device/messages" >"$evidence/device-again.json"
      grep -q '"taken"' "$evidence/device-first.json" && ! grep -q '"taken"' "$evidence/device-again.json" && grep -q '"already held"' "$evidence/device-again.json" \
        && pass 'device: the same messages sent again are answered already held and change nothing' || fail 'device: a resend was taken again'
      chart device-gap
      node -e "
        const m = JSON.parse(require('fs').readFileSync(process.argv[1])), now = Number(process.argv[2]), s = 1e9
        const gap = now - 3 * s
        const ours = m.missing.filter((x) => x.device === 'verify-walk-device' && x.vital === 'HR')
        const has = (t) => ours.some((x) => x.from <= t && t <= x.through)
        const held = m.readings.filter((r) => r.device === 'verify-walk-device').length
        if (!has(gap) || has(now - 4 * s) || has(now - 2 * s) || held !== 2) process.exit(1)" "$evidence/chart-device-gap.json" "$now" \
        && pass 'device: the gap in the sequence shows as a missing second, the held readings once each' || fail 'device: the sequence gap does not show as missing'
      {
        docker exec "$CW_DB_CONTAINER" psql -U postgres -c "UPDATE entry SET vital = 'HR'" 2>&1
        docker exec "$CW_DB_CONTAINER" psql -U postgres -c 'DELETE FROM device_message' 2>&1
        docker exec "$CW_DB_CONTAINER" psql -U postgres -c 'TRUNCATE entry_version' 2>&1
      } >"$evidence/postgres-refusals.txt"
      [ "$(grep -c 'append-only' "$evidence/postgres-refusals.txt")" -ge 3 ] && pass 'device: Postgres refuses an update, a delete and a truncate of chart rows' || fail 'device: Postgres let a chart row change'
      ;;
    *) fail "no feature named $feature" ;;
  esac
done

note "checks failed: $failures"
[ "$failures" -eq 0 ]
