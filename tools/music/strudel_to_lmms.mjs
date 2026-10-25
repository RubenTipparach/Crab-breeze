// Strudel score -> LMMS 1.2 project (.mmp), one instrument track per part: the final comp.
//
// General MIDI parts and the drums play through LMMS's Sf2Player on the soundfont
// named in music/instruments.json; Strudel synth parts play the LMMS preset that
// file gives them (TripleOscillator, LB302, Monstro ...). The mix (track volume
// in percent, pan -100..100, the soundfont's reverb) comes from instruments.json
// and can be set per score with header lines:
//     // @mix gm_trumpet vol=85 pan=-20
//     // @reverb 0.5            (Sf2Player reverb level for this score)
// The project renders headless (render.sh) and opens in the LMMS GUI for hand
// mixing. LMMS cannot import MIDI headless (it wants the GUI), so the project is
// written directly, as Tom Lander's strudel_to_lmms.mjs (dea3d50) does for one
// piano track; this version writes every part.
//
// Usage: node strudel_to_lmms.mjs <score.strudel> <out.mmp> [cycles]
import fs from 'fs';
import path from 'path';
import { loadScore, loadInstruments, describe, REPO } from './score_lib.mjs';

const [infile, outfile, cyclesArg] = process.argv.slice(2);
if (!infile || !outfile) {
  console.error('usage: node strudel_to_lmms.mjs <score.strudel> <out.mmp> [cycles]');
  process.exit(1);
}
const score = await loadScore(infile, cyclesArg);
const inst = loadInstruments();
console.log(describe(score));

// per-score mix lines
const code = fs.readFileSync(infile, 'utf8');
const mix = {};
for (const m of code.matchAll(/^\/\/\s*@mix\s+(\S+)\s+(.+?)\s*$/gm)) {
  mix[m[1]] = Object.fromEntries([...m[2].matchAll(/(\w+)=(-?[\d.]+)/g)].map((x) => [x[1], Number(x[2])]));
}
const reverbLine = code.match(/^\/\/\s*@reverb\s+([\d.]+)/m);
const reverb = reverbLine ? Number(reverbLine[1]) : inst.reverb;

const LT = 192;                       // LMMS ticks per 4/4 bar
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
const soundfont = path.resolve(inst.soundfont);
if (!fs.existsSync(soundfont)) throw new Error(`soundfont not found: ${soundfont} (music/instruments.json)`);

function sf2(bank, patch) {
  return `<instrument name="sf2player">
<sf2player src="${esc(soundfont)}" patch="${patch}" bank="${bank}" gain="${inst.sf2_gain}" reverbOn="1" reverbLevel="${reverb}" reverbRoomSize="0.8" reverbDamping="0.3" reverbWidth="1" chorusOn="0" chorusNum="3" chorusLevel="2" chorusSpeed="0.3" chorusDepth="8"/>
</instrument>
<eldata fwet="0" ftype="0" fcut="14000" fres="0.5">
<elvol pdel="0" amt="0" latt="0" rel="0.1" sustain="1" hold="0" dec="0.5" att="0" lamt="0" x100="0" lshp="0" ctlenvamt="0" lspd="0.1" userwavefile=""/>
<elcut pdel="0" amt="0" latt="0" rel="0.1" sustain="0.5" hold="0.5" dec="0.5" att="0" lamt="0" x100="0" lshp="0" ctlenvamt="0" lspd="0.1" userwavefile=""/>
<elres pdel="0" amt="0" latt="0" rel="0.1" sustain="0.5" hold="0.5" dec="0.5" att="0" lamt="0" x100="0" lshp="0" ctlenvamt="0" lspd="0.1" userwavefile=""/>
</eldata>
<chordcreator chordrange="1" chord="0" chord-enabled="0"/>
<arpeggiator arprange="1" arptime="100" arpmode="0" arpdir="0" arp="0" arpgt="100" arp-enabled="0" arpskip="0" arpmiss="0" arpcycle="0"/>
<midiport inputchannel="0" outputchannel="1" fixedoutputnote="-1" outputprogram="1" readable="0" writable="0" inputcontroller="0" outputcontroller="0" fixedinputvelocity="-1" fixedoutputvelocity="-1" basevelocity="127" receive="0" default-velocity="0"/>
<fxchain numofeffects="0" enabled="0"/>`;
}

// the inside of a preset's <instrumenttrack>, with its base note
function preset(rel) {
  const file = path.join(inst.lmms_presets, rel);
  const xml = fs.readFileSync(file, 'utf8');
  const m = xml.match(/<instrumenttrack\b([^>]*)>([\s\S]*?)<\/instrumenttrack>/);
  if (!m) throw new Error(`${file}: no <instrumenttrack>`);
  const base = m[1].match(/basenote="(\d+)"/);
  return { body: m[2], basenote: base ? Number(base[1]) : 57 };
}

let tracks = '';
for (const part of score.parts) {
  const def = part.kind === 'synth' ? inst.synths[part.key] : inst.mix[part.key] ?? {};
  if (part.kind === 'synth' && !def?.lmms_preset) throw new Error(`music/instruments.json: synths.${part.key}.lmms_preset missing`);
  const vol = mix[part.key]?.vol ?? def.vol ?? inst.default_vol;
  const pan = mix[part.key]?.pan ?? def.pan ?? 0;
  let body, basenote = 57;
  if (part.kind === 'synth') ({ body, basenote } = preset(def.lmms_preset));
  else if (part.kind === 'drums') body = sf2(128, 0);
  else body = sf2(0, part.program);
  // LMMS key 57 is A4 (MIDI 69) at base note 57; a preset's other base note shifts it
  const shift = 12 + (basenote - 57);
  let notes = '';
  for (const n of part.notes) {
    const pos = Math.round(n.t * LT);
    const len = part.kind === 'drums' ? 6 : Math.max(3, Math.round(n.dur * LT) - 1);
    const key = n.midi - shift;
    const v = Math.max(1, Math.min(200, Math.round(n.vel / 127 * 200)));
    notes += `<note pan="0" key="${key}" vol="${v}" pos="${pos}" len="${len}"/>\n`;
  }
  tracks += `<track muted="0" type="0" name="${esc(part.key)}" solo="0">
<instrumenttrack pitch="0" pitchrange="1" vol="${vol}" basenote="${basenote}" pan="${pan}" fxch="0" usemasterpitch="1">
${body}
</instrumenttrack>
<pattern muted="0" type="1" name="${esc(part.key)}" pos="0" len="${score.cycles * LT}" steps="16">
${notes}</pattern>
</track>
`;
}

const mmp = `<?xml version="1.0"?>
<!DOCTYPE lmms-project>
<lmms-project version="1.0" type="song" creator="LMMS" creatorversion="1.2.2">
<head timesig_numerator="4" mastervol="${inst.master_vol}" masterpitch="0" timesig_denominator="4" bpm="${score.bpm}"/>
<song>
<trackcontainer type="song" width="600" maximized="0" height="300" x="5" y="5" visible="1">
${tracks}</trackcontainer>
<fxmixer>
<fxchannel num="0" name="Master" volume="1" muted="0">
<fxchain numofeffects="0" enabled="0"/>
</fxchannel>
</fxmixer>
<controllers/>
<projectnotes><![CDATA[${score.title}. Generated by tools/music/strudel_to_lmms.mjs from ${path.relative(REPO, path.resolve(infile))}; edit the score, not this file.]]></projectnotes>
<timeline lp0pos="0" lpstate="0" lp1pos="${LT}" stopbehaviour="0"/>
</song>
</lmms-project>
`;
fs.writeFileSync(outfile, mmp);
console.log(`wrote ${outfile}: ${score.parts.length} tracks`);
