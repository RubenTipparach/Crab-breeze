// The page's own check: in each variation, count building triangles within 17 m of the highway's centre line (it must be 0).
// Road structures (decks, ramps, bridges, piers) are their own mesh, marked userData.road, and are not counted.
// It also prints the page's drive check (driveCheck: the fill, clearances, grades, piers and the bot's trips) and fails
// on any problem in it.
// node check_roads.js <city_layouts.html> <dir holding three/package (three@0.128.0 from npm pack)>
const { chromium } = require('playwright'); const fs = require('fs');
const S = process.argv[3];
let failed = false;
(async () => {
  const b = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-webgl', '--ignore-gpu-blocklist'] });
  const p = await b.newPage({ viewport: { width: 1200, height: 700 } });
  await p.route('**/three.min.js', r => r.fulfill({ contentType: 'text/javascript', body: fs.readFileSync(S + '/three/package/build/three.min.js', 'utf8') }));
  await p.route('**/OrbitControls.js', r => r.fulfill({ contentType: 'text/javascript', body: fs.readFileSync(S + '/three/package/examples/js/controls/OrbitControls.js', 'utf8') }));
  await p.route('**/fonts.googleapis.com/**', r => r.abort());
  await p.setContent('<!doctype html><html><head><meta charset="utf-8"></head><body>' + fs.readFileSync(process.argv[2], 'utf8') + '</body></html>');
  p.on('pageerror', e => console.log('PAGEERROR', e.message));
  await p.waitForTimeout(8000);
  for (const combo of [[], ['#w-over'], ['#w-elevated'], ['#w-bypass'], ['#d-large', '#w-through'], ['#w-over'], ['#w-elevated'], ['#w-bypass'], ['#h-street', '#w-through'], ['#h-twin'], ['#w-over'], ['#d-small', '#s-coast', '#w-through'], ['#w-bypass'], ['#w-elevated']]) {
    for (const c of combo) { await p.click(c); await p.waitForTimeout(3000); }
    const r = await p.evaluate(() => {
      let bad = 0;
      sceneFuture.traverse(o => { if (!o.isMesh || !o.castShadow || o.userData.road) return; const a = o.geometry.attributes.position.array;
        for (let t = 0; t < a.length; t += 9) { const y = Math.max(a[t + 1], a[t + 4], a[t + 7]); if (y < 2 || y > 400) continue;
          const x = (a[t] + a[t + 3] + a[t + 6]) / 3, z = (a[t + 2] + a[t + 5] + a[t + 8]) / 3;
          if (window.__hwDist && window.__hwDist([x, z]) < 17) bad++; } });
      const d = window.__drive;
      return { bad, line: driveLine(d), ok: d.ok, lost: d.lost.slice(0, 5).map(l => `${l.kind} ${Math.round(l.m)} m at ${l.at.map(Math.round)}`),
        hits: d.collisions.slice(0, 8).map(h => `${h.a}/${h.b} clear ${h.clear.toFixed(1)} m at ${h.at.map(Math.round)}`),
        steep: d.steep.slice(0, 5).map(t => `${t.kind} ${(t.grade * 100).toFixed(1)} % at ${t.at.map(Math.round)}`), tight: d.tight.slice(0, 5).map(t => `${t.kind} r ${Math.round(t.r)} m at ${t.at.map(Math.round)}`), loose: d.loose.slice(0, 5).map(q => q.at.map(Math.round).join(',')), piers: d.pierHits.slice(0, 5).map(q => q.kind + ' at ' + q.at.map(Math.round)) };
    });
    console.log('COMBO', combo.join(' ') || 'default', 'building triangles over the highway:', r.bad);
    console.log('  ' + r.line);
    for (const k of ['lost', 'hits', 'steep', 'tight', 'loose', 'piers']) if (r[k].length) console.log('  ' + k + ': ' + r[k].join('; '));
    if (r.bad || !r.ok) failed = true;
  }
  await b.close();
  console.log(failed ? 'FAIL' : 'PASS'); process.exit(failed ? 1 : 0);
})();
