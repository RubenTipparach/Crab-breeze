#!/usr/bin/env python3
"""Download the CC0 and public domain recordings Crab Breeze's sound effects are cut from.

Each pack in tools/audio/cc0_packs.json is fetched into --dest (default /opt/sfx-src, outside the
repository: the originals are not committed, the cuts are). An OpenGameArt pack's page is read for
its licence and refused unless it says CC0; a Kenney pack is CC0 by its site; an Internet Archive
file is checked against the item's licence URL. Archives are unpacked in place. A licence report
is written to <dest>/licences.json for assets/audio/sfx/SOURCES.md.

Usage: python3 tools/audio/fetch_cc0.py [--dest DIR] [pack ...]
Needs: curl, unzip, 7z (p7zip-full) for .7z packs.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))
UA = "Mozilla/5.0 (crab-breeze sfx fetch)"


def get(url: str) -> str:
    return subprocess.run(["curl", "-sL", "-m", "60", "-A", UA, url], capture_output=True,
                          text=True, errors="replace", check=True).stdout


def download(url: str, path: str) -> None:
    if os.path.exists(path) and os.path.getsize(path) > 0:
        return
    subprocess.run(["curl", "-sL", "-m", "1200", "-A", UA, "-o", path, url], check=True)


def unpack(path: str) -> None:
    out = os.path.splitext(path)[0]
    if os.path.isdir(out):
        return
    if path.endswith(".zip"):
        subprocess.run(["unzip", "-qo", path, "-d", out], check=True)
    elif path.endswith(".7z"):
        subprocess.run(["7z", "x", "-y", f"-o{out}", path], check=True, stdout=subprocess.DEVNULL)


def oga(slug: str, dest: str) -> dict:
    page = get(f"https://opengameart.org/content/{slug}")
    m = re.search(r"field-art-licenses.*?</div>\s*</div>", page, re.S)
    lic = sorted(set(re.findall(r"license-name'>([^<]+)<", m.group(0) if m else "")))
    author = re.search(r'field-name-author-submitter.*?<a href="/users/[^"]+">([^<]+)</a>', page, re.S)
    if "CC0" not in lic:
        return {"pack": slug, "licences": lic, "refused": "no CC0 option"}
    files = sorted(set(re.findall(
        r"https://opengameart.org/sites/default/files/[^\"'<>]+\.(?:zip|7z|wav|ogg|mp3|flac)", page)))
    files = [f for f in files if "/styles/" not in f and "/audio_preview/" not in f]
    d = os.path.join(dest, "oga", slug)
    os.makedirs(d, exist_ok=True)
    for f in files:
        p = os.path.join(d, urllib.parse.unquote(os.path.basename(f)))
        download(f, p)
        unpack(p)
    return {"pack": slug, "url": f"https://opengameart.org/content/{slug}", "licences": lic,
            "author": author.group(1) if author else None, "files": [os.path.basename(f) for f in files]}


def kenney(slug: str, dest: str) -> dict:
    page = get(f"https://kenney.nl/assets/{slug}")
    zips = re.findall(r"https://kenney.nl/media/pages/assets/[^\"']+\.zip", page)
    if not zips:
        return {"pack": slug, "refused": "no download link found"}
    d = os.path.join(dest, "kenney")
    os.makedirs(d, exist_ok=True)
    p = os.path.join(d, f"{slug}.zip")
    download(zips[0], p)
    unpack(p)
    return {"pack": slug, "url": f"https://kenney.nl/assets/{slug}", "licences": ["CC0"], "author": "Kenney",
            "files": [os.path.basename(zips[0])]}


def archive(spec: str, dest: str) -> dict:
    item, _, names = spec.partition(":")
    meta = json.loads(get(f"https://archive.org/metadata/{item}"))
    lic = meta.get("metadata", {}).get("licenseurl", "")
    if "publicdomain" not in lic and "/zero/" not in lic:
        return {"pack": item, "licences": [lic], "refused": "not public domain or CC0"}
    d = os.path.join(dest, "archive", item)
    os.makedirs(d, exist_ok=True)
    want = [n for n in names.split(",") if n]
    for f in meta.get("files", []):
        if f["name"] in want:
            download(f"https://archive.org/download/{item}/{urllib.parse.quote(f['name'])}", os.path.join(d, f["name"]))
    return {"pack": item, "url": f"https://archive.org/details/{item}", "licences": [lic],
            "author": meta.get("metadata", {}).get("creator"), "files": want}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--dest", default="/opt/sfx-src")
    ap.add_argument("packs", nargs="*")
    args = ap.parse_args()
    with open(os.path.join(HERE, "cc0_packs.json")) as f:
        packs = json.load(f)
    report_path = os.path.join(args.dest, "licences.json")
    report = {}
    if os.path.exists(report_path):
        with open(report_path) as f:
            report = json.load(f)
    fetch = {"oga": oga, "kenney": kenney, "archive": archive}
    for p in packs:
        site, _, slug = p.partition("/")
        key = p.split(":")[0]
        if args.packs and key not in args.packs and slug.split(":")[0] not in args.packs:
            continue
        try:
            report[key] = fetch[site](slug, args.dest)
        except (subprocess.CalledProcessError, json.JSONDecodeError) as e:
            report[key] = {"pack": slug, "refused": f"fetch failed: {e}"}
        r = report[key]
        print(f"{key:55} {'REFUSED ' + r['refused'] if 'refused' in r else ', '.join(r['licences'])}")
        with open(report_path, "w") as f:
            json.dump(report, f, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
