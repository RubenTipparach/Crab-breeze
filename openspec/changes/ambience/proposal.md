# Proposal: ambience

## Why

Owner, 2026-10-25: "if theres not mutch action on screen, some silence would be nice, can you get
some ambience in this game? like nature? city streets?". With the music now silent when nothing is
happening (`music`, "Calm is silence"), the world needs its own sound under the quiet: birds and
wind in open country, a forest, the traffic and murmur of a city, the clank of an industrial
district, and the wind at the cockpit's height.

## What changes

- **Nine stereo beds**, already cut from CC0 recordings (`assets/audio/ambience/`, each original
  named in its `SOURCES.md`, on the Crab Breeze Sounds page): `amb_meadow_day`, `amb_forest`,
  `amb_night`, `amb_wind_high`, `amb_rain`, `amb_rain_storm`, `amb_city_street`,
  `amb_city_night_rain`, `amb_industrial`. Each is a 34-42 s seamless loop.
- **Where the pilot is chooses the beds**: a pure rule in `cb-core` weighs city, forest, open
  ground and industrial district around the listener, and `cb-app` crossfades the beds to those
  weights. The wind bed rises with the listener's height, so the cockpit 24 m up is windier than
  the street.
- **Through the cockpit**: the sealed cabin muffles the beds the way it muffles every outside
  sound (`audio`, section 3).
- **Not placed in 3D**: beds are the one kind of sound effect that plays unplaced, in stereo
  (`audio`: "every sound is 3D except ambience, music and UI").

## Out of scope

Night and rain beds are cut but not chosen until the game has a time of day and weather (none is
designed yet); local spot sounds (a fountain, a single bird, a car passing in 3D) are later; music.
