"""Record one application window with OBS Studio, convert the recording to MP4,
and optionally copy it into a folder (such as a Google Drive for Desktop folder).

OBS is driven through its built-in WebSocket server (obs-websocket 5). Whatever
this script changes, it puts back: the video settings, the current scene, a
temporary scene it creates, and the WebSocket server switch. If OBS was not
running, it is started for the recording and closed afterwards.

Requires: OBS Studio 28+, Python 3.9+, `pip install obsws-python imageio-ffmpeg`.

Examples:
  # Launch a program, record 40 s after it prints "ready", half resolution.
  python obs_record.py --launch "C:/game/game.exe --demo" --window-exe game.exe \
      --start-on "ready" --duration 40 --scale 0.5 --out flight.mp4

  # Record an already-open window for 20 s and drop it into Google Drive.
  python obs_record.py --window-title "My App" --duration 20 \
      --out demo.mp4 --copy-to "G:/My Drive/recordings"
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

try:
    import imageio_ffmpeg
    import obsws_python as obs
except ImportError:
    sys.exit("missing dependencies: pip install obsws-python imageio-ffmpeg")

# obsws-python logs every refused connection attempt with a traceback while OBS
# is still starting; the script reports failures itself.
import logging

logging.getLogger("obsws_python").setLevel(logging.CRITICAL)

SCENE = "obs-record (temporary)"
SOURCE = "obs-record window (temporary)"
AUDIO = "obs-record sound (temporary)"


def obs_dir() -> Path:
    if sys.platform == "win32":
        return Path(os.environ["APPDATA"]) / "obs-studio"
    if sys.platform == "darwin":
        return Path.home() / "Library/Application Support/obs-studio"
    return Path.home() / ".config/obs-studio"


def ws_config_path() -> Path:
    return obs_dir() / "plugin_config/obs-websocket/config.json"


def obs_running() -> bool:
    if sys.platform == "win32":
        # An OBS that has exited can linger in the process list with status
        # UNKNOWN while something still holds a handle to it; it is not running.
        out = subprocess.run(["tasklist", "/FI", "IMAGENAME eq obs64.exe",
                              "/FI", "STATUS ne UNKNOWN"],
                             capture_output=True, text=True).stdout
        return "obs64.exe" in out
    return subprocess.run(["pgrep", "-x", OBS_PROCESS], capture_output=True).returncode == 0


# The OBS process's name outside Windows: `OBS` on macOS (the app bundle's binary), `obs` on Linux. Matching `obs` on
# the Mac never found the OBS this script started, so it was never closed and every run started another (ten were
# left running, 2026-10-30).
OBS_PROCESS = "OBS" if sys.platform == "darwin" else "obs"


def clear_stale_crash_markers():
    """OBS leaves a marker per run in `.sentinel` and deletes it on a clean exit.
    A leftover marker makes the next launch stop at a "did not properly shut
    down / run in Safe Mode?" prompt, and Safe Mode disables WebSockets. Only
    ever called while no OBS is running, so every marker is stale."""
    sentinel = obs_dir() / ".sentinel"
    if sentinel.is_dir():
        for f in sentinel.iterdir():
            try:
                f.unlink()
            except OSError:
                pass


def close_obs():
    """Close OBS the way a user would, so it writes its config and exits
    cleanly; force it only as a last resort, then clear what that leaves."""
    polite, force = ((["taskkill", "/IM", "obs64.exe"], ["taskkill", "/F", "/IM", "obs64.exe"])
                     if sys.platform == "win32" else
                     (["pkill", "-x", OBS_PROCESS], ["pkill", "-9", "-x", OBS_PROCESS]))
    subprocess.run(polite, capture_output=True)
    for _ in range(40):
        if not obs_running():
            return
        time.sleep(0.5)
    subprocess.run(force, capture_output=True)
    time.sleep(1)
    clear_stale_crash_markers()


def obs_executable() -> Path:
    candidates = [
        Path(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"),
        Path("/Applications/OBS.app/Contents/MacOS/OBS"),
        Path("/usr/bin/obs"),
    ]
    for c in candidates:
        if c.exists():
            return c
    sys.exit("OBS Studio not found; pass --obs PATH")


# NVENC on the NVIDIA GPU: dedicated encoding hardware, so the recording costs
# the program almost nothing. A laptop profile set to Intel QuickSync encodes on
# the integrated GPU, which shares the CPU's power budget and needs every frame
# copied across from the discrete GPU: on the Mining Mike laptop that took a
# game that ran at about 60 fps without OBS down to 26 with it. The quality is
# near lossless (CQP 16 by default) because the MP4 is meant to be re-encoded
# smaller afterwards, with HandBrake or ffmpeg.
NVENC = "obs_nvenc_h264_tex"


class RecordEncoder:
    """The profile's recording encoder, swapped to NVENC for the run and put back.

    Advanced output keeps it as [AdvOut] RecEncoder in basic.ini with its
    settings in recordEncoder.json; both files are copied first and written back
    byte for byte after. Only while OBS is closed: OBS reads the profile when it
    starts and writes it back when it exits, so an open OBS keeps its encoder."""

    def __init__(self, want: str, cqp: int):
        self.want, self.cqp = want, cqp
        prof = "Untitled"
        for ini in ("user.ini", "global.ini"):
            path = obs_dir() / ini
            if path.is_file():
                for line in path.read_text(encoding="utf-8-sig", errors="ignore").splitlines():
                    if line.startswith("ProfileDir="):
                        prof = line.split("=", 1)[1].strip()
                        break
                else:
                    continue
                break
        self.dir = obs_dir() / "basic" / "profiles" / prof
        self.saved: dict[Path, bytes] = {}

    def __enter__(self):
        if self.want == "keep":
            return self
        nvidia = bool(shutil.which("nvidia-smi")) or Path(r"C:\Windows\System32\nvidia-smi.exe").is_file()
        if self.want == "auto" and not nvidia:
            return self
        if obs_running():
            print("OBS is already open: keeping its encoder (close OBS first to record with NVENC)")
            return self
        ini, enc = self.dir / "basic.ini", self.dir / "recordEncoder.json"
        if not ini.is_file():
            print(f"no OBS profile at {self.dir}: keeping its encoder")
            return self
        text = ini.read_bytes().decode("utf-8-sig")
        if "Mode=Advanced" not in text:
            print("the OBS profile is in Simple output mode: keeping its encoder")
            return self
        for path in (ini, enc):
            if path.is_file():
                self.saved[path] = path.read_bytes()
        out, section = [], ""
        for line in text.splitlines():
            if line.startswith("["):
                section = line.strip()
            elif section == "[AdvOut]" and line.startswith("RecEncoder="):
                line = "RecEncoder=" + NVENC
            out.append(line)
        ini.write_text("\n".join(out) + "\n", encoding="utf-8-sig", newline="\n")
        enc.write_text(json.dumps({"rate_control": "CQP", "cqp": self.cqp, "preset2": "p4",
                                   "profile": "high"}), encoding="utf-8")
        print(f"recording with NVENC at CQP {self.cqp} (the profile's encoder is put back after)")
        return self

    def __exit__(self, *exc):
        for path, data in self.saved.items():
            path.write_bytes(data)
        if self.saved:
            print("OBS profile encoder restored")


class ObsSession:
    """Connects to OBS, enabling its WebSocket server (and starting OBS) only
    for as long as the recording needs, and undoing both afterwards."""

    def __init__(self, obs_path: Path | None):
        self.obs_path = obs_path
        self.started_obs = False
        self.enabled_ws = False
        self.config_backup: str | None = None
        self.client = None

    def __enter__(self):
        cfg_path = ws_config_path()
        if not cfg_path.exists():
            sys.exit(f"no obs-websocket config at {cfg_path}; open OBS once, then "
                     "Tools > WebSocket Server Settings > set a password")
        running = obs_running()
        backup = cfg_path.with_name(cfg_path.name + ".obs-record-backup")
        if backup.exists() and not running:
            # A previous run was killed before it could switch the server back
            # off: finish its cleanup first.
            cfg_path.write_text(backup.read_text(encoding="utf-8-sig"), encoding="utf-8")
            backup.unlink()
            print("restored OBS's WebSocket config left behind by an interrupted run")
        cfg = json.loads(cfg_path.read_text(encoding="utf-8-sig"))
        if not cfg.get("server_enabled"):
            if running:
                sys.exit("OBS is running with its WebSocket server off. Either close OBS "
                         "(this script then enables the server for the run and turns it "
                         "off again), or enable it in Tools > WebSocket Server Settings.")
            self.config_backup = cfg_path.read_text(encoding="utf-8-sig")
            # On disk too, so a run killed mid-way is undone by the next one.
            backup.write_text(self.config_backup, encoding="utf-8")
            cfg["server_enabled"] = True
            cfg_path.write_text(json.dumps(cfg, indent=4), encoding="utf-8")
            self.enabled_ws = True
        if not running:
            exe = self.obs_path or obs_executable()
            clear_stale_crash_markers()
            # A normal window, not --minimize-to-tray: a tray-only OBS cannot be
            # closed cleanly, and an unclean exit brings the Safe Mode prompt.
            # Never --multi (owner, 2026-10-28: "only 1 obs instance allowed"):
            # no running OBS was found above, so this is the only one.
            subprocess.Popen([str(exe), "--disable-shutdown-check"], cwd=str(exe.parent))
            self.started_obs = True
        for _ in range(180):
            try:
                self.client = obs.ReqClient(host="localhost", port=cfg["server_port"],
                                            password=cfg.get("server_password", ""),
                                            timeout=30)
                break
            except Exception:
                time.sleep(0.5)
        if self.client is None:
            self.__exit__(None, None, None)
            sys.exit("could not connect to OBS's WebSocket server within 90 s "
                     "(is OBS waiting on a dialog?)")
        # The server accepts connections before OBS has finished loading, and
        # answers "not ready" (code 207) until it has.
        for _ in range(120):
            try:
                self.client.get_video_settings()
                # Answering one request is not finished loading: on the Mac the next scene request still timed out
                # when sent at once (2026-10-30), so let it settle.
                if self.started_obs:
                    time.sleep(5.0)
                break
            except Exception:
                time.sleep(0.5)
        else:
            self.__exit__(None, None, None)
            sys.exit("OBS never became ready to take requests")
        return self.client

    def _restore_config(self):
        if self.enabled_ws and self.config_backup is not None:
            path = ws_config_path()
            path.write_text(self.config_backup, encoding="utf-8")
            path.with_name(path.name + ".obs-record-backup").unlink(missing_ok=True)

    def __exit__(self, *exc):
        if self.started_obs and obs_running():
            # OBS writes its config on exit; close it first, then restore.
            close_obs()
        self._restore_config()
        return False


def report_cadence(ff: str, video: str, fps: int) -> list[int]:
    """New frames in each second of `video`: a frame that is essentially the
    previous one again (OBS repeating it because the program did not deliver a
    new one in time) is not new. Frames are compared at 160x100 grey; after
    encoding, a repeat differs by under 0.03 of a level on average and real
    motion by more than 0.1 (measured on this project's captures). Prints a
    summary and returns the per-second counts."""
    w, h = 160, 100
    raw = subprocess.run([ff, "-v", "error", "-i", video, "-vf", f"scale={w}:{h},format=gray",
                          "-f", "rawvideo", "-"], capture_output=True).stdout
    n = w * h
    frames = [raw[i * n:(i + 1) * n] for i in range(len(raw) // n)]
    new = [True]
    for prev, cur in zip(frames, frames[1:]):
        diff = sum(abs(a - b) for a, b in zip(cur[::5], prev[::5])) / len(cur[::5])
        new.append(diff > 0.05)
    per_second = [sum(new[s:s + fps]) for s in range(0, len(new) - fps + 1, fps)]
    if per_second:
        full = sum(1 for c in per_second if c >= fps - 1)
        print(f"cadence: {len(per_second)} s, new frames per second min {min(per_second)} "
              f"median {sorted(per_second)[len(per_second) // 2]} of {fps}; "
              f"{full} of {len(per_second)} seconds at {fps - 1}+")
        worst = sorted(range(len(per_second)), key=lambda s: per_second[s])[:5]
        print("  slowest seconds: " + ", ".join(f"{s}s={per_second[s]}" for s in sorted(worst)))
    return per_second


def blank_lead(ff: str, video: str, look_s: float = 60.0) -> float:
    """Seconds of flat-colour frames (a white or black loading window) at the
    start of `video`: the time of the first frame with real content. Frames are
    sampled 20 times a second, shrunk to 64x40 grey; a frame is flat when its
    pixels barely vary. Zero when the video opens on content."""
    fps, w, h = 20, 64, 40
    raw = subprocess.run([ff, "-v", "error", "-i", video, "-t", str(look_s),
                          "-vf", f"fps={fps},scale={w}:{h},format=gray", "-f", "rawvideo", "-"],
                         capture_output=True).stdout
    n = w * h
    for i in range(len(raw) // n):
        frame = raw[i * n:(i + 1) * n]
        mean = sum(frame) / n
        spread = (sum((p - mean) ** 2 for p in frame) / n) ** 0.5
        if spread > 4.0:
            return i / fps
    return 0.0


def find_window(c, pattern_exe: str | None, pattern_title: str | None, timeout: float) -> str:
    """The window_capture item for the target window, e.g.
    'Title:WindowClass:app.exe'."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        items = c.get_input_properties_list_property_items(SOURCE, "window").property_items
        for item in items:
            value = item["itemValue"]
            # macOS (ScreenCaptureKit) lists a window id as the value and "[App] Title" as the name.
            label = f"{item.get('itemName', '')} {value}".lower()
            if pattern_exe and (str(value).lower().endswith(":" + pattern_exe.lower())
                                or (sys.platform == "darwin" and pattern_exe.lower() in label)):
                return value
            if pattern_title and pattern_title.lower() in label:
                return value
        time.sleep(0.5)
    sys.exit("target window never appeared")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    target = p.add_argument_group("target")
    target.add_argument("--launch", help="command line to start before recording")
    target.add_argument("--cwd", help="working directory for --launch (default: the program's folder)")
    target.add_argument("--window-exe", help="capture the window of this executable, e.g. game.exe")
    target.add_argument("--window-title", help="capture the window whose title contains this")
    target.add_argument("--capture", choices=["window", "game"], default="window",
                        help="window (default): Window Capture, which works on anything; game: "
                             "OBS Game Capture hooked to that window, far cheaper for a game "
                             "(Window Capture halved a GPU-bound game's frame rate on the Mining "
                             "Mike laptop)")
    timing = p.add_argument_group("timing")
    timing.add_argument("--start-on", help="regex on the launched program's output: --duration "
                        "counts from the first matching line")
    timing.add_argument("--duration", type=float, help="seconds to keep recording")
    timing.add_argument("--stop-on", help="regex on the launched program's output that ends the recording")
    timing.add_argument("--max-seconds", type=float, default=600, help="hard cap (default 600)")
    video = p.add_argument_group("video")
    video.add_argument("--scale", type=float, default=0.5, help="output size as a share of the window (default 0.5)")
    video.add_argument("--fps", type=int, default=60)
    video.add_argument("--encoder", choices=["auto", "nvenc", "keep"], default="auto",
                       help="auto (default): NVENC when an NVIDIA GPU is present and OBS is closed; "
                            "keep: the OBS profile's own encoder")
    video.add_argument("--quality", type=int, default=16,
                       help="NVENC constant quality, lower is better and bigger (default 16, near "
                            "lossless, for re-encoding afterwards)")
    out = p.add_argument_group("output")
    out.add_argument("--out", required=True, help="MP4 path to write")
    out.add_argument("--copy-to", help="folder to copy the MP4 into, e.g. a Google Drive for Desktop folder")
    out.add_argument("--keep-mkv", action="store_true", help="keep OBS's original MKV")
    out.add_argument("--keep-blank-lead", action="store_true",
                     help="keep flat-colour frames at the start (a loading screen) instead of cutting them")
    out.add_argument("--contact-sheet", action="store_true", help="also write <out>_frames.png, six frames across the video")
    out.add_argument("--check-cadence", action="store_true",
                     help="report how many new frames each second of the MP4 carries (60/60 is perfect at 60 fps)")
    p.add_argument("--obs", type=Path, help="path to the OBS executable")
    a = p.parse_args()

    if not (a.window_exe or a.window_title):
        p.error("say which window: --window-exe or --window-title")
    if a.duration is None and not a.stop_on:
        p.error("say when to stop: --duration and/or --stop-on")
    if (a.start_on or a.stop_on) and not a.launch:
        p.error("--start-on/--stop-on read the output of a program started with --launch")

    out_path = Path(a.out).resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    log_path = out_path.with_suffix(".log")

    with RecordEncoder(a.encoder, a.quality), ObsSession(a.obs) as c:
        orig_video = c.get_video_settings()
        orig_scene = c.get_current_program_scene().current_program_scene_name
        orig_format = c.get_profile_parameter("SimpleOutput", "RecFormat2").parameter_value
        created = False
        proc = None
        mkv = None
        try:
            if SCENE in [s["sceneName"] for s in c.get_scene_list().scenes]:
                c.remove_scene(SCENE)
            c.create_scene(SCENE)
            created = True
            # MKV survives a crash mid-recording; it is remuxed to MP4 after.
            c.set_profile_parameter("SimpleOutput", "RecFormat2", "mkv")

            start_evt, stop_evt = threading.Event(), threading.Event()
            if a.launch:
                argv = shlex.split(a.launch, posix=(sys.platform != "win32"))
                argv = [s.strip('"') for s in argv]
                cwd = a.cwd or str(Path(argv[0]).resolve().parent)
                proc = subprocess.Popen(argv, cwd=cwd, stdout=subprocess.PIPE,
                                        stderr=subprocess.STDOUT, text=True,
                                        encoding="utf-8", errors="replace")
                start_re = re.compile(a.start_on) if a.start_on else None
                stop_re = re.compile(a.stop_on) if a.stop_on else None

                def pump():
                    with open(log_path, "w", encoding="utf-8") as log:
                        for line in proc.stdout:
                            log.write(line)
                            log.flush()
                            if start_re and start_re.search(line):
                                start_evt.set()
                            if stop_re and stop_re.search(line):
                                stop_evt.set()
                    # The program has exited (or was closed): end the take now
                    # rather than recording an empty window until --max-seconds.
                    start_evt.set()
                    stop_evt.set()
                threading.Thread(target=pump, daemon=True).start()
            if not a.start_on:
                start_evt.set()

            if sys.platform == "darwin":
                # macOS: ScreenCaptureKit window capture (type 1). There is no Game Capture.
                c.create_input(SCENE, SOURCE, "screen_capture", {"type": 1, "show_cursor": False}, True)
                window = find_window(c, a.window_exe, a.window_title, 120)
                c.set_input_settings(SOURCE, {"type": 1, "window": window, "show_cursor": False}, True)
                # Window capture carries no sound on macOS: the desktop's sound through ScreenCaptureKit's audio
                # capture (type 0, desktop), so the take has the game's audio (a take with a silent track, 2026-10-30).
                try:
                    c.remove_input(AUDIO)
                except Exception:
                    pass
                c.create_input(SCENE, AUDIO, "sck_audio_capture", {"type": 0}, True)
            else:
                c.create_input(SCENE, SOURCE, "window_capture", {"method": 2, "cursor": False}, True)
                window = find_window(c, a.window_exe, a.window_title, 120)
                c.set_input_settings(SOURCE, {"window": window, "method": 2, "cursor": False}, True)
            src = SOURCE
            if a.capture == "game" and sys.platform != "darwin":
                # Found through Window Capture's list, then captured by hooking the
                # game's own frames on its GPU, which skips the compositor.
                # A new name: OBS takes a moment to let go of a removed one.
                c.remove_input(SOURCE)
                src = SOURCE + " game"
                c.create_input(SCENE, src, "game_capture",
                               {"capture_mode": "window", "window": window, "capture_cursor": False},
                               True)
            item = c.get_scene_item_id(SCENE, src).scene_item_id
            print(f"found window {window}")
            # On program first: OBS renders only what is showing, and a window
            # capture reports no size until it has rendered.
            c.set_current_program_scene(SCENE)
            # The canvas takes the window's own size; the output is scaled.
            w = h = 0
            # Game Capture has no size until its hook has seen the game present
            # frames, which can be past a long loading screen.
            for _ in range(720 if a.capture == "game" else 240):
                t = c.get_scene_item_transform(SCENE, item).scene_item_transform
                w, h = int(t["sourceWidth"]), int(t["sourceHeight"])
                if w and h:
                    break
                time.sleep(0.25)
            if not (w and h):
                sys.exit("the captured window reports no size (minimized, not drawing, "
                         "or Game Capture could not hook it)")
            ow = max(2, int(w * a.scale) // 2 * 2)
            oh = max(2, int(h * a.scale) // 2 * 2)
            # The canvas IS the output size, and the window is drawn scaled into
            # it: one small pass a frame. A full-size canvas composited and then
            # rescaled is two passes at full size, GPU time the captured program
            # pays for when it is GPU-bound (a game that held 64 fps alone held
            # 30 under OBS on the Mining Mike laptop).
            c.set_video_settings(a.fps, 1, ow, oh, ow, oh)
            c.set_scene_item_transform(SCENE, item, {
                "positionX": 0, "positionY": 0, "boundsType": "OBS_BOUNDS_SCALE_INNER",
                "boundsWidth": ow, "boundsHeight": oh})
            print(f"capturing {window} at {w}x{h}, output {ow}x{oh} @ {a.fps} fps")

            # Recording starts at once; a loading screen at the start is cut
            # afterwards (see `blank_lead`), so no program-specific cue is needed
            # to skip it. --start-on only says where --duration counts from.
            c.start_record()
            began = time.monotonic()
            print("recording")
            if not start_evt.wait(a.max_seconds):
                raise SystemExit("--start-on line never appeared")
            if a.start_on:
                print(f"start line seen {time.monotonic() - began:.1f} s in")
            limit = a.duration if a.duration is not None else a.max_seconds
            stop_evt.wait(max(0.0, min(limit, a.max_seconds - (time.monotonic() - began))))
            mkv = c.stop_record().output_path
            print(f"stopped after {time.monotonic() - began:.1f} s: {mkv}")
        finally:
            if proc and proc.poll() is None:
                proc.terminate()
            try:
                if c.get_record_status().output_active:
                    mkv = c.stop_record().output_path
            except Exception:
                pass
            # Stopping returns before the file is finished, and OBS refuses to
            # change video settings while the output is still active: a 17
            # minute take (2 GB) was still closing when the restore below ran,
            # and the script died with OBS left on the take's canvas.
            for _ in range(240):
                try:
                    if not c.get_record_status().output_active:
                        break
                except Exception:
                    break
                time.sleep(0.25)
            c.set_current_program_scene(orig_scene)
            if created:
                c.remove_scene(SCENE)
            if sys.platform == "darwin":
                try:
                    c.remove_input(AUDIO)
                except Exception:
                    pass
            c.set_video_settings(orig_video.fps_numerator, orig_video.fps_denominator,
                                 orig_video.base_width, orig_video.base_height,
                                 orig_video.output_width, orig_video.output_height)
            c.set_profile_parameter("SimpleOutput", "RecFormat2", orig_format)
            print("OBS settings restored")

    time.sleep(1.5)
    ff = imageio_ffmpeg.get_ffmpeg_exe()
    trim = 0.0 if a.keep_blank_lead else blank_lead(ff, mkv)
    if trim > 0:
        print(f"cutting {trim:.2f} s of blank frames from the start")
    if trim > 0:
        # A frame-exact cut needs a re-encode: a stream copy can only begin on a
        # keyframe, which may be seconds away.
        cmd = [ff, "-y", "-v", "error", "-ss", f"{trim:.3f}", "-i", mkv, "-c:v", "libx264",
               "-crf", "18", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-c:a", "copy",
               "-movflags", "+faststart", str(out_path)]
    else:
        cmd = [ff, "-y", "-v", "error", "-i", mkv, "-c", "copy", "-movflags", "+faststart",
               str(out_path)]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        sys.exit("remux failed: " + r.stderr)
    if not a.keep_mkv:
        Path(mkv).unlink(missing_ok=True)
    print(f"MP4: {out_path} ({out_path.stat().st_size / 1e6:.1f} MB)")

    if a.check_cadence:
        report_cadence(ff, str(out_path), a.fps)

    if a.contact_sheet:
        sheet = out_path.with_name(out_path.stem + "_frames.png")
        probe = subprocess.run([ff, "-i", str(out_path)], capture_output=True, text=True).stderr
        m = re.search(r"Duration: (\d+):(\d+):([\d.]+)", probe)
        dur = int(m[1]) * 3600 + int(m[2]) * 60 + float(m[3]) if m else 0
        if dur > 0:
            times = [dur * (i + 0.5) / 6 for i in range(6)]
            # A window shorter than one frame, so each tile is its own moment.
            win = 0.5 / max(a.fps, 1)
            vf = "select='" + "+".join(f"between(t,{t:.3f},{t + win:.3f})" for t in times) + "',tile=3x2"
            subprocess.run([ff, "-y", "-v", "error", "-i", str(out_path), "-vf", vf,
                            "-frames:v", "1", "-vsync", "0", str(sheet)], capture_output=True)
            if sheet.exists():
                print(f"contact sheet: {sheet}")

    if a.copy_to:
        dest = Path(a.copy_to)
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copy2(out_path, dest / out_path.name)
        print(f"copied to {dest / out_path.name}")


if __name__ == "__main__":
    main()
