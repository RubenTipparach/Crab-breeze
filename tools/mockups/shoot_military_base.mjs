// Shots of the military base page (docs/mockups/military-base.html) through its MOCKUP API, for the eye before the page
// is published: the overview, each area, the traffic at set moments, day and night.
//   python3 -m http.server 8772 -d docs/mockups &
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot_military_base.mjs http://localhost:8772/military-base.html <out dir>
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import fs from "fs";

const [url, out] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1500, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto(url);
await page.waitForSelector("#ready", { state: "visible", timeout: 300000 });
const ONLY = process.env.SHOTS ? process.env.SHOTS.split(",") : null;
const shot = async (name, setup, wait = 2500) => {
  if (ONLY && !ONLY.includes(name)) return;
  await page.evaluate(setup);
  await page.waitForTimeout(wait);
  await page.locator("canvas").screenshot({ path: `${out}/${name}.png` });
  console.log("shot", name);
};
const A = "window.MOCKUP";
await shot("overview", `${A}.still(); ${A}.setTime(0); ${A}.view("overview")`);
for (const v of ["alert", "shelters", "cargo", "ramp", "helis", "mech", "lineup", "munitions", "sam", "tower", "foot", "final"])
  await shot(v, `${A}.view("${v}")`);
await shot("ramp-open", `${A}.setTime(25); ${A}.view("ramp")`);
await shot("taxi", `${A}.setTime(20); ${A}.view("alert")`);
await shot("takeoff", `${A}.setTime(60); ${A}.view("chase")`);
await shot("climb", `${A}.setTime(66); ${A}.view("chase")`);
await shot("gunship-flying", `${A}.setTime(40); ${A}.view("helis")`);
await shot("night", `${A}.setNight(true); ${A}.view("overview")`, 4000);
await shot("night-shelters", `${A}.view("alert")`, 3000);
await browser.close();
