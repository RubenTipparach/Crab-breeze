// Screenshot the airport page at night (docs/mockups/airport.html, the Night button), one PNG per view.
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_airport_night.mjs docs/mockups/airport.html docs/screenshots/night-light
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import path from "node:path";
import fs from "node:fs";

const [page_, outDir] = process.argv.slice(2);
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: "localhost,127.0.0.1" } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP && document.getElementById("ready").style.display === "inline", null, { timeout: 600000 });
await page.evaluate(() => window.MOCKUP.setNight(true));
for (const view of (process.env.VIEWS || "gates,pier,terminals,tower,final,overview").split(",")) {
  await page.evaluate(v => window.MOCKUP.view(v), view);
  await page.waitForTimeout(2500);
  const box = await page.locator("canvas#view").boundingBox();
  await page.screenshot({ path: path.join(outDir, `airport-night-${view}.png`), clip: box, timeout: 300000 });
  console.log("shot", view);
}
await browser.close();
