// Screenshot the airfield at night (docs/mockups/runway_lights.html), one PNG per view.
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_runway_lights.mjs docs/mockups/runway_lights.html docs/screenshots/night-light
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import path from "node:path";
import fs from "node:fs";

const [page_, outDir] = process.argv.slice(2);
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: "localhost,127.0.0.1" } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 120000 });
for (const view of ["final", "apron", "tower", "above", "port"]) {
  await page.evaluate(v => window.MOCKUP.setView(v), view);
  await page.waitForTimeout(1500);
  const box = await page.locator(".stage").boundingBox();
  await page.screenshot({ path: path.join(outDir, `airfield-${view}.png`), clip: box, timeout: 180000 });
  console.log("shot", view);
}
await browser.close();
