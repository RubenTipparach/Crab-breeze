// Screenshot the alien crab page headless, one PNG per named shot, for looking at before it is shown.
// NODE_PATH=$(npm root -g) node tools/mockups/shoot_aliens.mjs docs/mockups/alien_mechs.html docs/screenshots/aliens [shot ...]
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright"); // global install: run with NODE_PATH=$(npm root -g)
import path from "node:path";
import fs from "node:fs";

const [page_, outDir, ...only] = process.argv.slice(2);
const ALL = [
  ["lineup", "all", "orbit", "idle"], ["skitter", "skitter", "orbit", "idle"], ["snapper", "snapper", "orbit", "snap"],
  ["bigboy", "bigboy", "orbit", "idle"], ["bigboy_front", "bigboy", "front", "idle"], ["snapper_scuttle", "snapper", "side", "scuttle"],
];
const SHOTS = only.length ? ALL.filter(s => only.includes(s[0])) : ALL;
fs.mkdirSync(outDir, { recursive: true });
const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const browser = await chromium.launch({ proxy, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
page.on("console", m => { if (m.type() === "error") console.log("console:", m.text()); });
page.on("pageerror", e => console.log("pageerror:", e.message));
await page.goto("file://" + path.resolve(page_));
await page.waitForFunction(() => window.MOCKUP_READY, null, { timeout: 180000 });
for (const [name, mech, view, act] of SHOTS) {
  await page.evaluate(([m, v, a]) => { window.MOCKUP.setAct(a); window.MOCKUP.setMech(m); window.MOCKUP.setView(v); }, [mech, view, act]);
  await page.waitForTimeout(2500);
  await page.locator(".stage").screenshot({ path: path.join(outDir, name + ".png"), timeout: 180000 });
  console.log("shot", name);
}
await browser.close();
