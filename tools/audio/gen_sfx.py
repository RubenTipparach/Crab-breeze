#!/usr/bin/env python3
"""Crab Breeze's first sound effects, synthesised: the one place each placeholder sound is made.

The cloud session cannot reach OpenGameArt, Kenney or Freesound (docs/analysis/audio-sources.md),
so the first set for openspec/changes/audio and weapon-kinds is built here from oscillators, noise,
filters and envelopes: deterministic (each cue seeds its own generator from its name), mono 44.1 kHz
(the game places them in 3D), peak-normalised to -1 dBFS (the cue's volume in data sets its level).
Voiced electronic and futuristic, to sit with the score (owner, 2026-10-25: "electronic, scifi
futuristic please!"). A recorded CC0 sound that beats one of these replaces it in
assets/audio/sfx/ with its SOURCES.md entry; the cue name stays.

Usage: python3 tools/audio/gen_sfx.py [--out DIR] [--only NAME ...]
Writes DIR/<cue>.wav (default output/sfx); tools/audio/render_sfx.sh encodes them.
"""
import argparse
import json
import os
import sys
import wave
import zlib

import numpy as np

SR = 44100


# ---------------------------------------------------------------- building blocks

def rng_for(name: str) -> np.random.Generator:
    """A generator seeded from the cue's name, so every run makes the same sound."""
    return np.random.default_rng(zlib.crc32(name.encode()))


def t_axis(seconds: float) -> np.ndarray:
    return np.arange(int(seconds * SR)) / SR


def phase(freq_hz) -> np.ndarray:
    """Running phase (radians) for a frequency curve, so sweeps stay continuous."""
    return 2 * np.pi * np.cumsum(np.asarray(freq_hz, dtype=np.float64)) / SR


def sine(freq_hz) -> np.ndarray:
    return np.sin(phase(freq_hz))


def saw(freq_hz) -> np.ndarray:
    p = phase(freq_hz) / (2 * np.pi)
    return 2 * (p - np.floor(p + 0.5))


def square(freq_hz, duty: float = 0.5) -> np.ndarray:
    p = (phase(freq_hz) / (2 * np.pi)) % 1.0
    return np.where(p < duty, 1.0, -1.0)


def noise(rng: np.random.Generator, n: int) -> np.ndarray:
    return rng.uniform(-1, 1, n)


def sweep(a: float, b: float, n: int, curve: float = 1.0) -> np.ndarray:
    """From a to b over n samples; curve > 1 lingers near a, < 1 rushes to b. Exponential in pitch."""
    x = np.linspace(0, 1, n) ** curve
    if a > 0 and b > 0:
        return a * (b / a) ** x
    return a + (b - a) * x


def exp_decay(n: int, seconds: float) -> np.ndarray:
    """An envelope falling by 60 dB over `seconds`."""
    return np.exp(-6.9 * np.arange(n) / (seconds * SR))


def adsr(n: int, a: float, d: float, s: float, r: float) -> np.ndarray:
    a_n, d_n, r_n = int(a * SR), int(d * SR), int(r * SR)
    s_n = max(0, n - a_n - d_n - r_n)
    env = np.concatenate([
        np.linspace(0, 1, a_n, endpoint=False),
        np.linspace(1, s, d_n, endpoint=False),
        np.full(s_n, s),
        np.linspace(s, 0, r_n),
    ])
    return np.pad(env, (0, max(0, n - len(env))))[:n]


def onepole_lp(x: np.ndarray, cutoff_hz) -> np.ndarray:
    """One-pole lowpass with a fixed or per-sample cutoff."""
    c = np.broadcast_to(np.asarray(cutoff_hz, dtype=np.float64), x.shape)
    a = 1 - np.exp(-2 * np.pi * c / SR)
    y = np.empty_like(x)
    z = 0.0
    for i in range(len(x)):
        z += a[i] * (x[i] - z)
        y[i] = z
    return y


def onepole_hp(x: np.ndarray, cutoff_hz) -> np.ndarray:
    return x - onepole_lp(x, cutoff_hz)


def bandpass(x: np.ndarray, centre_hz, q: float) -> np.ndarray:
    """A state-variable bandpass with a fixed or swept centre (Chamberlin form, stable below SR/6)."""
    f = np.broadcast_to(np.asarray(centre_hz, dtype=np.float64), x.shape)
    k = 2 * np.sin(np.pi * np.minimum(f, SR / 6.5) / SR)
    damp = 1.0 / q
    low = band = 0.0
    y = np.empty_like(x)
    for i in range(len(x)):
        low += k[i] * band
        high = x[i] - low - damp * band
        band += k[i] * high
        y[i] = band
    return y


def shape(x: np.ndarray, drive: float) -> np.ndarray:
    """Soft saturation."""
    return np.tanh(x * drive) / np.tanh(drive)


def _svf(x: np.ndarray, cutoff_hz, q: float, mode: str) -> np.ndarray:
    """One zero-delay-feedback state-variable filter stage (12 dB/octave), stable while its
    cutoff sweeps per sample (Zavalishin's topology-preserving form)."""
    c = np.broadcast_to(np.asarray(cutoff_hz, dtype=np.float64), x.shape)
    g = np.tan(np.pi * np.clip(c, 5.0, SR * 0.45) / SR)
    k = 1.0 / q
    a1 = 1.0 / (1.0 + g * (g + k))
    a2 = g * a1
    a3 = g * a2
    ic1 = ic2 = 0.0
    y = np.empty_like(x)
    low = mode == "lp"
    for i in range(len(x)):
        v3 = x[i] - ic2
        v1 = a1[i] * ic1 + a2[i] * v3
        v2 = ic2 + a2[i] * ic1 + a3[i] * v3
        ic1 = 2 * v1 - ic1
        ic2 = 2 * v2 - ic2
        y[i] = v2 if low else x[i] - k * v1 - v2
    return y


def lp(x: np.ndarray, cutoff_hz, order: int = 4, q: float = 0.707) -> np.ndarray:
    """A steep lowpass: `order` / 2 SVF stages (order 4 is 24 dB/octave). Combustion, blasts and
    rumble live under it: the onepole_lp above is too gentle and leaves them hissing."""
    for _ in range(order // 2):
        x = _svf(x, cutoff_hz, q, "lp")
    return x


def hp(x: np.ndarray, cutoff_hz, order: int = 2) -> np.ndarray:
    """A steep highpass (order 2 is 12 dB/octave); used at 20-30 Hz to keep sub energy audible."""
    for _ in range(order // 2):
        x = _svf(x, cutoff_hz, 0.707, "hp")
    return x


def brown(rng: np.random.Generator, n: int) -> np.ndarray:
    """Brown (red) noise: integrated white noise with a slight leak, the colour of a roar."""
    w = rng.uniform(-1, 1, n)
    y = np.empty(n)
    z = 0.0
    for i in range(n):
        z = 0.998 * z + w[i] * 0.05
        y[i] = z
    return y / (np.max(np.abs(y)) or 1.0)


def turbulence(rng: np.random.Generator, n: int, rate_hz: float = 7.0, depth: float = 0.45) -> np.ndarray:
    """A burning flame's unsteady swell: slow random amplitude between 1 - depth and 1 + depth."""
    m = lp(rng.uniform(-1, 1, n), rate_hz, order=2)
    m = m / (np.max(np.abs(m)) or 1.0)
    return 1.0 + depth * m


def crackle(rng: np.random.Generator, n: int, density: float = 0.004, cutoff_hz: float = 2500.0) -> np.ndarray:
    """Sparse pops of burning fuel, low passed so they sound like fire, not like static."""
    pops = rng.uniform(-1, 1, n) * (rng.uniform(0, 1, n) < density)
    return lp(pops, cutoff_hz, order=2)


def ring(freqs, decays_s, n: int, amps=None) -> np.ndarray:
    """Inharmonic struck-metal partials."""
    amps = amps or [1.0] * len(freqs)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for f, d, a in zip(freqs, decays_s, amps):
        out += a * np.sin(2 * np.pi * f * t) * np.exp(-6.9 * t / d)
    return out


def reverb(x: np.ndarray, room: float = 0.5, wet: float = 0.3, tail_s: float = 1.5) -> np.ndarray:
    """A small Schroeder reverb (four combs, two allpasses), with a tail appended."""
    x = np.concatenate([x, np.zeros(int(tail_s * SR))])
    out = np.zeros_like(x)
    for delay_ms, g in ((29.7, 0.805), (37.1, 0.827), (41.1, 0.783), (43.7, 0.764)):
        d = int(delay_ms * SR / 1000 * (0.6 + room))
        gg = g * (0.75 + 0.25 * room)
        buf = np.zeros_like(x)
        for i in range(len(x)):
            buf[i] = x[i] + (gg * buf[i - d] if i >= d else 0.0)
        out += buf
    for delay_ms, g in ((5.0, 0.7), (1.7, 0.7)):
        d = int(delay_ms * SR / 1000)
        y = np.zeros_like(out)
        for i in range(len(out)):
            xd = out[i - d] if i >= d else 0.0
            yd = y[i - d] if i >= d else 0.0
            y[i] = -g * out[i] + xd + g * yd
        out = y
    return x * (1 - wet) + out * wet * 0.25


def fit(x: np.ndarray, n: int) -> np.ndarray:
    return np.pad(x, (0, max(0, n - len(x))))[:n]


def mix(*parts: np.ndarray) -> np.ndarray:
    n = max(len(p) for p in parts)
    return sum(fit(p, n) for p in parts)


def loopable(x: np.ndarray, fade_s: float = 0.15) -> np.ndarray:
    """Crossfades the end into the start so the sound loops with no click."""
    f = int(fade_s * SR)
    head, body, tail = x[:f], x[f:-f], x[-f:]
    w = np.linspace(0, 1, f)
    return np.concatenate([tail * (1 - w) + head * w, body])


def at(x: np.ndarray, seconds: float, n: int) -> np.ndarray:
    """x placed `seconds` into a buffer of n samples."""
    s = int(seconds * SR)
    out = np.zeros(n)
    if s >= n:
        return out
    end = min(n, s + len(x))
    out[s:end] = x[: end - s]
    return out


# ---------------------------------------------------------------- the sounds

SOUNDS = {}


def sound(name, group, what, peak_db=-1.0):
    """Register a cue. peak_db is the level it is written at: -1 dBFS for blasts and footsteps,
    lower for cockpit tones that should sit under the fight rather than over it."""
    def deco(fn):
        SOUNDS[name] = (group, what, fn, peak_db)
        return fn
    return deco


# --- the mech

@sound("foot_stomp", "Mech", "A 24 m mech's foot landing: a sub thump you feel, the ground's low crunch, the leg's metal ringing and a hiss of hydraulics.")
def foot_stomp(r):
    n = int(1.8 * SR)
    thump = sine(sweep(58, 26, n, 0.5)) * exp_decay(n, 0.8) * 1.6
    crunch = lp(noise(r, n), sweep(1400, 120, n, 0.4)) * exp_decay(n, 0.5) * 3.0
    rumble = lp(brown(r, n), 90) * adsr(n, 0.01, 0.3, 0.3, 1.0) * 2.0
    clank = lp(ring([143, 331, 517, 911], [0.9, 0.6, 0.4, 0.25], n, [0.5, 0.35, 0.25, 0.15]), 2500, order=2)
    hiss = onepole_hp(noise(r, n), 3000) * adsr(n, 0.12, 0.2, 0.0, 0.01) * 0.15
    return hp(shape(mix(thump, crunch, rumble, clank * 0.5, at(hiss, 0.05, n)), 1.8), 22)


@sound("foot_splash", "Mech", "A foot coming down in water: the thump under a heavy wash of spray.")
def foot_splash(r):
    n = int(2.0 * SR)
    thump = sine(sweep(55, 30, n, 0.5)) * exp_decay(n, 0.6)
    wash = bandpass(noise(r, n), sweep(2400, 500, n, 0.6), 1.2) * exp_decay(n, 1.4) * 3
    drops = bandpass(noise(r, n) * (r.uniform(0, 1, n) > 0.9985), 3500, 4) * exp_decay(n, 1.8) * 6
    return mix(thump, wash, drops)


@sound("servo_torso", "Mech", "The torso twisting: a geared electric drive spinning up, holding and winding down.")
def servo_torso(r):
    n = int(1.6 * SR)
    spin = adsr(n, 0.25, 0.1, 0.85, 0.35)
    f = 70 + 55 * spin
    motor = saw(f) * 0.5 + saw(f * 2.01) * 0.25 + sine(f * 12.0) * 0.12
    gears = bandpass(noise(r, n), f * 9, 6) * 0.8
    return onepole_lp(motor + gears, 2600) * spin


@sound("hydraulic", "Mech", "A hydraulic ram extending: a pump stuttering under a rising pressurised hiss.")
def hydraulic(r):
    n = int(1.4 * SR)
    pump = sine(48) * (0.5 + 0.5 * square(9.0)) * adsr(n, 0.05, 0.1, 0.7, 0.2) * 0.6
    hiss = bandpass(noise(r, n), sweep(300, 1400, n), 3) * adsr(n, 0.1, 0.2, 0.8, 0.25) * 1.5
    stop = ring([210, 470, 830], [0.5, 0.3, 0.2], n) * 0.4
    return mix(pump, hiss, at(stop, 1.15, n))


@sound("hatch_seal", "Mech", "The cockpit seal breaking: a pressure pop and a short rush of air.")
def hatch_seal(r):
    n = int(1.2 * SR)
    pop = onepole_lp(noise(r, n), 900) * exp_decay(n, 0.08) * 3
    rush = onepole_hp(noise(r, n), 1200) * exp_decay(n, 0.6) * 0.8
    thunk = sine(sweep(120, 60, n)) * exp_decay(n, 0.15)
    return mix(pop, rush, thunk)


@sound("hatch_steam", "Mech", "Steam and mist venting from the opening chest: a long, loud hiss that settles.")
def hatch_steam(r):
    n = int(3.2 * SR)
    pink = onepole_lp(noise(r, n), 6000)
    env = adsr(n, 0.04, 0.5, 0.45, 2.2)
    flutter = 1 + 0.15 * sine(np.full(n, 13.0))
    return onepole_hp(pink, 700) * env * flutter * 1.6


@sound("hatch_motor", "Mech", "A hatch panel swinging on its motor and landing on its stop with a heavy clunk.")
def hatch_motor(r):
    n = int(2.2 * SR)
    run = adsr(n, 0.15, 0.1, 0.9, 0.15)[: int(1.7 * SR)]
    run = fit(run, n)
    motor = onepole_lp(saw(90 + 40 * run) + 0.3 * saw(181 + 80 * run), 1800) * run * 0.6
    clunk = mix(sine(sweep(90, 45, n)) * exp_decay(n, 0.3), ring([160, 372, 640], [0.7, 0.5, 0.3], n) * 0.5,
                onepole_lp(noise(r, n), 800) * exp_decay(n, 0.12) * 2)
    return mix(motor, at(clunk, 1.7, n))


@sound("jet_ignite", "Mech", "The jets lighting: igniters ticking, a deep whoomp as the fuel catches, then a roar swelling to full thrust with the cockpit shaking.")
def jet_ignite(r):
    n = int(2.6 * SR)
    ticks = lp(noise(r, n) * (np.sin(2 * np.pi * 14 * np.arange(n) / SR) > 0.97), 3500, order=2) * adsr(n, 0.0, 0.0, 1.0, 0.0) * 1.5
    ticks *= (np.arange(n) < int(0.35 * SR))
    whoomp = mix(sine(sweep(80, 32, n, 0.4)) * exp_decay(n, 0.7) * 1.8,
                 lp(brown(r, n), sweep(1800, 140, n, 0.35)) * exp_decay(n, 0.9) * 3.0)
    roar = lp(brown(r, n), sweep(180, 900, n, 0.6)) * turbulence(r, n, 6, 0.35) * adsr(n, 0.7, 0.3, 0.9, 0.25) * 2.4
    sub = lp(noise(r, n), 55) * adsr(n, 0.6, 0.3, 0.9, 0.3) * 6.0
    fire = crackle(r, n, 0.003, 1800) * adsr(n, 0.5, 0.2, 0.7, 0.3) * 2.0
    return hp(shape(mix(ticks, at(whoomp, 0.35, n), at(roar, 0.3, n), at(sub, 0.3, n), at(fire, 0.4, n)), 1.6), 22)


@sound("rocket_loop", "Mech", "The jets burning (loops): a deep, unsteady roar of burning fuel with crackle, for while the rockets hold the mech up.")
def rocket_loop(r):
    n = int(3.0 * SR)
    roar = lp(brown(r, n), 700) * turbulence(r, n, 5, 0.35) * 2.5
    body = lp(noise(r, n), 160) * turbulence(r, n, 9, 0.25) * 4.0
    fire = crackle(r, n, 0.004, 1600) * 2.0
    return loopable(hp(shape(roar + body + fire, 1.5), 22), 0.3)


@sound("rocket_rumble", "Mech", "The sub-bass under the jets (loops): the cockpit shaking, 25 to 60 Hz, ragged like the flame.")
def rocket_rumble(r):
    n = int(3.0 * SR)
    sub = lp(noise(r, n), 60) * turbulence(r, n, 4, 0.4) * 8.0
    tone = sine(38 + 6 * lp(r.uniform(-1, 1, n), 2, order=2) * 40) * 0.35
    return loopable(hp(shape(sub + tone, 1.3), 20), 0.3)


@sound("skate_roll", "Mech", "Skating on the powered wheels (loops): heavy tyres roaring on asphalt, a thump at every road seam, a gritty drive motor and the scrape of the wheels sliding, the boost hissing under it all.")
def skate_roll(r):
    n = int(3.0 * SR)
    t = np.arange(n) / SR
    road = lp(brown(r, n), 380) * turbulence(r, n, 11, 0.3) * 2.6
    grit = lp(onepole_hp(noise(r, n), 600), 1800) * turbulence(r, n, 17, 0.5) * 0.35
    seams = np.zeros(n)
    for s0 in np.arange(0.05, 3.0, 0.375):          # an expansion joint every 0.375 s at speed
        k = int(s0 * SR)
        seams = seams + at(sine(sweep(70, 40, int(0.18 * SR))) * exp_decay(int(0.18 * SR), 0.15), s0, n)
    f = 95 + 4 * np.sin(2 * np.pi * 0.7 * t)
    motor = lp(saw(f) * 0.5 + saw(f * 1.005) * 0.5 + saw(f * 2) * 0.25, 650) * 0.5
    boost = lp(brown(r, n), 1100) * turbulence(r, n, 6, 0.2) * 0.6
    return loopable(hp(shape(road + grit + seams * 1.4 + motor + boost, 1.4), 25), 0.3)


@sound("reactor_hum", "Mech", "The cockpit at rest (loops): the reactor's hum, slow beating and a faint electrical fizz.")
def reactor_hum(r):
    n = int(4.0 * SR)
    hum = sine(np.full(n, 60.0)) + 0.5 * sine(np.full(n, 120.4)) + 0.25 * sine(np.full(n, 180.9))
    fizz = onepole_hp(noise(r, n), 5000) * 0.05
    return loopable(hum * 0.5 * (1 + 0.15 * sine(np.full(n, 0.25))) + fizz, 0.4)


# --- weapons

@sound("autocannon", "Weapons", "One round of the 35 mm autocannon: a hard crack over a chesty low thump and a short muzzle roar (fires 8 a second).")
def autocannon(r):
    n = int(0.6 * SR)
    crack = lp(onepole_hp(noise(r, n), 800), 6000, order=2) * exp_decay(n, 0.04) * 1.6
    blast = lp(noise(r, n), sweep(2500, 180, n, 0.3)) * exp_decay(n, 0.18) * 3.5
    thump = sine(sweep(120, 45, n, 0.5)) * exp_decay(n, 0.16) * 1.4
    bolt = at(lp(ring([1200, 2700], [0.05, 0.03], n), 4000, order=2) * 0.25, 0.07, n)
    return hp(shape(mix(crack, blast, thump, bolt), 2.0), 25)


@sound("cannon", "Weapons", "The 120 mm cannon: a concussive boom, the charge burning out of the barrel, and a rumble rolling off the hills.")
def cannon(r):
    n = int(2.4 * SR)
    blast = lp(noise(r, n), sweep(3500, 90, n, 0.25)) * exp_decay(n, 0.9) * 4.0
    boom = sine(sweep(75, 24, n, 0.4)) * exp_decay(n, 1.1) * 1.8
    roll = lp(brown(r, n), 140) * turbulence(r, n, 3, 0.5) * adsr(n, 0.15, 0.4, 0.5, 1.6) * 2.5
    return hp(reverb(shape(mix(blast, boom, roll), 1.8), room=0.9, wet=0.3, tail_s=1.2), 22)


@sound("rocket_launch", "Weapons", "A dumbfire rocket leaving its pod: the motor lighting with a low whoomp, a roaring burn tearing away.")
def rocket_launch(r):
    n = int(2.0 * SR)
    whoomp = mix(sine(sweep(90, 35, n, 0.4)) * exp_decay(n, 0.35) * 1.5, lp(noise(r, n), sweep(2200, 200, n, 0.3)) * exp_decay(n, 0.25) * 3)
    burn = lp(brown(r, n), sweep(1500, 300, n, 0.7)) * turbulence(r, n, 8, 0.4) * adsr(n, 0.02, 0.2, 0.6, 1.3) * 2.8
    fire = crackle(r, n, 0.005, 2200) * exp_decay(n, 1.2) * 2
    return hp(shape(mix(whoomp, burn, fire), 1.6), 22)


@sound("missile_launch", "Weapons", "A guided missile away: an ejection thud, the motor catching with a deep roar that tears off, and the seeker's chirp.")
def missile_launch(r):
    n = int(2.2 * SR)
    kick = mix(sine(sweep(70, 30, n, 0.4)) * exp_decay(n, 0.25) * 1.4, lp(noise(r, n), 500) * exp_decay(n, 0.06) * 3)
    motor = lp(brown(r, n), sweep(400, 1800, n, 0.5)) * turbulence(r, n, 9, 0.35) * adsr(n, 0.05, 0.3, 0.6, 1.3) * 2.6
    sub = lp(noise(r, n), 70) * adsr(n, 0.05, 0.3, 0.4, 1.0) * 5
    chirp = sine(sweep(1200, 2400, int(0.25 * SR))) * adsr(int(0.25 * SR), 0.01, 0.05, 0.6, 0.1) * 0.15
    return hp(shape(mix(kick, at(motor, 0.12, n), at(sub, 0.12, n), at(chirp, 0.15, n)), 1.5), 22)


@sound("lock_tone", "Weapons", "The missile seeker hunting: soft, low beeps quickening as the lock builds (1.0 s, Ace Combat style).", peak_db=-14)
def lock_tone(r):
    n = int(1.0 * SR)
    out = np.zeros(n)
    t, gap = 0.0, 0.16
    while t < 1.0:
        b = int(0.05 * SR)
        beep = (sine(np.full(b, 740.0)) + 0.12 * sine(np.full(b, 1480.0))) * adsr(b, 0.006, 0.01, 0.8, 0.02)
        out += at(lp(beep, 1800, order=2), t, n)
        t += gap
        gap = max(0.05, gap * 0.82)
    return out


@sound("lock_on", "Weapons", "Locked: one steady, soft tone, an octave under the old one.", peak_db=-14)
def lock_on(r):
    n = int(1.0 * SR)
    tone = (sine(np.full(n, 880.0)) + 0.12 * sine(np.full(n, 1760.0))) * adsr(n, 0.015, 0.02, 0.9, 0.04)
    return lp(tone, 2000, order=2)


@sound("shotgun", "Weapons", "The scattergun: a wide, heavy blast that thumps the chest, then the pump-action clack.")
def shotgun(r):
    n = int(1.5 * SR)
    blast = lp(noise(r, n), sweep(5000, 160, n, 0.25)) * exp_decay(n, 0.45) * 4
    thump = sine(sweep(100, 32, n, 0.4)) * exp_decay(n, 0.35) * 1.8
    roll = lp(brown(r, n), 150) * exp_decay(n, 0.8) * 2
    clack = lp(mix(onepole_hp(noise(r, n), 2000) * exp_decay(n, 0.02) * 1.2, ring([900, 1900, 3100], [0.08, 0.05, 0.04], n) * 0.4), 5000, order=2)
    return hp(shape(mix(blast, thump, roll, at(clack, 0.7, n), at(clack * 0.8, 0.86, n)), 1.8), 22)


@sound("rail_charge", "Weapons", "The railgun charging: capacitors whining up through a second, with crackle as they fill.")
def rail_charge(r):
    n = int(1.0 * SR)
    f = sweep(180, 3200, n, 1.6)
    whine = sine(f) * 0.5 + sine(f * 1.5) * 0.2
    trem = 0.75 + 0.25 * sine(sweep(6, 40, n))
    crackle = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.992), 3000) * np.linspace(0, 3, n)
    return (whine * trem + crackle) * np.linspace(0.3, 1, n)


@sound("rail_fire", "Weapons", "The railgun firing: a supersonic crack, an electric zap, and a deep concussion that rolls away while the air rings.")
def rail_fire(r):
    n = int(2.4 * SR)
    crack = onepole_hp(noise(r, n), 2500) * exp_decay(n, 0.025) * 2
    zap = lp(saw(sweep(5200, 300, n, 0.25)), 6000, order=2) * exp_decay(n, 0.2) * 0.45
    boom = sine(sweep(85, 26, n, 0.4)) * exp_decay(n, 1.0) * 1.8
    roll = lp(brown(r, n), 160) * turbulence(r, n, 3, 0.5) * exp_decay(n, 1.4) * 2.5
    ion = ring([2300, 3550, 4980], [1.2, 0.9, 0.7], n) * 0.08
    return hp(reverb(shape(mix(crack, zap, boom, roll, ion), 1.6), room=0.8, wet=0.28, tail_s=0.8), 22)


@sound("laser_loop", "Weapons", "The laser beam held (loops): a buzzing, detuned hum with a searing high whine.")
def laser_loop(r):
    n = int(1.8 * SR)
    hum = saw(np.full(n, 220.0)) * 0.4 + saw(np.full(n, 221.7)) * 0.4
    ringmod = hum * (0.6 + 0.4 * sine(np.full(n, 55.0)))
    sear = sine(3400 + 40 * sine(np.full(n, 11.0))) * 0.15
    return loopable(onepole_lp(ringmod, 3000) + sear)


@sound("laser_stop", "Weapons", "The laser cutting off: the beam's power dropping away.")
def laser_stop(r):
    n = int(0.6 * SR)
    f = sweep(220, 60, n, 0.6)
    return onepole_lp(saw(f) * 0.5 + saw(f * 1.01) * 0.5, sweep(3000, 200, n)) * exp_decay(n, 0.5)


@sound("plasma_fire", "Weapons", "The plasma cannon: a deep, wobbling thoom as the ball leaves the barrel, with a low gas roar behind it.")
def plasma_fire(r):
    n = int(1.4 * SR)
    f = sweep(300, 55, n, 0.4) * (1 + 0.08 * sine(np.full(n, 23.0)))
    thoom = sine(f) * exp_decay(n, 1.0) * 1.6
    gas = lp(brown(r, n), sweep(1200, 200, n, 0.5)) * turbulence(r, n, 12, 0.4) * exp_decay(n, 0.7) * 2.5
    fizz = lp(bandpass(noise(r, n), sweep(3000, 900, n), 2), 3500, order=2) * exp_decay(n, 0.4) * 0.5
    return hp(shape(mix(thoom, gas, fizz), 1.5), 22)


@sound("plasma_burst", "Weapons", "A plasma ball bursting: a deep blast of burning gas, a sub drop, and electric crackle dying in the fire.")
def plasma_burst(r):
    n = int(3.0 * SR)
    blast = lp(noise(r, n), sweep(3500, 110, n, 0.25)) * exp_decay(n, 1.1) * 4
    boom = sine(sweep(70, 24, n, 0.4)) * exp_decay(n, 1.3) * 1.6
    fire = lp(brown(r, n), 500) * turbulence(r, n, 6, 0.5) * adsr(n, 0.05, 0.4, 0.4, 2.0) * 2.2
    sparks = lp(onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.988), 1500), 4500, order=2) * exp_decay(n, 2.0) * 2.5
    return hp(reverb(shape(mix(blast, boom, fire, sparks), 1.5), room=0.85, wet=0.28, tail_s=1.0), 22)


# --- impacts

@sound("impact_mech", "Impacts", "A round striking armour: a hard, sharp crack like a gunshot, a heavy thump into the plate, and a short dead clank.")
def impact_mech(r):
    n = int(0.8 * SR)
    snap = onepole_hp(noise(r, n), 1500) * exp_decay(n, 0.008) * 3.0
    crack = lp(noise(r, n), sweep(7000, 600, n, 0.3)) * exp_decay(n, 0.05) * 3.5
    thump = sine(sweep(180, 60, n, 0.4)) * exp_decay(n, 0.09) * 1.8
    body = lp(brown(r, n), 400) * exp_decay(n, 0.15) * 1.5
    clank = lp(ring([520, 1190, 2050], [0.12, 0.08, 0.05], n, [0.5, 0.35, 0.2]), 4000, order=2) * 0.6
    return hp(shape(mix(snap, crack, thump, body, clank), 2.5), 25)


@sound("impact_building", "Impacts", "A round hitting concrete: a dull crack and falling grit.")
def impact_building(r):
    n = int(1.2 * SR)
    crack = onepole_lp(noise(r, n), 2500) * exp_decay(n, 0.1) * 2.5
    grit = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.98), 1500) * exp_decay(n, 0.9) * 2
    return mix(crack, grit)


@sound("explosion_small", "Impacts", "A small explosion: a hard, deep blast and a short roll of fire.")
def explosion_small(r):
    n = int(2.0 * SR)
    blast = lp(noise(r, n), sweep(4000, 120, n, 0.25)) * exp_decay(n, 0.8) * 4
    boom = sine(sweep(80, 28, n, 0.4)) * exp_decay(n, 0.8) * 1.6
    fire = lp(brown(r, n), 300) * turbulence(r, n, 6, 0.5) * exp_decay(n, 1.2) * 2
    return hp(reverb(shape(mix(blast, boom, fire), 1.7), room=0.7, wet=0.25, tail_s=0.8), 22)


@sound("explosion_large", "Impacts", "A big explosion: a crushing low blast, a sub drop you feel, burning debris and a long rumble rolling off the hills.")
def explosion_large(r):
    n = int(4.5 * SR)
    blast = lp(noise(r, n), sweep(3000, 70, n, 0.22)) * exp_decay(n, 2.0) * 4.5
    boom = sine(sweep(60, 20, n, 0.4)) * exp_decay(n, 2.2) * 2.0
    fire = lp(brown(r, n), 350) * turbulence(r, n, 4, 0.5) * adsr(n, 0.1, 0.5, 0.5, 3.0) * 2.5
    roll = lp(noise(r, n), 80) * turbulence(r, n, 2, 0.6) * adsr(n, 0.3, 0.6, 0.6, 3.0) * 6
    debris = lp(onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.992), 900), 3500, order=2) * adsr(n, 0.3, 0.5, 0.4, 2.0) * 2
    return hp(reverb(shape(mix(blast, boom, fire, roll, debris), 1.6), room=0.95, wet=0.35, tail_s=1.5), 20)


@sound("collapse", "Impacts", "A storey giving way: groaning steel, a deep crash and a long, heavy rumble of falling concrete.")
def collapse(r):
    n = int(5.0 * SR)
    groan = lp(bandpass(saw(sweep(60, 40, n)) * 0.5, sweep(400, 200, n), 8), 900, order=2) * adsr(n, 0.3, 0.4, 0.5, 1.0) * 2
    crash = lp(noise(r, n), sweep(3000, 150, n, 0.3)) * exp_decay(n, 1.4) * 3.5
    rumble = lp(brown(r, n), 120) * turbulence(r, n, 3, 0.5) * adsr(n, 0.8, 0.5, 0.8, 2.2) * 4
    debris = lp(onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.985), 700), 3000, order=2) * adsr(n, 1.0, 0.5, 0.6, 2.0) * 3
    return hp(shape(mix(groan, rumble, at(crash, 0.9, n), debris), 1.5), 22)


# --- cockpit

@sound("heat_lock", "Cockpit", "Weapons overheated: a soft, low two-tone chime, three times.", peak_db=-16)
def heat_lock(r):
    n = int(1.6 * SR)
    out = np.zeros(n)
    for k in range(6):
        b = int(0.24 * SR)
        f = 587.0 if k % 2 == 0 else 440.0
        tone = (sine(np.full(b, f)) + 0.08 * sine(np.full(b, 2 * f))) * adsr(b, 0.03, 0.05, 0.7, 0.08)
        out += at(lp(tone, 1400, order=2), k * 0.26, n)
    return out


@sound("alarm_warning", "Cockpit", "Damage warning: a rising electronic whoop.")
def alarm_warning(r):
    n = int(1.2 * SR)
    out = np.zeros(n)
    for k in range(2):
        b = int(0.5 * SR)
        out += at(saw(sweep(500, 1300, b, 0.8)) * adsr(b, 0.02, 0.05, 0.9, 0.05) * 0.45, k * 0.6, n)
    return onepole_lp(out, 5000)


@sound("mfd_click", "Cockpit", "Pressing a cockpit button: a click and a soft confirming blip.")
def mfd_click(r):
    n = int(0.25 * SR)
    click = onepole_hp(noise(r, n), 2000) * exp_decay(n, 0.008) * 1.5
    b = int(0.07 * SR)
    blip = sine(np.full(b, 2100.0)) * adsr(b, 0.002, 0.02, 0.5, 0.04) * 0.4
    return mix(click, at(blip, 0.03, n))


@sound("radio_squelch", "Cockpit", "The radio opening: a squelch of static before a voice.")
def radio_squelch(r):
    n = int(0.6 * SR)
    static = bandpass(noise(r, n), 1800, 1.2) * adsr(n, 0.005, 0.05, 0.6, 0.15) * 2
    click = onepole_hp(noise(r, n), 3000) * exp_decay(n, 0.01) * 2
    return shape(mix(static, click), 2.5)


@sound("radar_ping", "Cockpit", "A radar contact: a clean ping with a fading echo.")
def radar_ping(r):
    n = int(1.6 * SR)
    p = sine(np.full(n, 1040.0)) * exp_decay(n, 0.6) * 0.6
    echo = at(p * 0.35, 0.28, n) + at(p * 0.12, 0.56, n)
    return mix(p, echo)


@sound("systems_boot", "Cockpit", "The mech waking as the pilot sits: the Call Sign ('CB' in Morse) in synth beeps over a power-up sweep.")
def systems_boot(r):
    n = int(2.6 * SR)
    sweep_up = onepole_lp(saw(sweep(40, 220, n, 0.7)) * 0.3, sweep(200, 4000, n, 0.8)) * adsr(n, 0.8, 0.3, 0.4, 1.0)
    beeps = np.zeros(n)
    unit = 0.075   # one Morse unit, s
    t = 0.7
    for letter, pitch in (("-.-.", 880.0), ("-...", 587.33)):
        for sym in letter:
            d = unit * (3 if sym == "-" else 1)
            b = int(d * SR)
            beeps += at(onepole_lp(square(np.full(b, pitch), 0.5), 5000) * adsr(b, 0.003, 0.01, 0.8, 0.01) * 0.35, t, n)
            t += d + unit
        t += unit * 2
    return mix(sweep_up, beeps)


# ---------------------------------------------------------------- output

def write_wav(path: str, x: np.ndarray, peak_db: float = -1.0, loop: bool = False) -> None:
    """Peak-normalised mono 16-bit WAV. A one-shot gets a 5 ms fade out; a loop must not, or the
    fade clicks at the seam on every repeat."""
    peak = float(np.max(np.abs(x))) or 1.0
    y = np.clip(x / peak * 10 ** (peak_db / 20), -1, 1)
    if not loop:
        fade = min(len(y), int(0.005 * SR))
        y[-fade:] *= np.linspace(1, 0, fade)
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes((y * 32767).astype("<i2").tobytes())


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default="output/sfx")
    ap.add_argument("--only", nargs="*")
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    manifest = []
    for name, (group, what, fn, peak_db) in SOUNDS.items():
        if args.only and name not in args.only:
            continue
        x = fn(rng_for(name))
        if not np.all(np.isfinite(x)):
            print(f"{name}: produced a non-finite sample", file=sys.stderr)
            return 1
        write_wav(os.path.join(args.out, f"{name}.wav"), x, peak_db, loop="(loops)" in what)
        # a 120-point peak outline for the page's waveform strip
        peaks = np.abs(x).reshape(-1)[: len(x) // 120 * 120].reshape(120, -1).max(axis=1)
        peaks = peaks / (peaks.max() or 1.0)
        manifest.append({"name": name, "group": group, "what": what, "seconds": round(len(x) / SR, 2),
                         "loop": "(loops)" in what, "peaks": [round(float(p), 3) for p in peaks]})
        print(f"{group:8} {name:18} {len(x) / SR:4.1f} s")
    with open(os.path.join(args.out, "sfx.json"), "w") as f:
        json.dump(manifest, f)
    return 0


if __name__ == "__main__":
    sys.exit(main())
