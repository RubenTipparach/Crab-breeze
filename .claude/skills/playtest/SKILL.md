---
name: playtest
description: Run, watch, record and report a Crab Breeze playtest the repository's way. A scripted pilot plays the real game through the same inputs a player uses, always in first person and in the cockpit when piloting; one game per device on the owner's most powerful connected machine; a check every five minutes while a desktop run plays; the take recorded with OBS between PLAYTEST_START and PLAYTEST_END, looked at on a contact sheet, a small share copy uploaded to Google Drive and confirmed with the Drive connector; a written report with the Drive link. Use whenever asked to playtest, test the game as it is played, record or upload gameplay, make a video of the mech, check that a feature works in the running game, run the bot or scripted pilot, or report on a run ("playtest it", "record a run", "upload the video", "does it play right", "show me the cockpit in game").
metadata:
  author: Crab Breeze (Claude Code), after Star Crew's `--playtest`, Mining Mike's playtest checks and Tom Lander's `bot-run`
  version: "1.0"
---

# Playtests

The owner, 2026-10-23: "the most important thing is playtesting, obs recording and uploading to
Google drive." A feature is not done until a playtest shows it in the running game. CLAUDE.md
section 9 has the rules; this is how to follow them.

## What a playtest is

A playtest is the game **as it is played**: through the pilot's own eyes, from start to finish of
what it shows, with nothing cut together.
- **First person, always** (CLAUDE.md, the law). On foot, the camera is the pilot's eyes. In the mech,
  the camera is the pilot's eyes in the cockpit seat: the canopy, the frame, the instruments and
  the pilot's hands are in view. There is no chase camera, no free camera and no cut to a menu.
- **The take shows the walk.** Boarding is shown: walking to the mech, climbing in, the cockpit
  coming alive. A take that jumps straight to the cockpit is a showcase, not a playtest.
- **The scripted pilot plays fair.** It sends the same input actions a player's keyboard, mouse or
  pad sends (after Star Crew's `sc-client --playtest`, which walks by the bots' steering and
  presses E like a player). It decides nothing a player could not, and the game checks it as it
  checks a player. It is a capture instrument (CLAUDE.md section 4).
- **A script is a list of beats** in data (`data/playtests/<name>.ron`): walk to, board, power up,
  stride to, turn to, look at, fire at, stop, disembark, look around. A new beat is added to the one
  scripted pilot, never written as a second bot.

The scripted pilot, the `--playtest <name>` flag and the beats do not exist yet. They are the
`playtest-pipeline` change in `openspec/changes/`. Until it is built, a playtest is the owner
playing and recording with `obs-record` by hand.

## The output lines

The game prints these on stdout, one per line, so tools can wait on them:

| Line | When |
| --- | --- |
| `PLAYTEST_READY <name>` | the world is loaded and the pilot can move |
| `PLAYTEST_START <name>` | the first beat begins (recording starts here) |
| `PLAYTEST_BEAT <i> <beat>` | each beat begins |
| `PLAYTEST_RESULT <ok|stuck|crash> <detail>` | the script ended, and how |
| `PLAYTEST_END <name>` | after a two-second hold on the last frame (recording stops here) |

`PLAYTEST_RESULT stuck` names the beat and where the pilot was. A beat with no progress for
10 seconds of game time is stuck.

## Where it runs

1. **Look for a connected session on the owner's hardware first** (`list_sessions`, then
   `send_message` with the commit and the command). The most powerful machine plays the game;
   any other connected machine is for network tests. Results come back through git: the report
   and the contact sheet are committed to the branch, and the reply names the commit.
2. **A cloud session has no GPU and no display.** It cannot run OBS. It may render headless
   stills through Mesa's lavapipe (`apt-get install -y mesa-vulkan-drivers`,
   `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`; Bevy renders through wgpu as Mining
   the Sky's renderer does) to check what a frame looks like, never how fast or smooth it is. It
   says so in the report.
3. **One game per device** (Mining Mike's rule). Before starting, look for a running
   `crab-breeze` process and wait for it, or stop it if it is yours and finished. Seeds are
   played in turn, never side by side, and no build or benchmark runs during a recorded take.

## Run and record

```bash
# Play a script and get a verdict (no recording):
cargo run --release -- --playtest boarding

# Record it with OBS (on the owner's Windows machine), then upload the share copy:
python .claude/skills/obs-record/scripts/obs_record.py \
  --launch "target/release/crab-breeze.exe --playtest boarding" --window-exe crab-breeze.exe \
  --capture game --start-on "^PLAYTEST_START" --stop-on "^PLAYTEST_END" \
  --fps 60 --out output/playtests/<date>-boarding.mp4 --contact-sheet
ffmpeg -i output/playtests/<date>-boarding.mp4 -c:v libx264 -crf 26 -preset slow -an \
  "G:/My Drive/dungeon-crawler-2026/crab-breeze-<date>-boarding.mp4"
```

- Record in **real time**, never a fast-forwarded or fixed-step run: a take is how the game feels.
- **Look at the contact sheet before calling it done.** A dialog over the game, a black capture,
  a missing HUD, the wrong window or a third-person view are all easy to miss otherwise.
- `output/` is git-ignored. The full OBS take stays local; only the share copy goes to Drive.

## Upload to Google Drive

Every recorded take is uploaded. The Drive for Desktop folder is
`G:/My Drive/dungeon-crawler-2026/` (the folder Tom Lander's and Mining Mike's videos already
use), and every Crab Breeze file in it is named `crab-breeze-<YYYY-MM-DD>-<what>.mp4`.
1. Write the share copy there (the ffmpeg line above, or `obs_record.py --copy-to`).
2. Confirm it with the Google Drive connector: `search_files` with
   `title contains 'crab-breeze-<date>-<what>'`. The connector cannot upload a video itself; Drive
   for Desktop does the upload.
3. Put the Drive link in the report and in the reply.
If the folder is missing or the copy is refused, say so and give the owner the copy command.

## While a desktop run plays: a check every five minutes

Mining Mike's rule, carried here: a playtest or scripted run on the desktop is checked every five
minutes for as long as it runs. Starting the first run of a session arms a recurring five-minute
check (`CronCreate` `*/5 * * * *`); it is removed (`CronDelete`) when no more runs are planned or
the owner says to stop. This is the one exception to "no scheduled check-ins" (CLAUDE.md section 8).
Each check takes a screenshot, looks at it, reports in two or three lines (the beat, what the
pilot is doing, anything wrong) and stops a run that is plainly stuck rather than leaving it to
its cap. With no game running, it reports the last result and what comes next.

## Report

Each playtest gets `docs/playtests/<YYYY-MM-DD>-<name>.md`:
- the commit and the machine (CPU, GPU, resolution), or "cloud, lavapipe stills, no frame time";
- the script and each beat's outcome, with the `PLAYTEST_RESULT` line;
- the Drive link and the contact sheet (`docs/playtests/<date>-<name>_frames.png`, committed);
- what looked wrong, each as one line; anything to fix becomes a task in its OpenSpec change.

## Do not

- Show a third-person, free or chase camera in a playtest, or cut between views.
- Record a take nobody asked for, or upload the full OBS file.
- Call a feature done from a unit test, a mockup or a still alone when it can be played.
- Run two games on one machine, or a build beside a recorded take.
