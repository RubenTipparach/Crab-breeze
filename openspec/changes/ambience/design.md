# Design: ambience

## 1. The beds

`data/audio/ambience.ron` lists each bed with its file, `volume_db` and the zone it belongs to:

| Bed | Zone | Length | Made from (all CC0, `assets/audio/ambience/SOURCES.md`) |
| --- | --- | --- | --- |
| `amb_meadow_day` | open ground | 42 s | morning birdsong, breeze, insects |
| `amb_forest` | forest | 41 s | woodland birds and leaves |
| `amb_night` | open ground at night | 42 s | crickets, low wind |
| `amb_wind_high` | height (always, by height) | 41 s | two winds panned wide |
| `amb_rain`, `amb_rain_storm` | rain, storm | 42 s, 41 s | rain, rain with far thunder |
| `amb_city_street` | city | 37 s | traffic, a city's hum, a murmuring crowd, a far siren |
| `amb_city_night_rain` | city at night or in rain | 34 s | rain on the street, wet tyres, the city's low hum |
| `amb_industrial` | industrial district | 42 s | a factory working, vents |

Files are stereo Vorbis (q4, 0.3-0.8 MB each, 4.9 MB for the nine), streamed, looping on their
3 s crossfaded seam. A missing file or an unknown key stops startup with its path (CLAUDE.md 6.4).

## 2. Choosing the beds

`cb-core::audio::ambience_weights(&World, listener) -> ZoneWeights` is a pure function, tested,
read at 4 Hz (ambience needs no more):

- **city** `c = 1 - smoothstep(r, r + 300 m, d)` with `d` the distance to the nearest city's
  centre and `r` its flat-ground radius (`island-world`): 1 in the streets, 0 300 m outside.
- **industrial** `i = c * share`, `share` the fraction of warehouses among the buildings within
  200 m; the city weight becomes `c - i`. A building's style is not stored, so a warehouse is the
  city generator's own warehouse test: at most 4 storeys over more than 900 m2.
- **forest** `f = (1 - c) * min(1, trees within 80 m / 40)` (`world::forest`).
- **open ground** `o = (1 - c) * (1 - min(1, trees within 80 m / 40))`, so the zone weights sum to 1.
- **height** `h = smoothstep(2 m, 24 m, listener height above ground)`: 0 on foot, 1 in the seat.

Each bed's gain is `volume_db + 20 log10(sqrt(weight))` (equal power, so a crossfade between two
zones keeps its loudness), eased toward its target with a 4 s time constant so a zone change is
a slow fade, never a cut. `amb_wind_high` plays at `-18 + 12 h` dB on top of the zone beds. A bed
below -40 dB is stopped and restarted at a random point of its loop when it is needed again.

**Until the game has a time of day and weather**, `amb_night`, `amb_rain`, `amb_rain_storm` and
`amb_city_night_rain` are never chosen; the rule takes `is_night` and `rain` inputs that are false
today, so the beds plug in when those systems exist (a data switch can force them for a take).

**The beds' files are quiet and far** (owner, 2026-10-25: "ambient sounds needs to be softer and
queiter", "they are too foregroundy, need to be more background audio"): each is compressed 3:1,
low passed at 3.5 kHz, given a diffuse reverb and written 11 dB lower when it is cut
(`tools/audio/cut_cc0.py`, `distance`), so in the game it plays at 0 dB and sits behind everything.

## 3. Heard from the cockpit

The same seal as every outside sound (`audio`, section 3): 6 dB down and low passed at 4 kHz. One
implementation: the beds go through the outside-sound path, not a copy of it. The music is never
ducked under the beds, and the beds are not ducked under combat; the mix comes from their levels.

## 4. Cost

Up to five streams at once (four zone beds crossfading plus the wind): five Vorbis decoders, well
under 1 % of a core. Memory: streamed, about 64 KB of buffer each. The rule is a few distance
checks and two neighbour counts at 4 Hz: microseconds.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| AM1 | Which beds | the nine above; more (harbour, highway, rubble after a battle) when those places exist | provisional |
| AM2 | Night and rain | the beds are ready; chosen when a time of day and weather are designed | provisional |
| AM3 | Beds in the cockpit | muffled like other outside sound, the wind louder up high | provisional |
