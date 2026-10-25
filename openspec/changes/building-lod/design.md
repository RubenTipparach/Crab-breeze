# Design: building-lod

## 1. The rooms mesh

`frame_meshes` and `dressing_meshes` put partitions, core walls and furniture into their own
`SurfaceMeshes`, keyed `Look::Room(surface)`; columns and slabs stay `Look::Surface(surface)`. A
room look reads the same shared material as its surface (one material per surface, so draws
batch). The room entities carry `NotShadowCaster`, and `show_rooms_near` shows them while their
building's footprint is within `rooms_radius_m` of the pilot's eye and hides them beyond
`rooms_radius_m + rooms_margin_m`, keeping their state in between (so a building on the edge does
not flicker). Bevy's `VisibilityRange` was tried first: with `use_aabb` it measures to the
bounding box only when the entity has one, and otherwise to the entity's origin, which for
building meshes (world-space vertices, transform at the origin) is the world origin, kilometres
away, so every room was culled. No cross-fade: rooms appear at 250 m, where furniture behind a
window is a few pixels.

## 2. Settings (`world.ron` `dressing`)

`rooms_radius_m: 250.0` (owner), `rooms_margin_m: 20.0`.

## 3. Cost

Saves, beyond 270 m from the eye, every room and furniture triangle of every damaged building in
every pass, and every room and furniture triangle at any distance in the four shadow cascades.
Adds up to 8 entities per building near the pilot (one per room surface). Measured with the city
combat benchmark interleaved against `building-gpu-state` (PR #5).

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| Furniture shadows near the pilot (under 30 m) | none for now; add back for the nearest few if a take shows flat rooms | provisional |
