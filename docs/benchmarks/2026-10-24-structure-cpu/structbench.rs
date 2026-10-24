//! Measurement instrument for openspec/changes/structural-destruction and rubble: the mockup's
//! structural graph, solve and debris step ported to plain Rust, timed on the CPU. Not game code.
use std::time::Instant;

const GROUND: u32 = u32::MAX;
#[derive(Clone, Copy, PartialEq)]
enum Kind { Glass, Wall, Furniture, Partition, Slab, Column }
fn rank(k: Kind) -> u8 { match k { Kind::Glass => 0, Kind::Furniture | Kind::Partition => 2, Kind::Wall => 3, Kind::Slab => 4, Kind::Column => 5 } }

struct E { kind: Kind, storey: i32, mass: f32, rests: Vec<u32>, lateral: Vec<u32>, carries: Vec<u32>, node: (i32, i32), alive: bool, sup: u8, load: f32, cap: f32 }
struct B { e: Vec<E>, order: Vec<u32>, line: Vec<bool>, storeys: i32, col_at: Vec<std::collections::HashMap<(i32, i32), u32>>, slab_at: Vec<std::collections::HashMap<(i32, i32), u32>> }

// A tower like the mockup's: 6 x 6 bays, a 3-storey podium, the rest inset one bay; per storey and
// cell about as many partitions and furniture items as the mockup generated (3 216 elements at 19 storeys).
fn tower(storeys: i32, n: i32, podium: i32, part_per_cell: usize, furn_per_cell: usize) -> B {
    let mut b = B { e: vec![], order: vec![], line: vec![], storeys, col_at: vec![], slab_at: vec![] };
    let mut push = |b: &mut B, kind, storey, mass, rests: Vec<u32>, node| { b.e.push(E { kind, storey, mass, rests, lateral: vec![], carries: vec![], node, alive: true, sup: 0, load: 0.0, cap: f32::INFINITY }); (b.e.len() - 1) as u32 };
    for k in 0..storeys {
        let ins = if k >= podium { 1 } else { 0 };
        let (i0, i1) = (ins, n - ins);
        let mut cols = std::collections::HashMap::new();
        for a in i0..=i1 { for c in i0..=i1 {
            let mut r = vec![];
            if k == 0 { r.push(GROUND) } else {
                if let Some(&x) = b.col_at[(k - 1) as usize].get(&(a, c)) { r.push(x) }
                for (i, j) in [(a - 1, c - 1), (a, c - 1), (a - 1, c), (a, c)] { if let Some(&s) = b.slab_at[(k - 1) as usize].get(&(i, j)) { r.push(s) } }
            }
            let id = push(&mut b, Kind::Column, k, 3000.0, r, (a, c)); cols.insert((a, c), id);
        } }
        b.col_at.push(cols);
        let mut slabs = std::collections::HashMap::new();
        for i in i0..i1 { for j in i0..i1 {
            let r = [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)].iter().map(|p| b.col_at[k as usize][p]).collect();
            let id = push(&mut b, Kind::Slab, k, 21600.0, r, (i, j)); slabs.insert((i, j), id);
        } }
        for (&(i, j), &id) in &slabs { for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] { if let Some(&nb) = slabs.get(&(i + di, j + dj)) { b.e[id as usize].lateral.push(nb) } } }
        b.slab_at.push(slabs);
        // facade: one wall and one glass per perimeter edge
        for side in 0..4 { for idx in i0..i1 {
            let (cell, nodes) = match side { 0 => ((idx, i0), [(idx, i0), (idx + 1, i0)]), 1 => ((idx, i1 - 1), [(idx, i1), (idx + 1, i1)]), 2 => ((i0, idx), [(i0, idx), (i0, idx + 1)]), _ => ((i1 - 1, idx), [(i1, idx), (i1, idx + 1)]) };
            let mut r = vec![if k == 0 { GROUND } else { b.slab_at[(k - 1) as usize][&cell] }];
            for nd in nodes { r.push(b.col_at[k as usize][&nd]) }
            let w = push(&mut b, Kind::Wall, k, 6000.0, r, cell);
            push(&mut b, Kind::Glass, k, 300.0, vec![w], cell);
        } }
        for i in i0..i1 { for j in i0..i1 {
            let under = if k == 0 { GROUND } else { b.slab_at[(k - 1) as usize][&(i, j)] };
            for _ in 0..part_per_cell { push(&mut b, Kind::Partition, k, 500.0, vec![under], (i, j)); }
            for _ in 0..furn_per_cell { push(&mut b, Kind::Furniture, k, 80.0, vec![under], (i, j)); }
        } }
    }
    for id in 0..b.e.len() { for r in b.e[id].rests.clone() { if r != GROUND { b.e[r as usize].carries.push(id as u32) } } }
    let mut order: Vec<u32> = (0..b.e.len() as u32).collect();
    order.sort_by(|&p, &q| b.e[q as usize].storey.cmp(&b.e[p as usize].storey).then(rank(b.e[p as usize].kind).cmp(&rank(b.e[q as usize].kind))).then(p.cmp(&q)));
    b.order = order; b.line = vec![false; b.e.len()];
    b
}

fn slab_factor(b: &B, id: usize) -> f32 {
    let held: Vec<(i32, i32)> = b.e[id].rests.iter().filter(|&&r| b.e[r as usize].alive && b.line[r as usize]).map(|&r| b.e[r as usize].node).collect();
    match held.len() { 4.. => 1.0, 3 => 0.85, 2 => if held[0].0 == held[1].0 || held[0].1 == held[1].1 { 0.45 } else { 0.7 }, 1 => 0.25, _ => 0.4 }
}

/// One solve; returns (loose elements, overloaded elements).
fn solve(b: &mut B, design: bool) -> (usize, usize) {
    for e in b.e.iter_mut() { e.sup = 0 }
    let mut q: Vec<u32> = vec![];
    for id in 0..b.e.len() { if b.e[id].alive && b.e[id].rests.contains(&GROUND) { b.e[id].sup = 1; q.push(id as u32) } }
    let spread = |b: &mut B, q: &mut Vec<u32>, mark: u8| { while let Some(id) = q.pop() { for ci in 0..b.e[id as usize].carries.len() { let c = b.e[id as usize].carries[ci] as usize; if b.e[c].alive && b.e[c].sup == 0 { b.e[c].sup = mark; q.push(c as u32) } } } };
    spread(b, &mut q, 1);
    let mut hung = vec![];
    for id in 0..b.e.len() { let e = &b.e[id]; if e.kind == Kind::Slab && e.alive && e.sup == 0 && e.lateral.iter().any(|&n| b.e[n as usize].alive && b.e[n as usize].sup == 1) { hung.push(id as u32) } }
    for &h in &hung { b.e[h as usize].sup = 2 }
    spread(b, &mut hung, 2);
    let mut loose = 0;
    for e in b.e.iter_mut() { if e.alive && e.sup == 0 { e.alive = false; loose += 1 } }
    // load line, bottom up (order is top-down, so walk it backwards)
    for i in (0..b.order.len()).rev() { let id = b.order[i] as usize; if b.e[id].kind != Kind::Column { continue } let e = &b.e[id];
        b.line[id] = e.alive && e.sup > 0 && (e.rests[0] == GROUND || (b.e[e.rests[0] as usize].kind == Kind::Column && b.e[e.rests[0] as usize].alive && b.line[e.rests[0] as usize])); }
    for e in b.e.iter_mut() { e.load = if e.alive && e.sup > 0 { e.mass * 9.81e-3 } else { 0.0 } }
    let mut fail_now = 0;
    let mut targets: Vec<u32> = Vec::with_capacity(8);
    for oi in 0..b.order.len() {
        let id = b.order[oi] as usize;
        if !b.e[id].alive || b.e[id].sup == 0 { continue }
        targets.clear();
        let e = &b.e[id];
        match e.kind {
            Kind::Slab => { for &r in &e.rests { if b.e[r as usize].alive && b.line[r as usize] { targets.push(r) } }
                if targets.is_empty() { for &n in &e.lateral { let ne = &b.e[n as usize]; if ne.alive && ne.sup == 1 && ne.rests.iter().any(|&r| b.e[r as usize].alive && b.line[r as usize]) { targets.push(n) } } }
                if targets.is_empty() { fail_now += 1; continue } }
            Kind::Column => { if b.line[id] { if e.rests[0] != GROUND { targets.push(e.rests[0]) } } else { for &r in &e.rests { if r != GROUND && b.e[r as usize].alive && b.e[r as usize].sup > 0 { targets.push(r) } } } }
            _ => { for &r in &e.rests { if r != GROUND && b.e[r as usize].alive && b.e[r as usize].sup > 0 { targets.push(r) } } }
        }
        if targets.is_empty() { continue }
        let share = b.e[id].load / targets.len() as f32;
        for &t in &targets { b.e[t as usize].load += share }
    }
    let mut over = 0;
    for id in 0..b.e.len() {
        let k = b.e[id].kind;
        if design { if k == Kind::Column || k == Kind::Slab { b.e[id].cap = b.e[id].load.max(1.0) * 2.0 } continue }
        if b.e[id].alive && b.e[id].cap.is_finite() {
            let f = if k == Kind::Slab { slab_factor(b, id) } else { 1.0 };
            if b.e[id].load / (b.e[id].cap * f) > 1.0 { over += 1 }
        }
    }
    (loose, over + fail_now)
}

fn time<F: FnMut()>(n: usize, mut f: F) -> (f64, f64, f64) {
    let mut v: Vec<f64> = (0..n).map(|_| { let t = Instant::now(); f(); t.elapsed().as_secs_f64() * 1e3 }).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (v[n / 2], v[n * 95 / 100], v[n - 1])
}

// Debris: the mockup's free-piece step (gravity, ground = heap bilinear + stack grid + slab lookup, settle).
struct Piece { c: [f32; 3], v: [f32; 3], hh: f32, settled: bool, rest: f32 }
fn debris_step(p: &mut [Piece], heap: &[f32], stack: &[f32], slabs: &[f32], dt: f32) {
    let (hn, sn) = (160usize, 640usize);
    for q in p.iter_mut() {
        if q.settled { continue }
        q.v[1] -= 9.81 * dt; for i in 0..3 { q.c[i] += q.v[i] * dt }
        let mut sup = 0.0f32;
        for (sx, sz) in [(0.0, 0.0), (-0.3, -0.3), (0.3, -0.3), (-0.3, 0.3), (0.3, 0.3)] {
            let (x, z) = (q.c[0] + sx, q.c[2] + sz);
            let fx = (x + 80.0).clamp(0.0, 158.0); let fz = (z + 70.0).clamp(0.0, 138.0);
            let (i, j) = (fx as usize, fz as usize); let (tx, tz) = (fx - i as f32, fz - j as f32);
            let h = heap[j * hn + i] * (1.0 - tx) * (1.0 - tz) + heap[j * hn + i + 1] * tx * (1.0 - tz) + heap[(j + 1) * hn + i] * (1.0 - tx) * tz + heap[(j + 1) * hn + i + 1] * tx * tz;
            let si = (((x + 80.0) / 0.25) as usize).min(sn - 1); let sj = (((z + 70.0) / 0.25) as usize).min(559);
            let st = stack[sj * sn + si];
            let cell = ((((x + 80.0) / 6.0) as usize).min(26) * 24 + (((z + 70.0) / 6.0) as usize).min(23)) * 20;
            let mut sl = 0.0f32; for k in (0..20).rev() { let top = slabs[cell + k]; if top > 0.0 && top <= q.c[1] + 0.35 { sl = top; break } }
            sup = sup.max(h).max(if st <= q.c[1] - q.hh + 0.1 { st } else { 0.0 }).max(sl);
        }
        if q.c[1] - q.hh < sup { q.c[1] = sup + q.hh; if q.v[1] < 0.0 { q.v[1] *= -0.18 } q.v[0] *= 0.72; q.v[2] *= 0.72;
            if q.v.iter().map(|v| v * v).sum::<f32>() < 0.25 { q.rest += dt; if q.rest > 0.3 { q.settled = true } } }
    }
}

fn main() {
    println!("machine: see sysctl; release build; CPU only, one core");
    for &(st, ppc, fpc) in &[(19, 2usize, 4usize), (60, 2, 4)] {
        let t = Instant::now();
        let mut b = tower(st, 6, 3, ppc, fpc);
        let build = t.elapsed().as_secs_f64() * 1e3;
        solve(&mut b, true);
        let n = b.e.len();
        let bonds: usize = b.e.iter().map(|e| e.rests.len() + e.lateral.len()).sum();
        let base = b.e.iter().map(|e| e.alive).collect::<Vec<_>>();
        let (p50, p95, mx) = time(400, || { solve(&mut b, false); });
        // the "side" demolition: every ground column on one side
        let side: Vec<usize> = (0..n).filter(|&i| b.e[i].kind == Kind::Column && b.e[i].storey == 0 && b.e[i].node.1 == 0).collect();
        let (s50, s95, smx) = time(400, || { for (i, e) in b.e.iter_mut().enumerate() { e.alive = base[i] } for &i in &side { b.e[i].alive = false } solve(&mut b, false); });
        for (i, e) in b.e.iter_mut().enumerate() { e.alive = base[i] } for &i in &side { b.e[i].alive = false }
        let (loose, over) = solve(&mut b, false);
        println!("tower {st} storeys: {n} elements, {bonds} bonds, build {build:.2} ms; solve intact p50 {p50:.3} p95 {p95:.3} max {mx:.3} ms; after losing a ground side p50 {s50:.3} p95 {s95:.3} max {smx:.3} ms ({loose} loose, {over} overloaded)");
    }
    let heap = vec![0.5f32; 160 * 140]; let mut stack = vec![0.0f32; 640 * 560]; for (i, s) in stack.iter_mut().enumerate() { if i % 7 == 0 { *s = 0.6 } }
    let slabs: Vec<f32> = (0..27 * 24 * 20).map(|i| if i % 3 == 0 { 0.3 + (i % 20) as f32 * 3.5 } else { 0.0 }).collect();
    for &n in &[600usize, 2000, 9000] {
        let mut p: Vec<Piece> = (0..n).map(|i| Piece { c: [(i % 97) as f32 - 48.0, 5.0 + (i % 13) as f32, (i % 89) as f32 - 44.0], v: [1.0, 0.0, -1.0], hh: 0.2, settled: false, rest: 0.0 }).collect();
        let (p50, p95, mx) = time(300, || { for q in p.iter_mut() { q.settled = false; q.c[1] = 30.0 } debris_step(&mut p, &heap, &stack, &slabs, 1.0 / 60.0); });
        println!("debris step, {n} pieces all moving: p50 {p50:.3} p95 {p95:.3} max {mx:.3} ms");
    }
    let t = Instant::now(); let mut s2 = vec![0.0f32; 640 * 560]; for k in 0..9000 { let (i0, j0) = ((k * 37) % 620, (k * 53) % 540); for j in j0..j0 + 8 { for i in i0..i0 + 8 { let ix = j * 640 + i; if s2[ix] < 1.0 { s2[ix] = 1.0 } } } } stack = s2;
    println!("restack of 9000 resting pieces (8 x 8 cells each): {:.3} ms (check {})", t.elapsed().as_secs_f64() * 1e3, stack[0]);
}
