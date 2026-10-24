// Screenshot a mockup page headless, one PNG per named shot, for looking at before it is shown.
// node tools/mockups/shoot.mjs docs/mockups/mech_designs.html docs/screenshots/mechs
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir] = process.argv.slice(2);
const SHOTS = [
  ["lineup", "all", "orbit", "idle"], ["breeze", "breeze", "orbit", "idle"], ["bulwark", "bulwark", "orbit", "idle"],
  ["ravager", "ravager", "orbit", "idle"], ["breeze_back", "breeze", "back", "idle"], ["breeze_skate", "breeze", "side", "skate"],
  ["ravager_walk", "ravager", "side", "walk"],
  ["breeze_cockpit", "breeze", "cockpit", "idle"], ["bulwark_cockpit", "bulwark", "cockpit", "idle"], ["ravager_cockpit", "ravager", "cockpit", "idle"],
];
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 120000 });
for (const [name, mech, view, act] of SHOTS) {
  await page.evaluate(([m, v, a]) => { window.MOCKUP.setMech(m); window.MOCKUP.setView(v); window.MOCKUP.setAct(a); }, [mech, view, act]);
  await page.waitForTimeout(1500);
  await page.locator(".stage").screenshot({ path: path.join(outDir, name + ".png") });
  console.log("shot", name);
}
await browser.close();
