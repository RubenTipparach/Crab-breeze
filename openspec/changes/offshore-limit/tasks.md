# Tasks: offshore-limit

## 1. Write-up
- [x] 1.1 Proposal, design, spec; `openspec validate --all`

## 2. Core
- [x] 2.1 `cb_core::bounds` and `bounds:` in `data/world/world.ron`, tests at several bearings and at the edge
- [x] 2.2 `Loss::OutOfBounds` in `mission` and free-roam `missions`; fails once after `grace_s`

## 3. App
- [x] 3.1 HUD mission line `RETURN TO COAST`, alarm sound placed at the cockpit
- [x] 3.2 Respawn: the failure banner, the nearest city, a fresh mech dropped by the swap (on foot beside it, not seated)
- [ ] 3.3 Scripted playtest `offshore-limit.ron`: skate out to sea, fail, respawn
- [ ] 3.4 Headless stills of the warning and the respawn
