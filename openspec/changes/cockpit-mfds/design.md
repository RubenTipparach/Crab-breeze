# Design: cockpit-mfds

## 1. One camera, one texture

One `Camera2d` renders to a 1536 x 512 image; the readouts' UI root targets it (`UiTargetCamera`) and
lays out three 512 x 512 pages in a row. Every camera costs per frame (the cockpit pass is about
0.7 ms, 2026-10-24), so it is one camera for all three screens, not one each.

## 2. The screens

When the interior binds, each `HUD_mfd_N_screen` mesh gets an unlit material showing the image with a
UV transform that picks one page (a third of the width). MFD N starts on page N (MECH, FLIGHT,
MISSION); the page is just that offset, so changing page changes nothing but the material.

## 3. Interaction

With free look held, a left click casts a ray from the eye along the view; the MFD screen it meets
moves to its next page. Seated controls send no fire while free look is held, so the click is never a
shot. (Survey C12: provisional; the recommendation is this.)

## 4. No MFDs

A cockpit whose interior has no `HUD_mfd_*_screen` (the Canopy) leaves the readouts on the pilot's
view, the three pages along the bottom.

## 5. Cost

One extra camera drawing UI into a 1536 x 512 texture (no 3D), measured with the perf suite against
the overlay.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C12 | How the pilot presses an MFD | Hold free look, look at it, click; no firing while free look is held | provisional (survey C12) |
