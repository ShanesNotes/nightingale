import { render } from 'pele';
import { readFileSync, writeFileSync } from 'node:fs';

const [chartPath, reportPath, outPath] = process.argv.slice(2);
const PHOSPHOR = '#9CF2B8';
const AMBER = '#FFB547';
const MUTED = '#8FA3B0';
const SCREEN = '#0F1A24';
const DEEP = '#05090c';
const FONT = "'DejaVu Sans Mono', 'Menlo', 'Consolas', 'Liberation Mono', monospace";

const report = readFileSync(reportPath, 'utf8');
const reached = report.match(/TLC reached (\d+) states/)?.[1] ?? '?';
const held = (report.match(/## Rules that hold\n(.+)/)?.[1] ?? '').split(', ').filter(Boolean);
const rules = held.filter((r) => r !== 'NoDeadlock').length;
const deadlock = held.includes('NoDeadlock') ? 'NO DEADLOCK' : 'DEADLOCK';

let chart = readFileSync(chartPath, 'utf8').replace(/^---[\s\S]*?---\n/, '');
chart = chart.replace(/  state "Legend" as legend \{[\s\S]*?\n  \}\n/, '').replace(/^  class lg\d .*\n/gm, '');

const targets = [...chart.matchAll(/^\s*(\S+) --> (\S+?)(?::|$)/gm)].map((m) => m[2]);
const classOf = Object.fromEntries([...chart.matchAll(/^\s*class (s\d+) (\w+)$/gm)].map((m) => [m[1], m[2]]));
const tone = (cls) => (cls === 'orange' ? AMBER : cls === 'tan' ? MUTED : PHOSPHOR);

const measurer = { width: (text, size) => [...text].length * size * 0.62 };
const { svg, width, height } = render(chart, {
  limit: 1_000_000, outputLimit: 8_000_000, maxEdges: 10_000,
  measurer, fontFamily: 'monospace', fontFamilyMono: 'monospace', fontSize: 18, responsive: false,
});

let body = svg.replace(/^<svg[^>]*>/, '').replace(/<\/svg>$/, '');

body = body.replace(/<g class="pele-node pele-state (\w+)"([^>]*)>([\s\S]*?)<\/g>/g, (_, cls, attrs, inner) => {
  const color = tone(cls);
  const rect = inner.match(/<rect x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="([\d.]+)"/);
  const [x, y, w, h] = rect.slice(1).map(Number);
  const dash = cls === 'blue' ? ' stroke-dasharray="8 5"' : cls === 'tan' ? ' stroke-dasharray="2 5" stroke-linecap="round"' : '';
  const fillOpacity = cls === 'teal' ? 0.1 : 0.05;
  let shape = `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="6" fill="${color}" fill-opacity="${fillOpacity}" stroke="${color}" stroke-width="2"${dash}/>`;
  if (cls === 'orange') shape += `<rect x="${x + 4}" y="${y + 4}" width="${w - 8}" height="${h - 8}" rx="3" fill="none" stroke="${color}" stroke-width="1.2" stroke-opacity="0.8"/>`;
  const text = inner.replace(/<rect[^>]*\/>/, '').replace(/>([^<]+)</g, (_, t) => `>${t.toUpperCase()}<`)
    .replace('<text ', `<text fill="${color}" letter-spacing="1" `);
  return `<g${attrs}>${shape}${text}</g>`;
});

body = body.replace(/<g class="pele-edge pele-transition" data-id="edge(\d+)">([\s\S]*?)<\/g>/g, (_, n, inner) => {
  const color = tone(classOf[targets[n]]);
  return `<g stroke="${color}" stroke-opacity="0.7" stroke-width="1.6">${inner.replace(/fill="var\(--_l\)"/g, `fill="${color}" fill-opacity="0.85"`)}</g>`;
});

body = body.replace(/<g class="pele-edge-label" data-id="edge(\d+)">([\s\S]*?)<\/g>/g, (_, n, inner) => {
  const color = tone(classOf[targets[n]]);
  return `<g>${inner.replace(/fill="var\(--_bg\)"/, `fill="${SCREEN}" fill-opacity="0.92"`).replace('<text ', `<text fill="${color}" fill-opacity="0.9" `)}</g>`;
});

body = body
  .replace(/<text class="pele-title"[\s\S]*?<\/text>/, '')
  .replace(/<circle([^>]*)fill="var\(--_fg\)"/g, `<circle$1fill="${PHOSPHOR}"`)
  .replace(/var\(--_l\)/g, PHOSPHOR).replace(/var\(--_fg\)/g, PHOSPHOR).replace(/var\(--_\w+\)/g, 'none')
  .replace(/ class="[^"]*"/g, '').replace(/ data-id="[^"]*"/g, '');

const top = Number(svg.match(/<g transform="translate\(0,([\d.]+)\)"/)?.[1] ?? 0);
const chartW = width;
const chartH = height - top;
const bezel = 44;
const inset = 40;
const header = 64;
const legend = 96;
const foot = 92;
const screenW = chartW + inset * 2;
const screenH = header + chartH + legend + inset;
const W = screenW + bezel * 2;
const H = screenH + bezel + foot;
const sx = bezel;
const sy = bezel;

const legendY = sy + header + chartH + inset * 0.6;
const swatch = (x, y, kind) => {
  const s = { solid: [PHOSPHOR, ''], dashed: [PHOSPHOR, ' stroke-dasharray="8 5"'], dotted: [MUTED, ' stroke-dasharray="2 5" stroke-linecap="round"'], double: [AMBER, ''] }[kind];
  let r = `<rect x="${x}" y="${y}" width="44" height="22" rx="4" fill="${s[0]}" fill-opacity="0.08" stroke="${s[0]}" stroke-width="2"${s[1]}/>`;
  if (kind === 'double') r += `<rect x="${x + 4}" y="${y + 4}" width="36" height="14" rx="2" fill="none" stroke="${AMBER}" stroke-width="1.2"/>`;
  return r;
};
const legendItems = [
  ['solid', PHOSPHOR, 'RUNS TODAY IN NG-CONTRACT'],
  ['dashed', PHOSPHOR, 'TYPED IN THE CONTRACT SKETCH, BODIES TODO'],
  ['double', AMBER, 'NOT ADMITTED, REFUSED OR FAULTED'],
  ['dotted', MUTED, 'PLANNED, NOT IN THE SKETCH'],
];
const colW = screenW / 2 - inset;
const legendSvg = legendItems.map(([kind, color, label], i) => {
  const x = sx + inset + (i % 2) * colW;
  const y = legendY + Math.floor(i / 2) * 36;
  return `${swatch(x, y, kind)}<text x="${x + 58}" y="${y + 16}" fill="${color}" font-size="15" letter-spacing="0.5">${label}</text>`;
}).join('');

const screws = [[22, 22], [W - 22, 22], [22, H - 22], [W - 22, H - 22]].map(([x, y], i) =>
  `<g transform="translate(${x},${y}) rotate(${[20, -35, 60, 5][i]})"><circle r="7" fill="url(#screw)" stroke="#0b0a09" stroke-width="1"/><path d="M-5,0H5" stroke="#0b0a09" stroke-width="1.6"/></g>`).join('');

const knob = (x, y, label, turn) => `<g transform="translate(${x},${y})"><circle r="19" fill="#0d0c0b"/><circle r="16" fill="url(#knob)" stroke="#050505"/><path d="M0,-4V-13" stroke="#d8d2c4" stroke-width="2.2" stroke-linecap="round" transform="rotate(${turn})"/><text y="36" text-anchor="middle" fill="#8b857a" font-size="11" letter-spacing="1.5">${label}</text></g>`;

const footY = sy + screenH + foot / 2 - 4;
const scratches = [[0.12, 0.03, 70, -8], [0.71, 0.015, 110, 4], [0.9, 0.6, 40, 80], [0.02, 0.42, 55, 85], [0.38, 0.985, 90, -3]]
  .map(([fx, fy, len, a]) => `<path d="M0,0H${len}" transform="translate(${(W * fx).toFixed(1)},${(H * fy).toFixed(1)}) rotate(${a})" stroke="#fff" stroke-opacity="0.06" stroke-width="1"/>`).join('');

const out = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" font-family="${FONT}" font-size="18" role="img" aria-label="Nightingale engine lifecycle statechart">
<title>Nightingale engine lifecycle</title>
<defs>
<linearGradient id="case" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3b3833"/><stop offset="0.5" stop-color="#2a2824"/><stop offset="1" stop-color="#1a1917"/></linearGradient>
<linearGradient id="lip" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#000" stop-opacity="0.9"/><stop offset="1" stop-color="#4a4640" stop-opacity="0.6"/></linearGradient>
<radialGradient id="screw" cx="0.35" cy="0.35" r="0.8"><stop offset="0" stop-color="#7a756c"/><stop offset="1" stop-color="#2b2925"/></radialGradient>
<radialGradient id="knob" cx="0.35" cy="0.3" r="0.9"><stop offset="0" stop-color="#4b4741"/><stop offset="1" stop-color="#121110"/></radialGradient>
<radialGradient id="tube" cx="0.5" cy="0.45" r="0.75"><stop offset="0" stop-color="#13242c"/><stop offset="0.55" stop-color="${SCREEN}"/><stop offset="1" stop-color="${DEEP}"/></radialGradient>
<radialGradient id="vignette" cx="0.5" cy="0.5" r="0.72"><stop offset="0.6" stop-color="#000" stop-opacity="0"/><stop offset="0.88" stop-color="#000" stop-opacity="0.35"/><stop offset="1" stop-color="#000" stop-opacity="0.75"/></radialGradient>
<linearGradient id="glass" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.07"/><stop offset="0.35" stop-color="#fff" stop-opacity="0.015"/><stop offset="0.36" stop-color="#fff" stop-opacity="0"/></linearGradient>
<linearGradient id="sweep" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="${PHOSPHOR}" stop-opacity="0"/><stop offset="0.8" stop-color="${PHOSPHOR}" stop-opacity="0.03"/><stop offset="0.96" stop-color="${PHOSPHOR}" stop-opacity="0.06"/><stop offset="1" stop-color="${PHOSPHOR}" stop-opacity="0"/></linearGradient>
<pattern id="scan" width="4" height="4" patternUnits="userSpaceOnUse"><rect width="4" height="1.6" fill="#000" fill-opacity="0.32"/></pattern>
<pattern id="grid" width="48" height="48" patternUnits="userSpaceOnUse" x="${sx + inset}" y="${sy + header}"><path d="M48,0V48H0" fill="none" stroke="${PHOSPHOR}" stroke-opacity="0.06"/><circle cx="24" cy="24" r="0.9" fill="${PHOSPHOR}" fill-opacity="0.1"/></pattern>
<filter id="glow" x="-5%" y="-5%" width="110%" height="110%"><feGaussianBlur in="SourceGraphic" stdDeviation="2.6" result="blur"/><feComponentTransfer in="blur" result="wide"><feFuncA type="linear" slope="0.75"/></feComponentTransfer><feMerge><feMergeNode in="wide"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
<filter id="led" x="-200%" y="-200%" width="500%" height="500%"><feGaussianBlur stdDeviation="3"/><feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge></filter>
<filter id="grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed="7"/><feColorMatrix values="0 0 0 0 0.6  0 0 0 0 0.95  0 0 0 0 0.75  0 0 0 0.09 0"/></filter>
<clipPath id="screen"><rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" rx="26"/></clipPath>
</defs>
<rect width="${W}" height="${H}" rx="30" fill="url(#case)"/>
<rect x="1.5" y="1.5" width="${W - 3}" height="${H - 3}" rx="29" fill="none" stroke="#fff" stroke-opacity="0.08" stroke-width="2"/>
${scratches}${screws}
<rect x="${sx - 10}" y="${sy - 10}" width="${screenW + 20}" height="${screenH + 20}" rx="34" fill="url(#lip)"/>
<g clip-path="url(#screen)">
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" fill="url(#tube)"/>
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" fill="url(#grid)"/>
<rect x="${sx + screenW * 0.58}" y="${sy}" width="${screenW * 0.16}" height="${screenH}" fill="url(#sweep)"/>
<g filter="url(#glow)">
<text x="${sx + inset}" y="${sy + 40}" fill="${PHOSPHOR}" font-size="22" font-weight="bold" letter-spacing="2">NIGHTINGALE ENGINE LIFECYCLE</text>
<text x="${sx + screenW - inset}" y="${sy + 40}" fill="${PHOSPHOR}" fill-opacity="0.85" font-size="15" letter-spacing="1" text-anchor="end">TLC ${reached} STATES  ${rules} RULES HOLD  ${deadlock}</text>
<path d="M${sx + inset},${sy + header - 8}H${sx + screenW - inset}" stroke="${PHOSPHOR}" stroke-opacity="0.35"/>
<g transform="translate(${sx + inset},${sy + header})">${body.replace(/^\s*<g transform="translate\(0,[\d.]+\)">/, '<g>')}</g>
<path d="M${sx + inset},${legendY - 18}H${sx + screenW - inset}" stroke="${PHOSPHOR}" stroke-opacity="0.2" stroke-dasharray="2 6"/>
${legendSvg}
</g>
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" filter="url(#grain)"/>
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" fill="url(#scan)"/>
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" fill="url(#vignette)"/>
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" fill="url(#glass)"/>
</g>
<rect x="${sx}" y="${sy}" width="${screenW}" height="${screenH}" rx="26" fill="none" stroke="#000" stroke-width="3"/>
<text x="${sx + 6}" y="${footY + 8}" fill="#0e0d0c" font-size="22" font-weight="bold" letter-spacing="6">NIGHTINGALE</text>
<text x="${sx + 5}" y="${footY + 7}" fill="#7d776c" font-size="22" font-weight="bold" letter-spacing="6">NIGHTINGALE</text>
<text x="${sx + 5}" y="${footY + 28}" fill="#6a655c" font-size="11" letter-spacing="2.5">MODEL CHECKED WITH TLA+ AND TLC</text>
${knob(W - bezel - 210, footY, 'INTENSITY', -40)}${knob(W - bezel - 130, footY, 'FOCUS', 25)}
<circle cx="${W - bezel - 44}" cy="${footY}" r="6" fill="${PHOSPHOR}" filter="url(#led)"/>
<circle cx="${W - bezel - 44}" cy="${footY}" r="6" fill="none" stroke="#0b0a09" stroke-width="1.5"/>
<text x="${W - bezel - 44}" y="${footY + 36}" text-anchor="middle" fill="#8b857a" font-size="11" letter-spacing="1.5">POWER</text>
</svg>
`;
writeFileSync(outPath, out);
console.log(`${W}x${H}`);
