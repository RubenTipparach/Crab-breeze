"""Fetches the pinned character packs into a cache outside the repository, and checks them.

Borrowed from Undercity (RubenTipparach/fps-game-demo, tools/deps/fetch_character_tools.py at
1363650ab52145915dc4835ce44b13890faba734), with the cache renamed for Crab Breeze. It owns the
allowlist in tools/deps/character_packs.json: reading it, where the cache is, the SHA-256 check of
every pack, unpacking a verified zip, and telling whether a file on disk is a file of a pack. It is
plain Python with no Blender import, because the people build (tools/blender/build_people.py) imports
it inside Blender to resolve every asset against the packs the fetch verified (openspec change
`people-vat`, design section 1).

The cache is $CRAB_BREEZE_DEPS, or ~/.cache/crab-breeze/deps when that is unset. It must not be
inside the repository: the packs are about 330 MB, and one of them is a GPL tool that never ships.

Run:  python3 tools/deps/fetch_character_tools.py [--verify] [--only <pack id>]
  (no options)   downloads each missing pack, skips a pack already cached with the right
                 hash, and exits non-zero naming the pack when a hash differs or a pack
                 can't be fetched
  --verify       downloads nothing; checks every cached pack against its pin
"""
import hashlib
import json
import os
import shutil
import sys
import urllib.request
import zipfile
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
MANIFEST = os.path.join(HERE, "character_packs.json")
MANIFESTS = sorted(os.path.join(HERE, f) for f in os.listdir(HERE) if f.endswith("_packs.json"))
STAMP = ".crab_breeze_pack.json"
CHUNK = 1 << 20
USER_AGENT = "crab-breeze-fetch-character-tools/1"

PACK_KEYS = ("id", "name", "author", "file", "fetch", "url", "page", "sha256", "size_bytes", "licence", "licence_stated_in")
FETCH_KINDS = ("http", "manual")


class PackError(RuntimeError):
    """A pack that is missing, fails its pin or can't be fetched. The message names the pack."""


def load_packs(path=MANIFEST):
    """Read and validate one pack list (character_packs.json by default). Returns its packs, in file order."""
    try:
        with open(path, encoding="utf-8") as f:
            raw = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        raise PackError(f"{path}: {e}") from None
    if not isinstance(raw, dict) or not isinstance(raw.get("packs"), list) or not raw["packs"]:
        raise PackError(f"{path}: expected an object with a non-empty \"packs\" list")
    for k in raw:
        if k != "packs" and not k.startswith("_comment"):
            raise PackError(f"{path}.{k}: unknown key")
    packs, seen = [], set()
    for i, p in enumerate(raw["packs"]):
        where = f"{path}.packs[{i}]"
        if not isinstance(p, dict):
            raise PackError(f"{where}: expected an object")
        for k in p:
            if k not in PACK_KEYS and not k.startswith("_comment"):
                raise PackError(f"{where}.{k}: unknown key (known: {', '.join(PACK_KEYS)})")
        for k in PACK_KEYS:
            if k not in p:
                raise PackError(f"{where}.{k}: required field is missing")
        for k in PACK_KEYS:
            if k != "size_bytes" and (not isinstance(p[k], str) or not p[k]):
                raise PackError(f"{where}.{k}: expected a non-empty string")
        if p["id"] in seen:
            raise PackError(f"{where}.id: duplicate pack id {p['id']!r}")
        seen.add(p["id"])
        if p["fetch"] not in FETCH_KINDS:
            raise PackError(f"{where}.fetch: {p['fetch']!r} is not one of {FETCH_KINDS}")
        sha = p["sha256"]
        if len(sha) != 64 or any(c not in "0123456789abcdef" for c in sha):
            raise PackError(f"{where}.sha256: expected 64 lower-case hex digits")
        if isinstance(p["size_bytes"], bool) or not isinstance(p["size_bytes"], int) or p["size_bytes"] <= 0:
            raise PackError(f"{where}.size_bytes: expected a positive integer")
        if os.path.basename(p["file"]) != p["file"]:
            raise PackError(f"{where}.file: a file name, not a path")
        packs.append({k: p[k] for k in PACK_KEYS})
    return packs


def load_all(paths=None):
    """Every pack of every list in tools/deps (*_packs.json), in file order; an id twice is refused."""
    packs, seen = [], {}
    for path in paths if paths is not None else MANIFESTS:
        for p in load_packs(path):
            if p["id"] in seen:
                raise PackError(f"{path}: pack id {p['id']!r} is also in {seen[p['id']]}")
            seen[p["id"]] = path
            packs.append(p)
    return packs


def pack_by_id(pack_id, packs=None):
    """The pack with this id, or PackError."""
    for p in packs if packs is not None else load_all():
        if p["id"] == pack_id:
            return p
    raise PackError(f"pack {pack_id!r} is in none of {', '.join(os.path.basename(m) for m in MANIFESTS)}")


def cache_dir():
    """The cache root: $CRAB_BREEZE_DEPS or ~/.cache/crab-breeze/deps. Refused inside the repository."""
    path = os.path.realpath(os.path.expanduser(os.environ.get("CRAB_BREEZE_DEPS") or "~/.cache/crab-breeze/deps"))
    root = os.path.realpath(ROOT)
    if path == root or path.startswith(root + os.sep):
        raise PackError(f"the dependency cache {path} is inside the repository; "
                        f"set CRAB_BREEZE_DEPS to a folder outside it")
    return path


def zip_path(pack):
    """Where the pack's download lives in the cache."""
    return os.path.join(cache_dir(), "packs", pack["file"])


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(CHUNK), b""):
            h.update(block)
    return h.hexdigest()


def verify(pack):
    """Raise PackError unless the cached pack exists and matches its SHA-256 pin. Returns its path."""
    path = zip_path(pack)
    if not os.path.isfile(path):
        raise PackError(f"pack {pack['id']}: not in the cache at {path}; "
                        f"run python3 tools/deps/fetch_character_tools.py")
    got = sha256_of(path)
    if got != pack["sha256"]:
        raise PackError(f"pack {pack['id']}: {path} has SHA-256 {got}, its pin in tools/deps is "
                        f"{pack['sha256']}; nothing is built from it (delete it and fetch again)")
    return path


def fetch(pack, log=print):
    """Make the cache hold the pinned pack. Skips a cached file with the right hash, refuses a wrong one."""
    path = zip_path(pack)
    if os.path.exists(path):
        verify(pack)
        log(f"ok       {pack['id']}: cached, SHA-256 matches ({path})")
        return path
    if pack["fetch"] == "manual":
        raise PackError(f"pack {pack['id']}: the pinned file can't be downloaded automatically (see _comment_fetch in "
                        f"its list in tools/deps); put a copy with SHA-256 {pack['sha256']} at {path} and run this again")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    part = path + ".part"
    log(f"fetching {pack['id']}: {pack['url']}")
    h = hashlib.sha256()
    try:
        # A named agent: some hosts (Cloudflare in front of extensions.blender.org) refuse Python's default one.
        req = urllib.request.Request(pack["url"], headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(req, timeout=60) as r, open(part, "wb") as out:
            for block in iter(lambda: r.read(CHUNK), b""):
                h.update(block)
                out.write(block)
    except OSError as e:
        if os.path.exists(part):
            os.remove(part)
        raise PackError(f"pack {pack['id']}: download failed: {e}") from None
    if h.hexdigest() != pack["sha256"]:
        os.remove(part)
        raise PackError(f"pack {pack['id']}: the download has SHA-256 {h.hexdigest()}, the pin is {pack['sha256']}; "
                        f"refused and deleted")
    os.replace(part, path)
    log(f"fetched  {pack['id']}: {os.path.getsize(path)} bytes, SHA-256 matches")
    return path


def members(pack):
    """{path inside the pack: (size, CRC-32)} for every file in the verified zip."""
    with zipfile.ZipFile(zip_path(pack)) as z:
        return {i.filename: (i.file_size, i.CRC) for i in z.infolist() if not i.is_dir()}


def unpack(pack, dest):
    """Extract the verified pack into dest, once: a stamp with its SHA-256 marks a finished extraction.

    A folder without a matching stamp is removed and extracted again, so a half-written or older
    unpack never stands in for the pinned one."""
    stamp = os.path.join(dest, STAMP)
    try:
        with open(stamp, encoding="utf-8") as f:
            if json.load(f).get("sha256") == pack["sha256"]:
                return dest
    except (OSError, ValueError):
        pass
    verify(pack)
    if os.path.isdir(dest):
        shutil.rmtree(dest)
    os.makedirs(dest)
    real = os.path.realpath(dest)
    with zipfile.ZipFile(zip_path(pack)) as z:
        for info in z.infolist():
            target = os.path.realpath(os.path.join(dest, info.filename))
            if not target.startswith(real + os.sep):
                raise PackError(f"pack {pack['id']}: member {info.filename!r} would extract outside {dest}")
        z.extractall(dest)
    with open(stamp, "w", encoding="utf-8") as f:
        json.dump({"id": pack["id"], "sha256": pack["sha256"]}, f)
    return dest


def crc32_of(path):
    crc = 0
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(CHUNK), b""):
            crc = zlib.crc32(block, crc)
    return crc & 0xFFFFFFFF


class Allowlist:
    """Answers "which allowlisted pack is this file from?" for files unpacked from verified packs.

    roots: {pack id: the folder the pack was unpacked into}. A file belongs to a pack when it
    lies under that pack's folder, its relative path is a member of the pinned zip, and its size
    and CRC-32 match the member. Anything else belongs to no pack."""

    def __init__(self, roots, packs=None):
        packs = packs if packs is not None else load_all()
        self.roots = [(os.path.realpath(d), pack_by_id(pid, packs)) for pid, d in sorted(roots.items())]
        self._members = {p["id"]: members(p) for _, p in self.roots}
        self._checked = {}

    def pack_of(self, path):
        """The id of the pack the file comes from, or None."""
        real = os.path.realpath(path)
        if real in self._checked:
            return self._checked[real]
        found = None
        for root, pack in self.roots:
            if real.startswith(root + os.sep):
                rel = os.path.relpath(real, root).replace(os.sep, "/")
                want = self._members[pack["id"]].get(rel)
                if want and os.path.getsize(real) == want[0] and crc32_of(real) == want[1]:
                    found = pack["id"]
                break
        self._checked[real] = found
        return found


def main(argv):
    only = argv[argv.index("--only") + 1] if "--only" in argv else None
    try:
        packs = load_all()
        if only:
            packs = [pack_by_id(only, packs)]
        print(f"cache: {cache_dir()}")
    except PackError as e:
        print("FAIL", e)
        return 1
    failed = []
    for p in packs:
        try:
            if "--verify" in argv:
                verify(p)
                print(f"ok       {p['id']}: SHA-256 matches")
            else:
                fetch(p)
        except PackError as e:
            print("FAIL", e)
            failed.append(p["id"])
    if failed:
        print(f"FAIL {len(failed)} of {len(packs)} packs: {', '.join(failed)}")
        return 1
    print(f"OK {len(packs)} packs")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
