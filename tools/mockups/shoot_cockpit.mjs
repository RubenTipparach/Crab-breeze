// Screenshot the cockpit page headless: the boarding sequence from outside, then the seat.
// NODE_PATH=$(npm root -g) node tools/mockups/shoot_cockpit.mjs http://localhost:8765/ docs/screenshots/cockpit [only,names]
// The page fetches its model files, so serve output/cockpit_site (written by gen_cockpit_page.py) first:
//   python3 -m http.server 8765 -d output/cockpit_site
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, only] = process.argv.slice(2);
// name, timeline position (s), then a camera: ["orbit", target xyz, camera xyz] | ["hatch", camera xyz, target xyz in the pod] | ["seat", yaw, pitch, fov]
const SHOTS = [
  ["standing", 0, ["orbit", [0, 5, 1], [11, 7, 15]]],
  ["sitting", 2.0, ["orbit", [0, 3, 1], [12, 4, 10]]],
  ["visor_lifting", 5.3, ["hatch", [-2.6, 1.6, 4.2], [0, 0.9, 0.8]]],
  ["open_hatch", 9, ["hatch", [-2.2, 1.4, 4.0], [0, 0.8, 0.6]]],
  ["open_front", 9, ["hatch", [0.0, 0.9, 4.6], [0, 0.8, 0.4]]],
  ["open_wide", 9, ["orbit", [0, 2.5, 1.5], [9, 4, 10]]],
  ["seat_forward", 0, ["seat", 0, -0.08, 80]],
  ["seat_left", 0, ["seat", 1.0, -0.2, 80]],
  ["seat_up", 0, ["seat", 0.0, 0.7, 80]],
  ["seat_dash", 0, ["seat", 0, -0.45, 55]],
];
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const browser = await chromium.launch({ proxy: proxy && { ...proxy, bypass: "localhost,127.0.0.1" }, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto(/^https?:/.test(page_) ? page_ : "file://" + path.resolve(page_));
await page.locator(".stage").scrollIntoViewIfNeeded();
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 240000 });
for (const [name, t, c] of SHOTS) {
  if (only && !only.split(",").includes(name)) continue;
  await page.evaluate((t) => { window.MOCKUP.set("steam", false); window.MOCKUP.setT(t); }, t);
  await page.waitForTimeout(1500);
  await page.evaluate(([t, c]) => {
    if (c[0] === "orbit") window.MOCKUP.orbitTo(...c[1], ...c[2]);
    else if (c[0] === "hatch") window.MOCKUP.hatchView(...c[1], ...c[2]);
    else window.MOCKUP.look(c[1], c[2], c[3]);
  }, [t, c]);
  await page.waitForTimeout(2500);
  const box = await page.locator(".stage").boundingBox();
  await page.screenshot({ path: path.join(outDir, name + ".png"), clip: box, timeout: 180000 });
  console.log("shot", name);
}
await browser.close();
