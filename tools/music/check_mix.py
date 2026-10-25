#!/usr/bin/env python3
"""Checks a rendered cue: clipped samples in the raw mix, and the loudness shape.

Tom Lander's audio.md step 5: before calling a render done, look at its RMS per
block and confirm the arrangement's shape (intro quiet, peaks loud, tails
fading). A flat or empty profile means a wrong cycle count or a missing part.

Usage: check_mix.py <file.wav> [--profile] [--block SECONDS]
Exits 1 when more than 0.01 % of samples sit at full scale (a clipped comp).
"""
import array
import math
import sys
import wave


def main() -> int:
    path = sys.argv[1]
    profile = "--profile" in sys.argv
    block_s = float(sys.argv[sys.argv.index("--block") + 1]) if "--block" in sys.argv else 5.0
    w = wave.open(path)
    if w.getsampwidth() != 2:
        print(f"{path}: expected 16-bit PCM", file=sys.stderr)
        return 1
    ch, rate = w.getnchannels(), w.getframerate()
    data = array.array("h", w.readframes(w.getnframes()))
    if not data:
        print(f"{path}: empty", file=sys.stderr)
        return 1
    clipped = sum(1 for x in data if x >= 32767 or x <= -32768)
    frac = clipped / len(data)
    secs = len(data) / ch / rate
    print(f"{path.rsplit('/', 1)[-1]}: {secs:.1f} s, {clipped} clipped samples ({frac * 100:.3f} %)")
    if profile:
        n = int(block_s * rate * ch)
        bars = []
        for i in range(0, len(data), n):
            blk = data[i:i + n]
            rms = math.sqrt(sum(x * x for x in blk[::4]) / max(1, len(blk[::4])))
            db = 20 * math.log10(rms / 32768) if rms > 0 else -96.0
            bars.append(db)
        for i, db in enumerate(bars):
            width = max(0, int((db + 50) * 1.2))
            print(f"  {i * block_s:6.0f} s {db:6.1f} dBFS {'#' * width}")
    if frac > 0.0001:
        print("  the comp clips: lower track volumes (// @mix) or music/instruments.json master_vol", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
