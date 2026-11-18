## ADDED Requirements

### Requirement: The game is built and shipped for Windows, Linux and Mac on request

A manually triggered workflow SHALL build the game with the release profile on Windows, Linux and macOS, package each as
the executable with `data/` and `assets/` beside it and a README, keep each package as a workflow artifact, and, when
asked, push each to itch.io with butler on its own channel.

#### Scenario: A dry run
- **WHEN** the workflow is run with deploy unticked
- **THEN** three packages are built and kept as artifacts, and nothing is pushed

#### Scenario: A deploy without the key
- **WHEN** the workflow is run to deploy and the `BUTLER_API_KEY` secret is missing
- **THEN** the deploy fails at once with a message saying where to add it

### Requirement: A shipped game reads its files beside its executable

The game SHALL read `data/` and `assets/`, and keep `saves/`, in its executable's folder when that folder holds
`data/world/world.ron`, wherever it is started from; otherwise it SHALL find them as a dev build does.

#### Scenario: Started from another folder
- **WHEN** a packaged game is started with a working directory that is not its own folder
- **THEN** it reads the data beside its executable and writes its saves there
