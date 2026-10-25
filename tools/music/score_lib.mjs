// Crab Breeze score reader: the one place a .strudel score becomes parts.
//
// A score is a plain Strudel file (it plays as written at https://strudel.cc)
// with a two-line header:
//     // @title Crab Breeze: Sortie
//     // @cycles 48
// Every layer names its instrument with `s(...)`: a General MIDI name the way
// Strudel spells it (`gm_french_horn`), a drum (`bd`, `sd`, `hh`, `cr`, ...),
// or a Strudel synth (`sawtooth`, `supersaw`, ...). This module evaluates the
// file with Strudel's own pattern engine, queries every event in the score's
// cycles and groups the notes into one part per instrument, so the MIDI and
// LMMS exporters (strudel_to_midi.mjs, strudel_to_lmms.mjs) share a single
// reading of the score and cannot disagree about a note.
//
// Adapted from Tom Lander tools/strudel (strudel_to_midi.mjs,
// core_suite_to_midi.mjs) at tom-lander dea3d50: there each multitrack score
// copied its patterns into its own exporter; here the instrument rides on the
// event (`s`), so one exporter serves every score.
import fs from 'fs';
import path from 'path';
import { evalScope, noteToMidi } from '@strudel/core';

/** Repository root (tools/music/ is two levels down). */
export const REPO = path.resolve(import.meta.dirname, '..', '..');

/**
 * General MIDI programs 0-127 under Strudel's names (@strudel/soundfonts
 * 1.2.6 gm.mjs, in GM order). `gm_piano` is Strudel's alias for program 0.
 */
export const GM_NAMES = `acoustic_piano bright_acoustic_piano electric_grand_piano
honky_tonk_piano epiano1 epiano2 harpsichord clavinet celesta glockenspiel music_box
vibraphone marimba xylophone tubular_bells dulcimer drawbar_organ percussive_organ
rock_organ church_organ reed_organ accordion harmonica bandoneon acoustic_guitar_nylon
acoustic_guitar_steel electric_guitar_jazz electric_guitar_clean electric_guitar_muted
overdriven_guitar distortion_guitar guitar_harmonics acoustic_bass electric_bass_finger
electric_bass_pick fretless_bass slap_bass_1 slap_bass_2 synth_bass_1 synth_bass_2 violin
viola cello contrabass tremolo_strings pizzicato_strings orchestral_harp timpani
string_ensemble_1 string_ensemble_2 synth_strings_1 synth_strings_2 choir_aahs voice_oohs
synth_choir orchestra_hit trumpet trombone tuba muted_trumpet french_horn brass_section
synth_brass_1 synth_brass_2 soprano_sax alto_sax tenor_sax baritone_sax oboe english_horn
bassoon clarinet piccolo flute recorder pan_flute blown_bottle shakuhachi whistle ocarina
lead_1_square lead_2_sawtooth lead_3_calliope lead_4_chiff lead_5_charang lead_6_voice
lead_7_fifths lead_8_bass_lead pad_new_age pad_warm pad_poly pad_choir pad_bowed
pad_metallic pad_halo pad_sweep fx_rain fx_soundtrack fx_crystal fx_atmosphere
fx_brightness fx_goblins fx_echoes fx_sci_fi sitar banjo shamisen koto kalimba bagpipe
fiddle shanai tinkle_bell agogo steel_drums woodblock taiko_drum melodic_tom synth_drum
reverse_cymbal guitar_fret_noise breath_noise seashore bird_tweet telephone helicopter
applause gunshot`.split(/\s+/).map((n) => `gm_${n}`);
if (GM_NAMES.length !== 128) throw new Error(`GM table has ${GM_NAMES.length} names, not 128`);

/** Drum names (Strudel's sample names) to General MIDI percussion keys (channel 10). */
export const DRUM_MAP = {
  bd: 36, bd2: 35, sd: 38, sd2: 40, rim: 37, cp: 39,
  hh: 42, ph: 44, oh: 46,                        // hats: closed, pedal, open
  ft: 41, lt: 45, mt: 47, ht: 50,                // toms: floor, low, mid, high
  cr: 49, cr2: 57, sp: 55, china: 52,            // crashes, splash, china
  rd: 51, rb: 53, rd2: 59,                       // ride, bell, ride 2
  tb: 54, sh: 70, cow: 56, clav: 75, cga: 63, bon: 60, // tambourine, shaker, cowbell, claves, conga, bongo
};

/** Strudel's built-in synth names, voiced by LMMS presets (music/instruments.json). */
export const SYNTHS = ['sawtooth', 'square', 'triangle', 'sine', 'supersaw'];

/** Loads music/instruments.json: the soundfont, the synth presets and the default mix. */
export function loadInstruments() {
  const file = path.join(REPO, 'music', 'instruments.json');
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

/** Reads `// @key value` header lines from a score's source. */
function header(code) {
  const out = {};
  for (const m of code.matchAll(/^\/\/\s*@(\w+)\s+(.+?)\s*$/gm)) out[m[1]] = m[2];
  return out;
}

const toMidi = (v) => (typeof v === 'number' ? Math.round(v) : noteToMidi(v));
const clamp = (x, lo, hi) => Math.max(lo, Math.min(hi, x));

/**
 * Classifies an `s` value. Returns { key, kind, program } where kind is
 * 'gm', 'drums' or 'synth', or null for a sound the exporters cannot voice.
 */
export function classify(s) {
  if (s === undefined || s === null) return { key: 'gm_acoustic_piano', kind: 'gm', program: 0 };
  if (s === 'gm_piano') return { key: 'gm_acoustic_piano', kind: 'gm', program: 0 };
  const program = GM_NAMES.indexOf(s);
  if (program >= 0) return { key: s, kind: 'gm', program };
  if (DRUM_MAP[s] !== undefined) return { key: 'drums', kind: 'drums', program: 0 };
  if (SYNTHS.includes(s)) return { key: s, kind: 'synth', program: 0 };
  return null;
}

/**
 * Evaluates a score and returns
 * { title, bpm, cycles, parts: [{ key, kind, program, notes: [{ t, dur, midi, vel }] }] }
 * with times in cycles (one cycle is one 4/4 bar) and parts in first-heard order.
 * An unknown instrument stops the export with its name: a silent dropped part
 * would ship a cue missing a voice.
 */
export async function loadScore(file, cyclesOverride) {
  const code = fs.readFileSync(file, 'utf8');
  const meta = header(code);
  const cycles = Number(cyclesOverride ?? meta.cycles);
  if (!Number.isFinite(cycles) || cycles <= 0) {
    throw new Error(`${file}: no "// @cycles N" header (the sum of the bars in arrange())`);
  }
  let cps = 0.5;
  await evalScope(import('@strudel/core'), import('@strudel/mini'),
    { setcps: (x) => { cps = x; }, setCps: (x) => { cps = x; } });
  const { evaluate } = await import('@strudel/transpiler');
  const { pattern } = await evaluate(code);

  const parts = new Map();
  const unknown = new Set();
  for (const hap of pattern.queryArc(0, cycles)) {
    if (!hap.hasOnset()) continue;
    const v = hap.value ?? {};
    const c = classify(v.s);
    if (!c) { unknown.add(v.s); continue; }
    const t = hap.whole.begin.valueOf();
    const dur = hap.whole.end.valueOf() - t;
    let midi;
    if (c.kind === 'drums') midi = DRUM_MAP[v.s];
    else {
      if (v.note === undefined && v.n === undefined) continue;
      midi = toMidi(v.note ?? v.n);
    }
    if (!Number.isFinite(midi)) continue;
    const vel = clamp(Math.round(100 * (v.velocity ?? 1) * (v.gain ?? 1)), 1, 127);
    if (!parts.has(c.key)) parts.set(c.key, { ...c, notes: [] });
    parts.get(c.key).notes.push({ t, dur, midi: clamp(midi, 0, 127), vel });
  }
  if (unknown.size) {
    throw new Error(`${file}: no instrument for s(${[...unknown].map((u) => JSON.stringify(u)).join(', ')})`);
  }
  for (const p of parts.values()) p.notes.sort((a, b) => a.t - b.t || a.midi - b.midi);
  return {
    title: meta.title ?? path.basename(file, '.strudel'),
    bpm: Math.round(cps * 240),
    cycles,
    parts: [...parts.values()],
  };
}

/** One line per part, for the exporters' logs. */
export function describe(score) {
  const secs = score.cycles * 240 / score.bpm;
  const lines = [`${score.title}: ${score.bpm} BPM, ${score.cycles} bars, ${secs.toFixed(1)} s`];
  for (const p of score.parts) lines.push(`  ${p.key.padEnd(26)} ${p.kind.padEnd(5)} ${p.notes.length} notes`);
  return lines.join('\n');
}
