// The view from a mech's seat on the mech design page, as the game should show it: the camera at the mech's
// EYE_pilot, the game's seated field of view, the hull hidden by the game's own rule (a mesh whose nearest contract
// bone is torso_twist, torso or cockpit, with no GUN_ joint between: crates/cb-app/src/mech.rs, is_hull), the cockpit
// interior hidden too (the game draws it in its own pass, with the world on its screens), everything else drawn.
// For comparing with the game's stills from the seat (`CB_STILLS_SEAT=1`, crates/cb-app/src/capture.rs).
//   python3 tools/mockups/gen_mech_designs.py && python3 -m http.server 8766 -d output/mech_site
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_seat_view.mjs http://localhost:8766/ <out dir> [mech] [fov_deg]
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, mech = "sapper", fov = "95"] = process.argv.slice(2);
// name, degrees left of the mech's nose, degrees up: the same five as the game's SEAT_VIEWS
const VIEWS = [["ahead", 0, 0], ["left45", 45, 0], ["right45", -45, 0], ["down-left", 45, -30], ["down-right", -45, -30]];
// The game's contract bones (cb_core::mech::rig::BONES) and the hull among them.
const BONES = ["root", "pelvis", "torso_twist", "torso", "cockpit", "shoulder_L", "upper_arm_L", "forearm_L", "hand_L",
  "shoulder_R", "upper_arm_R", "forearm_R", "hand_R", "thigh_L", "shin_L", "hock_L", "foot_L", "toe_L", "thigh_R",
  "shin_R", "hock_R", "foot_R", "toe_R"];
const HULL = ["torso_twist", "torso", "cockpit"];

fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: "localhost,127.0.0.1" } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto(/^https?:/.test(page_) ? page_ : "file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 300000 });
const hidden = await page.evaluate(([m, bones, hull]) => {
  window.MOCKUP.setMech(m); window.MOCKUP.setAct("idle"); window.MOCKUP.setT(0);
  const b = mechs[m];
  const isHull = o => {
    for (let p = o.parent; p; p = p.parent) {
      if (p.name.startsWith("GUN_")) return false;
      if (bones.includes(p.name)) return hull.includes(p.name);
    }
    return false;
  };
  const names = [];
  b.root.traverse(o => { if (o.isMesh && isHull(o)) { o.visible = false; names.push(o.name); } });
  if (b.interior) b.interior.visible = false;
  return names;
}, [mech, BONES, HULL]);
console.log(`hid ${hidden.length} hull meshes`);
await page.waitForTimeout(1500);
for (const [name, left, up] of VIEWS) {
  await page.evaluate(([m, l, u, f]) => {
    window.MOCKUP.look(l * Math.PI / 180, u * Math.PI / 180, f);
    // look() goes through setView, which may show the interior again: keep it hidden
    if (mechs[m].interior) mechs[m].interior.visible = false;
  }, [mech, left, up, Number(fov)]);
  await page.waitForTimeout(2500);
  await page.locator(".stage").first().screenshot({ path: path.join(outDir, `seat-${name}.png`), timeout: 180000 });
  console.log("shot", name);
}
await browser.close();
