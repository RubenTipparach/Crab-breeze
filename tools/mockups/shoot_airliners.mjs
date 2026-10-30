// Shots of the airliners page (docs/mockups/airliners.html) through its MOCKUP_API: day and night, gear down and up,
// the cabin cut away, the aisle and the flight deck, the zones pulled apart. For the eye, before the page is published.
//   python3 -m http.server 8771 -d docs/mockups &
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_airliners.mjs http://localhost:8771/airliners.html <out dir>
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import fs from "fs";

const [url, out] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
await page.goto(url);
await page.waitForFunction(() => window.MOCKUP_READY === true, null, { timeout: 240000 });
const shot = async (name, setup, wait = 1500) => {
  await page.evaluate(setup);
  await page.waitForTimeout(wait);
  await page.locator("canvas").screenshot({ path: `${out}/${name}.png` });
  console.log("shot", name);
};
const A = "window.MOCKUP_API";
await shot("all-day", `${A}.home()`);
await shot("propliner", `${A}.show("propliner")`);
await shot("airliner", `${A}.show("airliner")`);
await shot("widebody", `${A}.show("widebody")`);
await shot("airliner-gear-up", `${A}.show("airliner"); ${A}.setGear(1); ${A}.camOn("airliner", [-26, -2.5, -24], [0, -1.5, -2])`);
await shot("airliner-gear-down-low", `${A}.setGear(0); ${A}.camOn("airliner", [-26, -2.5, -24], [0, -1.5, -2])`);
await shot("widebody-night", `${A}.show("widebody"); ${A}.night(true)`, 2500);
await shot("airliner-break-solid", `${A}.show("airliner"); ${A}.explode(100); ${A}.camOn("airliner", [9, 3, -11], [0, 0, -6])`);
await shot("airliner-cut", `${A}.explode(0); ${A}.night(false); ${A}.show("airliner"); ${A}.cut(true); ${A}.camOn("airliner", [14, 7, -10], [0, -0.5, -6])`);
await shot("propliner-gear-up-night", `${A}.cut(false); ${A}.show("propliner"); ${A}.setGear(1); ${A}.night(true); ${A}.camOn("propliner", [-14, 2, -16], [0, 0, -2])`, 2500);
await shot("widebody-gear-up", `${A}.night(false); ${A}.show("widebody"); ${A}.setGear(1); ${A}.camOn("widebody", [-30, -3, -10], [0, -2, 0])`);
await shot("widebody-aisle", `${A}.cut(false); ${A}.show("widebody"); ${A}.inside("aisle")`);
await shot("airliner-deck", `${A}.show("airliner"); ${A}.inside("deck")`);
await shot("propliner-aisle", `${A}.setGear(0); ${A}.show("propliner"); ${A}.inside("aisle")`);
await shot("widebody-tail", `${A}.show("widebody"); ${A}.camOn("widebody", [-16, 6, 12], [0, 4, 21])`);
await shot("widebody-shot", `${A}.explode(0); ${A}.caps(false); ${A}.show("widebody"); ${A}.camOn("widebody", [-34, 6, -30], [0, 2, 0]); ${A}.shoot("widebody", "cabin_aft"); ${A}.shoot("widebody", "wing_outer_l")`, 700);
await shot("widebody-shot-later", `0`, 1800);
await shot("airliner-apart", `${A}.show("airliner"); ${A}.explode(100); ${A}.caps(true)`);
await browser.close();
