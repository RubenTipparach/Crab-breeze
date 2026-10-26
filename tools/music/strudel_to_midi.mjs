// Strudel score -> multitrack General MIDI file (format 1), one track per instrument.
//
// The MIDI file is the score's hand-off to any DAW (LMMS, Ardour, Reaper) and the
// input of the fluidsynth preview (render.sh --preview). Parts come from
// score_lib.mjs; a Strudel synth part plays its General MIDI stand-in from
// music/instruments.json (`synths.<name>.gm`). Drums go to channel 10.
//
// Usage: node strudel_to_midi.mjs <score.strudel> <out.mid> [cycles]
//
// SMF writer adapted from Tom Lander tools/strudel/strudel_to_midi.mjs (dea3d50).
import fs from 'fs';
import { loadScore, loadInstruments, describe } from './score_lib.mjs';

const [infile, outfile, cyclesArg] = process.argv.slice(2);
if (!infile || !outfile) {
  console.error('usage: node strudel_to_midi.mjs <score.strudel> <out.mid> [cycles]');
  process.exit(1);
}
const score = await loadScore(infile, cyclesArg);
const kitLine = fs.readFileSync(infile, 'utf8').match(/^\/\/\s*@drumkit\s+(\d+)/m);
const drumkit = kitLine ? Number(kitLine[1]) : 0;   // GS drum kit: a program change on channel 10
const inst = loadInstruments();
console.log(describe(score));

const PPQ = 480, TPC = PPQ * 4; // ticks per quarter, per cycle (one 4/4 bar)
const vlq = (n) => { const o = [n & 0x7f]; n >>= 7; while (n > 0) { o.unshift((n & 0x7f) | 0x80); n >>= 7; } return o; };
const be16 = (n) => [(n >> 8) & 0xff, n & 0xff];
const be32 = (n) => [(n >>> 24) & 0xff, (n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
const chunk = (id, d) => [...id].map((c) => c.charCodeAt(0)).concat(be32(d.length), d);
const text = (type, s) => { const b = [...Buffer.from(s, 'utf8')]; return [0xff, type, ...vlq(b.length), ...b]; };
const trackChunk = (events) => {
  events.sort((a, b) => a.tick - b.tick || a.order - b.order);
  const d = []; let last = 0;
  for (const ev of events) { d.push(...vlq(ev.tick - last), ...ev.bytes); last = ev.tick; }
  d.push(...vlq(0), 0xff, 0x2f, 0x00);
  return chunk('MTrk', d);
};

const us = Math.round(60000000 / score.bpm);
const tracks = [trackChunk([
  { tick: 0, order: -3, bytes: [0xff, 0x51, 0x03, (us >> 16) & 0xff, (us >> 8) & 0xff, us & 0xff] },
  { tick: 0, order: -2, bytes: [0xff, 0x58, 0x04, 4, 2, 24, 8] },
  { tick: 0, order: -1, bytes: text(0x03, score.title) },
])];

const melodic = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15];
for (const part of score.parts) {
  let ch, program;
  if (part.kind === 'drums') { ch = 9; program = drumkit || undefined; }
  else {
    if (!melodic.length) throw new Error(`${infile}: more than 15 melodic instruments`);
    ch = melodic.shift();
    program = part.kind === 'gm' ? part.program : inst.synths[part.key]?.gm;
    if (program === undefined) throw new Error(`music/instruments.json: synths.${part.key}.gm missing`);
  }
  const ev = [{ tick: 0, order: -2, bytes: text(0x03, part.key) }];
  if (program !== undefined) ev.push({ tick: 0, order: -1, bytes: [0xc0 | ch, program] });
  for (const n of part.notes) {
    const tick = Math.round(n.t * TPC);
    const dur = part.kind === 'drums' ? 60 : Math.max(30, Math.round(n.dur * TPC) - 4);
    ev.push({ tick, order: 1, bytes: [0x90 | ch, n.midi, n.vel] });
    ev.push({ tick: tick + dur, order: 0, bytes: [0x80 | ch, n.midi, 0] });
  }
  tracks.push(trackChunk(ev));
}
const head = chunk('MThd', [...be16(1), ...be16(tracks.length), ...be16(PPQ)]);
fs.writeFileSync(outfile, Buffer.from([...head, ...tracks.flat()]));
console.log(`wrote ${outfile}`);
