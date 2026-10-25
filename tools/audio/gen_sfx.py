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


def sound(name, group, what):
    def deco(fn):
        SOUNDS[name] = (group, what, fn)
        return fn
    return deco


# --- the mech

@sound("foot_stomp", "Mech", "A 24 m mech's foot landing: a sub thump, the ground's crunch, the leg's metal ringing and a hiss of hydraulics.")
def foot_stomp(r):
    n = int(1.6 * SR)
    thump = sine(sweep(62, 28, n, 0.5)) * exp_decay(n, 0.7)
    crunch = onepole_lp(noise(r, n), 520) * exp_decay(n, 0.35) * 2.2
    clank = ring([143, 331, 517, 911], [0.9, 0.6, 0.4, 0.25], n, [0.5, 0.35, 0.25, 0.15])
    hiss = onepole_hp(noise(r, n), 3000) * adsr(n, 0.12, 0.2, 0.0, 0.01) * 0.25
    return shape(mix(thump * 1.4, crunch, clank * 0.6, at(hiss, 0.05, n)), 1.6)


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


@sound("jet_ignite", "Mech", "The jets lighting: an igniter crackle and a roar that swells to full thrust.")
def jet_ignite(r):
    n = int(2.0 * SR)
    crackle = bandpass(noise(r, n) * (r.uniform(0, 1, n) > 0.97), 2500, 3) * adsr(n, 0.0, 0.3, 0.0, 0.0) * 4
    roar = onepole_lp(noise(r, n), sweep(300, 2500, n, 0.6)) * adsr(n, 0.5, 0.3, 0.9, 0.2) * 2.5
    whine = sine(sweep(400, 1800, n, 0.7)) * adsr(n, 0.6, 0.2, 0.3, 0.3) * 0.25
    return mix(crackle, roar, whine)


@sound("rocket_loop", "Mech", "The jets burning (loops): a ragged roar with crackle, for while the rockets hold the mech up.")
def rocket_loop(r):
    n = int(2.4 * SR)
    roar = onepole_lp(noise(r, n), 1600) * 2.0
    crackle = bandpass(noise(r, n) * (r.uniform(0, 1, n) > 0.985), 3000, 2) * 3
    body = onepole_lp(noise(r, n), 180) * 3
    return loopable(roar * (1 + 0.2 * sine(np.full(n, 7.3))) + crackle + body)


@sound("rocket_rumble", "Mech", "The sub-bass under the jets (loops): the cockpit shaking at 30 to 55 Hz.")
def rocket_rumble(r):
    n = int(2.4 * SR)
    wob = 42 + 10 * sine(np.full(n, 0.9))
    sub = sine(wob) + 0.4 * sine(wob * 1.5)
    grit = onepole_lp(noise(r, n), 90) * 2
    return loopable(sub * 0.8 + grit)


@sound("skate_roll", "Mech", "Skating on the powered wheels (loops): an electric drive whine over the roar of tyres on road.")
def skate_roll(r):
    n = int(2.4 * SR)
    f = 410 + 6 * sine(np.full(n, 0.5))
    whine = sine(f) * 0.35 + sine(f * 2) * 0.15 + sine(f * 3.02) * 0.08
    road = onepole_lp(noise(r, n), 700) * 1.2
    return loopable(whine + road)


@sound("reactor_hum", "Mech", "The cockpit at rest (loops): the reactor's hum, slow beating and a faint electrical fizz.")
def reactor_hum(r):
    n = int(4.0 * SR)
    hum = sine(np.full(n, 60.0)) + 0.5 * sine(np.full(n, 120.4)) + 0.25 * sine(np.full(n, 180.9))
    fizz = onepole_hp(noise(r, n), 5000) * 0.05
    return loopable(hum * 0.5 * (1 + 0.15 * sine(np.full(n, 0.25))) + fizz, 0.4)


# --- weapons

@sound("autocannon", "Weapons", "One round of the 35 mm autocannon: a hard crack and a short mechanical thump (fires 8 a second).")
def autocannon(r):
    n = int(0.45 * SR)
    crack = onepole_hp(noise(r, n), 1500) * exp_decay(n, 0.05) * 2
    body = onepole_lp(noise(r, n), 700) * exp_decay(n, 0.14) * 2.5
    thump = sine(sweep(140, 55, n)) * exp_decay(n, 0.12)
    bolt = at(ring([1200, 2700], [0.05, 0.03], n) * 0.3, 0.06, n)
    return shape(mix(crack, body, thump, bolt), 2.0)


@sound("cannon", "Weapons", "The 120 mm cannon: a deep boom that rolls away.")
def cannon(r):
    n = int(1.4 * SR)
    blast = onepole_lp(noise(r, n), sweep(4000, 300, n, 0.3)) * exp_decay(n, 0.8) * 3
    boom = sine(sweep(90, 32, n, 0.4)) * exp_decay(n, 1.0) * 1.3
    return reverb(shape(mix(blast, boom), 1.8), room=0.8, wet=0.35, tail_s=1.2)


@sound("rocket_launch", "Weapons", "A dumbfire rocket leaving its pod: an ignition pop and a whoosh that tears away.")
def rocket_launch(r):
    n = int(1.6 * SR)
    pop = onepole_lp(noise(r, n), 1200) * exp_decay(n, 0.06) * 2.5
    whoosh = bandpass(noise(r, n), sweep(1800, 400, n, 0.5), 1.5) * adsr(n, 0.01, 0.2, 0.5, 1.2) * 3
    return mix(pop, whoosh)


@sound("missile_launch", "Weapons", "A guided missile away: a pneumatic kick, the motor's whoosh and a rising seeker chirp.")
def missile_launch(r):
    n = int(1.8 * SR)
    kick = onepole_lp(noise(r, n), 600) * exp_decay(n, 0.05) * 2.5
    motor = bandpass(noise(r, n), sweep(900, 2600, n, 0.5), 2) * adsr(n, 0.08, 0.3, 0.6, 1.0) * 2.5
    chirp = sine(sweep(1200, 2400, int(0.25 * SR))) * adsr(int(0.25 * SR), 0.01, 0.05, 0.6, 0.1) * 0.3
    return mix(kick, motor, at(chirp, 0.1, n))


@sound("lock_tone", "Weapons", "The missile seeker hunting: beeps quickening as the lock builds (1.0 s, Ace Combat style).")
def lock_tone(r):
    n = int(1.0 * SR)
    out = np.zeros(n)
    t, gap = 0.0, 0.16
    while t < 1.0:
        b = int(0.045 * SR)
        beep = square(np.full(b, 1250.0), 0.4) * adsr(b, 0.002, 0.01, 0.8, 0.01)
        out += at(onepole_lp(beep, 5000) * 0.6, t, n)
        t += gap
        gap = max(0.05, gap * 0.82)
    return out


@sound("lock_on", "Weapons", "Locked: one steady high tone.")
def lock_on(r):
    n = int(1.0 * SR)
    tone = square(np.full(n, 1650.0), 0.45) * adsr(n, 0.005, 0.02, 0.9, 0.05)
    return onepole_lp(tone, 6000) * 0.6


@sound("shotgun", "Weapons", "The scattergun: a wide, heavy blast with a pump-action clack after it.")
def shotgun(r):
    n = int(1.3 * SR)
    blast = onepole_lp(noise(r, n), sweep(7000, 500, n, 0.3)) * exp_decay(n, 0.35) * 3
    thump = sine(sweep(110, 40, n, 0.4)) * exp_decay(n, 0.3) * 1.2
    clack = mix(onepole_hp(noise(r, n), 2000) * exp_decay(n, 0.02) * 1.5, ring([900, 1900, 3100], [0.08, 0.05, 0.04], n) * 0.4)
    return shape(mix(blast, thump, at(clack, 0.62, n), at(clack * 0.8, 0.78, n)), 1.7)


@sound("rail_charge", "Weapons", "The railgun charging: capacitors whining up through a second, with crackle as they fill.")
def rail_charge(r):
    n = int(1.0 * SR)
    f = sweep(180, 3200, n, 1.6)
    whine = sine(f) * 0.5 + sine(f * 1.5) * 0.2
    trem = 0.75 + 0.25 * sine(sweep(6, 40, n))
    crackle = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.992), 3000) * np.linspace(0, 3, n)
    return (whine * trem + crackle) * np.linspace(0.3, 1, n)


@sound("rail_fire", "Weapons", "The railgun firing: a supersonic crack, an electric zap and a low boom, ringing as the air ionises.")
def rail_fire(r):
    n = int(1.6 * SR)
    crack = onepole_hp(noise(r, n), 2500) * exp_decay(n, 0.03) * 3
    zap = saw(sweep(5200, 300, n, 0.25)) * exp_decay(n, 0.25) * 0.6
    boom = sine(sweep(100, 35, n, 0.4)) * exp_decay(n, 0.9) * 1.2
    ion = ring([2300, 3550, 4980], [1.2, 0.9, 0.7], n) * 0.15
    return reverb(shape(mix(crack, zap, boom, ion), 1.6), room=0.7, wet=0.3, tail_s=0.8)


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


@sound("plasma_fire", "Weapons", "The plasma cannon: a deep, wobbling 'thoom' as the ball leaves the barrel.")
def plasma_fire(r):
    n = int(1.2 * SR)
    f = sweep(420, 70, n, 0.4) * (1 + 0.08 * sine(np.full(n, 23.0)))
    thoom = sine(f) * exp_decay(n, 0.9) * 1.2
    fizz = bandpass(noise(r, n), sweep(4000, 900, n), 2) * exp_decay(n, 0.5)
    return shape(mix(thoom, fizz, sine(f * 2) * exp_decay(n, 0.4) * 0.3), 1.4)


@sound("plasma_burst", "Weapons", "A plasma ball bursting: a blast that fizzes and crackles with electricity as it dies.")
def plasma_burst(r):
    n = int(2.4 * SR)
    blast = onepole_lp(noise(r, n), sweep(6000, 400, n, 0.3)) * exp_decay(n, 0.9) * 3
    boom = sine(sweep(80, 30, n, 0.4)) * exp_decay(n, 1.2)
    sparks = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.985), 2500) * exp_decay(n, 2.0) * 4
    hum = saw(sweep(300, 90, n)) * exp_decay(n, 1.0) * 0.2
    return reverb(shape(mix(blast, boom, sparks, hum), 1.5), room=0.8, wet=0.3, tail_s=1.0)


# --- impacts

@sound("impact_mech", "Impacts", "A round striking armour: a hard clang that rings.")
def impact_mech(r):
    n = int(1.0 * SR)
    hit = onepole_hp(noise(r, n), 800) * exp_decay(n, 0.04) * 2
    clang = ring([380, 870, 1420, 2210, 3170], [0.6, 0.45, 0.3, 0.2, 0.15], n, [0.6, 0.5, 0.35, 0.25, 0.2])
    return shape(mix(hit, clang), 1.5)


@sound("impact_building", "Impacts", "A round hitting concrete: a dull crack and falling grit.")
def impact_building(r):
    n = int(1.2 * SR)
    crack = onepole_lp(noise(r, n), 2500) * exp_decay(n, 0.1) * 2.5
    grit = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.98), 1500) * exp_decay(n, 0.9) * 2
    return mix(crack, grit)


@sound("explosion_small", "Impacts", "A small explosion: a sharp blast and a short roll.")
def explosion_small(r):
    n = int(1.6 * SR)
    blast = onepole_lp(noise(r, n), sweep(5000, 250, n, 0.3)) * exp_decay(n, 0.8) * 3
    boom = sine(sweep(85, 35, n, 0.4)) * exp_decay(n, 0.7)
    return reverb(shape(mix(blast, boom), 1.6), room=0.6, wet=0.25, tail_s=0.8)


@sound("explosion_large", "Impacts", "A big explosion: a deep blast, debris, and a long rumble rolling off the hills.")
def explosion_large(r):
    n = int(3.5 * SR)
    blast = onepole_lp(noise(r, n), sweep(6000, 150, n, 0.25)) * exp_decay(n, 2.2) * 3
    boom = sine(sweep(70, 25, n, 0.4)) * exp_decay(n, 2.0) * 1.5
    debris = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.99), 1200) * adsr(n, 0.3, 0.5, 0.4, 2.0) * 2
    return reverb(shape(mix(blast, boom, debris), 1.5), room=0.95, wet=0.4, tail_s=1.5)


@sound("collapse", "Impacts", "A storey giving way: groaning steel, a crash and a long rumble of falling concrete.")
def collapse(r):
    n = int(4.5 * SR)
    groan = bandpass(saw(sweep(60, 40, n)) * 0.5, sweep(400, 200, n), 8) * adsr(n, 0.3, 0.4, 0.5, 1.0) * 2
    rumble = onepole_lp(noise(r, n), 220) * adsr(n, 0.8, 0.5, 0.8, 2.0) * 4
    crash = onepole_lp(noise(r, n), 3000) * exp_decay(n, 1.2) * 2.5
    debris = onepole_hp(noise(r, n) * (r.uniform(0, 1, n) > 0.985), 900) * adsr(n, 1.0, 0.5, 0.6, 2.0) * 3
    return mix(groan, rumble, at(crash, 0.9, n), debris)


# --- cockpit

@sound("heat_lock", "Cockpit", "Weapons overheated: a two-tone alarm, three times.")
def heat_lock(r):
    n = int(1.5 * SR)
    out = np.zeros(n)
    for k in range(6):
        b = int(0.22 * SR)
        f = 880 if k % 2 == 0 else 660
        out += at(onepole_lp(square(np.full(b, float(f)), 0.5), 4000) * adsr(b, 0.005, 0.02, 0.9, 0.02) * 0.5, k * 0.25, n)
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

def write_wav(path: str, x: np.ndarray) -> None:
    peak = float(np.max(np.abs(x))) or 1.0
    y = np.clip(x / peak * 10 ** (-1 / 20), -1, 1)
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
    for name, (group, what, fn) in SOUNDS.items():
        if args.only and name not in args.only:
            continue
        x = fn(rng_for(name))
        if not np.all(np.isfinite(x)):
            print(f"{name}: produced a non-finite sample", file=sys.stderr)
            return 1
        write_wav(os.path.join(args.out, f"{name}.wav"), x)
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
