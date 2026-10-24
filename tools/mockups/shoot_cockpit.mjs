// Screenshot the cockpit page headless from the pilot's eye, one PNG per named shot.
// NODE_PATH=$(npm root -g) node tools/mockups/shoot_cockpit.mjs docs/mockups/cockpit.html docs/screenshots/cockpit
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import path from "node:path";
import fs from "node:fs";

const [page_, outDir] = process.argv.slice(2);
// name, yaw, pitch, fov (radians, radians, degrees), or a named view
const SHOTS = [
  ["forward", 0, -0.12, 80], ["dash", 0, -0.42, 50], ["mfd_center", 0, -0.33, 28], ["annunciator", 0.62, -0.38, 30],
  ["left", 0.8, -0.85, 70], ["right", -0.8, -0.85, 70], ["knees", 0, -1.0, 70], ["throttle", 0.75, -0.95, 40],
  ["right_wall", -1.35, -0.45, 60], ["back", 2.5, -0.3, 85], ["orbit", "orbit"],
];
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 180000 });
await page.evaluate(() => window.MOCKUP.set("anim", false));
for (const [name, a, b, c] of SHOTS) {
  if (a === "orbit") await page.evaluate(() => window.MOCKUP.setView("orbit"));
  else await page.evaluate(([y, p, f]) => window.MOCKUP.look(y, p, f), [a, b, c]);
  await page.waitForTimeout(1200);
  await page.locator(".stage").screenshot({ path: path.join(outDir, name + ".png") });
  console.log("shot", name);
}
await browser.close();
