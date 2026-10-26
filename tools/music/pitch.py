#!/usr/bin/env python3
"""Measures how far an LMMS preset sounds from the note it is given (calibrate.sh uses it).

Many LMMS presets carry an octave offset in their oscillators (a bass preset sounding an octave or two
below its key, a pluck an octave above). calibrate.sh renders A3 (220 Hz) through each preset and
calls this on the WAV. Two estimators must agree before an offset is trusted: the autocorrelation's
first strong peak and the harmonic product spectrum. Each alone makes octave errors on detuned or
resonant sounds (a reese, a resonant filter); agreeing, they have not on any preset measured.

Usage: pitch.py <file.wav> [expected_hz]
Prints "<offset in semitones>" rounded to an octave, or "?" with both estimates when they disagree.
"""
import sys
import wave

import numpy as np


def load(path: str) -> np.ndarray:
    w = wave.open(path)
    d = np.frombuffer(w.readframes(w.getnframes()), "<i2").astype(float)
    return d.reshape(-1, w.getnchannels())[:, 0]


def autocorr_hz(seg: np.ndarray, sr: int = 44100) -> float:
    seg = seg - seg.mean()
    ac = np.correlate(seg, seg, "full")[len(seg) - 1:]
    lo, hi = int(sr / 1200), int(sr / 40)
    r = ac[lo:hi] / ac[0]
    i = int(np.argmax(r > 0.85 * r.max()))
    while i + 1 < len(r) and r[i + 1] > r[i]:
        i += 1
    return sr / (lo + i)


def hps_hz(seg: np.ndarray, sr: int = 44100) -> float:
    sp = np.abs(np.fft.rfft(seg * np.hanning(len(seg))))
    fr = np.fft.rfftfreq(len(seg), 1 / sr)
    h = sp.copy()
    for k in (2, 3, 4):
        dec = sp[::k]
        h[: len(dec)] *= dec
    m = (fr > 40) & (fr < 1200)
    return float(fr[m][np.argmax(h[m])])


def main() -> int:
    d = load(sys.argv[1])
    expected = float(sys.argv[2]) if len(sys.argv) > 2 else 220.0
    seg = d[int(0.25 * 44100): int(0.25 * 44100) + 32768]
    if len(seg) < 8192 or np.max(np.abs(seg)) < 20:
        print("? silent")
        return 0
    a = 12 * np.log2(autocorr_hz(seg[:8192]) / expected)
    b = 12 * np.log2(hps_hz(seg) / expected)
    oa, ob = round(a / 12) * 12, round(b / 12) * 12
    if oa == ob and abs(a - oa) < 0.6 and abs(b - ob) < 0.6:
        print(oa)
    else:
        print(f"? {a:+.1f} {b:+.1f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
