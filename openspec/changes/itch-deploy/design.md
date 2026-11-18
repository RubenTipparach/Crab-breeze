# Design: itch-deploy

## 1. The workflow

`.github/workflows/deploy-itch.yml`, triggered by `workflow_dispatch` alone, after Pale Blue Dot's.

| Input | Type | Default | Meaning |
| --- | --- | --- | --- |
| `deploy` | boolean | `true` | Push to itch. Untick for a build-only dry run. |
| `channel_suffix` | string | empty | Appended to every channel name: `-beta` gives `windows-beta`. |

- **`build`**, a matrix over `windows-latest`, `ubuntu-latest` and `macos-15` (Apple Silicon), `fail-fast: false`:
  - checks out the code;
  - installs Rust 1.99.1, the rustc this tree builds with today (the workspace's `rust-version` is 1.97);
  - restores `Swatinem/rust-cache`;
  - on Linux, installs Bevy's system packages (ALSA, udev, Wayland, xkbcommon, X11);
  - runs `cargo build --release --locked -p cb-app` (the `release` profile is the played and measured one; thin LTO).
- **Packaging** makes a folder holding the exe, copies of `data/` and `assets/` (about 750 MB, all runtime: textures,
  models, audio, shaders) and a `README.txt` saying what the build is (commit, branch, platform, date), how to run it,
  the controls in one line, and the Mac quarantine step. The job zips the folder itself so the Unix executable bit
  survives, and uploads it as the artifact `crab-breeze-<platform>`.
- **`deploy`** runs on Ubuntu after every build succeeds, only when `deploy` is true. It checks the `BUTLER_API_KEY`
  secret first (failing with a message saying where to add it), installs butler, downloads the three zips and pushes
  each through `.github/scripts/butler-push.sh` (Pale Blue Dot's, from godot-sandbox: it retries only itch's "latest
  build is still processing") to `<ITCH_GAME>:<windows|linux|mac><suffix>` with `--userversion <short sha>-<branch>`.
  One platform at a time, and a `concurrency` group (`deploy-itch`, never cancelling) queues runs.

One secret, `BUTLER_API_KEY` (https://itch.io/user/settings/api-keys), set in the repository's Settings, Secrets and
variables, Actions. The game is not a secret: `ITCH_GAME` in the workflow's `env` (provisional, IT1).

## 2. Files beside the exe

`crates/cb-app/src/cli.rs`:

```rust
pub fn repo_root() -> PathBuf            // the one the game uses: CB_ROOT, shipped, working directory, compiled-in
fn shipped_in(exe: Option<&Path>) -> Option<PathBuf>   // the rule, tested
```

`shipped_in` is the exe's folder when it holds `data/world/world.ron`. It comes after `CB_ROOT` (a deliberate override)
and before the working directory's ancestors (a shipped game started from anywhere reads its own files, never a checkout
it happens to be started in). Saves follow: `career.rs` and `main_menu.rs` write `repo_root()/saves`, so a shipped game
keeps them beside its exe. Itch's app installs into a writable per-user folder, and a player who unzips by hand unzips
somewhere they can write.

## 3. Limits

- **The Mac build is unsigned.** Gatekeeper refuses a downloaded unsigned binary; the player runs
  `xattr -dr com.apple.quarantine .` in the folder once, or right-clicks the exe and chooses Open. The itch app does not
  set the quarantine flag. The README says so.
- **Apple Silicon only** (`macos-15` is arm64).
- **Build time.** A cold `--release` build with thin LTO and one codegen unit is long (about 12 minutes in a cloud
  session); GitHub's runners are slower. `rust-cache` keeps dependencies between runs of one branch and OS.
- **Size.** About 750 MB of assets a platform; butler sends only what changed after the first push.
- **Linux glibc.** The Linux build links `ubuntu-latest`'s glibc, so an older distribution may refuse it.
- **The game is not run in CI.** Runners have no GPU worth the name; a build is checked by opening it (task 3.3).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| IT1 | The itch page the builds go to | `ruben-tipparach/crab-breeze` (Pale Blue Dot's pattern: `ruben-tipparach/pale-blue-dot`); the page must exist on itch before the first deploy | provisional |
