#!/usr/bin/env python3
"""Writes the list of every recording the game's sounds are cut from into
docs/analysis/audio-sources.md, between its `<!-- recordings -->` markers.

One row per pack: its page, who recorded or uploaded it, its licence as the page stated it when
tools/audio/fetch_cc0.py read it, and the cues cut from it. Built from tools/audio/cc0_cues.json
(which pack each cue uses) and tools/audio/cc0_licences.json (the fetcher's licence report, copied
from <dest>/licences.json), so the list cannot drift from what is actually used.

Usage: python3 tools/audio/list_sources.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
DOC = os.path.join(REPO, "docs/analysis/audio-sources.md")
START, END = "<!-- recordings -->", "<!-- /recordings -->"


def main() -> int:
    cues = json.load(open(os.path.join(HERE, "cc0_cues.json")))
    lic = json.load(open(os.path.join(HERE, "cc0_licences.json")))
    used: dict[str, set] = {}
    synth = set()
    for name, cue in cues.items():
        if name.startswith("_"):
            continue
        for layer in cue["layers"]:
            if "synth" in layer:
                synth.add(name)
            else:
                used.setdefault(layer["src"].split(":")[0], set()).add(name)
    rows = []
    for pack in sorted(used, key=lambda p: (p.split("/")[0], p.split("/", 1)[1])):
        info = lic.get(pack, {})
        site = {"oga": "OpenGameArt", "kenney": "Kenney", "archive": "Internet Archive"}[pack.split("/")[0]]
        title = pack.split("/", 1)[1].replace("-", " ")
        licence = "public domain" if any("publicdomain" in s for s in info.get("licences", [])) else ", ".join(info.get("licences", ["?"]))
        rows.append(f"| [{title}]({info.get('url', '')}) | {site} | {info.get('author') or '-'} | {licence} | "
                    + ", ".join(f"`{c}`" for c in sorted(used[pack])) + " |")
    body = "\n".join([
        START,
        f"{len(used)} packs, every one CC0 by its own page when it was fetched. Rebuild this list with "
        "`python3 tools/audio/list_sources.py`; each cue's exact files and edits are in "
        "`assets/audio/sfx/SOURCES.md` and `assets/audio/ambience/SOURCES.md`.",
        "",
        "| Pack | Site | Recorded or uploaded by | Licence | Cues cut from it |",
        "| --- | --- | --- | --- | --- |",
        *rows,
        "",
        "Layered with our own synthesis (`tools/audio/gen_sfx.py`): " + ", ".join(f"`{c}`" for c in sorted(synth)) + ". "
        "Synthesised outright, no recording: `lock_tone`, `lock_on`, `heat_lock`, `alarm_warning`, `radar_ping`, "
        "`systems_boot`. Music: LMMS's own synths and the GeneralUser GS soundfont (licence above), rendered from "
        "the scores in `music/`.",
        END,
    ])
    doc = open(DOC).read()
    if START in doc:
        a, b = doc.index(START), doc.index(END) + len(END)
        doc = doc[:a] + body + doc[b:]
    else:
        anchor = "## Recorded CC0 replacements"
        i = doc.index(anchor)
        doc = doc[:i] + "## Every recording in the game\n\n" + body + "\n\n" + doc[i:]
    open(DOC, "w").write(doc)
    print(f"{len(used)} packs listed in {os.path.relpath(DOC, REPO)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
