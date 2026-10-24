---
name: obs-record
description: Record an application window with OBS Studio, convert it to MP4, and optionally drop it into a Google Drive (or any synced) folder. Use when the user wants a video or screen recording of a game, demo, test run or app ("record a flight", "capture a video of X", "make a clip and upload it"). It can launch the program itself, start timing on a line of its output, stop after a duration or on another output line, and record at a reduced resolution.
metadata:
  author: Pale Blue Dot (Claude Code)
  version: "1.0"
---

## In Crab Breeze

Copied from godot-sandbox (Mining Mike, `.claude/skills/obs-record` at `3336b88`, 2026-10-23),
which copied it from Pale Blue Dot; provenance in `.claude/skills/PROVENANCE.md`. Everything below
this note is the source's text. Its "Mining Mike recipes" show how a recipe section looks; they are
not commands for this repository.

- **Crab Breeze recipes** (the `playtest` skill drives this for a playtest):
  - The game is `crab-breeze.exe` (Bevy, Vulkan or DX12 by wgpu's choice); target it with
    `--window-exe crab-breeze.exe`. Prefer `--capture game`, and check the contact sheet for the
    HUD and the cockpit.
  - The game prints `PLAYTEST_START` once the pilot has control and `PLAYTEST_END` when the
    script is done: record with `--start-on "^PLAYTEST_START" --stop-on "^PLAYTEST_END"`.
  - **Every recording is uploaded to Google Drive**: re-encode a share copy (ffmpeg H.264
    `-crf 26 -preset slow`), copy it to the Drive for Desktop folder
    `G:/My Drive/dungeon-crawler-2026/` with `--copy-to` (the folder Tom Lander's and Mining
    Mike's videos already use), named `crab-breeze-<date>-<what>.mp4`, then confirm it with the Google Drive connector (search the
    title) and give the owner the link. The full OBS take stays local.
- **On the owner's Mac** (added 2026-10-24): the script uses OBS's ScreenCaptureKit window
  capture (`screen_capture`, type 1) and matches `--window-title "Crab Breeze"`; there is no Game
  Capture on macOS, so `--capture game` falls back to it. Keep the display awake
  (`caffeinate -d -u`): a sleeping display renders the game black. Drive for Desktop is under
  `~/Library/CloudStorage/GoogleDrive-*/My Drive/`.
- A cloud session cannot run OBS: it has no display or GPU. Recording is done by a session on
  the owner's machine (CLAUDE.md section 9).

# Record a window with OBS

`scripts/obs_record.py` does the whole job in one command. It restores everything it changes:
- OBS's video settings, current scene and recording format;
- the temporary scene it creates;
- OBS's WebSocket server switch;
- OBS itself: if OBS wasn't running, the script starts it and closes it again.

## Before the first run on a machine

- **OBS:** OBS Studio 28 or later, opened at least once. The obs-websocket password must be set in **Tools → WebSocket Server Settings**; the script reads it from OBS's config and never prints it.
- **Python packages:** `pip install obsws-python imageio-ffmpeg`. imageio-ffmpeg bundles ffmpeg, so no separate install is needed.
- **WebSocket switch:** if the WebSocket server is switched off and OBS is closed, the script turns it on for the run and back off after. If OBS is **open** with the server off, the script stops and says so. Ask the user to close OBS or enable the server; do not edit the config under a running OBS, because OBS overwrites it on exit.

## Usage

```bash
python .claude/skills/obs-record/scripts/obs_record.py \
  --launch "<program and args>" --window-exe <program>.exe \
  --start-on "<regex in its output>" --duration <seconds> \
  --scale 0.5 --fps 60 --out <name>.mp4 [--copy-to "<folder>"] [--contact-sheet]
```

- **Target:** `--window-exe` (matches the executable) or `--window-title` (substring of the title). Without `--launch`, the window must already be open.
- **Timing:** recording starts once the window is captured. `--duration` counts from the `--start-on` line when given, otherwise from the start. `--stop-on` ends it early on an output line, and `--max-seconds` is a hard cap.
- **Loading screens are cut automatically.** Flat-colour frames at the start (a white or black window while the program loads) are detected and removed with a frame-exact re-encode. `--keep-blank-lead` keeps them.
- **Size:** `--scale 0.5` records at half the window's size. The canvas is the output size, with the window drawn scaled into it: one small pass a frame instead of a full-size composite that is then rescaled.
- **Encoder:** `--encoder auto` (the default) records with NVENC when an NVIDIA GPU is present and OBS is closed, at near-lossless `--quality 16`, and puts the profile's own encoder back after. The file is big on purpose: re-encode it smaller afterwards (HandBrake or ffmpeg). `--encoder keep` uses the profile's encoder.
- **Capture:** `--capture window` (the default) works on anything. `--capture game` hooks a game's own frames with OBS Game Capture, which is far cheaper for a game that is already busy on its GPU. The hook takes until the game presents frames, so the script waits up to three minutes for it.
- **Output:** OBS records MKV (safe if anything crashes) and the script remuxes it to MP4 without re-encoding. `--contact-sheet` also writes `<out>_frames.png` with six frames across the video. Use it to **look at the result before you call it done**: a menu or dialog over the scene, a black capture or the wrong window are all easy to miss otherwise.
- **Upload:** `--copy-to` copies the MP4 into a folder. For Google Drive, use the Google Drive for Desktop folder (for example `G:/My Drive/<folder>`); Drive for Desktop uploads it. The Drive connector's create-file tool takes file contents inline, which cannot carry a video, so confirm the upload by searching Drive for the file title instead.

## Troubleshooting

Run the script in the foreground, or with `python -u` and its output sent to a file. Do not kill it with an outer timeout: a killed run cannot clean up. The next run undoes a killed run's WebSocket change from its backup file, `config.json.obs-record-backup`.

- **"OBS Studio did not properly shut down / Run in Safe Mode?"** OBS was force-closed. Safe Mode disables WebSockets, so the script cannot connect. The script clears OBS's stale markers in `%APPDATA%/obs-studio/.sentinel` before it launches OBS, and closes OBS politely. Never start OBS with `--minimize-to-tray`: a tray-only OBS cannot be closed politely.
- **"OBS is already running" dialog** while no OBS window exists: a dead `obs64.exe` is still listed with status "Unknown" (seen after a force-close during a capture). It blocks new instances until it clears; if it has not cleared, a reboot does it.
- **"the captured window reports no size"**: the window is minimized or never drew a frame.

## Mining Mike recipes

Copied from the Pale Blue Dot repo (`codex/vehicles-improve`), where the script
was written and proven. The recipes below are this project's own.

- **A showcase battle has its own recorder:** `tools/record_showcase.py <scenario>`
  (`--list` shows them). It holds the fixes this laptop needed:
  - an F15 press every 20 s keeps the vendor screensaver off the game
  - NVENC in place of QuickSync, which dropped a fifth of the frames
  - OBS's own skipped and lagged frame counts reported for every take

  It records at the window's full size. For half size, run its MP4 through
  ffmpeg (`-vf scale=iw/2:-2`). Use this script for anything that isn't a
  showcase scenario.
- **Godot's window** belongs to `Godot_v4.7.1-stable_win64.exe`. Launch
  through the `_console` build so `--start-on` can read the game's output.
- **Never render with DirectX.** No `--rendering-driver d3d12`, not for a take
  and not for a test: "Never render with direct x" (2026-10-06). The game is
  Vulkan. Window Capture of the Vulkan window once dropped the whole HUD (a 17
  minute Queen take had no UI), so use **Game Capture** (`--capture game`) as
  the bot recipe below does, and check the contact sheet for the HUD.
- **Launch the game `--fullscreen`.** A window nearly the panel's size
  (1920 x 1009 on the ZenBook Duo) wrapped and broke into black lines on the
  panel under OBS, though captures stayed clean (2026-10-06).
- **Never run `--fixed-fps` for a take.** It drops vsync to fast-forward.
- **The autopilot's lines:** `BOT_START` when the level is up, `BOT_RESULT`
  and then `BOT_END` once the win or fail screen has arrived (`docs/BOT.md`). A showcase prints
  `SHOWCASE_READY` when the swarm reaches the base.
- **A bot run, as recorded on 2026-09-30:** launch `tools/run_bot.gd` with
  `BOT_SEED` and `BOT_MAX_FPS=30` set, `--capture game --fps 30 --scale 0.5`,
  `--start-on BOT_START --stop-on BOT_END`. On the laptop, Window Capture
  halved the game (64 fps alone, 25 to 30 under OBS with QuickSync or NVENC);
  Game Capture with NVENC held about 30. The bot's report gives the game's own
  frame times under the recording (`frame_ms`, `frame_phases_ms`): read those,
  because the cadence check overcounts new frames when encoder noise makes a
  repeated frame look new.
- **Record only the take that was asked for,** and check it with
  `--contact-sheet` before calling it done.
