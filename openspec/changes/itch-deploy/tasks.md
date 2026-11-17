# Tasks

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta (this change); `openspec validate --all`.

## 2. Files beside the exe

- [x] 2.1 `cli::repo_root()` takes the exe's folder when it holds `data/world/world.ron`, the rule a pure function
      (`shipped_in`) with tests for both cases.

## 3. Workflow

- [x] 3.1 `.github/workflows/deploy-itch.yml` and `.github/scripts/butler-push.sh`.
- [x] 3.2 `actionlint` passes.
- [ ] 3.3 The owner adds the `BUTLER_API_KEY` secret and the itch page (IT1), runs the workflow once as a dry run and
      once deploying, and opens each platform's build.
