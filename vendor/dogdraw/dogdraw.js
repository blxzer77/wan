/**
 * Procedural SVG dogs.
 * Inspired by fishdraw by Lingdong Huang; includes adapted drawing utilities.
 * https://github.com/LingDong-/fishdraw — MIT, see LICENSE.
 */

let jsr = 0x5EED;
const { PI, sin, cos, atan2, hypot, min, max, abs, floor, sqrt, pow: mpow } = Math;

function seed_rand(seedStr) {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < seedStr.length; i++) {
    h ^= seedStr.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  jsr = (h >>> 0) || 0x5EED;
  perlin = null;
}

function rand() {
  jsr ^= (jsr << 17);
  jsr ^= (jsr >> 13);
  jsr ^= (jsr << 5);
  return (jsr >>> 0) / 4294967295;
}

function rand_range(a, b) {
  return a + rand() * (b - a);
}

// --- 3D Perlin Noise ---
const PERLIN_YWRAPB = 4, PERLIN_YWRAP = 1 << PERLIN_YWRAPB;
const PERLIN_ZWRAPB = 8, PERLIN_ZWRAP = 1 << PERLIN_ZWRAPB;
const PERLIN_SIZE = 4095;
let perlin = null;

function scaled_cosine(i) {
  return 0.5 * (1.0 - cos(i * PI));
}

function noise(x, y = 0, z = 0) {
  if (perlin == null) {
    perlin = new Array(PERLIN_SIZE + 1);
    for (let i = 0; i < PERLIN_SIZE + 1; i++) perlin[i] = rand();
  }
  if (x < 0) x = -x;
  if (y < 0) y = -y;
  if (z < 0) z = -z;
  let xi = floor(x), yi = floor(y), zi = floor(z);
  let xf = x - xi, yf = y - yi, zf = z - zi;
  let r = 0, ampl = 0.5;
  for (let o = 0; o < 4; o++) {
    let of = xi + (yi << PERLIN_YWRAPB) + (zi << PERLIN_ZWRAPB);
    let rxf = scaled_cosine(xf), ryf = scaled_cosine(yf);
    let n1 = perlin[of & PERLIN_SIZE];
    n1 += rxf * (perlin[(of + 1) & PERLIN_SIZE] - n1);
    let n2 = perlin[(of + PERLIN_YWRAP) & PERLIN_SIZE];
    n2 += rxf * (perlin[(of + PERLIN_YWRAP + 1) & PERLIN_SIZE] - n2);
    n1 += ryf * (n2 - n1);
    of += PERLIN_ZWRAP;
    n2 = perlin[of & PERLIN_SIZE];
    n2 += rxf * (perlin[(of + 1) & PERLIN_SIZE] - n2);
    let n3 = perlin[(of + PERLIN_YWRAP) & PERLIN_SIZE];
    n3 += rxf * (perlin[(of + PERLIN_YWRAP + 1) & PERLIN_SIZE] - n3);
    n2 += ryf * (n3 - n2);
    n1 += scaled_cosine(zf) * (n2 - n1);
    r += n1 * ampl;
    ampl *= 0.5;
    xi <<= 1; xf *= 2;
    yi <<= 1; yf *= 2;
    zi <<= 1; zf *= 2;
    if (xf >= 1.0) { xi++; xf--; }
    if (yf >= 1.0) { yi++; yf--; }
    if (zf >= 1.0) { zi++; zf--; }
  }
  return r;
}

// --- 2D Geometry & Spline Primitives ---
function dist(x0, y0, x1, y1) { return hypot(x1 - x0, y1 - y0); }
function lerp(a, b, t) { return a * (1 - t) + b * t; }
function lerp2d(p0, p1, t) { return [p0[0] * (1 - t) + p1[0] * t, p0[1] * (1 - t) + p1[1] * t]; }

function get_bbox(points) {
  let xmin = Infinity, ymin = Infinity, xmax = -Infinity, ymax = -Infinity;
  for (let i = 0; i < points.length; i++) {
    let [x, y] = points[i];
    if (x < xmin) xmin = x;
    if (y < ymin) ymin = y;
    if (x > xmax) xmax = x;
    if (y > ymax) ymax = y;
  }
  return { x: xmin, y: ymin, w: xmax - xmin, h: ymax - ymin };
}

function trsl_poly(poly, dx, dy) {
  return poly.map(p => [p[0] + dx, p[1] + dy]);
}

function rot_poly(poly, ang, cx = 0, cy = 0) {
  let c = cos(ang), s = sin(ang);
  return poly.map(([x, y]) => {
    let dx = x - cx, dy = y - cy;
    return [cx + dx * c - dy * s, cy + dx * s + dy * c];
  });
}

function catmull_rom(pts, samplesPerSeg = 8, closed = false) {
  if (pts.length < 2) return pts.slice();
  let p = closed
    ? [pts[pts.length - 1], ...pts, pts[0], pts[1]]
    : [pts[0], ...pts, pts[pts.length - 1]];
  let out = [];
  for (let i = 1; i < p.length - 2; i++) {
    let p0 = p[i - 1], p1 = p[i], p2 = p[i + 1], p3 = p[i + 2];
    for (let j = 0; j < samplesPerSeg; j++) {
      let t = j / samplesPerSeg;
      let t2 = t * t, t3 = t2 * t;
      let x = 0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3);
      let y = 0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3);
      out.push([x, y]);
    }
  }
  if (!closed) out.push(pts[pts.length - 1].slice());
  else out.push(out[0].slice());
  return out;
}

function resample(polyline, step = 2.5) {
  if (polyline.length < 2) return polyline.slice();
  let out = [polyline[0].slice()];
  let acc = 0;
  let prev = polyline[0];
  for (let i = 1; i < polyline.length; i++) {
    let curr = polyline[i];
    let d = dist(prev[0], prev[1], curr[0], curr[1]);
    while (acc + d >= step) {
      let t = (step - acc) / d;
      let nx = lerp(prev[0], curr[0], t);
      let ny = lerp(prev[1], curr[1], t);
      out.push([nx, ny]);
      prev = [nx, ny];
      d = dist(prev[0], prev[1], curr[0], curr[1]);
      acc = 0;
    }
    acc += d;
    prev = curr;
  }
  let last = polyline[polyline.length - 1];
  if (dist(out[out.length - 1][0], out[out.length - 1][1], last[0], last[1]) > step * 0.25) {
    out.push(last.slice());
  }
  return out;
}

// Add organic hand-inked nib micro-variation
function organic_ink(poly, amp = 0.65, freq = 0.045, seedZ = 1) {
  return poly.map(([x, y], idx) => {
    let t = poly.length > 1 ? idx / (poly.length - 1) : 0.5;
    let env = sin(t * PI) * 0.75 + 0.25;
    let dx = (noise(x * freq, y * freq, seedZ) - 0.5) * 2 * amp * env;
    let dy = (noise(x * freq, y * freq, seedZ + 19.7) - 0.5) * 2 * amp * env;
    return [x + dx, y + dy];
  });
}

// Add fur-tuft serration along neck/chest/belly outline segments (preserving crisp facial contours)
function fur_serrate(poly, tuftLen = 2.4, freq = 0.25, dirX = 0.3, dirY = 0.7, minX = -Infinity) {
  let res = resample(poly, 2.2);
  if (res.length < 4 || tuftLen <= 0.5) return res;
  let out = [];
  for (let i = 0; i < res.length; i++) {
    let [x, y] = res[i];
    if (x < minX) {
      out.push([x, y]);
      continue;
    }
    let t = i / (res.length - 1);
    let env = sin(t * PI);
    let saw = ((i * freq) % 1.0);
    let n = noise(x * 0.05, y * 0.05, 7);
    let spike = (saw < 0.65 ? saw / 0.65 : (1 - saw) / 0.35) * tuftLen * env * (0.5 + n * 0.8);
    out.push([x + dirX * spike, y + dirY * spike]);
  }
  return out;
}

// --- Ray-Casting Polygon Clipping & Hidden-Line Removal ---
function pt_in_pl(x, y, x0, y0, x1, y1) {
  return (x - x0) * (y1 - y0) - (y - y0) * (x1 - x0);
}

function seg_isect(p0x, p0y, p1x, p1y, q0x, q0y, q1x, q1y, is_ray = false) {
  let d0x = p1x - p0x, d0y = p1y - p0y;
  let d1x = q1x - q0x, d1y = q1y - q0y;
  let vc = d0x * d1y - d0y * d1x;
  if (abs(vc) < 1e-9) return null;
  let vcn = vc * vc;
  let q0x_p0x = q0x - p0x, q0y_p0y = q0y - p0y;
  let vc_vcn = vc / vcn;
  let t = (q0x_p0x * d1y - q0y_p0y * d1x) * vc_vcn;
  let s = (q0x_p0x * d0y - q0y_p0y * d0x) * vc_vcn;
  if (0 <= t && (is_ray || t < 1) && 0 <= s && s < 1) {
    return {
      t, s,
      side: pt_in_pl(p0x, p0y, p1x, p1y, q0x, q0y) < 0 ? 1 : -1,
      xy: [p1x * t + p0x * (1 - t), p1y * t + p0y * (1 - t)]
    };
  }
  return null;
}

function seg_isect_poly(x0, y0, x1, y1, poly, is_ray = false) {
  let n = poly.length, isects = [];
  for (let i = 0; i < n; i++) {
    let a = poly[i], b = poly[(i + 1) % n];
    let xx = seg_isect(x0, y0, x1, y1, a[0], a[1], b[0], b[1], is_ray);
    if (xx) isects.push(xx);
  }
  isects.sort((a, b) => a.t - b.t);
  return isects;
}

function pt_in_poly(x, y, polygon) {
  if (!polygon || polygon.length < 3) return false;
  return seg_isect_poly(x, y, x + Math.E, y + PI, polygon, true).length % 2 !== 0;
}

function clip(polyline, polygon) {
  if (!polyline || !polyline.length || !polygon || polygon.length < 3) {
    return { true: [], false: polyline ? [polyline] : [] };
  }
  let zero = pt_in_poly(polyline[0][0], polyline[0][1], polygon);
  let out = { true: [[]], false: [[]] };
  let io = zero;
  for (let i = 0; i < polyline.length; i++) {
    let a = polyline[i], b = polyline[i + 1];
    out[io][out[io].length - 1].push(a);
    if (!b) break;
    let isects = seg_isect_poly(a[0], a[1], b[0], b[1], polygon, false);
    for (let j = 0; j < isects.length; j++) {
      out[io][out[io].length - 1].push(isects[j].xy);
      io = !io;
      out[io].push([isects[j].xy]);
    }
  }
  out.true = out.true.filter(x => x.length > 1);
  out.false = out.false.filter(x => x.length > 1);
  return out;
}

function clip_multi(polylines, polygon) {
  let out = { true: [], false: [] };
  for (let i = 0; i < polylines.length; i++) {
    let c = clip(polylines[i], polygon);
    out.true.push(...c.true);
    out.false.push(...c.false);
  }
  return out;
}

function clip_multi_many(polylines, polygons) {
  let cur = polylines;
  for (let poly of polygons) {
    if (!poly || poly.length < 3) continue;
    cur = clip_multi(cur, poly).false;
  }
  return cur;
}

// --- Copperplate Engraving & Form-Following Shaders ---

function engraving_hatch(poly, step = 3.2, dx = -9, dy = -13, angle = 0.62, curvature = 0.14) {
  let bbox = get_bbox(poly);
  let pad = max(bbox.w, bbox.h) + step * 4;
  let cx = bbox.x + bbox.w * 0.5, cy = bbox.y + bbox.h * 0.5;
  let ca = cos(angle), sa = sin(angle);
  let nx = -sa, ny = ca;
  let rawLines = [];

  for (let d = -pad; d <= pad; d += step) {
    let px = cx + d * nx, py = cy + d * ny;
    let segs = 14;
    let line = [];
    for (let k = 0; k <= segs; k++) {
      let s = (k / segs) * 2 - 1;
      let bow = (1 - s * s) * curvature * bbox.h * 0.35;
      let lx = px + s * pad * ca + nx * bow;
      let ly = py + s * pad * sa + ny * bow;
      line.push([lx, ly]);
    }
    rawLines.push(line);
  }

  let inside = clip_multi(rawLines, poly).true;
  if (dx !== 0 || dy !== 0) {
    let carve = trsl_poly(poly, dx, dy);
    inside = clip_multi(inside, carve).false;
  }

  let out = [];
  for (let i = 0; i < inside.length; i++) {
    let L = inside[i];
    if (L.length < 2) continue;
    let cutRatio = 0.12 + rand() * 0.25;
    let keepCount = max(2, floor(L.length * (1 - cutRatio * 0.5)));
    out.push( dy < 0 ? L.slice(L.length - keepCount) : L.slice(0, keepCount) );
  }
  return out;
}

function dense_shadow_hatch(poly, step = 2.1, angle = -0.48) {
  let h1 = engraving_hatch(poly, step, 0, 0, angle, 0.05);
  let h2 = engraving_hatch(poly, step * 1.45, -3, -4, angle + 0.55, 0.05);
  return [...h1, ...h2];
}

// Flowing anatomical hair locks (layered short/medium/long strokes following body flow)
function coat_flow_strokes(poly, spineCurve, density = 1.0, len = 10, waviness = 0.35, curl = 0) {
  let bbox = get_bbox(poly);
  let stepX = max(3.2, 8.2 / sqrt(density));
  let stepY = max(3.0, 6.4 / sqrt(density));
  let strokes = [];

  for (let y = bbox.y + 2; y < bbox.y + bbox.h - 2; y += stepY) {
    for (let x = bbox.x + 2; x < bbox.x + bbox.w - 2; x += stepX) {
      let sx = x + (rand() - 0.5) * stepX * 0.85;
      let sy = y + (rand() - 0.5) * stepY * 0.85;
      if (!pt_in_poly(sx, sy, poly)) continue;

      // Leave a natural specular sheen band along the upper-middle ribcage & thigh
      let ny = (sy - bbox.y) / max(1, bbox.h);
      let nx = (sx - bbox.x) / max(1, bbox.w);
      let sheen = mpow(sin(ny * PI), 1.6) * sin(nx * PI);
      if (curl < 0.5 && rand() < sheen * 0.68) continue;

      if (curl > 0.5) {
        let r = len * (0.28 + rand() * 0.22);
        let a0 = rand() * PI * 2;
        let arc = [];
        let segs = 8;
        for (let k = 0; k <= segs; k++) {
          let a = a0 + (k / segs) * PI * 1.55;
          let rr = r * (0.85 + 0.25 * (k / segs));
          arc.push([sx + cos(a) * rr, sy + sin(a) * rr * 0.9]);
        }
        strokes.push(arc);
      } else {
        let baseAng = 0.16 + ny * 0.32;
        if (spineCurve && spineCurve.length > 1) {
          let bestD = Infinity, bestA = baseAng;
          for (let s = 0; s < spineCurve.length - 1; s++) {
            let p0 = spineCurve[s], p1 = spineCurve[s + 1];
            let d = dist(sx, sy, p0[0], p0[1]);
            if (d < bestD) {
              bestD = d;
              bestA = atan2(p1[1] - p0[1], p1[0] - p0[0]) + ny * 0.35;
            }
          }
          baseAng = bestA;
        }
        let pts = [[sx, sy]];
        let cx = sx, cy = sy;
        let steps = max(3, floor(len / 2.2));
        let stepLen = len / steps;
        let ang = baseAng + (noise(sx * 0.025, sy * 0.025, 5) - 0.5) * waviness * 1.8;
        for (let k = 0; k < steps; k++) {
          ang += (noise(cx * 0.045, cy * 0.045, k + 11) - 0.47) * waviness;
          cx += cos(ang) * stepLen;
          cy += sin(ang) * stepLen;
          pts.push([cx, cy]);
        }
        strokes.push(pts);
      }
    }
  }
  return clip_multi(strokes, poly).true;
}

// Long cascading silky locks with S-wave locks
function silky_locks(poly, spacing = 2.8, overhang = 12, waveAmp = 3.2, angleOffset = 0.08, minX = -Infinity) {
  let bbox = get_bbox(poly);
  let strands = [];
  for (let x = max(bbox.x + 2, minX); x < bbox.x + bbox.w - 2; x += spacing) {
    let isects = seg_isect_poly(x, bbox.y - 5, x, bbox.y + bbox.h + 5, poly, false);
    if (isects.length >= 2) {
      let yStart = lerp(isects[0].xy[1], isects[isects.length - 1].xy[1], 0.15 + rand() * 0.35);
      let yEnd = isects[isects.length - 1].xy[1] + overhang * (0.45 + rand() * 0.65);
      if (yEnd - yStart > 7) {
        let s = [];
        let n = max(6, floor((yEnd - yStart) / 3.0));
        let phase = noise(x * 0.05, 1.3) * PI * 2;
        for (let i = 0; i <= n; i++) {
          let t = i / n;
          let py = lerp(yStart, yEnd, t);
          let wave = sin(t * PI * 1.6 + phase) * waveAmp * mpow(t, 0.75);
          let px = x + t * (yEnd - yStart) * sin(angleOffset) + wave;
          s.push([px, py]);
        }
        strands.push(s);
      }
    }
  }
  return strands;
}

// Coat patterns (Dalmatian spots, Jack Russell patches, German Shepherd/Basset saddle, Husky/Shiba/Corgi countershading)
function coat_markings(poly, patternType, bbox, minX = -Infinity, landmarks = null) {
  if (!patternType || patternType === 'solid') return [];
  let lines = [];
  if (patternType === 'spots') {
    let count = 34;
    for (let i = 0; i < count; i++) {
      let sx = bbox.x + rand_range(0.06, 0.94) * bbox.w;
      let sy = bbox.y + rand_range(0.08, 0.88) * bbox.h;
      if (!pt_in_poly(sx, sy, poly)) continue;
      let r = sx < minX ? rand_range(1.6, 3.2) : rand_range(2.4, 5.8);
      let spot = [];
      for (let k = 0; k < 12; k++) {
        let a = (k / 12) * PI * 2;
        let rr = r * (0.72 + noise(sx * 0.1 + cos(a), sy * 0.1 + sin(a), i) * 0.55);
        spot.push([sx + cos(a) * rr, sy + sin(a) * rr * 0.86]);
      }
      spot.push(spot[0].slice());
      let cOut = clip(spot, poly).true;
      let cHatch = clip_multi(engraving_hatch(spot, 1.35, 0, 0, 0.75, 0.05), poly).true;
      lines.push(...cOut, ...cHatch);
    }
  } else if (patternType === 'patches') {
    // Classic Jack Russell Terrier piebald markings:
    // 1. Tan/Black eye-and-ear crown patch (pirate patch)
    // 2. Back / loin spot
    // 3. Croup / tail-root spot
    let patches = [];
    if (landmarks) {
      let { eyeX, eyeY, earBaseX, earBaseY, backMid, croup } = landmarks;
      patches.push({ cx: lerp(eyeX, earBaseX, 0.45), cy: lerp(eyeY, earBaseY, 0.35), rx: 14.5, ry: 11.5, seed: 1 });
      patches.push({ cx: backMid[0] + 4, cy: backMid[1] + 8, rx: 19, ry: 13.5, seed: 2 });
      patches.push({ cx: croup[0] - 8, cy: croup[1] + 7, rx: 16, ry: 14, seed: 3 });
    } else {
      patches.push({ cx: bbox.x + bbox.w * 0.18, cy: bbox.y + bbox.h * 0.25, rx: 14, ry: 11, seed: 1 });
      patches.push({ cx: bbox.x + bbox.w * 0.62, cy: bbox.y + bbox.h * 0.38, rx: 18, ry: 13, seed: 2 });
      patches.push({ cx: bbox.x + bbox.w * 0.88, cy: bbox.y + bbox.h * 0.42, rx: 15, ry: 13, seed: 3 });
    }
    for (let p of patches) {
      let patchPoly = [];
      let n = 20;
      for (let k = 0; k < n; k++) {
        let a = (k / n) * PI * 2;
        let wobble = 0.82 + noise(cos(a) * 1.6 + p.seed, sin(a) * 1.6 + p.seed, p.seed * 7) * 0.42;
        patchPoly.push([p.cx + cos(a) * p.rx * wobble, p.cy + sin(a) * p.ry * wobble]);
      }
      patchPoly.push(patchPoly[0].slice());
      let cBorder = clip(patchPoly, poly).true;
      let h1 = clip_multi(engraving_hatch(patchPoly, 1.9, 0, 0, -0.52, 0.08), poly).true;
      let h2 = clip_multi(engraving_hatch(patchPoly, 2.9, -2, -3, 0.42, 0.06), poly).true;
      lines.push(...cBorder, ...h1, ...h2);
    }
  } else if (patternType === 'saddle') {
    let saddlePoly = [];
    let x0 = max(minX + 18, bbox.x + bbox.w * 0.28), x1 = bbox.x + bbox.w * 0.86;
    let yTop = bbox.y - 12, yBot = bbox.y + bbox.h * 0.55;
    for (let i = 0; i <= 20; i++) {
      let t = i / 20;
      let x = lerp(x0, x1, t);
      let y = yBot + sin(t * PI) * bbox.h * 0.14 + (noise(x * 0.04, 1) - 0.5) * 8;
      saddlePoly.push([x, y]);
    }
    saddlePoly.push([x1, yTop], [x0, yTop]);
    let rawHatch = engraving_hatch(poly, 2.5, 0, 0, -0.58, 0.1);
    let crossHatch = engraving_hatch(poly, 3.8, 0, 0, 0.35, 0.08);
    lines.push(
      ...clip_multi(rawHatch, saddlePoly).true,
      ...clip_multi(crossHatch, trsl_poly(saddlePoly, 0, -bbox.h * 0.14)).true
    );
  } else if (patternType === 'countershade') {
    let mantlePoly = [];
    let x0 = bbox.x - 15, x1 = bbox.x + bbox.w + 15;
    let yTop = bbox.y - 20;
    for (let i = 0; i <= 24; i++) {
      let t = i / 24;
      let x = lerp(x0, x1, t);
      let y = bbox.y + bbox.h * (0.46 + 0.10 * cos(t * PI * 2.2) + (noise(x * 0.03, 2) - 0.5) * 0.08);
      mantlePoly.push([x, y]);
    }
    mantlePoly.push([x1, yTop], [x0, yTop]);
    let rawHatch = engraving_hatch(poly, 2.8, 0, 0, -0.52, 0.1);
    lines.push(...clip_multi(rawHatch, mantlePoly).true);
  }
  return lines;
}

// ============================================================================
// ANATOMICAL FORE & HIND LIMB SCULPTORS (With Articulated Toes, Pads & Tendons)
// ============================================================================

function build_forelimb(shX, shY, groundY, legW, pawSize, feathery = 0, stepOffset = 0) {
  // Side-view shoulder layback: elbow sits behind the scapula, pastern short.
  let reach = max(22, groundY - shY);
  let elbX = shX + max(6, reach * 0.2) + stepOffset * 0.28;
  let elbY = shY + reach * 0.47;
  let wristX = shX + reach * 0.01 + stepOffset * 0.72;
  let wristY = groundY - max(6.5, min(12, reach * 0.13));
  let pawTipX = wristX - pawSize * 1.02;

  let frontCtrl = [
    [shX - legW * 1.35, shY - 4],
    [elbX - legW * 1.05, lerp(shY, elbY, 0.55)],
    [elbX - legW * 0.85, elbY],
    [wristX - legW * 0.62, lerp(elbY, wristY, 0.55)],
    [wristX - legW * 0.58, wristY - 1],
    [pawTipX + 2.2, groundY - 5.2],
    [pawTipX, groundY - 1.5],
    [pawTipX + 2.5, groundY]
  ];

  let fFeather = feathery * 6.5;
  let backCtrl = [
    [pawTipX + pawSize * 0.65, groundY],
    [wristX + legW * 0.62, groundY - 0.5],
    [wristX + legW * 0.72, wristY - 2],
    [wristX + legW * 0.55 + fFeather * 0.6, lerp(wristY, elbY, 0.35)],
    [elbX + legW * 0.82 + fFeather, lerp(wristY, elbY, 0.75)],
    [elbX + legW * 1.18, elbY + 2],
    [elbX + legW * 1.25, elbY - 4],
    [shX + legW * 1.45, shY + 2]
  ];

  let poly = [
    ...catmull_rom(frontCtrl, 6, false),
    ...catmull_rom(backCtrl, 6, false)
  ];
  poly = organic_ink(resample(poly, 2.2), 0.45, 0.05, 3);

  let details = [];
  for (let t = 1; t <= 2; t++) {
    let tx = pawTipX + t * (pawSize * 0.28);
    let ty = groundY - 0.3;
    details.push(catmull_rom([
      [tx, ty],
      [tx + 1.2, ty - 3.8],
      [tx + 3.2, ty - 5.2]
    ], 4, false));
  }
  details.push([[pawTipX, groundY - 1.2], [pawTipX - 1.8, groundY - 0.2]]);

  details.push(catmull_rom([
    [elbX + legW * 0.35, elbY - 6],
    [elbX + legW * 0.55, elbY + 2],
    [wristX + legW * 0.2, lerp(elbY, wristY, 0.65)]
  ], 5, false));

  if (feathery > 0) {
    for (let i = 0; i < 7; i++) {
      let ft = (i + 0.5) / 7;
      let fy = lerp(elbY + 2, wristY - 2, ft);
      let fx = lerp(elbX + legW * 0.9, wristX + legW * 0.55, ft);
      let flen = (6 + feathery * 7) * sin(ft * PI);
      details.push(catmull_rom([
        [fx - 2, fy - 2],
        [fx + flen * 0.6, fy + 2],
        [fx + flen, fy + 6]
      ], 4, false));
    }
  }

  return {
    poly,
    details,
    joints: [[shX, shY], [elbX, elbY], [wristX, wristY], [pawTipX + 3, groundY - 2]]
  };
}

function build_hindlimb(hipX, hipY, groundY, legW, hockSweep, pawSize, feathery = 0, stepOffset = 0) {
  // Stifle forward, hock back — the profile angulation Muybridge plates show at the stand.
  let reach = max(22, groundY - hipY);
  let stifleX = hipX - max(9, reach * 0.18) - hockSweep * 0.28 + stepOffset * 0.32;
  let stifleY = hipY + reach * 0.44;
  let hockX = hipX + hockSweep * 0.92 + stepOffset * 0.55;
  let hockY = hipY + reach * 0.7;
  let ankleX = hockX - max(3, reach * 0.05) + stepOffset * 0.15;
  let ankleY = groundY - max(6, min(11, reach * 0.12));
  let pawTipX = ankleX - pawSize * 0.98;

  let frontCtrl = [
    [hipX - legW * 1.85, hipY + 2],
    [stifleX - legW * 1.25, stifleY - 4],
    [stifleX - legW * 0.95, stifleY + 5],
    [lerp(stifleX, hockX, 0.65) - legW * 0.62, lerp(stifleY, hockY, 0.65)],
    [ankleX - legW * 0.58, ankleY - 1],
    [pawTipX + 2.0, groundY - 5.0],
    [pawTipX, groundY - 1.5],
    [pawTipX + 2.5, groundY]
  ];

  let bFeather = feathery * 7.5;
  let backCtrl = [
    [pawTipX + pawSize * 0.65, groundY],
    [ankleX + legW * 0.58, groundY - 0.5],
    [ankleX + legW * 0.55, ankleY],
    [hockX + legW * 0.72, hockY + 2],
    [hockX + legW * 0.85, hockY - 2.5],
    [lerp(hockX, hipX, 0.45) + legW * 0.85 + bFeather * 0.6, lerp(hockY, hipY, 0.42)],
    [hipX + legW * 1.55 + bFeather, lerp(hockY, hipY, 0.75)],
    [hipX + legW * 1.45, hipY - 2]
  ];

  let poly = [
    ...catmull_rom(frontCtrl, 6, false),
    ...catmull_rom(backCtrl, 6, false)
  ];
  poly = organic_ink(resample(poly, 2.2), 0.45, 0.05, 5);

  let details = [];
  for (let t = 1; t <= 2; t++) {
    let tx = pawTipX + t * (pawSize * 0.27);
    let ty = groundY - 0.3;
    details.push(catmull_rom([
      [tx, ty],
      [tx + 1.1, ty - 3.6],
      [tx + 3.0, ty - 4.8]
    ], 4, false));
  }
  details.push([[pawTipX, groundY - 1.2], [pawTipX - 1.7, groundY - 0.2]]);

  details.push(catmull_rom([
    [hipX - legW * 0.15, hipY + 10],
    [stifleX + legW * 0.25, stifleY + 2],
    [hockX + legW * 0.15, hockY - 2]
  ], 6, false));

  if (feathery > 0) {
    for (let i = 0; i < 7; i++) {
      let ft = (i + 0.5) / 7;
      let fy = lerp(hipY + 12, hockY - 2, ft);
      let fx = lerp(hipX + legW * 1.4, hockX + legW * 0.7, ft);
      let flen = (7 + feathery * 8) * sin(ft * PI);
      details.push(catmull_rom([
        [fx - 2, fy - 2],
        [fx + flen * 0.6, fy + 2],
        [fx + flen, fy + 6]
      ], 4, false));
    }
  }

  return {
    poly,
    details,
    joints: [[hipX, hipY], [stifleX, stifleY], [hockX, hockY], [ankleX, ankleY], [pawTipX + 2.4, groundY - 1.6]]
  };
}

// Poodle rosette puff sculptor
function build_pom(cx, cy, rx, ry, rot = 0) {
  let poly = [];
  let n = 28;
  for (let i = 0; i < n; i++) {
    let a = (i / n) * PI * 2;
    let lobe = 1 + 0.12 * sin(a * 11) + (noise(cos(a) * 2.5, sin(a) * 2.5, cx * 0.1) - 0.5) * 0.16;
    poly.push([cx + cos(a) * rx * lobe, cy + sin(a) * ry * lobe]);
  }
  poly = rot_poly(poly, rot, cx, cy);
  poly.push(poly[0].slice());
  let curls = coat_flow_strokes(poly, null, 1.45, 8.0, 0.3, 1.0);
  let shade = engraving_hatch(poly, 3.0, -5, -7, 0.65, 0.15);
  return { poly, strokes: [poly, ...curls, ...shade] };
}

// Grounded cast shadow & turf hatching under paws
function build_ground_shadow(xMin, xMax, groundY) {
  let strokes = [];
  let cx = (xMin + xMax) * 0.5;
  let rx = (xMax - xMin) * 0.56;
  let ry = 5.2;
  let shadowPoly = [];
  for (let i = 0; i < 24; i++) {
    let a = (i / 24) * PI * 2;
    shadowPoly.push([cx + cos(a) * rx, groundY + 1.2 + sin(a) * ry]);
  }
  shadowPoly.push(shadowPoly[0].slice());
  let h = engraving_hatch(shadowPoly, 2.3, 0, 0, 0.04, 0.02);
  strokes.push(...h);
  return strokes;
}

// --- HERSHEY VECTOR STROKE FONT ---
const SIMPLEX_CHARS = {
  'A': [[[0,21],[7,0],[14,21]],[[3,14],[11,14]]],
  'B': [[[0,0],[0,21],[9,21],[12,19],[13,17],[13,14],[12,12],[9,10],[0,10]],[[9,10],[12,8],[13,6],[13,3],[12,1],[9,0],[0,0]]],
  'C': [[[14,5],[12,2],[10,0],[6,0],[3,2],[1,5],[0,8],[0,13],[1,16],[3,19],[6,21],[10,21],[12,19],[14,16]]],
  'D': [[[0,0],[0,21],[7,21],[10,19],[12,16],[13,13],[13,8],[12,5],[10,2],[7,0],[0,0]]],
  'E': [[[12,0],[0,0],[0,21],[12,21]],[[0,10],[8,10]]],
  'F': [[[12,0],[0,0],[0,21]],[[0,10],[8,10]]],
  'G': [[[14,5],[12,2],[10,0],[6,0],[3,2],[1,5],[0,8],[0,13],[1,16],[3,19],[6,21],[10,21],[12,19],[14,16],[14,12],[9,12]]],
  'H': [[[0,0],[0,21]],[[14,0],[14,21]],[[0,10],[14,10]]],
  'I': [[[0,0],[6,0]],[[3,0],[3,21]],[[0,21],[6,21]]],
  'J': [[[10,0],[10,16],[9,19],[7,21],[4,21],[2,19],[1,16],[1,14]]],
  'K': [[[0,0],[0,21]],[[13,0],[0,13]],[[5,8],[13,21]]],
  'L': [[[0,0],[0,21],[12,21]]],
  'M': [[[0,21],[0,0],[8,18],[16,0],[16,21]]],
  'N': [[[0,21],[0,0],[14,21],[14,0]]],
  'O': [[[6,0],[3,2],[1,5],[0,8],[0,13],[1,16],[3,19],[6,21],[10,21],[13,19],[15,16],[16,13],[16,8],[15,5],[13,2],[10,0],[6,0]]],
  'P': [[[0,21],[0,0],[9,0],[12,2],[13,4],[13,7],[12,9],[9,11],[0,11]]],
  'Q': [[[6,0],[3,2],[1,5],[0,8],[0,13],[1,16],[3,19],[6,21],[10,21],[13,19],[15,16],[16,13],[16,8],[15,5],[13,2],[10,0],[6,0]],[[9,15],[15,22]]],
  'R': [[[0,21],[0,0],[9,0],[12,2],[13,4],[13,7],[12,9],[9,11],[0,11]],[[7,11],[13,21]]],
  'S': [[[13,3],[11,1],[8,0],[4,0],[1,1],[0,3],[0,5],[1,7],[3,8],[10,11],[12,12],[13,15],[13,18],[11,20],[8,21],[4,21],[1,20],[0,18]]],
  'T': [[[0,0],[14,0]],[[7,0],[7,21]]],
  'U': [[[0,0],[0,15],[1,18],[3,20],[6,21],[9,21],[12,20],[14,18],[15,15],[15,0]]],
  'V': [[[0,0],[7,21],[14,0]]],
  'W': [[[0,0],[4,21],[9,5],[14,21],[18,0]]],
  'X': [[[0,0],[14,21]],[[14,0],[0,21]]],
  'Y': [[[0,0],[7,10],[7,21]],[[14,0],[7,10]]],
  'Z': [[[0,0],[14,0],[0,21],[14,21]]],
  '-': [[[1,10],[10,10]]],
  '.': [[[2,19],[3,20],[2,21],[1,20],[2,19]]],
  '(': [[[5,0],[2,4],[1,9],[1,13],[2,18],[5,22]]],
  ')': [[[1,0],[4,4],[5,9],[5,13],[4,18],[1,22]]],
  '0': [[[6,0],[2,3],[0,8],[0,13],[2,18],[6,21],[10,21],[14,18],[16,13],[16,8],[14,3],[10,0],[6,0]]],
  '1': [[[3,4],[8,0],[8,21]],[[3,21],[13,21]]],
  '2': [[[1,5],[3,1],[7,0],[11,0],[14,2],[15,5],[14,9],[1,21],[15,21]]],
  '3': [[[2,0],[14,0],[7,8],[11,8],[14,10],[15,14],[13,19],[9,21],[4,21],[1,18]]],
  '4': [[[11,0],[0,14],[15,14]],[[11,0],[11,21]]],
  '5': [[[14,0],[2,0],[1,9],[6,7],[11,7],[14,10],[15,14],[13,19],[9,21],[4,21],[1,18]]],
  '6': [[[13,2],[9,0],[5,0],[2,4],[0,10],[0,16],[3,20],[7,21],[11,20],[14,16],[14,12],[11,9],[6,9],[2,12]]],
  '7': [[[0,0],[15,0],[6,21]]],
  '8': [[[5,0],[2,2],[1,5],[2,8],[6,10],[10,10],[13,8],[14,5],[13,2],[10,0],[5,0]],[[6,10],[2,12],[0,15],[2,19],[5,21],[10,21],[13,19],[15,15],[13,12],[10,10]]],
  '9': [[[14,9],[10,12],[5,12],[2,9],[2,5],[5,1],[9,0],[13,2],[15,7],[15,13],[13,18],[9,21],[4,21]]],
  ' ': []
};

function stroke_text(str, cx, y, scale = 0.42) {
  let s = str.toUpperCase();
  let polylines = [];
  let cursor = 0;
  for (let i = 0; i < s.length; i++) {
    let ch = s[i];
    let paths = SIMPLEX_CHARS[ch] || SIMPLEX_CHARS[' '];
    let maxX = 6;
    for (let p of paths) {
      let sp = p.map(([px, py]) => [cursor + px - py * 0.12, py]);
      for (let [px] of p) if (px > maxX) maxX = px;
      polylines.push(sp);
    }
    cursor += (ch === ' ' ? 9 : maxX + 5);
  }
  let totalW = cursor * scale;
  let ox = cx - totalW * 0.5;
  return polylines.map(p => p.map(([px, py]) => [ox + px * scale, y + py * scale]));
}

// ============================================================================
// BREED MORPHOLOGICAL DNA — side profiles tuned to AKC/FCI ratios
// (square Jack Russell, GSD 10:8.5, long-and-low hounds, foxy corgi).
// ============================================================================
const BREED_CATALOG = [
  {
    id: 'jack_russell',
    name: 'Jack Russell Terrier',
    trName: 'Jack Russell Terrier',
    group: 'Terrier',
    body_len: 98, chest_depth: 46, waist_tuck: 12, croup_drop: 2, back_arch: 1,
    neck_len: 32, neck_ang: -0.92, neck_w: 20,
    skull_r: 17.5, skull_dome: 0.88, stop_depth: 6.2, muzzle_len: 23, muzzle_h: 11.5, muzzle_taper: 0.62, jowl: 0.4, underbite: 0, Roman_nose: 0,
    ear_type: 'button', ear_len: 16, ear_w: 12,
    leg_len: 64, leg_w: 9.2, hock_sweep: 10, paw_size: 8.6,
    tail_type: 'saber_up', tail_len: 42, tail_curl: 0.22,
    coat_type: 'smooth', fur_len: 6, pattern: 'patches', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'greyhound',
    name: 'Greyhound',
    trName: 'İngiliz Tazısı (Greyhound)',
    group: 'Sighthound',
    body_len: 150, chest_depth: 62, waist_tuck: 40, croup_drop: 10, back_arch: 14,
    neck_len: 58, neck_ang: -0.9, neck_w: 20,
    skull_r: 17, skull_dome: 0.84, stop_depth: 2.5, muzzle_len: 37, muzzle_h: 10.5, muzzle_taper: 0.44, jowl: 0, underbite: 0, Roman_nose: 0.12,
    ear_type: 'rose', ear_len: 18, ear_w: 8,
    leg_len: 102, leg_w: 8.6, hock_sweep: 22, paw_size: 9.4,
    tail_type: 'whip', tail_len: 84, tail_curl: 0.22,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'dachshund',
    name: 'Dachshund',
    trName: 'Sosis Köpek (Dachshund)',
    group: 'Hound',
    body_len: 152, chest_depth: 54, waist_tuck: 8, croup_drop: 3, back_arch: 0,
    neck_len: 32, neck_ang: -0.8, neck_w: 24,
    skull_r: 19, skull_dome: 0.94, stop_depth: 5.5, muzzle_len: 34, muzzle_h: 12.5, muzzle_taper: 0.52, jowl: 1, underbite: 0, Roman_nose: 0,
    ear_type: 'drop_long', ear_len: 39, ear_w: 16.5,
    leg_len: 34, leg_w: 11.4, hock_sweep: 7, paw_size: 10.2,
    tail_type: 'saber', tail_len: 56, tail_curl: 0.25,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'pug',
    name: 'Pug',
    trName: 'Pug',
    group: 'Toy / Brachycephalic',
    body_len: 104, chest_depth: 53, waist_tuck: 6, croup_drop: 3, back_arch: 0,
    neck_len: 23, neck_ang: -0.96, neck_w: 31,
    skull_r: 23.5, skull_dome: 1.16, stop_depth: 14, muzzle_len: 9.5, muzzle_h: 18, muzzle_taper: 0.95, jowl: 4.5, underbite: 2.5, Roman_nose: 0,
    ear_type: 'button', ear_len: 21, ear_w: 11.5,
    leg_len: 55, leg_w: 11, hock_sweep: 6, paw_size: 9.5,
    tail_type: 'curl', tail_len: 38, tail_curl: 1.45,
    coat_type: 'smooth', fur_len: 5, pattern: 'mask', beard: 0, wrinkles: 5, collar: 1
  },
  {
    id: 'poodle',
    name: 'Standard Poodle',
    trName: 'Kaniş (Standard Poodle)',
    group: 'Non-Sporting',
    body_len: 132, chest_depth: 52, waist_tuck: 22, croup_drop: 3, back_arch: 2,
    neck_len: 50, neck_ang: -1.08, neck_w: 21,
    skull_r: 19, skull_dome: 1.08, stop_depth: 6, muzzle_len: 32, muzzle_h: 11.5, muzzle_taper: 0.58, jowl: 0, underbite: 0, Roman_nose: 0,
    ear_type: 'drop_long', ear_len: 41, ear_w: 16.5,
    leg_len: 88, leg_w: 9.8, hock_sweep: 11, paw_size: 10,
    tail_type: 'pom', tail_len: 48, tail_curl: 0.12,
    coat_type: 'poodle', fur_len: 9, pattern: 'solid', beard: 0, wrinkles: 0, collar: 0
  },
  {
    id: 'german_shepherd',
    name: 'German Shepherd',
    trName: 'Alman Çoban Köpeği',
    group: 'Herding',
    body_len: 132, chest_depth: 58, waist_tuck: 16, croup_drop: 16, back_arch: 2,
    neck_len: 46, neck_ang: -0.72, neck_w: 30,
    skull_r: 22, skull_dome: 0.95, stop_depth: 7, muzzle_len: 34, muzzle_h: 15.5, muzzle_taper: 0.65, jowl: 1, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 32, ear_w: 14,
    leg_len: 86, leg_w: 12.4, hock_sweep: 26, paw_size: 12,
    tail_type: 'plume_low', tail_len: 74, tail_curl: 0.42,
    coat_type: 'double', fur_len: 10.5, pattern: 'saddle', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'afghan_hound',
    name: 'Afghan Hound',
    trName: 'Afgan Tazısı',
    group: 'Sighthound',
    body_len: 152, chest_depth: 56, waist_tuck: 26, croup_drop: 8, back_arch: 4,
    neck_len: 55, neck_ang: -0.98, neck_w: 22,
    skull_r: 18, skull_dome: 0.95, stop_depth: 3, muzzle_len: 38, muzzle_h: 11, muzzle_taper: 0.48, jowl: 0, underbite: 0, Roman_nose: 0.15,
    ear_type: 'drop_long', ear_len: 49, ear_w: 17,
    leg_len: 92, leg_w: 14.5, hock_sweep: 14, paw_size: 12.5,
    tail_type: 'ring', tail_len: 74, tail_curl: 0.88,
    coat_type: 'silky', fur_len: 16, pattern: 'solid', beard: 0, wrinkles: 0, collar: 0
  },
  {
    id: 'english_bulldog',
    name: 'English Bulldog',
    trName: 'İngiliz Bulldog',
    group: 'Non-Sporting',
    body_len: 108, chest_depth: 68, waist_tuck: 12, croup_drop: -2, back_arch: 11,
    neck_len: 22, neck_ang: -0.58, neck_w: 40,
    skull_r: 27, skull_dome: 0.96, stop_depth: 16, muzzle_len: 10, muzzle_h: 22, muzzle_taper: 0.98, jowl: 10, underbite: 6.5, Roman_nose: 0,
    ear_type: 'rose', ear_len: 16, ear_w: 11,
    leg_len: 42, leg_w: 15.2, hock_sweep: 5, paw_size: 13,
    tail_type: 'bob', tail_len: 20, tail_curl: -0.3,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 6, collar: 1
  },
  {
    id: 'bull_terrier',
    name: 'Bull Terrier',
    trName: 'Bull Terrier',
    group: 'Terrier',
    body_len: 118, chest_depth: 56, waist_tuck: 14, croup_drop: 3, back_arch: 2,
    neck_len: 36, neck_ang: -0.82, neck_w: 31,
    skull_r: 22, skull_dome: 0.66, stop_depth: -5, muzzle_len: 34, muzzle_h: 18, muzzle_taper: 0.7, jowl: 0.5, underbite: 0, Roman_nose: 1.25,
    ear_type: 'prick', ear_len: 23, ear_w: 10,
    leg_len: 64, leg_w: 11.8, hock_sweep: 9, paw_size: 11,
    tail_type: 'saber', tail_len: 48, tail_curl: 0.05,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'corgi',
    name: 'Welsh Corgi',
    trName: 'Pembroke Welsh Corgi',
    group: 'Herding',
    body_len: 128, chest_depth: 48, waist_tuck: 8, croup_drop: 2, back_arch: 0,
    neck_len: 32, neck_ang: -0.86, neck_w: 28,
    skull_r: 19, skull_dome: 0.96, stop_depth: 6.5, muzzle_len: 24, muzzle_h: 12, muzzle_taper: 0.5, jowl: 0, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 24, ear_w: 13,
    leg_len: 44, leg_w: 12, hock_sweep: 8, paw_size: 10.5,
    tail_type: 'bob', tail_len: 16, tail_curl: 0.25,
    coat_type: 'double', fur_len: 9.5, pattern: 'countershade', beard: 0, wrinkles: 0, collar: 0
  },
  {
    id: 'dalmatian',
    name: 'Dalmatian',
    trName: 'Dalmaçyalı (Dalmatian)',
    group: 'Non-Sporting',
    body_len: 144, chest_depth: 54, waist_tuck: 20, croup_drop: 5, back_arch: 2,
    neck_len: 45, neck_ang: -0.90, neck_w: 24.5,
    skull_r: 20, skull_dome: 0.95, stop_depth: 7, muzzle_len: 30, muzzle_h: 14.5, muzzle_taper: 0.72, jowl: 2, underbite: 0, Roman_nose: 0,
    ear_type: 'button', ear_len: 26, ear_w: 13,
    leg_len: 82, leg_w: 10.8, hock_sweep: 12, paw_size: 11,
    tail_type: 'saber', tail_len: 68, tail_curl: 0.3,
    coat_type: 'smooth', fur_len: 5, pattern: 'spots', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'schnauzer',
    name: 'Schnauzer',
    trName: 'Schnauzer',
    group: 'Terrier / Working',
    body_len: 114, chest_depth: 52, waist_tuck: 15, croup_drop: 5, back_arch: 0,
    neck_len: 41, neck_ang: -0.96, neck_w: 27,
    skull_r: 20, skull_dome: 0.92, stop_depth: 7, muzzle_len: 29, muzzle_h: 17, muzzle_taper: 0.88, jowl: 2, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 22, ear_w: 10,
    leg_len: 72, leg_w: 13.2, hock_sweep: 10, paw_size: 11.5,
    tail_type: 'bob', tail_len: 25, tail_curl: 0.4,
    coat_type: 'wire', fur_len: 9.5, pattern: 'solid', beard: 1.05, wrinkles: 0, collar: 1
  },
  {
    id: 'husky',
    name: 'Siberian Husky',
    trName: 'Sibirya Kurdu (Husky)',
    group: 'Working',
    body_len: 136, chest_depth: 54, waist_tuck: 14, croup_drop: 4, back_arch: 1,
    neck_len: 41, neck_ang: -0.88, neck_w: 30,
    skull_r: 21, skull_dome: 0.98, stop_depth: 7, muzzle_len: 29, muzzle_h: 14, muzzle_taper: 0.58, jowl: 0, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 27, ear_w: 13,
    leg_len: 78, leg_w: 11.8, hock_sweep: 12, paw_size: 11.8,
    tail_type: 'plume', tail_len: 70, tail_curl: 1.05,
    coat_type: 'double', fur_len: 11, pattern: 'countershade', beard: 0, wrinkles: 0, collar: 0
  },
  {
    id: 'basset_hound',
    name: 'Basset Hound',
    trName: 'Basset Hound',
    group: 'Hound',
    body_len: 156, chest_depth: 58, waist_tuck: 6, croup_drop: 3, back_arch: 0,
    neck_len: 34, neck_ang: -0.7, neck_w: 33,
    skull_r: 23, skull_dome: 1.12, stop_depth: 7, muzzle_len: 34, muzzle_h: 18, muzzle_taper: 0.82, jowl: 8.5, underbite: 0, Roman_nose: 0,
    ear_type: 'drop_long', ear_len: 64, ear_w: 18,
    leg_len: 36, leg_w: 14.2, hock_sweep: 6, paw_size: 13,
    tail_type: 'saber_up', tail_len: 62, tail_curl: 0.55,
    coat_type: 'smooth', fur_len: 5, pattern: 'saddle', beard: 0, wrinkles: 4, collar: 1
  },
  {
    id: 'chihuahua',
    name: 'Chihuahua',
    trName: 'Chihuahua',
    group: 'Toy',
    body_len: 96, chest_depth: 38, waist_tuck: 12, croup_drop: 2, back_arch: 1,
    neck_len: 27, neck_ang: -0.96, neck_w: 16.5,
    skull_r: 22.5, skull_dome: 1.34, stop_depth: 13.5, muzzle_len: 14, muzzle_h: 9, muzzle_taper: 0.45, jowl: 0, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 31, ear_w: 15,
    leg_len: 54, leg_w: 7.4, hock_sweep: 7, paw_size: 7.5,
    tail_type: 'plume', tail_len: 44, tail_curl: 0.88,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'great_dane',
    name: 'Great Dane',
    trName: 'Danua (Great Dane)',
    group: 'Working',
    body_len: 162, chest_depth: 64, waist_tuck: 26, croup_drop: 5, back_arch: 2,
    neck_len: 59, neck_ang: -0.95, neck_w: 28.5,
    skull_r: 23, skull_dome: 0.95, stop_depth: 10, muzzle_len: 35, muzzle_h: 22, muzzle_taper: 0.92, jowl: 7.5, underbite: 0, Roman_nose: 0,
    ear_type: 'button', ear_len: 29, ear_w: 14,
    leg_len: 104, leg_w: 12.2, hock_sweep: 14, paw_size: 13,
    tail_type: 'saber', tail_len: 78, tail_curl: 0.22,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 1, collar: 1
  },
  {
    id: 'shiba_inu',
    name: 'Shiba Inu',
    trName: 'Shiba Inu',
    group: 'Spitz / Non-Sporting',
    body_len: 112, chest_depth: 50, waist_tuck: 13, croup_drop: 2, back_arch: 1,
    neck_len: 35, neck_ang: -0.92, neck_w: 29,
    skull_r: 21, skull_dome: 1.0, stop_depth: 7, muzzle_len: 23, muzzle_h: 13, muzzle_taper: 0.52, jowl: 0, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 22, ear_w: 12,
    leg_len: 68, leg_w: 10.2, hock_sweep: 9, paw_size: 10,
    tail_type: 'curl', tail_len: 52, tail_curl: 1.62,
    coat_type: 'double', fur_len: 9.5, pattern: 'countershade', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'golden_retriever',
    name: 'Golden Retriever',
    trName: 'Golden Retriever',
    group: 'Sporting',
    body_len: 136, chest_depth: 56, waist_tuck: 13, croup_drop: 3, back_arch: 1,
    neck_len: 43, neck_ang: -0.84, neck_w: 29,
    skull_r: 22, skull_dome: 1.0, stop_depth: 8, muzzle_len: 29, muzzle_h: 16, muzzle_taper: 0.78, jowl: 2, underbite: 0, Roman_nose: 0,
    ear_type: 'drop_long', ear_len: 33, ear_w: 15.5,
    leg_len: 74, leg_w: 11.8, hock_sweep: 12, paw_size: 11.8,
    tail_type: 'plume_low', tail_len: 74, tail_curl: 0.28,
    coat_type: 'feathered', fur_len: 12, pattern: 'solid', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'doberman',
    name: 'Doberman Pinscher',
    trName: 'Doberman',
    group: 'Working',
    body_len: 142, chest_depth: 60, waist_tuck: 26, croup_drop: 6, back_arch: 2,
    neck_len: 53, neck_ang: -1.02, neck_w: 25.5,
    skull_r: 19, skull_dome: 0.88, stop_depth: 4, muzzle_len: 34, muzzle_h: 14, muzzle_taper: 0.68, jowl: 0, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 29, ear_w: 10,
    leg_len: 90, leg_w: 10.8, hock_sweep: 13, paw_size: 11,
    tail_type: 'bob', tail_len: 22, tail_curl: 0.35,
    coat_type: 'smooth', fur_len: 5, pattern: 'solid', beard: 0, wrinkles: 0, collar: 1
  },
  {
    id: 'scottish_terrier',
    name: 'Scottish Terrier',
    trName: 'İskoç Teriyeri (Scotty)',
    group: 'Terrier',
    body_len: 136, chest_depth: 52, waist_tuck: 4, croup_drop: 2, back_arch: 0,
    neck_len: 31, neck_ang: -0.92, neck_w: 28,
    skull_r: 20, skull_dome: 0.90, stop_depth: 5, muzzle_len: 32, muzzle_h: 18, muzzle_taper: 0.92, jowl: 2, underbite: 0, Roman_nose: 0,
    ear_type: 'prick', ear_len: 23, ear_w: 10,
    leg_len: 40, leg_w: 12.8, hock_sweep: 6, paw_size: 11,
    tail_type: 'saber_up', tail_len: 38, tail_curl: 0.25,
    coat_type: 'skirted', fur_len: 13, pattern: 'solid', beard: 1.35, wrinkles: 0, collar: 1
  },
  {
    id: 'borzoi',
    name: 'Borzoi',
    trName: 'Rus Kurt Tazısı (Borzoi)',
    group: 'Sighthound',
    body_len: 164, chest_depth: 60, waist_tuck: 35, croup_drop: 12, back_arch: 15,
    neck_len: 53, neck_ang: -0.84, neck_w: 22.5,
    skull_r: 17, skull_dome: 0.82, stop_depth: 1, muzzle_len: 43, muzzle_h: 10.5, muzzle_taper: 0.38, jowl: 0, underbite: 0, Roman_nose: 0.58,
    ear_type: 'rose', ear_len: 21, ear_w: 9,
    leg_len: 96, leg_w: 10.8, hock_sweep: 17, paw_size: 11,
    tail_type: 'plume_low', tail_len: 86, tail_curl: 0.32,
    coat_type: 'feathered', fur_len: 13, pattern: 'solid', beard: 0, wrinkles: 0, collar: 0
  }
];

// ============================================================================
// CORE PARAMETRIC CANINE GENERATOR (v2.1 Polished Edition)
// ============================================================================
function generate_dog(dnaInput, seedName = null) {
  let dna = Object.assign({}, dnaInput);
  seed_rand(seedName || dna.name || dna.id);

  let cx0 = 210; // Withers / scapula top X
  let cy0 = 135; // Withers / scapula top Y

  // 1. UNIFIED HEAD + NECK + TORSO + PELVIS ANATOMICAL SPLINE
  let withers = [cx0, cy0];
  let backDip = [cx0 + dna.body_len * 0.24, cy0 + 2.5 - dna.back_arch * 0.35];
  let backMid = [cx0 + dna.body_len * 0.52, cy0 + 1.5 - dna.back_arch];
  let loin = [cx0 + dna.body_len * 0.78, cy0 - dna.back_arch * 0.55 + dna.croup_drop * 0.28];
  let croup = [cx0 + dna.body_len, cy0 + dna.croup_drop];

  // Neck & Cranial anchor (extending forward -X and upward -Y from withers)
  let headX = cx0 - cos(dna.neck_ang) * dna.neck_len;
  let headY = cy0 + sin(dna.neck_ang) * dna.neck_len;

  // Cranial & Muzzle landmarks (Facing Left -X)
  let occiput = [headX + dna.skull_r * 0.52, headY - dna.skull_r * 0.72];
  let craniumTop = [headX - dna.skull_r * 0.14, headY - dna.skull_r * dna.skull_dome];
  let stopBridge = [headX - dna.skull_r * 0.74, headY - dna.skull_r * 0.58];
  let stopPt = [headX - dna.skull_r * 0.94, headY - dna.skull_r * 0.34 + dna.stop_depth * 0.55];

  let noseX = stopPt[0] - dna.muzzle_len;
  let noseY = stopPt[1] + (dna.Roman_nose > 0 ? 4.8 * dna.Roman_nose : 1.2);
  let bridgeMid = [
    lerp(stopPt[0], noseX, 0.52),
    lerp(stopPt[1], noseY, 0.52) - dna.Roman_nose * 5.2
  ];
  let noseTip = [noseX - dna.underbite * 0.4, noseY];
  let noseBase = [noseX - dna.underbite * 0.6 + 1.5, noseY + max(4.5, dna.muzzle_h * 0.38)];
  let upperLipFront = [noseX - dna.underbite, noseY + dna.muzzle_h * dna.muzzle_taper];
  let flews = [
    lerp(noseX, stopPt[0], 0.66),
    noseY + dna.muzzle_h + dna.jowl
  ];
  let lowerJaw = [
    lerp(noseX, stopPt[0], 0.42),
    noseY + dna.muzzle_h * 0.92 + dna.jowl * 0.25
  ];
  let throat = [headX - dna.skull_r * 0.12, headY + dna.skull_r * 0.78 + dna.jowl * 0.35];

  // Neck crest & ventral chest / brisket / flank landmarks
  let crestMid = [
    lerp(occiput[0], withers[0], 0.52) + 2,
    lerp(occiput[1], withers[1], 0.52) - 3
  ];
  let prosternum = [cx0 - 14, cy0 + dna.chest_depth * 0.48]; // Prominent forechest point
  let throatMid = [
    lerp(throat[0], prosternum[0], 0.52) - 2,
    lerp(throat[1], prosternum[1], 0.5)
  ];
  let brisket = [cx0 + dna.body_len * 0.22, cy0 + dna.chest_depth]; // Deepest chest point
  let ribsEnd = [cx0 + dna.body_len * 0.50, cy0 + dna.chest_depth * 0.90];
  let flank = [cx0 + dna.body_len * 0.78, cy0 + dna.chest_depth - dna.waist_tuck];
  let pelvisRear = [cx0 + dna.body_len * 0.98, cy0 + dna.croup_drop + max(18, (dna.chest_depth - dna.waist_tuck) * 0.66)];

  // Single seamless master polygon for Head + Neck + Torso
  let bodyMasterCtrl = [
    occiput,
    craniumTop,
    stopBridge,
    stopPt,
    bridgeMid,
    noseTip,
    noseBase,
    upperLipFront,
    lowerJaw,
    flews,
    throat,
    throatMid,
    prosternum,
    brisket,
    ribsEnd,
    flank,
    pelvisRear,
    croup,
    loin,
    backMid,
    backDip,
    withers,
    crestMid
  ];

  if (dna.coat_type === 'skirted') {
    bodyMasterCtrl = [
      occiput, craniumTop, stopBridge, stopPt, bridgeMid, noseTip, noseBase, upperLipFront, lowerJaw, flews,
      throat, throatMid, prosternum,
      [cx0 - 12, cy0 + dna.chest_depth + 16],
      [cx0 + dna.body_len * 0.48, cy0 + dna.chest_depth + 20],
      [croup[0] + 4, cy0 + dna.chest_depth + 15],
      pelvisRear, croup, loin, backMid, backDip, withers, crestMid
    ];
  }

  let bodyPoly = organic_ink(resample(catmull_rom(bodyMasterCtrl, 8, true), 2.2), 0.55, 0.04, 11);
  let spineCurve = catmull_rom([occiput, crestMid, withers, backMid, loin, croup], 8, false);

  // 2. EXPRESSIVE FACIAL ANATOMY (Eye, Iris, Specular Cutout, Nostril, Mouth, Whiskers)
  let headDetails = [];
  let eyeX = stopPt[0] + dna.skull_r * 0.38;
  let eyeY = stopPt[1] + 2.8;
  let eyeR = max(2.8, min(4.6, dna.skull_r * 0.19));

  let eyeCtrl = [
    [eyeX - eyeR * 1.25, eyeY + 0.3],
    [eyeX - eyeR * 0.2, eyeY - eyeR * 0.92],
    [eyeX + eyeR * 1.22, eyeY - 0.15],
    [eyeX + eyeR * 0.1, eyeY + eyeR * 0.85]
  ];
  let eyePoly = catmull_rom(eyeCtrl, 6, true);

  let specX = eyeX - eyeR * 0.28, specY = eyeY - eyeR * 0.25, specR = eyeR * 0.32;
  let specPoly = [];
  for (let i = 0; i < 10; i++) {
    let a = (i / 10) * PI * 2;
    specPoly.push([specX + cos(a) * specR, specY + sin(a) * specR]);
  }

  let irisLines = [];
  for (let r = eyeR * 0.18; r <= eyeR * 0.78; r += 0.85) {
    let ring = [];
    for (let i = 0; i <= 14; i++) {
      let a = (i / 14) * PI * 2;
      ring.push([eyeX + cos(a) * r * 0.95, eyeY + sin(a) * r * 0.82]);
    }
    irisLines.push(...clip(ring, specPoly).false);
  }
  let upperLid = catmull_rom([
    [eyeX - eyeR * 1.45, eyeY + 0.5],
    [eyeX - eyeR * 0.1, eyeY - eyeR * 1.28],
    [eyeX + eyeR * 1.48, eyeY + 0.1]
  ], 5, false);
  let lowerLid = catmull_rom([
    [eyeX - eyeR * 1.1, eyeY + 1.1],
    [eyeX + eyeR * 0.1, eyeY + eyeR * 1.18],
    [eyeX + eyeR * 1.15, eyeY + 0.5]
  ], 4, false);

  let noseW = max(5.2, dna.muzzle_h * 0.40);
  let noseH = max(4.5, dna.muzzle_h * 0.36);
  let nosePoly = catmull_rom([
    [noseTip[0] - 0.5, noseTip[1] - 0.5],
    [noseTip[0] + noseW, noseTip[1] - 0.8],
    [noseTip[0] + noseW * 0.72, noseTip[1] + noseH],
    [noseTip[0] + 0.2, noseTip[1] + noseH * 0.68]
  ], 5, true);
  let nostrilPoly = catmull_rom([
    [noseTip[0] + noseW * 0.28, noseTip[1] + noseH * 0.28],
    [noseTip[0] + noseW * 0.58, noseTip[1] + noseH * 0.22],
    [noseTip[0] + noseW * 0.45, noseTip[1] + noseH * 0.62]
  ], 4, true);
  let noseShade = engraving_hatch(nosePoly, 1.15, 0, 1.2, 0.4, 0.05);

  let mouthCleft = catmull_rom([
    [upperLipFront[0] + 0.8, upperLipFront[1] - 0.6],
    [lerp(noseX, stopPt[0], 0.38), noseY + dna.muzzle_h * 0.74],
    [flews[0], flews[1] - dna.jowl * 0.18],
    [flews[0] + 4.5, flews[1] - dna.jowl * 0.45 - 1.5]
  ], 6, false);
  let cheekArch = catmull_rom([
    [stopPt[0] + 2, stopPt[1] + dna.muzzle_h * 0.55],
    [headX - dna.skull_r * 0.15, headY + dna.skull_r * 0.38],
    [headX + dna.skull_r * 0.25, headY + dna.skull_r * 0.12]
  ], 5, false);

  let whiskerDots = [];
  for (let r = 0; r < 2; r++) {
    for (let c = 0; c < 3; c++) {
      let wx = lerp(noseX + 4, stopPt[0] - 5, (c + 0.5) / 3.5);
      let wy = noseY + dna.muzzle_h * (0.48 + r * 0.18);
      if (pt_in_poly(wx, wy, bodyPoly)) {
        whiskerDots.push([[wx - 0.5, wy], [wx + 0.5, wy]]);
      }
    }
  }

  headDetails.push(
    eyePoly, upperLid, lowerLid, ...irisLines,
    nosePoly, nostrilPoly, ...noseShade,
    mouthCleft, cheekArch, ...whiskerDots
  );

  if (dna.wrinkles > 0) {
    for (let w = 0; w < dna.wrinkles; w++) {
      let wx = stopPt[0] - 1.5 + w * 3.0;
      let wy0 = stopPt[1] - 2 + w * 1.1;
      let wy1 = flews[1] - 2.5;
      let fold = catmull_rom([
        [wx + 2.2, wy0],
        [wx - 2.2, lerp(wy0, wy1, 0.5)],
        [wx + 1.8, wy1]
      ], 5, false);
      headDetails.push(...clip(fold, bodyPoly).true);
    }
  }

  let beardPolys = [];
  let beardStrokes = [];
  if (dna.beard > 0) {
    let bLen = 18 * dna.beard;
    let beardCtrl = [
      [noseTip[0] + 2, noseTip[1] + 3.5],
      [noseTip[0] - 2.5, flews[1] + bLen * 0.72],
      [flews[0] - 1, flews[1] + bLen],
      [throat[0] - 2, throat[1] + bLen * 0.32],
      [stopPt[0] - 2, stopPt[1] + 5]
    ];
    let bPoly = organic_ink(catmull_rom(beardCtrl, 6, true), 0.9, 0.07, 33);
    beardPolys.push(bPoly);
    beardStrokes.push(bPoly, ...silky_locks(bPoly, 2.1, 4, 2.2, -0.08));
    for (let b = 0; b < 3; b++) {
      headDetails.push(catmull_rom([
        [eyeX + eyeR * 1.5, eyeY - eyeR * (0.9 + b * 0.35)],
        [eyeX - eyeR * 0.6, eyeY - eyeR * (2.1 + b * 0.4)],
        [eyeX - eyeR * 2.5, eyeY - eyeR * (1.3 + b * 0.25)]
      ], 5, false));
    }
  }

  if (dna.pattern === 'mask') {
    let maskPoly = [
      [stopPt[0] + 7, stopPt[1] - 6],
      [noseTip[0] - 5, noseTip[1] - 5],
      [upperLipFront[0] - 5, flews[1] + 6],
      [flews[0] + 6, flews[1] + 5],
      [eyeX + 5, eyeY + 5]
    ];
    let mHatch = engraving_hatch(bodyPoly, 1.9, 0, 0, -0.42, 0.08);
    let mClipped = clip_multi(mHatch, maskPoly).true;
    mClipped = clip_multi(mClipped, specPoly).false;
    headDetails.push(...mClipped);
  }

  let collarPolys = [];
  let collarStrokes = [];
  if (dna.collar && dna.coat_type !== 'silky' && dna.coat_type !== 'poodle') {
    let cTop = lerp2d(occiput, withers, 0.58);
    let cBot = lerp2d(throat, prosternum, 0.52);
    let cAng = atan2(cBot[1] - cTop[1], cBot[0] - cTop[0]);
    let nx = cos(cAng + PI * 0.5) * 4.2, ny = sin(cAng + PI * 0.5) * 4.2;
    let cPoly = catmull_rom([
      [cTop[0] - nx, cTop[1] - ny],
      [lerp(cTop[0], cBot[0], 0.5) - nx * 1.15, lerp(cTop[1], cBot[1], 0.5) - ny * 1.15],
      [cBot[0] - nx, cBot[1] - ny],
      [cBot[0] + nx, cBot[1] + ny],
      [lerp(cTop[0], cBot[0], 0.5) + nx * 0.85, lerp(cTop[1], cBot[1], 0.5) + ny * 0.85],
      [cTop[0] + nx, cTop[1] + ny]
    ], 5, true);
    let cClipped = clip(cPoly, bodyPoly).true;
    if (cClipped.length) {
      collarPolys.push(cPoly);
      let cHatch = clip_multi(engraving_hatch(cPoly, 1.6, 0, 0, cAng + 0.3, 0.05), bodyPoly).true;
      let ringR = 3.0, rx = cBot[0] - 2.8, ry = cBot[1] + 2.2;
      let ring = [];
      for (let i = 0; i <= 12; i++) {
        let a = (i / 12) * PI * 2;
        ring.push([rx + cos(a) * ringR, ry + sin(a) * ringR]);
      }
      collarStrokes.push(...cClipped, ...cHatch, ring);
    }
  }

  // 3. EARS (Prick, Drop Long, Button, Rose)
  let earBaseX = headX + dna.skull_r * 0.16;
  let earBaseY = headY - dna.skull_r * 0.64;
  let earPoly = [];
  let earStrokes = [];

  if (dna.ear_type === 'prick') {
    let tipX = earBaseX - dna.ear_w * 0.22;
    let tipY = earBaseY - dna.ear_len;
    earPoly = catmull_rom([
      [earBaseX - dna.ear_w * 0.55, earBaseY + 3],
      [lerp(earBaseX - dna.ear_w * 0.55, tipX, 0.5) - 2.2, lerp(earBaseY, tipY, 0.5)],
      [tipX, tipY],
      [lerp(earBaseX + dna.ear_w * 0.55, tipX, 0.45) + 3.2, lerp(earBaseY, tipY, 0.5)],
      [earBaseX + dna.ear_w * 0.55, earBaseY + 5]
    ], 6, true);
    let innerEar = catmull_rom([
      [earBaseX - dna.ear_w * 0.30, earBaseY + 2],
      [tipX + 0.8, tipY + dna.ear_len * 0.20],
      [earBaseX + dna.ear_w * 0.18, earBaseY + 3]
    ], 5, true);
    let conchaHatch = engraving_hatch(innerEar, 1.8, 0, 0, 0.35, 0.08);
    earStrokes.push(earPoly, innerEar, ...conchaHatch);

    let farEar = rot_poly(trsl_poly(earPoly, -dna.ear_w * 0.62, 1.5), -0.12, tipX, earBaseY);
    let farInner = rot_poly(trsl_poly(innerEar, -dna.ear_w * 0.62, 1.5), -0.12, tipX, earBaseY);
    let farClipped = clip_multi_many([farEar, ...engraving_hatch(farInner, 1.8, 0, 0, -0.35, 0.05)], [bodyPoly, earPoly]);
    earStrokes.push(...farClipped);
  } else if (dna.ear_type === 'drop_long') {
    let bottomY = earBaseY + dna.ear_len;
    let midX = earBaseX + 3.5;
    earPoly = organic_ink(catmull_rom([
      [earBaseX - dna.ear_w * 0.38, earBaseY + 1],
      [midX - dna.ear_w * 0.68, lerp(earBaseY, bottomY, 0.52)],
      [midX - 1.5, bottomY],
      [midX + dna.ear_w * 0.68, lerp(earBaseY, bottomY, 0.58)],
      [earBaseX + dna.ear_w * 0.42, earBaseY + 2]
    ], 7, true), 0.75, 0.06, 41);
    let earFold = catmull_rom([
      [earBaseX - dna.ear_w * 0.22, earBaseY + 3],
      [midX - dna.ear_w * 0.35, lerp(earBaseY, bottomY, 0.48)],
      [midX - 2, bottomY - 5]
    ], 5, false);
    let earShade = engraving_hatch(earPoly, 2.4, -4, -6, 0.52, 0.14);
    earStrokes.push(earPoly, earFold, ...earShade);
    if (dna.coat_type === 'silky' || dna.coat_type === 'feathered' || dna.coat_type === 'poodle') {
      earStrokes.push(...silky_locks(earPoly, 2.2, 8, 3.2, 0.05));
    }
  } else if (dna.ear_type === 'button') {
    let foldX = earBaseX - dna.ear_w * 0.42;
    let tipX = earBaseX - dna.ear_w * 0.88;
    let tipY = earBaseY + dna.ear_len * 0.78;
    earPoly = catmull_rom([
      [earBaseX - dna.ear_w * 0.3, earBaseY - 1],
      [foldX - 3, earBaseY + 3],
      [tipX, tipY],
      [earBaseX + dna.ear_w * 0.45, earBaseY + dna.ear_len * 0.38],
      [earBaseX + dna.ear_w * 0.45, earBaseY]
    ], 5, true);
    let underShadow = catmull_rom([
      [foldX - 1, earBaseY + 5],
      [tipX + 1, tipY + 2.5],
      [earBaseX + dna.ear_w * 0.35, earBaseY + dna.ear_len * 0.42]
    ], 4, false);
    earStrokes.push(earPoly, underShadow, ...engraving_hatch(earPoly, 2.1, -3, -4, 0.6, 0.1));
  } else if (dna.ear_type === 'rose') {
    let tipX = earBaseX + dna.ear_len * 0.96;
    let tipY = earBaseY + 5;
    earPoly = catmull_rom([
      [earBaseX - 3, earBaseY + 1],
      [earBaseX + dna.ear_len * 0.4, earBaseY - 4],
      [tipX, tipY],
      [earBaseX + dna.ear_len * 0.35, earBaseY + dna.ear_w],
      [earBaseX - 2, earBaseY + 5]
    ], 5, true);
    let roseCrease = catmull_rom([
      [earBaseX, earBaseY + 2.5],
      [earBaseX + dna.ear_len * 0.45, earBaseY + 2],
      [tipX - 2, tipY]
    ], 4, false);
    earStrokes.push(earPoly, roseCrease, ...engraving_hatch(earPoly, 2.1, -3, -3, 0.4, 0.08));
  }

  // 4. ARTICULATED FORE & HIND LIMBS (Near & Far Pairs with Natural Stance)
  let groundY = cy0 + dna.chest_depth * 0.48 + dna.leg_len;
  let featheryLeg = (dna.coat_type === 'feathered' || dna.coat_type === 'silky') ? 1.0 : 0;

  let shX = cx0 + 4, shY = cy0 + dna.chest_depth * 0.45;
  let nearFore = build_forelimb(shX, shY, groundY, dna.leg_w, dna.paw_size, featheryLeg, 0);
  let farFore = build_forelimb(shX - 12, shY - 1.5, groundY - 1.5, dna.leg_w * 0.94, dna.paw_size * 0.94, featheryLeg, -6);

  let hipX = cx0 + dna.body_len - 11, hipY = cy0 + dna.croup_drop + 16;
  let nearHind = build_hindlimb(hipX, hipY, groundY, dna.leg_w, dna.hock_sweep, dna.paw_size, featheryLeg, 0);
  let farHind = build_hindlimb(hipX - 14, hipY - 1.5, groundY - 1.5, dna.leg_w * 0.94, dna.hock_sweep * 0.92, dna.paw_size * 0.94, featheryLeg, -7);

  // Internal anatomical muscle definition creases (Scapula, Flank fold)
  let anatomicalCreases = [
    catmull_rom([
      [cx0 + 5, cy0 + 9],
      [shX - 7, shY - 8],
      [shX + 4, shY + 5]
    ], 5, false),
    catmull_rom([
      [loin[0] - 12, loin[1] + 12],
      [flank[0] - 8, flank[1] - 6],
      [ribsEnd[0] + 4, ribsEnd[1] - 3]
    ], 5, false)
  ];

  // 5. TAIL GEOMETRY & PLUME LOCKS
  let tailBase = [croup[0] - 2, croup[1] + 4.5];
  let tailSpine = [tailBase];
  let tSteps = 14;
  let tx = tailBase[0], ty = tailBase[1];
  let baseTailAng = -0.25;
  if (dna.tail_type === 'whip') baseTailAng = 0.68;
  else if (dna.tail_type === 'plume_low') baseTailAng = 0.42;
  else if (dna.tail_type === 'saber') baseTailAng = 0.25;
  else if (dna.tail_type === 'saber_up' || dna.tail_type === 'pom') baseTailAng = -0.95;
  else if (dna.tail_type === 'plume' || dna.tail_type === 'ring') baseTailAng = -0.55;
  else if (dna.tail_type === 'curl') baseTailAng = -0.78;
  else if (dna.tail_type === 'bob') baseTailAng = -0.68;

  let curAng = baseTailAng;
  for (let i = 1; i <= tSteps; i++) {
    let t = i / tSteps;
    if (dna.tail_type === 'curl') curAng -= (dna.tail_curl * 2.85) / tSteps;
    else if (dna.tail_type === 'ring') curAng -= (t > 0.42 ? 3.25 : 0.55) / tSteps;
    else if (dna.tail_type === 'whip') curAng += (t < 0.5 ? 0.42 : -0.92) / tSteps;
    else curAng -= (dna.tail_curl * 1.32) / tSteps;
    tx += cos(curAng) * (dna.tail_len / tSteps);
    ty += sin(curAng) * (dna.tail_len / tSteps);
    tailSpine.push([tx, ty]);
  }

  let tailTop = [], tailBot = [], plumeStrokes = [];
  for (let i = 0; i < tailSpine.length; i++) {
    let t = i / (tailSpine.length - 1);
    let [px, py] = tailSpine[i];
    let prev = tailSpine[max(0, i - 1)], next = tailSpine[min(tailSpine.length - 1, i + 1)];
    let tang = atan2(next[1] - prev[1], next[0] - prev[0]);
    let nx = -sin(tang), ny = cos(tang);
    let w = lerp(7.2, 1.8, t);
    if (dna.tail_type === 'plume') w = lerp(6.8, 11.5, sin(t * PI));
    if (dna.tail_type === 'whip') w = lerp(4.8, 1.0, t);
    tailTop.push([px + nx * w, py + ny * w]);
    tailBot.push([px - nx * w, py - ny * w]);

    if ((dna.tail_type === 'plume' || dna.tail_type === 'plume_low' || dna.coat_type === 'silky') && i > 1) {
      let fLen = (dna.coat_type === 'silky' ? 28 : 19) * sin(t * PI);
      if (fLen > 3.5) {
        let fAng = tang + PI * 0.40;
        plumeStrokes.push(catmull_rom([
          [px - nx * w * 0.4, py - ny * w * 0.4],
          [px + cos(fAng) * fLen * 0.55, py + sin(fAng) * fLen * 0.55 + 3],
          [px + cos(fAng) * fLen, py + sin(fAng) * fLen + 7]
        ], 4, false));
      }
    }
  }
  let tailPoly = [...tailTop, ...tailBot.reverse()];

  // 6. OCCLUSION, ENGRAVING SHADING & FUR TEXTURE COMPOSITION
  let layerOutlines = [];
  let layerShading = [];
  let layerCoat = [];

  let foreSeamMask = trsl_poly(bodyPoly, 0, -11);
  let hindSeamMask = trsl_poly(bodyPoly, -9, -9);

  let nearForeVisible = clip(nearFore.poly, foreSeamMask).false;
  let nearHindVisible = clip(nearHind.poly, hindSeamMask).false;

  let foregroundBlockers = [nearFore.poly, nearHind.poly];
  if (dna.ear_type === 'drop_long' || dna.ear_type === 'button') foregroundBlockers.push(earPoly);
  if (beardPolys.length) foregroundBlockers.push(beardPolys[0]);
  if (collarPolys.length) foregroundBlockers.push(collarPolys[0]);

  let bodyVisible = clip_multi_many([bodyPoly], foregroundBlockers);

  // Add subtle fur-tuft serration to neck ruff, chest & torso (never on the muzzle/skull)
  if (dna.coat_type === 'double' || dna.coat_type === 'wire' || dna.coat_type === 'feathered') {
    bodyVisible = bodyVisible.map(seg => fur_serrate(seg, 2.0, 0.26, 0.25, 0.65, headX + 6));
  }

  let farForeBlockers = [bodyPoly, nearFore.poly];
  let farForeShade = dense_shadow_hatch(farFore.poly, 2.0, -0.45);
  let farForeVisible = clip_multi_many([farFore.poly, ...farFore.details], farForeBlockers);
  let farForeHatch = clip_multi_many(farForeShade, farForeBlockers);

  let farHindBlockers = [bodyPoly, nearHind.poly];
  let farHindShade = dense_shadow_hatch(farHind.poly, 2.0, -0.45);
  let farHindVisible = clip_multi_many([farHind.poly, ...farHind.details], farHindBlockers);
  let farHindHatch = clip_multi_many(farHindShade, farHindBlockers);

  let tailBlockers = [bodyPoly, nearHind.poly];
  let tailShadeRaw = engraving_hatch(tailPoly, 2.6, -3, -4, 0.5, 0.1);
  let tailVisible = clip_multi_many([tailPoly, ...plumeStrokes], tailBlockers);
  let tailShade = clip_multi_many(tailShadeRaw, tailBlockers);

  let bodyShadeBlockers = [nearFore.poly, nearHind.poly, eyePoly, nosePoly, ...beardPolys, ...collarPolys];
  if (dna.ear_type === 'drop_long' || dna.ear_type === 'button') bodyShadeBlockers.push(earPoly);

  let bodyShadePrimary = engraving_hatch(bodyPoly, 3.2, -8, -14, 0.64, 0.16);
  let bodyShadeDeep = engraving_hatch(bodyPoly, 2.6, -3, -7, 0.22, 0.12);
  let dorsalRimShade = engraving_hatch(bodyPoly, 4.2, 5, 6, -0.55, 0.10);

  let bodyShade = clip_multi_many(
    [...bodyShadePrimary, ...bodyShadeDeep, ...dorsalRimShade],
    bodyShadeBlockers
  );

  let nearForeShade = [
    ...engraving_hatch(nearFore.poly, 2.8, -4.5, -5, 0.62, 0.12),
    ...engraving_hatch(nearFore.poly, 2.4, -2, -3, 0.18, 0.10)
  ];
  let nearHindShade = [
    ...engraving_hatch(nearHind.poly, 2.8, -5.5, -6, 0.62, 0.14),
    ...engraving_hatch(nearHind.poly, 2.4, -2.5, -3.5, 0.18, 0.10)
  ];

  let bodyBbox = get_bbox(bodyPoly);
  let markings = clip_multi_many(
    coat_markings(bodyPoly, dna.pattern, bodyBbox, headX, { eyeX, eyeY, earBaseX, earBaseY, backMid, croup }),
    bodyShadeBlockers
  );

  let furBody = [], furFore = [], furHind = [];
  if (dna.coat_type === 'double' || dna.coat_type === 'feathered' || dna.coat_type === 'wire') {
    furBody.push(...coat_flow_strokes(bodyPoly, spineCurve, 1.15, dna.fur_len, 0.42, 0));
    furHind.push(...coat_flow_strokes(nearHind.poly, null, 0.9, dna.fur_len * 0.82, 0.35, 0));
    furFore.push(...coat_flow_strokes(nearFore.poly, null, 0.75, dna.fur_len * 0.72, 0.3, 0));
  } else if (dna.coat_type === 'silky') {
    furBody.push(...silky_locks(bodyPoly, 2.7, 13, 3.8, 0.06, headX + 4));
    furFore.push(...silky_locks(nearFore.poly, 2.3, 6, 2.6, 0.04));
    furHind.push(...silky_locks(nearHind.poly, 2.3, 6, 2.6, 0.04));
  } else if (dna.coat_type === 'skirted') {
    furBody.push(...silky_locks(bodyPoly, 2.5, 6, 2.4, 0.04, headX + 6));
    furHind.push(...coat_flow_strokes(nearHind.poly, null, 0.9, 9, 0.4, 0));
  } else if (dna.coat_type === 'smooth') {
    furBody.push(...coat_flow_strokes(bodyPoly, spineCurve, 0.52, 5.5, 0.22, 0));
  }
  let furStrokes = clip_multi_many([...furBody, ...furFore, ...furHind], bodyShadeBlockers);

  let pomHead = [], pomBody = [], pomWrist = [], pomAnkle = [], pomHind = [], pomTail = [];
  let pomStrokes = [];
  if (dna.coat_type === 'poodle') {
    let topknot = build_pom(headX + 2, headY - dna.skull_r * 0.92, 17.5, 13.5, -0.15);
    let jacket = build_pom(cx0 + dna.body_len * 0.16, cy0 + dna.chest_depth * 0.34, 42, 34, 0.05);
    let hipRosette = build_pom(hipX - 2, hipY - 5, 16, 13, -0.1);
    let tailPom = build_pom(tailSpine[tailSpine.length - 1][0], tailSpine[tailSpine.length - 1][1], 11.5, 11.5, 0);
    let fWristPom = build_pom(nearFore.joints[2][0], nearFore.joints[2][1] - 4, 8.8, 7.2, 0);
    let hAnklePom = build_pom(nearHind.joints[3][0], nearHind.joints[3][1] - 4, 8.8, 7.2, 0);
    pomHead.push(...topknot.strokes);
    pomBody.push(...jacket.strokes);
    pomHind.push(...hipRosette.strokes);
    pomTail.push(...tailPom.strokes);
    pomWrist.push(...fWristPom.strokes);
    pomAnkle.push(...hAnklePom.strokes);
    pomStrokes.push(...pomHead, ...pomBody, ...pomHind, ...pomTail, ...pomWrist, ...pomAnkle);
  }

  let groundShadow = build_ground_shadow(
    farFore.joints[3][0] - dna.paw_size,
    nearHind.joints[nearHind.joints.length - 1][0] + 8,
    groundY
  );

  layerOutlines.push(
    ...bodyVisible,
    ...headDetails,
    ...beardStrokes,
    ...collarStrokes,
    ...earStrokes,
    ...nearForeVisible,
    ...nearFore.details,
    ...nearHindVisible,
    ...nearHind.details,
    ...farForeVisible,
    ...farHindVisible,
    ...tailVisible,
    ...anatomicalCreases
  );

  layerShading.push(
    ...bodyShade,
    ...nearForeShade,
    ...nearHindShade,
    ...farForeHatch,
    ...farHindHatch,
    ...tailShade,
    ...groundShadow
  );

  layerCoat.push(
    ...markings,
    ...furStrokes,
    ...pomStrokes
  );

  let allPolylines = [...layerOutlines, ...layerShading, ...layerCoat];

  let rigJoints = [
    noseTip, stopPt, [headX, headY], withers, loin, croup,
    ...nearFore.joints,
    ...nearHind.joints
  ];
  let rigBones = [
    [noseTip, stopPt, [headX, headY], withers, backMid, loin, croup, ...tailSpine],
    [withers, ...nearFore.joints],
    [croup, ...nearHind.joints]
  ];

  let earTip = [earBaseX, earBaseY - 12];
  if (earPoly && earPoly.length) {
    let bestD = 0;
    for (let i = 0; i < earPoly.length; i++) {
      let dx = earPoly[i][0] - earBaseX, dy = earPoly[i][1] - earBaseY;
      let d = dx * dx + dy * dy;
      if (d > bestD) { bestD = d; earTip = earPoly[i].slice(); }
    }
  }
  let tailPts = [];
  for (let i = 0; i < 4; i++) {
    let idx = Math.round(i * (tailSpine.length - 1) / 3);
    tailPts.push(tailSpine[Math.min(tailSpine.length - 1, idx)].slice());
  }
  let pawOf = (limb) => limb.joints[limb.joints.length - 1].slice();

  let anim = {
    earType: dna.ear_type,
    groundY,
    joints: {
      root: croup.slice(),
      loin: loin.slice(),
      back: backMid.slice(),
      withers: withers.slice(),
      neck: [headX, headY],
      head: stopPt.slice(),
      nose: noseTip.slice(),
      earBase: [earBaseX, earBaseY],
      ear: earTip,
      tail1: tailPts[0], tail2: tailPts[1], tail3: tailPts[2], tail4: tailPts[3],
      nSh: nearFore.joints[0].slice(),
      nEl: nearFore.joints[1].slice(),
      nWr: nearFore.joints[2].slice(),
      nPw: nearFore.joints[3].slice(),
      fSh: farFore.joints[0].slice(),
      fEl: farFore.joints[1].slice(),
      fWr: farFore.joints[2].slice(),
      fPw: farFore.joints[3].slice(),
      nHip: nearHind.joints[0].slice(),
      nSt: nearHind.joints[1].slice(),
      nHo: nearHind.joints[2].slice(),
      nAk: nearHind.joints[3].slice(),
      nAn: pawOf(nearHind),
      fHip: farHind.joints[0].slice(),
      fSt: farHind.joints[1].slice(),
      fHo: farHind.joints[2].slice(),
      fAk: farHind.joints[3].slice(),
      fAn: pawOf(farHind)
    },
    parents: {
      loin: 'root', back: 'loin', withers: 'back', neck: 'withers',
      head: 'neck', nose: 'head', earBase: 'head', ear: 'earBase',
      tail1: 'root', tail2: 'tail1', tail3: 'tail2', tail4: 'tail3',
      nSh: 'withers', nEl: 'nSh', nWr: 'nEl', nPw: 'nWr',
      fSh: 'withers', fEl: 'fSh', fWr: 'fEl', fPw: 'fWr',
      nHip: 'root', nSt: 'nHip', nHo: 'nSt', nAk: 'nHo', nAn: 'nAk',
      fHip: 'root', fSt: 'fHip', fHo: 'fSt', fAk: 'fHo', fAn: 'fAk'
    },
    groups: {
      body: {
        outlines: [bodyPoly, ...anatomicalCreases],
        shading: [...bodyShadePrimary, ...bodyShadeDeep, ...dorsalRimShade],
        coat: [...markings, ...furBody, ...pomBody]
      },
      head: { outlines: headDetails, shading: [], coat: [...pomHead] },
      ear: { outlines: earStrokes, shading: [], coat: [] },
      beard: { outlines: beardStrokes, shading: [], coat: [] },
      collar: { outlines: collarStrokes, shading: [], coat: [] },
      nFore: { outlines: [nearFore.poly, ...nearFore.details], shading: nearForeShade, coat: [...furFore, ...pomWrist] },
      fFore: { outlines: [farFore.poly, ...farFore.details], shading: farForeShade, coat: [] },
      nHind: { outlines: [nearHind.poly, ...nearHind.details], shading: nearHindShade, coat: [...furHind, ...pomAnkle, ...pomHind] },
      fHind: { outlines: [farHind.poly, ...farHind.details], shading: farHindShade, coat: [] },
      tail: { outlines: [tailPoly, ...plumeStrokes], shading: tailShadeRaw, coat: [...pomTail] },
      ground: { outlines: [], shading: groundShadow, coat: [] }
    },
    masks: {
      eye: eyePoly,
      nose: nosePoly,
      ear: (dna.ear_type === 'drop_long' || dna.ear_type === 'button') ? earPoly : null,
      beard: beardPolys[0] || null,
      collar: collarPolys[0] || null
    }
  };

  return {
    dna,
    polylines: allPolylines,
    layers: {
      outlines: layerOutlines,
      shading: layerShading,
      coat: layerCoat,
      rigBones,
      rigJoints
    },
    anim
  };
}

// Reframe polylines into a target box [W x H] with Hershey plotter title
function reframe_dog(dogResult, W = 500, H = 320, pad = 22, label = null, frozenBbox = null) {
  let textH = label ? 22 : 0;
  let availW = W - pad * 2;
  let availH = H - pad * 2 - textH;
  let flatPts = dogResult.polylines.flat();
  let bbox = frozenBbox || get_bbox(flatPts);
  let s = min(availW / max(1, bbox.w), availH / max(1, bbox.h));
  let ox = pad + (availW - bbox.w * s) * 0.5;
  let oy = pad + (availH - bbox.h * s) * 0.5;

  const xform = (poly) => poly.map(([x, y]) => [
    Math.round(((x - bbox.x) * s + ox) * 100) / 100,
    Math.round(((y - bbox.y) * s + oy) * 100) / 100
  ]);

  let framed = {
    outlines: dogResult.layers.outlines.map(xform),
    shading: dogResult.layers.shading.map(xform),
    coat: dogResult.layers.coat.map(xform),
    rigBones: dogResult.layers.rigBones.map(xform),
    rigJoints: xform(dogResult.layers.rigJoints),
    label: label ? stroke_text(label, W * 0.5, H - pad - 8, 0.42) : []
  };
  framed.all = [...framed.outlines, ...framed.shading, ...framed.coat, ...framed.label];
  return framed;
}

function polylines_to_path_d(polylines) {
  let d = '';
  for (let i = 0; i < polylines.length; i++) {
    let p = polylines[i];
    if (!p || p.length < 2) continue;
    d += 'M' + p[0][0] + ' ' + p[0][1];
    for (let j = 1; j < p.length; j++) {
      d += 'L' + p[j][0] + ' ' + p[j][1];
    }
  }
  return d;
}

function draw_dog_svg(dnaOrId, options = {}) {
  let dna = typeof dnaOrId === 'string'
    ? BREED_CATALOG.find(b => b.id === dnaOrId) || BREED_CATALOG[0]
    : dnaOrId;
  let W = options.width || 520;
  let H = options.height || 340;
  let mode = options.mode || 'plotter';
  let raw = options.raw || generate_dog(dna, options.seed || (dna && dna.name));
  let framed = reframe_dog(
    raw,
    W - 20,
    H - 20,
    options.pad || 20,
    options.label !== undefined ? options.label : (dna && dna.name),
    options.frozenBbox || null
  );

  let shift = (polys) => polys.map(p => p.map(([x, y]) => [x + 10, y + 10]));
  let dShade = polylines_to_path_d(shift(framed.shading));
  let dCoat = polylines_to_path_d(shift(framed.coat));
  let dOut = polylines_to_path_d(shift(framed.outlines));
  let dLabel = polylines_to_path_d(shift(framed.label));

  if (mode === 'blueprint') {
    let dBones = polylines_to_path_d(shift(framed.rigBones));
    let jointsSvg = shift([framed.rigJoints])[0]
      .map(([jx, jy]) => `<circle cx="${jx}" cy="${jy}" r="2.6" fill="#e63946" stroke="#fff" stroke-width="0.8"/>`)
      .join('');
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}">
      <rect x="0" y="0" width="${W}" height="${H}" fill="#fdfbf7"/>
      <rect x="10" y="10" width="${W - 20}" height="${H - 20}" stroke="#1d3557" stroke-width="1" fill="none"/>
      <rect x="13" y="13" width="${W - 26}" height="${H - 26}" stroke="#1d3557" stroke-width="0.35" fill="none"/>
      <path d="${dShade}" stroke="#457b9d" stroke-width="0.62" fill="none" opacity="0.65" stroke-linecap="round"/>
      <path d="${dCoat}" stroke="#2a9d8f" stroke-width="0.72" fill="none" opacity="0.82" stroke-linecap="round"/>
      <path d="${dOut}" stroke="#1d1e22" stroke-width="1.15" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="${dBones}" stroke="#e63946" stroke-width="1.3" stroke-dasharray="3 2" fill="none"/>
      ${jointsSvg}
      <path d="${dLabel}" stroke="#1d1e22" stroke-width="0.95" fill="none" stroke-linecap="round"/>
    </svg>`;
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}">
    <rect x="0" y="0" width="${W}" height="${H}" fill="#fbf8f1"/>
    <rect x="10" y="10" width="${W - 20}" height="${H - 20}" stroke="#181614" stroke-width="1.0" fill="none"/>
    <rect x="13" y="13" width="${W - 26}" height="${H - 26}" stroke="#181614" stroke-width="0.35" fill="none"/>
    <path d="${dShade}" stroke="#262320" stroke-width="0.52" opacity="0.82" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="${dCoat}" stroke="#1e1b18" stroke-width="0.66" opacity="0.90" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="${dOut}" stroke="#141210" stroke-width="1.12" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="${dLabel}" stroke="#181614" stroke-width="0.90" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>`;
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    BREED_CATALOG, generate_dog, reframe_dog, draw_dog_svg, seed_rand, get_bbox,
    trsl_poly, clip, clip_multi, clip_multi_many
  };
}
