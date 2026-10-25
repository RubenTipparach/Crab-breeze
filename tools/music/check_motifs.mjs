// Checks that every score in music/ carries the motif library word for word.
//
// music/motifs.strudel holds the canonical text of each motif between the
// MOTIF LIBRARY markers. A score is a self-contained Strudel file (so it can be
// pasted into strudel.cc), so it copies that block; this check refuses a copy
// that drifted, the way a change to a motif must reach every cue at once.
// Change a motif in motifs.strudel, then copy the block into each score:
//     node tools/music/check_motifs.mjs --fix
//
// Usage: node tools/music/check_motifs.mjs [--fix]
import fs from 'fs';
import path from 'path';
import { REPO } from './score_lib.mjs';

const START = /^\/\/ ── MOTIF LIBRARY ─+$/m;
const END = /^\/\/ ── END MOTIF LIBRARY ─+$/m;
const block = (text, file) => {
  const a = text.search(START), b = text.search(END);
  if (a < 0 || b < a) throw new Error(`${file}: no MOTIF LIBRARY block`);
  return [a, b + text.slice(b).indexOf('\n')];
};

const dir = path.join(REPO, 'music');
const canonText = fs.readFileSync(path.join(dir, 'motifs.strudel'), 'utf8');
const [ca, cb] = block(canonText, 'motifs.strudel');
const canon = canonText.slice(ca, cb);
const fix = process.argv.includes('--fix');
let bad = 0;
for (const f of fs.readdirSync(dir).filter((n) => n.endsWith('.strudel') && n !== 'motifs.strudel').sort()) {
  const text = fs.readFileSync(path.join(dir, f), 'utf8');
  const [a, b] = block(text, f);
  if (text.slice(a, b) === canon) { console.log(`ok    ${f}`); continue; }
  if (fix) {
    fs.writeFileSync(path.join(dir, f), text.slice(0, a) + canon + text.slice(b));
    console.log(`fixed ${f}`);
  } else {
    console.log(`DRIFT ${f}: its motif library differs from motifs.strudel (run with --fix)`);
    bad++;
  }
}
process.exit(bad ? 1 : 0);
