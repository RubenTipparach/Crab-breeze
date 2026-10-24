// The cockpit's screen wall for the mockup pages: the world as the mech's cameras see it. A cube camera at
// the pilot's eye renders the world (with the mech hidden); every screen made of the material named
// SCREEN_allview samples it by the direction from the eye, so the screens read as windows. Scanlines, a
// phosphor grade, the HUD on the front screen by direction, a screen by screen power-up. Inlined by
// tools/mockups/gen_mech_designs.py.
//   const wall = makeScreenWall(THREE, scene, hudDataUrl);  mesh.material = wall.material;
//   wall.update(dt, eyeWorld, cockpitObject3D, powerOn, hudOn);  wall.capture(renderer, eyeWorld, hide);
function makeScreenWall(T, scene, hudSrc) {
  const rt = new T.WebGLCubeRenderTarget(512, { generateMipmaps: true, minFilter: T.LinearMipmapLinearFilter });
  const cam = new T.CubeCamera(0.3, 5000, rt);
  scene.add(cam);
  const material = new T.ShaderMaterial({
    side: T.DoubleSide, toneMapped: false,
    uniforms: { env: { value: rt.texture }, eye: { value: new T.Vector3() }, power: { value: 1 }, hud: { value: null },
                hudOn: { value: 1 }, podInv: { value: new T.Matrix4() }, time: { value: 0 } },
    vertexShader: "varying vec3 wp; void main(){ vec4 w = modelMatrix * vec4(position,1.0); wp = w.xyz; gl_Position = projectionMatrix * viewMatrix * w; }",
    fragmentShader: `
      uniform samplerCube env; uniform sampler2D hud; uniform vec3 eye; uniform float power, hudOn, time; uniform mat4 podInv;
      varying vec3 wp;
      float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
      void main(){
        vec3 d = normalize(wp - eye);
        vec3 ld = normalize((podInv * vec4(d, 0.0)).xyz);
        float az = atan(ld.x, ld.z), el = asin(clamp(ld.y, -1.0, 1.0));
        vec2 tile = vec2(floor(az / 0.9 + 0.5), floor(el / 0.6));
        float on = smoothstep(0.0, 0.12, power * 1.35 - hash(tile) * 0.35);
        float flick = step(0.5, fract(sin(time * 40.0 + hash(tile) * 10.0) * 7.0)) * (1.0 - step(0.999, power));
        vec3 c = textureCube(env, d).rgb;
        c = pow(c, vec3(0.95)) * vec3(0.94, 1.0, 0.99) * 1.05;
        c *= 0.93 + 0.07 * sin(gl_FragCoord.y * 1.6);
        if (hudOn > 0.5 && abs(az) < 0.42 && el > -0.2 && el < 0.44) {
          vec4 h = texture2D(hud, vec2(0.5 - az / 0.84, (el + 0.2) / 0.64));
          c = mix(c, h.rgb * 1.2, h.a * 0.9);
        }
        c = mix(vec3(0.012, 0.016, 0.02), c, on * (1.0 - flick * 0.7));
        gl_FragColor = vec4(c, 1.0);
      }`
  });
  if (hudSrc) {
    const img = new Image();
    img.onload = () => { const t = new T.Texture(img); t.encoding = T.sRGBEncoding; t.needsUpdate = true; material.uniforms.hud.value = t; };
    img.src = hudSrc;
  }
  let power = 1, last = new T.Vector3(1e9, 0, 0), timer = 0;
  function update(dt, eye, cockpit, on, hudOn) {
    power = on ? Math.min(1, power + dt / 2.5) : Math.max(0, power - dt / 0.6);
    const u = material.uniforms;
    u.eye.value.copy(eye); u.podInv.value.copy(cockpit.matrixWorld).invert();
    u.power.value = power; u.hudOn.value = hudOn && power > 0.95 ? 1 : 0; u.time.value += dt;
    timer -= dt;
  }
  // re-render the world from the eye when it has moved (hide: objects to hide during the capture)
  function capture(renderer, eye, hide, force) {
    if (!force && (eye.distanceTo(last) < 0.01 || timer > 0)) return;
    const was = hide.map(o => o.visible); hide.forEach(o => { o.visible = false; });
    cam.position.copy(eye); cam.update(renderer, scene);
    hide.forEach((o, i) => { o.visible = was[i]; });
    last.copy(eye); timer = 0.1;
  }
  return { material, update, capture, get power() { return power; } };
}
