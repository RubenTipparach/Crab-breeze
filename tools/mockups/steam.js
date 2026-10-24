// Steam and mist for the mockup pages: one point cloud of soft discs, aged on the CPU, so a few thousand
// puffs are one draw. Inlined by tools/mockups/gen_mech_designs.py (and any page that vents).
//   const steam = makeSteam(THREE, scene);  steam.emit(worldPos, worldDir, mist, k);  steam.step(dt, camera, renderer);
// Steam: a fast jet that slows, rises and spreads. Mist: slow, heavy, it sinks and rolls along the floor.
function makeSteam(T, scene, N) {
  N = N || 2400;
  const geo = new T.BufferGeometry();
  const pos = new Float32Array(N * 3), size = new Float32Array(N), alpha = new Float32Array(N), tint = new Float32Array(N);
  geo.setAttribute("position", new T.BufferAttribute(pos, 3));
  geo.setAttribute("size", new T.BufferAttribute(size, 1));
  geo.setAttribute("alpha", new T.BufferAttribute(alpha, 1));
  geo.setAttribute("tint", new T.BufferAttribute(tint, 1));
  const parts = Array.from({ length: N }, () => ({ live: false, p: new T.Vector3(), v: new T.Vector3(), age: 0, life: 1, s0: 0.1, s1: 1, a: 0.5, mist: false }));
  const mat = new T.ShaderMaterial({
    transparent: true, depthWrite: false,
    uniforms: { scale: { value: 800 } },
    vertexShader: `attribute float size; attribute float alpha; attribute float tint; varying float va; varying float vt; uniform float scale;
      void main(){ vec4 mv = modelViewMatrix * vec4(position, 1.0); gl_Position = projectionMatrix * mv; gl_PointSize = size * scale / max(0.1, -mv.z); va = alpha; vt = tint; }`,
    fragmentShader: `varying float va; varying float vt;
      void main(){ vec2 c = gl_PointCoord - 0.5; float r = length(c) * 2.0; if (r > 1.0) discard;
        float a = (1.0 - r * r) * (1.0 - r * r) * va; vec3 col = mix(vec3(0.93, 0.94, 0.95), vec3(0.80, 0.87, 0.93), vt); gl_FragColor = vec4(col, a); }`
  });
  const cloud = new T.Points(geo, mat); cloud.frustumCulled = false; cloud.renderOrder = 5; scene.add(cloud);
  let next = 0;
  const j = new T.Vector3();
  function emit(p, dir, mist, k, scaleM) {
    const s = scaleM || 1, q = parts[next]; next = (next + 1) % N;
    q.live = true; q.age = 0; q.mist = mist; q.p.copy(p);
    j.set(Math.random() - 0.5, Math.random() - 0.5, Math.random() - 0.5);
    if (mist) { q.v.copy(dir).multiplyScalar((0.35 + Math.random() * 0.4) * s).addScaledVector(j, 0.25 * s); q.life = 4 + Math.random() * 3; q.s0 = 0.25 * s; q.s1 = (1.6 + Math.random()) * s; q.a = 0.22; }
    else { q.v.copy(dir).multiplyScalar((1.6 + Math.random() * 2.2 * (k || 1)) * s).addScaledVector(j, 0.9 * s); q.life = 1.2 + Math.random() * 1.3; q.s0 = 0.08 * s; q.s1 = (0.9 + Math.random() * 0.8) * s; q.a = 0.55; }
  }
  function step(dt, camera, renderer) {
    for (let i = 0; i < N; i++) {
      const q = parts[i];
      if (q.live) {
        q.age += dt; if (q.age > q.life) q.live = false;
        const u = q.age / q.life;
        if (q.mist) { q.v.y -= 0.35 * dt; q.v.multiplyScalar(1 - 0.5 * dt); if (q.p.y < 0.15) { q.p.y = 0.15; q.v.y = 0; q.v.x *= 1 + dt; q.v.z *= 1 + dt * 0.5; } }
        else { q.v.multiplyScalar(1 - 2.2 * dt); q.v.y += 0.9 * dt; }
        q.p.addScaledVector(q.v, dt);
        pos[i * 3] = q.p.x; pos[i * 3 + 1] = q.p.y; pos[i * 3 + 2] = q.p.z;
        size[i] = q.s0 + (q.s1 - q.s0) * Math.sqrt(u);
        alpha[i] = q.a * Math.min(1, u * 8) * (1 - u) * (1 - u);
        tint[i] = q.mist ? 1 : 0;
      } else { alpha[i] = 0; size[i] = 0; }
    }
    for (const a of ["position", "size", "alpha", "tint"]) geo.attributes[a].needsUpdate = true;
    if (camera && renderer) mat.uniforms.scale.value = renderer.domElement.height / (2 * Math.tan(T.MathUtils.degToRad(camera.fov) / 2));
  }
  return { emit, step, cloud };
}
