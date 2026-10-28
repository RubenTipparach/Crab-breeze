# Proposal: day-night-cycle

## Why

Owner, 2026-10-27: "how about you work on day night cycles? 24 mins for day time, 24 minutes for night, and we'll
need street lights, and building lights, lots of defered lighting I'm guessing since lights will be dynamic, I'll let
you pick. probably no shadows from night lights, except for on whatever verhcile we have, like if I'm on a mech my
mech casts shaddows, if I'm on ac ar my car casts shadows".

Today the world is lit by one fixed sun (`cb_core::world::sun_shade::sun_rotation`). Several things are baked from
that fixed sun:
- the terrain's far shadow, in the ground meshes' vertex colours;
- the clouds' shade map;
- the far building facades;
- the exposure.

Nothing in the world gives light of its own: no street lamps, lit windows or headlights. The only clock is the
fog's, which runs at one game hour per 300 s and only thins the morning mist.

## What changes (designed, not built)

- **One world clock.** It runs in the fixed step in `cb-core` at one game hour per 120 s: day (06:00-18:00) is 24
  minutes and night is 24 minutes. The fog's morning mist, the ambience's night beds and everything below read
  the hour from it. The fog keeps no clock of its own.
- **A sun that moves, and a moon.** The sun rises in the east and sets in the west at 60 degrees' peak elevation.
  A full moon rides opposite it. Each is a directional light, and the one above the horizon casts the cascaded
  shadows, so at night the moon lights and shadows everything, the player's mech or car included. Bevy's
  scattering sky does the dusk and dawn colours. Stars and the moon's disc show as the sky darkens.
- **Exposure follows the light.** The exposure follows a curve of the sun's height, set in data, so night is dark
  but playable, as in Ace Combat's night missions. It is not auto exposure: no histogram, the same numbers every
  run.
- **What was baked from the fixed sun works at any hour:**
  - The terrain's far shadow becomes a horizon map, baked once at load, from which the ground's shader shades any
    sun or moon.
  - The clouds' shade map rebakes on a worker thread every 2 degrees of sun travel.
  - The far facades gain a night layer with lit windows.
- **Lights, in three tiers, with Bevy's clustered forward lighting (forward+), not deferred.**
  - **Street lamps everywhere.** They are 1980s sodium lamps placed along every city street, arterial and the
    roads near cities, built by one rule from the road network.
  - **Real lights near the eye.** The nearest 96 lamps within 250 m are real Bevy spot lights, without shadows,
    fading in and out by distance so none pops.
  - **Glowing heads further out.** Every lamp's head is emissive, and far ones carry a glow sprite, so a city at
    night reads as a grid of lights from the air.
- **Lit windows.** Windows light by a rule per window: most at dusk, fewer after midnight, a few all night, in
  warm and cool interior colours. They are drawn in the glass shader at no extra draw, and in the far facades'
  night layer.
- **Vehicle lights.** Cars' headlights and tail lights glow on every car. The nearest 12 cars' headlights are
  real spot lights. The player's own mech or car carries lamps whose light casts shadows: the only night lights
  that do (owner).
- **Air bases** get runway, apron and beacon lights as emissive points and glow sprites.
- Every number above is tuning in `data/world/day_night.ron`.

## Not in this change

- Night changing gameplay (fewer people and cars late at night, sensors) is survey question N6.
- Shadows from street lamps, and light pools on far roads seen from the air, are survey questions N3 and N4.
- Measured costs come from the owner's machine; a cloud session gives estimates only.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| N1 | What hour a world starts at | 07:00, so a session begins in the morning mist, with 22 minutes of day before dusk | open (survey) |
| N2 | The moon | a full moon every night, bright enough to play by without lights | open (survey) |
| N3 | Which night lights cast shadows | the moon (on everything) and the player's own vehicle's lamps; street lamps none | open (survey); the owner's brief, read this way |
| N4 | Light pools on far roads seen from the air | not at first: lamp heads and glow sprites carry a city at night; add pools if it reads too dark | open (survey) |
| N5 | Street lamp colour | 1980s high-pressure sodium, orange (2,000 K), with white mercury lamps at the air bases | open (survey) |
| N6 | Does night change play | fewer cars and people after 23:00; sensors unchanged | open (survey) |
| N7 | Fixing the hour | an Options setting: cycle, always day, always night; a mission may set its own hour | open (survey) |
| N8 | Forward+ or deferred | forward+ (Bevy's clustered forward), with real lights only near the eye | provisional (owner: "I'll let you pick"); design section 5 |
