// @strudel/core imports SalatRepl from @kabelsalat/web, whose published
// bundle is browser-only and breaks Node ESM resolution. Stub it out: the
// offline renderer never touches the kabelsalat integration.
import fs from 'fs';
import path from 'path';
const dir = path.join(import.meta.dirname, 'node_modules', '@kabelsalat', 'web');
if (fs.existsSync(dir)) {
  fs.writeFileSync(path.join(dir, 'index.mjs'),
    'export class SalatRepl { play() {} stop() {} }\nexport default {};\n');
  const pj = JSON.parse(fs.readFileSync(path.join(dir, 'package.json')));
  Object.assign(pj, { main: 'index.mjs', module: 'index.mjs', type: 'module',
                      exports: { '.': './index.mjs' } });
  fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify(pj, null, 2));
  console.log('patched @kabelsalat/web for Node');
} else {
  console.log('no @kabelsalat/web found (nothing to patch)');
}
