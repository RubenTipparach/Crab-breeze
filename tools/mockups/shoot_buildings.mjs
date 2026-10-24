// Screenshot the destructible-buildings mockup headless, one PNG per named shot, for looking at before it is shown.
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_buildings.mjs docs/mockups/buildings.html docs/screenshots/buildings [only,names]
// Each shot resets the block to seed 42, runs a scripted demolition through window.MOCKUP and steps the
// simulation a fixed time (fixed 1/60 s steps), so the same shot shows the same thing every time.
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, only] = process.argv.slice(2);
const ORBIT = [[115, 95, 150], [0, 18, 0]];
// name, camera [position, target] or a view name, steps to run in the page before the shot
const SHOTS = [
  ["block", "orbit", []],
  ["roofs", "roofs", []],
  ["street", "street", []],
  ["cockpit_height", "mech", []],
  ["tower_cutaway", [[-80, 40, -70], [-33, 14, -23]], [["select", 0], ["cut", 6], ["skin", true]]],
  ["walkup_cutaway", [[22, 34, 70], [-2, 6, 26]], [["select", 4], ["cut", 3], ["skin", true]]],
  ["office_plan_top", [[8, 70, -25.9], [8, 10, -26]], [["select", 1], ["cut", 4], ["skin", true]]],
  ["hole", [[22, 18, -66], [8, 14, -38]], [["demo", "hole"], ["run", 2.5]]],
  ["corner_stress", [[-95, 45, -90], [-33, 12, -23]], [["demo", "corner"], ["run", 0.1], ["stress", true]]],
  ["side_0s", [[-95, 55, -110], [-33, 25, -23]], [["demo", "side"], ["run", 0.1]]],
  ["side_2s", [[-95, 55, -110], [-33, 25, -23]], [["demo", "side"], ["run", 2.0]]],
  ["side_5s", [[-95, 55, -110], [-33, 25, -23]], [["demo", "side"], ["run", 5.0]]],
  ["side_12s", [[-95, 55, -110], [-33, 25, -23]], [["demo", "side"], ["run", 12.0]]],
  ["warehouse_drop", [[-100, 35, 85], [-33, 6, 23]], [["demo", "warehouse"], ["run", 8.0]]],
  ["volley", [[30, 30, 85], [-2, 8, 26]], [["demo", "walkup"], ["run", 6.0]]],
  ["side_12s_full", [[-95, 55, -110], [-33, 25, -23]], [["tier", "full"], ["demo", "side"], ["run", 12.0]]],
  ["warehouse_drop_full", [[-100, 35, 85], [-33, 6, 23]], [["tier", "full"], ["demo", "warehouse"], ["run", 8.0]]],
];
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: "localhost,127.0.0.1" } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
let errors = 0;
page.on("console", m => { if (m.type() === "error") { errors++; console.log("console:", m.text()); } else if (m.type() === "warning") console.log("console:", m.text()); });
page.on("pageerror", e => { errors++; console.log("pageerror:", e.message); });
await page.goto(/^https?:/.test(page_) ? page_ : "file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 240000 });
for (const [name, view, steps] of SHOTS) {
  if (only && !only.split(",").includes(name)) continue;
  await page.evaluate(([view, steps, orbit]) => {
    const M = window.MOCKUP;
    M.reset(42); M.pause(true); M.stress(false); M.tier("game");
    if (typeof view === "string") M.setView(view); else M.look(view[0], view[1]);
    for (const [fn, arg] of steps) M[fn](arg);
  }, [view, steps, ORBIT]);
  await page.waitForTimeout(1500);
  const box = await page.locator(".stage").boundingBox();
  await page.screenshot({ path: path.join(outDir, name + ".png"), clip: box, timeout: 180000 });
  if (name.includes("cutaway") || name.includes("plan") || name.includes("stress")) await page.locator("#plan").screenshot({ path: path.join(outDir, name + "_floorplan.png") });
  console.log("shot", name, await page.evaluate(() => { const p = sim.pieces.filter(Boolean); return `pieces ${p.length}, moving ${p.filter(q => !q.settled).length}, heap ${HEAP.vol.toFixed(0)} m3, tris ${renderer.info.render.triangles}, calls ${renderer.info.render.calls}`; }));
}
const stats = await page.evaluate(() => document.getElementById("stats").innerText);
console.log(stats.replace(/\n/g, " | "));
await browser.close();
if (errors) { console.log(`${errors} page errors`); process.exit(1); }
