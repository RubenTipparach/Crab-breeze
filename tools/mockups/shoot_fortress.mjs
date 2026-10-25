// Screenshot the crab air fortress page headless, one PNG per named shot, for looking at before it is shown.
// python3 tools/mockups/gen_crab_fortress.py
// NODE_PATH=$(npm root -g) node tools/mockups/shoot_fortress.mjs docs/mockups/crab_fortress.html docs/screenshots/fortress [shot ...]
// Where the CDN is not reachable (a cloud session's proxy), serve three.js from an unpacked `npm pack three@0.147.0`:
//   THREE_DIR=<dir>/package NODE_PATH=$(npm root -g) node tools/mockups/shoot_fortress.mjs ...
// Every shot sets the clock and steps the fall a fixed number of fixed steps, so a shot is the same every run.
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, ...only] = process.argv.slice(2);
// [name, view, clock s, thrusters killed, fall s, climb s]
const ALL = [
  ["orbit", "orbit", 46, [], 0],
  ["underside", "under", 47.5, [], 0],
  ["deck", "deck", 12, [], 0],
  ["pylon", "pylon", 30, [], 0],
  ["bow", "orbit", 21.8, [], 0, null, [[-60, 140, 330], [0, 40, 150]]],
  ["two_a_side", "orbit", 46, ["L1", "L4", "R2", "R3"], 0],
  ["falling", "orbit", 46, ["R1", "R2", "R3"], 14, null, [[-520, 40, -460], [0, -40, 0]]],
  ["crashed", "orbit", 46, ["R1", "R2", "R3", "R4"], 60, null, [[-620, 200, 560], [0, -60, 0]]],
  ["climb_low", "climb", 30, [], 0, 4],
  ["climb_high", "climb", 30, [], 0, 10.5],
  ["climb_landed", "climb", 30, [], 0, 18],
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
for (const [name, view, clock, killed, fall, climb, orbit] of SHOTS) {
  await page.evaluate(([v, c, k, f, cl, o]) => {
    const M = window.MOCKUP; M.pause(); M.reset(); M.labels(f === 0); k.forEach(M.kill); M.fall(f); M.setClock(c);
    if (cl !== null) M.climb(cl); else { M.setView(v); if (o) M.orbitFrom(o[0], o[1]); }
    M.render();
  }, [view, clock, killed, fall, climb ?? null, orbit ?? null]);
  await page.waitForTimeout(2500);
  await page.locator(".stage").screenshot({ path: path.join(outDir, name + ".png"), timeout: 180000 });
  console.log("shot", name);
}
await browser.close();
