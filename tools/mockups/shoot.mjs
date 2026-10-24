// Screenshot the mech design page headless, one PNG per named shot, for looking at before it is shown.
// The page fetches its models, so serve output/mech_site (written by gen_mech_designs.py) first:
//   python3 -m http.server 8766 -d output/mech_site
//   NODE_PATH=$(npm root -g) node tools/mockups/shoot.mjs http://localhost:8766/ docs/screenshots/mechs [only,names]
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, only] = process.argv.slice(2);
// name, mech, view, act, timeline ("end" = open, a number = seconds), [look yaw, pitch, fov] for the cockpit
const SHOTS = [
  ["lineup", "all", "orbit", "idle", 0],
  ["foreman", "foreman", "orbit", "idle", 0], ["stevedore", "stevedore", "orbit", "idle", 0], ["ironhorse", "ironhorse", "orbit", "idle", 0],
  ["foreman_opening", "foreman", "hatch", "idle", 4.6], ["foreman_open", "foreman", "hatch", "idle", "end"],
  ["stevedore_opening", "stevedore", "hatch", "idle", 4.3], ["stevedore_open", "stevedore", "hatch", "idle", "end"],
  ["ironhorse_opening", "ironhorse", "hatch", "idle", 4.6], ["ironhorse_open", "ironhorse", "hatch", "idle", "end"],
  ["foreman_open_wide", "foreman", "orbit", "idle", "end"], ["ironhorse_open_wide", "ironhorse", "orbit", "idle", "end"],
  ["foreman_cockpit", "foreman", "cockpit", "idle", 0, [0, -0.1, 80]], ["stevedore_cockpit", "stevedore", "cockpit", "idle", 0, [0, -0.2, 80]],
  ["ironhorse_cockpit", "ironhorse", "cockpit", "idle", 0, [0, -0.1, 80]],
];
// the opening, frame by frame, for each chassis's contact sheet (tools/mockups/opening_sheet.py)
for (const m of ["foreman", "stevedore", "ironhorse"]) [0, 2.2, 3.9, 4.6, 5.3, "end"].forEach((t, i) => SHOTS.push([`${m}_seq${i + 1}`, m, "hatch3q", "idle", t]));
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: "localhost,127.0.0.1" } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
page.on("console", m => { if (m.type() === "error" || m.type() === "warning") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto(/^https?:/.test(page_) ? page_ : "file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 240000 });
for (const [name, mech, view, act, t, look] of SHOTS) {
  if (only && !only.split(",").includes(name)) continue;
  await page.evaluate(([m, a, t]) => { window.MOCKUP.setMech(m); window.MOCKUP.setAct(a); window.MOCKUP.setT(t === "end" ? window.MOCKUP.end() : t); }, [mech, act, t]);
  await page.waitForTimeout(1500);
  await page.evaluate(([v, look]) => { if (look) window.MOCKUP.look(...look); else window.MOCKUP.setView(v); }, [view, look || null]);
  await page.waitForTimeout(2000);
  const box = await page.locator(".stage").boundingBox();
  await page.screenshot({ path: path.join(outDir, name + ".png"), clip: box, timeout: 180000 });
  console.log("shot", name);
}
await browser.close();
