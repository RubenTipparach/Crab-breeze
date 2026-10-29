// Screenshot the night window light comparison (docs/mockups/night_light.html), one PNG per option and view.
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_night_light.mjs docs/mockups/night_light.html docs/screenshots/night-light
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import path from "node:path";
import fs from "node:fs";

const [page_, outDir] = process.argv.slice(2);
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: "localhost,127.0.0.1" } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
page.on("console", m => console.log("console:", m.text()));
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 240000 });
// Every option with the game's glass (8 % see-through), then C again with 25 %.
const runs = [];
for (const view of ["street", "close", "across", "high"]) {
  for (const mode of ["a", "b", "c"]) runs.push([view, mode, 0.92, `${view}-${mode}`]);
  runs.push([view, "c", 0.75, `${view}-c25`]);
}
for (const [view, mode, glass, name] of runs) {
  await page.evaluate(([m, v, g]) => { window.MOCKUP.setGlass(g); window.MOCKUP.setMode(m); window.MOCKUP.setView(v); }, [mode, view, glass]);
  await page.waitForTimeout(1200);
  const box = await page.locator(".stage").boundingBox();
  await page.screenshot({ path: path.join(outDir, `${name}.png`), clip: box, timeout: 180000 });
  console.log("shot", name);
}
await browser.close();
