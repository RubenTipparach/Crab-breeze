// glTF JSON for the artifact pages: the models sit beside the page as .gltf.json with their buffer embedded
// (the host serves .json but not .glb, .gltf or .bin). Inlined by tools/mockups/gen_mech_designs.py and
// gen_cockpit_page.py:   const loader = gltfJsonLoader(THREE);   loader.loadAsync(url).then(gltf => ...)
// Without createImageBitmap, GLTFLoader decodes textures through <img> elements rather than fetch(): the
// host refuses fetch() of blob: URLs as it does data: URLs, but draws images from either.
try { self.createImageBitmap = undefined; } catch (e) {}
// The artifact host refuses fetch() of data: URLs, so a glTF JSON whose buffer is a data URI cannot load
// its buffer. Fetch the JSON (a file beside the page), decode the buffer here and hand the loader a binary
// glTF built in memory: nothing is fetched but the JSON and the texture files beside it.
function gltfJsonLoader(T) {
  const loader = new T.GLTFLoader();
  loader.loadAsync = url => fetch(url).then(r => { if (!r.ok) throw new Error(url + ": " + r.status); return r.json(); }).then(js => {
    const b = js.buffers[0], m = /^data:[^,]*;base64,(.*)$/.exec(b.uri || "");
    if (!m) throw new Error(url + ": buffer is not embedded");
    const bin = Uint8Array.from(atob(m[1]), c => c.charCodeAt(0));
    delete b.uri;
    b.byteLength = bin.length;
    const enc = new TextEncoder(), pad4 = n => (n + 3) & ~3;
    let jb = enc.encode(JSON.stringify(js));
    const jl = pad4(jb.length), bl = pad4(bin.length), glb = new Uint8Array(12 + 8 + jl + 8 + bl);
    const dv = new DataView(glb.buffer);
    dv.setUint32(0, 0x46546C67, true); dv.setUint32(4, 2, true); dv.setUint32(8, glb.length, true);
    dv.setUint32(12, jl, true); dv.setUint32(16, 0x4E4F534A, true);
    glb.fill(0x20, 20, 20 + jl); glb.set(jb, 20);
    dv.setUint32(20 + jl, bl, true); dv.setUint32(24 + jl, 0x004E4942, true);
    glb.set(bin, 28 + jl);
    const base = url.includes("/") ? url.slice(0, url.lastIndexOf("/") + 1) : "";
    return new Promise((ok, no) => loader.parse(glb.buffer, base, ok, no));
  });
  return loader;
}
