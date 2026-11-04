// The page's own check: in each variation, count building triangles within 17 m of the highway's centre line (it must be 0).
// node check_roads.js <city_layouts.html> <dir holding three/package (three@0.128.0 from npm pack)>
const { chromium } = require('playwright'); const fs = require('fs');
const S = process.argv[3];
(async () => {
  const b = await chromium.launch({ args: ['--use-gl=swiftshader', '--enable-webgl', '--ignore-gpu-blocklist'] });
  const p = await b.newPage({ viewport: { width: 1200, height: 700 } });
  await p.route('**/three.min.js', r => r.fulfill({ contentType: 'text/javascript', body: fs.readFileSync(S + '/three/package/build/three.min.js', 'utf8') }));
  await p.route('**/OrbitControls.js', r => r.fulfill({ contentType: 'text/javascript', body: fs.readFileSync(S + '/three/package/examples/js/controls/OrbitControls.js', 'utf8') }));
  await p.route('**/fonts.googleapis.com/**', r => r.abort());
  await p.setContent('<!doctype html><html><head><meta charset="utf-8"></head><body>' + fs.readFileSync(process.argv[2], 'utf8') + '</body></html>');
  await p.waitForTimeout(8000);
  for (const combo of [[], ['#w-bypass'], ['#d-large', '#w-through'], ['#w-bypass'], ['#h-street', '#w-through'], ['#h-twin']]) {
    for (const c of combo) { await p.click(c); await p.waitForTimeout(3000); }
    const r = await p.evaluate(() => {
      let bad = 0;
      sceneFuture.traverse(o => { if (!o.isMesh || !o.castShadow) return; const a = o.geometry.attributes.position.array;
        for (let t = 0; t < a.length; t += 9) { const y = Math.max(a[t + 1], a[t + 4], a[t + 7]); if (y < 2 || y > 400) continue;
          const x = (a[t] + a[t + 3] + a[t + 6]) / 3, z = (a[t + 2] + a[t + 5] + a[t + 8]) / 3;
          if (window.__hwDist && window.__hwDist([x, z]) < 17) bad++; } });
      return bad;
    });
    console.log('COMBO', combo.join(' ') || 'default', 'building triangles over the highway:', r);
  }
  await b.close();
})();
