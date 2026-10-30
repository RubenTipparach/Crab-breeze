## ADDED Requirements

### Requirement: Far grass and cars' small parts cast no shadow, and cars' small parts are drawn only near
Grass tiles past the nearest `shadow_lods` levels SHALL cast no shadow; a car's cab insides, lamps, bumpers and wheels
SHALL be drawn only within their `CarDetail` ranges, dithering out over the last `fade_share`, and SHALL cast no
shadow; the body and cabin are drawn to the traffic's edge and cast shadows.

#### Scenario: A car 300 m away
- **WHEN** a car stands 300 m from the eye with the shipped `detail` (30, 200, 400 m)
- **THEN** its body, cabin and wheels are drawn, its lamps, bumpers and dashboard are not, and only its body and
  cabin are in the shadow pass (test `car_detail_ranges_must_run_from_the_cab_out_to_the_wheels` holds the ranges'
  order; `a_range_dithers_out_over_the_last_share_of_its_distance` the fade)
