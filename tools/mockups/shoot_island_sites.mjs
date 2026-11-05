// Shots of the island sites page (docs/mockups/island-sites.html), for the eye before it is published.
//   python3 -m http.server 8772 -d docs/mockups &
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_island_sites.mjs http://localhost:8772/island-sites.html <out dir>
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import fs from "fs";

const [url, out] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1500, height: 1100 } });
page.on("pageerror", e => console.log("pageerror:", e.message));
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
await page.goto(url);
await page.waitForSelector("#ready", { state: "visible", timeout: 120000 });
for (const v of ["overview", "west", "east", "north", "edge"]) {
  await page.evaluate(`window.MOCKUP.view("${v}")`); await page.waitForTimeout(1500);
  await page.locator("canvas").screenshot({ path: `${out}/${v}.png` }); console.log("shot", v);
}
await page.evaluate(`window.MOCKUP.show("today"); window.MOCKUP.view("overview")`); await page.waitForTimeout(1500);
await page.locator("canvas").screenshot({ path: `${out}/today.png` });
await page.screenshot({ path: `${out}/page.png`, fullPage: true }); console.log("shot page");
await browser.close();
