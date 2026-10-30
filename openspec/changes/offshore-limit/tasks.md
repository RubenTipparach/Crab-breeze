# Tasks: offshore-limit

## 1. Write-up
- [ ] 1.1 Proposal, design, spec; `openspec validate --all`

## 2. Core
- [ ] 2.1 `cb_core::bounds::offshore` and `data/world/bounds.ron`, tests at several bearings and at the edge
- [ ] 2.2 `Loss::OutOfBounds` in `mission` and free-roam `missions`; fails once after `grace_s`

## 3. App
- [ ] 3.1 HUD mission line `RETURN TO COAST`, alarm sound placed at the cockpit
- [ ] 3.2 `respawn`: failed screen, nearest city, `drop_site`, repair, seat, stream-in
- [ ] 3.3 Scripted playtest `offshore-limit.ron`: skate out to sea, fail, respawn
- [ ] 3.4 Headless stills of the warning and the respawn
