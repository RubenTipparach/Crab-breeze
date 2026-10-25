// Screenshot the crab air fortress page headless, one PNG per named shot, for looking at before it is shown.
// python3 tools/mockups/gen_crab_fortress.py
// NODE_PATH=$(npm root -g) node tools/mockups/shoot_fortress.mjs docs/mockups/crab_fortress.html docs/screenshots/fortress [shot ...]
// Where the CDN is not reachable (a cloud session's proxy), serve three.js from an unpacked `npm pack three@0.147.0`:
//   THREE_DIR=<dir>/package NODE_PATH=$(npm root -g) node tools/mockups/shoot_fortress.mjs ...
// Every shot starts from a reset and steps the page's fixed 1/30 s simulation, so a shot is the same every run.
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, ...only] = process.argv.slice(2);
// each shot is a script run in the page against window.MOCKUP (M), after M.reset()
const NOZZLE = [0, -19.5, 0];   // a lift thruster's nozzle, in its own frame: a weak point from under it
const IDS = ["L1", "L2", "L3", "L4", "R1", "R2", "R3", "R4", "HL1", "HL2", "HL3", "HR1", "HR2", "HR3"];
// fire the cannon at each part until it is gone (at most 12 shells each), aiming at `local` in its frame
const shootDown = (ids, local) => `for (const id of ${JSON.stringify(ids)}) for (let k = 0; k < 12 && M.state().hp[${JSON.stringify(IDS)}.indexOf(id)] > 0; k++) { M.select(id); M.shoot(id, "cannon", ${local ? JSON.stringify(local) : "null"}); M.run(2.1); }`;
const UNDER = `M.setPlayer("under"); M.setView("fight"); M.run(2);`;
const ALL = [
  ["orbit", `M.setPlayer("air"); M.run(34); M.setView("orbit");`],
  ["catapult", `M.run(5.4); M.setView("catapult");`],
  ["turrets", `M.setPlayer("air"); M.run(40); M.setView("orbit"); M.labels(false); M.orbitFrom([840, -10, -380], [0, -40, 0]);`],
  ["deck_fight", `M.setPlayer("deck"); M.run(12); M.setView("deck");`],
  ["drops", `M.run(22); M.setView("under");`],
  ["armour_plate", `M.setPlayer("hover"); M.setView("fight"); M.run(3); M.select("R2"); M.shoot("R2", "cannon", ${JSON.stringify(NOZZLE)}); M.run(0.7); M.lookAtPart("R2");`],
  ["under_fire", `${UNDER} M.select("R2"); M.shoot("R2", "cannon", ${JSON.stringify(NOZZLE)}); M.run(0.7); M.lookAtPart("R2");`],
  ["hull_opening", `${UNDER} ${shootDown(["R1", "R2", "R3"], NOZZLE)}; M.run(1.5); M.setView("orbit"); M.labels(false); M.orbitFrom([-520, -330, 300], [-60, -20, 0]);`],
  ["hull_open", `${UNDER} ${shootDown(["R1", "R2", "R3"], NOZZLE)}; M.run(6); M.setView("orbit"); M.orbitFrom([-420, -380, 260], [-60, -20, 0]);`],
  ["hull_port_side", `${UNDER} ${shootDown(["R1", "R2", "R3"], NOZZLE)}; M.run(6); M.setPlayer("hover"); M.setView("fight"); M.run(1); M.select("HR2"); M.shoot("HR2", "cannon"); M.run(0.8); M.lookAtPart("HR2");`],
  ["hull_port_under", `${UNDER} ${shootDown(["R1", "R2", "R3"], NOZZLE)}; M.run(6); M.select("HR2"); M.shoot("HR2", "cannon"); M.run(0.5); M.lookAtPart("HR2");`],
  ["fight_falling", `${UNDER} ${shootDown(["R1", "R2", "R3"], NOZZLE)}; M.run(6); ${shootDown(["HR1", "HR2"])}; M.run(8); M.setPlayer("hover"); M.setView("fight"); M.lookAtPart("R4");`],
  ["crashed", `${UNDER} ${shootDown(["R1", "R2", "R3"], NOZZLE)}; M.run(6); ${shootDown(["HR1", "HR2", "HR3"])}; M.run(50); M.setView("orbit"); M.labels(false); M.orbitFrom([-700, 260, 620], [0, -60, 0]);`],
  ["leg_falling", `M.run(3); M.destroy("R2"); M.run(4); M.setView("orbit"); M.labels(false); M.orbitFrom([-620, -200, 420], [-200, -220, 60]);`],
  ["leg_down", `M.run(3); M.destroy("R2"); M.run(10.9); M.setView("orbit"); M.labels(false); M.orbitFrom([-560, -380, 380], [-220, -560, 60]);`],
  ["leg_wreck", `M.run(3); M.destroy("R2"); M.run(16); M.setView("orbit"); M.labels(false); M.orbitFrom([-520, -420, 360], [-220, -590, 60]);`],
  ["shell", `M.run(8); M.setView("orbit"); M.labels(false); M.orbitFrom([-300, 170, 330], [-40, 10, 60]);`],
  ["climb_high", `M.climb(10.5);`],
];
const SHOTS = only.length ? ALL.filter(s => only.includes(s[0])) : ALL;
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
if (process.env.THREE_DIR) {
  await page.route(/cdn\.jsdelivr\.net\/npm\/three@0\.147\.0\//, r => {
    const rel = new URL(r.request().url()).pathname.replace(/^\/npm\/three@0\.147\.0\//, "");
    r.fulfill({ path: path.join(process.env.THREE_DIR, rel), contentType: "application/javascript" });
  });
  await page.route(/fonts\.(googleapis|gstatic)\.com/, r => r.abort());
}
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 240000 });
for (const [name, script] of SHOTS) {
  const state = await page.evaluate(`(() => { const M = window.MOCKUP; M.pause(); M.reset(); M.labels(true); M.setView("orbit"); ${script}; M.render(); return M.state(); })()`);
  await page.waitForTimeout(2000);
  await page.locator(".stage").screenshot({ path: path.join(outDir, name + ".png"), timeout: 180000 });
  console.log("shot", name, JSON.stringify({ y: Math.round(state.y), crashed: state.crashed, open: state.open, outer: state.alive.map(Number).join(""), hull: state.hull.map(Number).join("") }));
}
await browser.close();
