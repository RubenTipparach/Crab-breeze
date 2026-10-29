// Screenshot a site page (docs/mockups/airport.html or seaport.html) by day or at night (TIME=day|night, the Night
// button; night by default), one PNG per view, named <page>-<time>-<view>.png. FOV sets the camera's vertical field of
// view in degrees (the game's stills are 75 on foot) so a shot can be laid beside the game's from the same view.
//   VIEWS=gates,pier TIME=day FOV=75 NODE_PATH=$(npm root -g) node tools/mockups/shoot_site.mjs docs/mockups/airport.html out/
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
const time = process.env.TIME || "night";
await page.evaluate(n => window.MOCKUP.setNight && window.MOCKUP.setNight(n), time === "night");
if (process.env.FOV) await page.evaluate(f => { camera.fov = f; camera.updateProjectionMatrix(); }, Number(process.env.FOV));
for (const view of (process.env.VIEWS || "gates,pier,terminals,tower,final,overview").split(",")) {
  await page.evaluate(v => window.MOCKUP.view(v), view);
  await page.waitForTimeout(2500);
  const box = await page.locator("canvas#view").boundingBox();
  await page.screenshot({ path: path.join(outDir, `${path.basename(page_, ".html")}-${time}-${view}.png`), clip: box, timeout: 300000 });
  console.log("shot", view);
}
await browser.close();
