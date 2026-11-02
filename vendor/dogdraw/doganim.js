// Animation clips for the dogdraw rig.
var DogAnim = (function () {
// Gaits follow Hildebrand's lateral-sequence walk (LH → LF → RH → RF),
// the diagonal trot, and the rotary gallop with a suspension — the side-view
// order in Eadweard Muybridge's Animal Locomotion (1887).
// Near legs are the fully drawn pair; far legs are the occluded pair.

const DOG = (typeof module !== 'undefined' && module.exports)
  ? require('./dogdraw.js')
  : { clip, clip_multi_many, trsl_poly, draw_dog_svg, get_bbox };

const { PI, sin, cos, atan2, hypot, min, max, abs } = Math;

const GROUP_BONES = {
  body: [['root', 'loin'], ['loin', 'back'], ['back', 'withers'], ['withers', 'neck'], ['neck', 'head'], ['head', 'nose']],
  head: [['neck', 'head']],
  ear: [['earBase', 'ear']],
  beard: [['neck', 'head']],
  collar: [['withers', 'neck'], ['neck', 'head']],
  nFore: [['nSh', 'nEl'], ['nEl', 'nWr'], ['nWr', 'nPw']],
  fFore: [['fSh', 'fEl'], ['fEl', 'fWr'], ['fWr', 'fPw']],
  nHind: [['nHip', 'nSt'], ['nSt', 'nHo'], ['nHo', 'nAk'], ['nAk', 'nAn']],
  fHind: [['fHip', 'fSt'], ['fSt', 'fHo'], ['fHo', 'fAk'], ['fAk', 'fAn']],
  tail: [['root', 'tail1'], ['tail1', 'tail2'], ['tail2', 'tail3'], ['tail3', 'tail4']],
  ground: []
};

const MASK_BONES = {
  eye: GROUP_BONES.head,
  nose: GROUP_BONES.head,
  ear: GROUP_BONES.ear,
  beard: GROUP_BONES.beard,
  collar: GROUP_BONES.collar
};

function clamp(v, a, b) { return max(a, min(b, v)); }

function metrics(j, dna = {}) {
  const leg = hypot(j.nPw[0] - j.nSh[0], j.nPw[1] - j.nSh[1]);
  const back = hypot(j.root[0] - j.withers[0], j.root[1] - j.withers[1]);
  const heavy = clamp(((dna.leg_w || 10) / leg - 0.12) * 1.7, 0, 0.55);
  return {
    leg: max(20, leg), back: max(36, back),
    stride: 1 - heavy * 0.35,
    bounce: 1 - heavy * 0.65,
    cadence: clamp(Math.sqrt(leg / 64), 0.78, 1.3) * (1 + heavy * 0.3),
    shortMuzzle: (dna.muzzle_len || 24) / (dna.skull_r || 18) < 0.7,
    ear: dna.ear_type === 'prick' ? 0.35 : dna.ear_type === 'drop_long' ? 1.2 : 0.65,
    tail: dna.tail_type === 'curl' ? 0.25 : 1
  };
}

function animFrameBox(raw) {
  const b = DOG.get_bbox(raw.polylines.flat());
  const leg = metrics(raw.anim.joints).leg;
  return {
    x: b.x - leg * 0.45,
    y: b.y - leg * 0.95,
    w: b.w + leg * 1.05,
    h: b.h + leg * 1.35
  };
}

function smoothstep(edge0, edge1, t) {
  const x = clamp((t - edge0) / (edge1 - edge0), 0, 1);
  return x * x * (3 - 2 * x);
}

function phase01(phase) {
  let u = phase % 1;
  if (u < 0) u += 1;
  return u;
}

// In-place gait, dog facing -X. Stance plants the paw forward and carries it
// back under the body. Swing lifts early, then reaches forward to the next plant.
function footAt(restPaw, phase, stride, lift, duty) {
  const u = phase01(phase);
  const x0 = restPaw[0];
  const y0 = restPaw[1];
  const half = stride * 0.5;
  if (u < duty) {
    const t = u / duty;
    const ramp = 0.08;
    const travel = t < ramp ? t * t / (2 * ramp * (1 - ramp))
      : t > 1 - ramp ? 1 - (1 - t) ** 2 / (2 * ramp * (1 - ramp))
      : (t - ramp * 0.5) / (1 - ramp);
    return [x0 - half + stride * travel, y0];
  }
  const t = (u - duty) / (1 - duty);
  const s = t * t * (3 - 2 * t);
  const arc = sin(t * PI) ** 2;
  return [x0 + half - stride * s, y0 - lift * arc];
}

function quadrupedFeet(j, phase, stride, lift, duty, offsets) {
  return {
    nAn: footAt(j.nAn, phase + offsets.nAn, stride, lift, duty),
    nPw: footAt(j.nPw, phase + offsets.nPw, stride, lift, duty),
    fAn: footAt(j.fAn, phase + offsets.fAn, stride, lift, duty),
    fPw: footAt(j.fPw, phase + offsets.fPw, stride, lift, duty)
  };
}

const WALK_OFFSETS = { nAn: 0, nPw: 0.75, fAn: 0.5, fPw: 0.25 };
const TROT_OFFSETS = { nAn: 0, fPw: 0, fAn: 0.5, nPw: 0.5 };
const GALLOP_OFFSETS = { nAn: 0, fAn: 0.9, fPw: 0.5, nPw: 0.4 };

function wag(rot, phase, amp, freq, base) {
  const b = base || 0;
  for (let i = 1; i <= 4; i++) {
    const along = 0.4 + i * 0.16;
    const lag = sin(phase * PI * 2 * freq - i * 0.62);
    const drift = sin(phase * PI * 2 + i * 0.4) * 0.2;
    rot['tail' + i] = b * along + (lag + drift) * amp * (0.12 + i * 0.09);
  }
}

// -1 at contact (limb forward), +1 at liftoff (limb back).
function limbSwing(phase, offset, duty, amp) {
  const u = phase01(phase + offset);
  const s = u < duty
    ? -1 + 2 * smoothstep(0, duty, u)
    : 1 - 2 * smoothstep(duty, 1, u);
  return s * amp;
}

function planted(j) {
  return {
    nPw: [j.nPw[0], j.nPw[1]],
    fPw: [j.fPw[0], j.fPw[1]],
    nAn: [j.nAn[0], j.nAn[1]],
    fAn: [j.fAn[0], j.fAn[1]]
  };
}

function riseFall(phase, a, b) {
  if (phase <= a || phase >= b) return 0;
  return sin((phase - a) / (b - a) * PI);
}

function sampleIdle(phase, m, j) {
  const w = phase01(phase);
  const breath = (1 - cos(w * PI * 2)) * 0.5;
  const sway = sin(w * PI * 2);
  const flick = Math.exp(-(((w - 0.76) / 0.045) ** 2)) - Math.exp(-(((w - 0.84) / 0.06) ** 2)) * 0.4;
  const rot = {
    root: sway * 0.007, loin: breath * 0.012, back: -breath * 0.009,
    neck: sway * 0.025 - breath * 0.02, head: -sway * 0.012,
    ear: (sin(w * PI * 2 - 0.4) * 0.025 + flick * 0.14) * m.ear
  };
  wag(rot, w, 0.055 * m.tail, 1, 0);
  return {rot, root: [sway * m.leg * 0.012, m.leg * 0.018 - breath * m.leg * 0.007], feet: planted(j)};
}

function sampleWalk(phase, m, j) {
  const duty = 0.64;
  const beat = cos(phase * PI * 4);
  const sway = sin(phase * PI * 2);
  const rot = {
    root: sway * 0.009, loin: sway * 0.022, back: -sway * 0.018,
    neck: -beat * 0.027, head: beat * 0.012,
    ear: sin(phase * PI * 4 - 0.65) * 0.07 * m.ear,
    nSh: limbSwing(phase, WALK_OFFSETS.nPw, duty, 0.1),
    fSh: limbSwing(phase, WALK_OFFSETS.fPw, duty, 0.1)
  };
  wag(rot, phase, 0.07 * m.tail, 2, 0);
  return {
    rot, root: [0, m.leg * (0.075 + beat * 0.009 * m.bounce)],
    feet: quadrupedFeet(j, phase, m.leg * 0.56 * m.stride, m.leg * 0.16, duty, WALK_OFFSETS)
  };
}

function sampleTrot(phase, m, j) {
  const duty = 0.43;
  const beat = cos(phase * PI * 4);
  const rot = {
    root: sin(phase * PI * 2) * 0.008,
    loin: sin(phase * PI * 4) * 0.024, back: -sin(phase * PI * 4) * 0.015,
    neck: -beat * 0.018, head: beat * 0.009,
    ear: sin(phase * PI * 4 - 0.75) * 0.12 * m.ear,
    nSh: limbSwing(phase, TROT_OFFSETS.nPw, duty, 0.16),
    fSh: limbSwing(phase, TROT_OFFSETS.fPw, duty, 0.16)
  };
  wag(rot, phase, 0.09 * m.tail, 2, -0.035);
  return {
    rot, root: [0, m.leg * (0.095 + beat * 0.025 * m.bounce)],
    feet: quadrupedFeet(j, phase, m.leg * 0.86 * m.stride, m.leg * 0.3, duty, TROT_OFFSETS)
  };
}

function sampleGallop(phase, m, j) {
  const duty = 0.22;
  const wave = sin((phase - 0.1) * PI * 2);
  const flight = riseFall(phase, 0.32, 0.5) ** 2 + riseFall(phase, 0.82, 1) ** 2;
  const rise = m.leg * flight * 0.14 * m.bounce;
  const stride = m.leg * 1.06 * m.stride;
  const feet = quadrupedFeet(j, phase, stride, m.leg * 0.34, duty, GALLOP_OFFSETS);
  feet.nAn = footAt(j.nAn, phase + GALLOP_OFFSETS.nAn, stride, m.leg * 0.22, duty);
  feet.fAn = footAt(j.fAn, phase + GALLOP_OFFSETS.fAn, stride, m.leg * 0.22, duty);
  for (const paw of Object.values(feet)) paw[1] -= rise;
  const rot = {
    root: wave * 0.025, loin: wave * 0.1 * m.bounce,
    back: -wave * 0.065 * m.bounce, withers: -wave * 0.018,
    neck: -wave * 0.035, head: wave * 0.015,
    ear: sin(phase * PI * 2 - 1.1) * 0.2 * m.ear,
    nSh: limbSwing(phase, GALLOP_OFFSETS.nPw, duty, 0.21),
    fSh: limbSwing(phase, GALLOP_OFFSETS.fPw, duty, 0.21)
  };
  wag(rot, phase, 0.11 * m.tail, 1, -0.07);
  return {
    rot, root: [m.leg * 0.025 * wave, m.leg * (0.1 - flight * 0.14 * m.bounce)],
    feet
  };
}

function sampleSit(phase, m, j) {
  const k = smoothstep(0.08, 0.78, phase);
  const settle = riseFall(phase, 0.7, 1) * 0.02;
  const hindHeight = j.nAn[1] - j.nHip[1];
  const sitDepth = min(hindHeight * 0.72, m.clearance * 0.9 + hindHeight * 0.18);
  const drop = sitDepth * k + hindHeight * settle;
  const pitch = Math.atan2(drop * 0.87, m.back);
  const rot = {root: pitch, neck: -pitch * 0.84, head: -pitch * 0.16, ear: -0.06 * k * m.ear};
  wag(rot, phase, 0.012 * m.tail * k, 2, 0.08 * k);
  return {
    rot, root: [m.leg * 0.1 * k, drop],
    feet: {
      nPw: [j.nPw[0], j.nPw[1]], fPw: [j.fPw[0], j.fPw[1]],
      nAn: [j.nAn[0] - m.leg * 0.26 * k, j.nAn[1]],
      fAn: [j.fAn[0] - m.leg * 0.23 * k, j.fAn[1]]
    }
  };
}

function sampleLie(phase, m, j) {
  const hind = smoothstep(0.05, 0.42, phase);
  const chest = smoothstep(0.28, 0.82, phase);
  const rearDrop = m.clearance * 0.83 * hind;
  const frontDrop = m.clearance * 0.91 * chest;
  const pitch = Math.atan2(rearDrop - frontDrop, m.back);
  const reach = m.leg * 0.8 * clamp(m.clearance / (m.leg * 0.5), 0.08, 1);
  const rot = {
    root: pitch, neck: -pitch - 0.12 * chest, head: -0.04 * chest,
    ear: 0.08 * chest * m.ear
  };
  wag(rot, phase, 0.008 * m.tail, 1, 0.02 * chest);
  return {
    rot, root: [m.leg * 0.04 * hind, rearDrop],
    feet: {
      nPw: [j.nPw[0] - reach * chest, j.nPw[1]],
      fPw: [j.fPw[0] - reach * 0.92 * chest, j.fPw[1]],
      nAn: [j.nAn[0] - m.leg * 0.33 * hind, j.nAn[1]],
      fAn: [j.fAn[0] - m.leg * 0.29 * hind, j.fAn[1]]
    }
  };
}

function barkHit(phase, center) {
  const d = phase - center;
  return {
    ant: riseFall(d, -0.09, 0) ** 2,
    k: riseFall(d, 0, 0.23) ** 2
  };
}

function sampleBark(phase, m, j) {
  const a = barkHit(phase, 0.17), b = barkHit(phase, 0.61);
  const ant = max(a.ant, b.ant), k = max(a.k, b.k);
  const rot = {
    root: k * 0.018, withers: -k * 0.015,
    neck: -0.045 * ant + 0.12 * k, head: 0.035 * k,
    ear: (0.16 * k - 0.1 * ant) * m.ear
  };
  wag(rot, phase, 0.12 * m.tail, 3, k * 0.03);
  return {rot, root: [m.leg * 0.018 * k, m.leg * (0.045 + 0.018 * ant - 0.018 * k)], feet: planted(j)};
}

function sampleSniff(phase, m, j) {
  const seek = sin(phase * PI * 2);
  const breath = sin(phase * PI * 8);
  const drop = min(m.leg * 0.43, m.clearance * 0.72);
  const pitch = -Math.atan2(drop, m.back);
  const root = [0, m.leg * 0.025];
  const dx = j.withers[0] - j.root[0], dy = j.withers[1] - j.root[1];
  const shoulder = [j.root[0] + dx * cos(pitch) - dy * sin(pitch), j.root[1] + root[1] + dx * sin(pitch) + dy * cos(pitch)];
  const neckLength = hypot(j.neck[0] - j.withers[0], j.neck[1] - j.withers[1]);
  const headLength = hypot(j.nose[0] - j.neck[0], j.nose[1] - j.neck[1]);
  const nose = [min(j.nose[0] + m.leg * 0.2, j.nPw[0] - m.leg * 0.55) + seek * m.leg * 0.03, j.nPw[1] - 4 - breath * 0.35];
  const ik = ik2(...shoulder, neckLength, headLength, ...nose, 1);
  const neckAngle = atan2(ik.mid[1] - shoulder[1], ik.mid[0] - shoulder[0]) - atan2(j.neck[1] - j.withers[1], j.neck[0] - j.withers[0]);
  const faceAngle = atan2(ik.end[1] - ik.mid[1], ik.end[0] - ik.mid[0]) - atan2(j.nose[1] - j.neck[1], j.nose[0] - j.neck[0]);
  const wrap = a => atan2(sin(a), cos(a));
  const rot = {
    root: pitch, neck: wrap(neckAngle - pitch), head: wrap(faceAngle - neckAngle),
    ear: (0.07 + breath * 0.014) * m.ear
  };
  if (m.shortMuzzle) {
    rot.neck = -0.8 + seek * 0.015;
    rot.head = -0.08 + breath * 0.012;
  }
  wag(rot, phase, 0.035 * m.tail, 1, 0.03);
  return {rot, root, feet: planted(j)};
}

function sampleShake(phase, m, j) {
  const env = riseFall(phase, 0.08, 0.86) ** 2;
  const w = phase * PI * 2 * 6;
  const rot = {
    root: sin(w - 1.4) * 0.012 * env,
    loin: sin(w - 0.9) * 0.018 * env, back: sin(w - 0.5) * 0.022 * env,
    neck: sin(w) * 0.17 * env, head: sin(w + 0.2) * 0.055 * env,
    ear: sin(w - 0.8) * 0.55 * env * m.ear
  };
  wag(rot, phase, 0.2 * env * m.tail, 6, 0);
  return {rot, root: [sin(w - 1.1) * m.leg * 0.01 * env, m.leg * (0.045 + 0.02 * env)], feet: planted(j)};
}

function sampleJump(phase, m, j) {
  const crouch = smoothstep(0.02, 0.18, phase) * (1 - smoothstep(0.18, 0.32, phase));
  const land = smoothstep(0.69, 0.79, phase) * (1 - smoothstep(0.8, 0.98, phase));
  const airT = clamp((phase - 0.28) / 0.44, 0, 1);
  const air = 16 * airT * airT * (1 - airT) * (1 - airT);
  const tuck = riseFall(phase, 0.3, 0.67) ** 2;
  const up = m.leg * 0.58 * m.bounce;
  const feet = planted(j);
  for (const key of Object.keys(feet)) {
    const fore = key.endsWith('Pw');
    feet[key][1] -= up * air + m.leg * 0.12 * tuck;
    feet[key][0] += (fore ? -1 : 1) * m.leg * 0.16 * tuck;
  }
  const rot = {
    root: -0.035 * crouch + 0.04 * land,
    loin: 0.035 * tuck, back: -0.025 * tuck,
    neck: 0.025 * crouch - 0.035 * land, head: -0.015 * tuck,
    ear: (sin(phase * PI * 4) * air * 0.15 - land * 0.1) * m.ear
  };
  wag(rot, phase, 0.05 * m.tail, 1, -air * 0.08);
  return {
    rot, root: [0, m.leg * 0.035 + min(m.leg * 0.16, m.clearance * 0.55) * (crouch + land * 0.65) - up * air],
    feet
  };
}

const DOG_ANIMS = [
  { id: 'idle', name: 'Idle', tr: 'Bekleme', loop: true, duration: 3.2, sample: sampleIdle },
  { id: 'walk', name: 'Walk', tr: 'Yürüyüş', loop: true, duration: 1.18, sample: sampleWalk },
  { id: 'trot', name: 'Trot', tr: 'Tırıs', loop: true, duration: 0.58, sample: sampleTrot },
  { id: 'gallop', name: 'Gallop', tr: 'Dörtnal', loop: true, duration: 0.44, sample: sampleGallop },
  { id: 'sit', name: 'Sit', tr: 'Oturuş', loop: false, duration: 1.25, sample: sampleSit },
  { id: 'lie', name: 'Lie', tr: 'Yatış', loop: false, duration: 1.5, sample: sampleLie },
  { id: 'bark', name: 'Bark', tr: 'Havlama', loop: true, duration: 1.05, sample: sampleBark },
  { id: 'sniff', name: 'Sniff', tr: 'Koklama', loop: true, duration: 1.85, sample: sampleSniff },
  { id: 'shake', name: 'Shake', tr: 'Silkinme', loop: true, duration: 0.95, sample: sampleShake },
  { id: 'jump', name: 'Jump', tr: 'Sıçrama', loop: true, duration: 1.28, sample: sampleJump }
];

const DOG_ANIMS_BY_ID = {};
DOG_ANIMS.forEach(clip => { DOG_ANIMS_BY_ID[clip.id] = clip; });

function segDist(px, py, ax, ay, bx, by) {
  const abx = bx - ax, aby = by - ay;
  const l2 = abx * abx + aby * aby || 1e-6;
  let t = ((px - ax) * abx + (py - ay) * aby) / l2;
  t = clamp(t, 0, 1);
  const qx = ax + abx * t, qy = ay + aby * t;
  return hypot(px - qx, py - qy);
}

function bindPoint(pt, bones, rest) {
  if (!bones || !bones.length) return null;
  if ([GROUP_BONES.nFore, GROUP_BONES.fFore, GROUP_BONES.nHind, GROUP_BONES.fHind].includes(bones)) {
    const [root, mid] = bones[0], pivot = rest[root];
    const length = hypot(rest[mid][0] - pivot[0], rest[mid][1] - pivot[1]);
    if (pt[1] < pivot[1] + length * 0.45) return {
      pivot: root, dx: pt[0] - pivot[0], dy: pt[1] - pivot[1],
      a: root, b: mid, t: smoothstep(-2, length * 0.45, pt[1] - pivot[1])
    };
  }
  if (bones === GROUP_BONES.body || bones === GROUP_BONES.collar) {
    const dx = rest.neck[0] - rest.withers[0], dy = rest.neck[1] - rest.withers[1];
    const along = ((pt[0] - rest.withers[0]) * dx + (pt[1] - rest.withers[1]) * dy) / (dx * dx + dy * dy);
    const radius = hypot(rest.head[0] - rest.neck[0], rest.head[1] - rest.neck[1]);
    const skull = (1 - smoothstep(radius * 1.05, radius * 1.4, hypot(pt[0] - rest.neck[0], pt[1] - rest.neck[1])))
      * smoothstep(rest.neck[0] + radius * 0.35, rest.neck[0] - radius * 0.1, pt[0]);
    const weight = max(smoothstep(0.6, 1, along), skull);
    if (weight > 0) return {
      pivot: 'neck', dx: pt[0] - rest.neck[0], dy: pt[1] - rest.neck[1],
      a: 'neck', b: 'head', t: weight
    };
  }
  const distances = bones.map(([a, b]) => segDist(pt[0], pt[1], ...rest[a], ...rest[b]));
  let nearest = distances.indexOf(min(...distances));
  const adjacent = [nearest - 1, nearest + 1].filter(i => i >= 0 && i < bones.length);
  const other = adjacent.sort((a, b) => distances[a] - distances[b])[0];
  if (other !== undefined) {
    const first = min(nearest, other), second = max(nearest, other);
    const a = rest[bones[first][0]], pivot = rest[bones[first][1]], b = rest[bones[second][1]];
    const l1 = hypot(pivot[0] - a[0], pivot[1] - a[1]);
    const l2 = hypot(b[0] - pivot[0], b[1] - pivot[1]);
    const vx = (pivot[0] - a[0]) / l1 + (b[0] - pivot[0]) / l2;
    const vy = (pivot[1] - a[1]) / l1 + (b[1] - pivot[1]) / l2;
    const length = hypot(vx, vy) || 1;
    const along = ((pt[0] - pivot[0]) * vx + (pt[1] - pivot[1]) * vy) / length;
    const band = max(1.5, min(l1, l2) * 0.36);
    const t = smoothstep(-band, band, along);
    if (t > 0 && t < 1) {
      return {pivot: bones[first][1], dx: pt[0] - pivot[0], dy: pt[1] - pivot[1], a: bones[first][1], b: bones[second][1], t};
    }
    nearest = t === 0 ? first : second;
  }
  const bone = bones[nearest], pivot = rest[bone[0]];
  return {pivot: bone[0], dx: pt[0] - pivot[0], dy: pt[1] - pivot[1], a: bone[1]};
}

function applyBind(b, wpos, wrot) {
  let c = cos(wrot[b.a] || 0), s = sin(wrot[b.a] || 0);
  if (b.b) {
    c = c * (1 - b.t) + cos(wrot[b.b] || 0) * b.t;
    s = s * (1 - b.t) + sin(wrot[b.b] || 0) * b.t;
    const length = max(0.75, hypot(c, s));
    c /= length; s /= length;
  }
  return [wpos[b.pivot][0] + b.dx * c - b.dy * s, wpos[b.pivot][1] + b.dx * s + b.dy * c];
}

function skinPoly(poly, bones, rest, wpos, wrot, cache) {
  if (!poly || poly.length < 2 || !bones || !bones.length) return poly || [];
  let rec = cache.get(poly);
  if (!rec) {
    rec = poly.map(pt => bindPoint(pt, bones, rest));
    cache.set(poly, rec);
  }
  return rec.map(b => applyBind(b, wpos, wrot));
}

function skinList(list, bones, rest, wpos, wrot, cache) {
  if (!list) return [];
  return list.map(poly => skinPoly(poly, bones, rest, wpos, wrot, cache));
}

function ik2(ax, ay, l1, l2, tx, ty, bendSign) {
  let dx = tx - ax, dy = ty - ay;
  let dist = hypot(dx, dy) || 0.001;
  const maxD = l1 + l2 - 0.001;
  const minD = abs(l1 - l2) + 0.001;
  if (dist > maxD) { dx *= maxD / dist; dy *= maxD / dist; dist = maxD; }
  else if (dist < minD) { dx *= minD / dist; dy *= minD / dist; dist = minD; }
  const ang = atan2(dy, dx);
  let cosA = (l1 * l1 + dist * dist - l2 * l2) / (2 * l1 * dist);
  cosA = clamp(cosA, -1, 1);
  const midA = ang + bendSign * Math.acos(cosA);
  return {
    mid: [ax + cos(midA) * l1, ay + sin(midA) * l1],
    end: [ax + dx, ay + dy]
  };
}

function setBoneRot(wrot, rest, wpos, a, b) {
  const ra = atan2(rest[b][1] - rest[a][1], rest[b][0] - rest[a][0]);
  const pa = atan2(wpos[b][1] - wpos[a][1], wpos[b][0] - wpos[a][0]);
  let d = pa - ra;
  while (d > PI) d -= PI * 2;
  while (d < -PI) d += PI * 2;
  wrot[b] = d;
}

function solveTwoBone(rest, wpos, wrot, root, mid, end, target, bendSign) {
  const l1 = max(1, hypot(rest[mid][0] - rest[root][0], rest[mid][1] - rest[root][1]));
  const l2 = max(1, hypot(rest[end][0] - rest[mid][0], rest[end][1] - rest[mid][1]));
  const ik = ik2(wpos[root][0], wpos[root][1], l1, l2, target[0], target[1], bendSign);
  wpos[mid] = ik.mid;
  wpos[end] = ik.end;
  setBoneRot(wrot, rest, wpos, root, mid);
  setBoneRot(wrot, rest, wpos, mid, end);
  return ik.end;
}

function solveFore(rest, wpos, wrot, sh, el, wr, pw, pawTarget) {
  const swing = pawTarget[0] - rest[pw][0];
  const leg = hypot(rest[pw][0] - rest[sh][0], rest[pw][1] - rest[sh][1]);
  const airborne = smoothstep(0, leg * 0.15, rest[pw][1] - pawTarget[1]);
  const tilt = clamp(-swing / leg * 0.9, -0.55, 0.55) * airborne;
  const pdx = rest[pw][0] - rest[wr][0];
  const pdy = rest[pw][1] - rest[wr][1];
  const c = cos(tilt), s = sin(tilt);
  const rdx = pdx * c - pdy * s, rdy = pdx * s + pdy * c;
  const wrist = solveTwoBone(rest, wpos, wrot, sh, el, wr, [pawTarget[0] - rdx, pawTarget[1] - rdy], -1);
  wpos[pw] = [wrist[0] + rdx, wrist[1] + rdy];
  setBoneRot(wrot, rest, wpos, wr, pw);
}

function solveHind(rest, wpos, wrot, hip, st, ho, ak, an, pawTarget) {
  const swing = pawTarget[0] - rest[an][0];
  const leg = hypot(rest[an][0] - rest[hip][0], rest[an][1] - rest[hip][1]);
  const airborne = smoothstep(0, leg * 0.2, rest[an][1] - pawTarget[1]);
  const tilt = clamp(-swing / leg * 0.8, -0.5, 0.5) * airborne;
  const pdx = rest[an][0] - rest[ak][0];
  const pdy = rest[an][1] - rest[ak][1];
  const c = cos(tilt), s = sin(tilt);
  const rdx = pdx * c - pdy * s, rdy = pdx * s + pdy * c;
  const ankle = [pawTarget[0] - rdx, pawTarget[1] - rdy];
  const compression = clamp(1 - (pawTarget[1] - wpos[hip][1]) / (rest[an][1] - rest[hip][1]), 0, 0.9);
  const hockTilt = tilt * 0.7 + compression * 1.45;
  const dx = rest[ak][0] - rest[ho][0], dy = rest[ak][1] - rest[ho][1];
  const hx = dx * cos(hockTilt) - dy * sin(hockTilt);
  const hy = dx * sin(hockTilt) + dy * cos(hockTilt);
  const hock = solveTwoBone(rest, wpos, wrot, hip, st, ho, [ankle[0] - hx, ankle[1] - hy], 1);
  wpos[ak] = [hock[0] + hx, hock[1] + hy];
  wpos[an] = [wpos[ak][0] + rdx, wpos[ak][1] + rdy];
  const upper = [wpos[st][0] - hock[0], wpos[st][1] - hock[1]];
  const l2 = hypot(...upper), l3 = hypot(hx, hy);
  const minAngle = PI / 6;
  if ((upper[0] * hx + upper[1] * hy) / (l2 * l3) > cos(minAngle)) {
    const l1 = hypot(rest[st][0] - rest[hip][0], rest[st][1] - rest[hip][1]);
    const distance = hypot(ankle[0] - wpos[hip][0], ankle[1] - wpos[hip][1]);
    const foldedLength = min(l2 + l3 - 0.001, l1 + distance - 0.001,
      max(Math.sqrt(l2 * l2 + l3 * l3 - 2 * l2 * l3 * cos(minAngle)), abs(l1 - distance) + 0.001));
    const proximal = ik2(...wpos[hip], l1, foldedLength, ...ankle, 1);
    const distal = ik2(...proximal.mid, l2, l3, ...ankle, -1);
    wpos[st] = proximal.mid;
    wpos[ho] = distal.mid;
    wpos[ak] = distal.end;
    wpos[an] = [distal.end[0] + rdx, distal.end[1] + rdy];
    setBoneRot(wrot, rest, wpos, hip, st);
    setBoneRot(wrot, rest, wpos, st, ho);
  }
  setBoneRot(wrot, rest, wpos, ho, ak);
  setBoneRot(wrot, rest, wpos, ak, an);
}

function fkPose(anim, rot, rootOff) {
  const rest = anim.joints;
  const wpos = {
    root: [rest.root[0] + (rootOff[0] || 0), rest.root[1] + (rootOff[1] || 0)]
  };
  const wrot = { root: rot.root || 0 };
  const names = Object.keys(anim.parents);
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    const p = anim.parents[name];
    const dx = rest[name][0] - rest[p][0];
    const dy = rest[name][1] - rest[p][1];
    const a = (wrot[p] || 0) + (rot[name] || 0);
    const c = cos(a), s = sin(a);
    wpos[name] = [wpos[p][0] + dx * c - dy * s, wpos[p][1] + dx * s + dy * c];
    wrot[name] = a;
  }
  return { wpos, wrot };
}

function solvePose(anim, spec) {
  const rot = spec.rot || {};
  const posed = fkPose(anim, rot, spec.root || [0, 0]);
  const feet = spec.feet;
  if (!feet) return posed;
  const rest = anim.joints;
  const { wpos, wrot } = posed;
  let support = 0;
  for (const side of ['n', 'f']) {
    const [sh, el, wr, pw] = ['Sh', 'El', 'Wr', 'Pw'].map(k => side + k);
    if (!feet[pw]) continue;
    const reach = hypot(rest[el][0] - rest[sh][0], rest[el][1] - rest[sh][1])
      + hypot(rest[wr][0] - rest[el][0], rest[wr][1] - rest[el][1]) - 0.002;
    const leg = hypot(rest[pw][0] - rest[sh][0], rest[pw][1] - rest[sh][1]);
    const airborne = smoothstep(0, leg * 0.15, rest[pw][1] - feet[pw][1]);
    const tilt = clamp(-(feet[pw][0] - rest[pw][0]) / leg * 0.9, -0.55, 0.55) * airborne;
    const toeX = rest[pw][0] - rest[wr][0], toeY = rest[pw][1] - rest[wr][1];
    const wristX = feet[pw][0] - toeX * cos(tilt) + toeY * sin(tilt);
    const wristY = feet[pw][1] - toeX * sin(tilt) - toeY * cos(tilt);
    const dx = wristX - wpos[sh][0];
    if (abs(dx) < reach) support = max(support, wristY - Math.sqrt(reach * reach - dx * dx) - wpos[sh][1]);
  }
  if (support > 0) for (const point of Object.values(wpos)) point[1] += support;
  let tailLift = 0;
  for (let i = 1; i <= 4; i++) {
    const point = wpos['tail' + i];
    const dx = point[0] - wpos.root[0], dy = point[1] - wpos.root[1];
    if (dx > 0 && point[1] > anim.groundY - 3) {
      const limit = Math.asin(clamp((anim.groundY - 3 - wpos.root[1]) / hypot(dx, dy), -1, 1));
      tailLift = min(tailLift, limit - atan2(dy, dx));
    }
  }
  if (tailLift < 0) for (let i = 1; i <= 4; i++) {
    const name = 'tail' + i;
    const dx = wpos[name][0] - wpos.root[0], dy = wpos[name][1] - wpos.root[1];
    wpos[name] = [wpos.root[0] + dx * cos(tailLift) - dy * sin(tailLift), wpos.root[1] + dx * sin(tailLift) + dy * cos(tailLift)];
    wrot[name] += tailLift;
  }
  if (feet.nPw) solveFore(rest, wpos, wrot, 'nSh', 'nEl', 'nWr', 'nPw', feet.nPw);
  if (feet.fPw) solveFore(rest, wpos, wrot, 'fSh', 'fEl', 'fWr', 'fPw', feet.fPw);
  if (feet.nAn) solveHind(rest, wpos, wrot, 'nHip', 'nSt', 'nHo', 'nAk', 'nAn', feet.nAn);
  if (feet.fAn) solveHind(rest, wpos, wrot, 'fHip', 'fSt', 'fHo', 'fAk', 'fAn', feet.fAn);
  return posed;
}

function keepLines(list) {
  return list.filter(p => p && p.length > 1);
}

function outside(lines, blockers) {
  const polys = (blockers || []).filter(p => p && p.length > 2);
  if (!polys.length) return keepLines(lines);
  return DOG.clip_multi_many(keepLines(lines), polys);
}

function aboveGround(lines, ground) {
  const result = [];
  for (const line of lines) {
    let visible = [];
    for (let i = 0; i < line.length; i++) {
      const point = line[i];
      const previous = line[i - 1];
      if (previous && (previous[1] <= ground) !== (point[1] <= ground)) {
        const t = (ground - previous[1]) / (point[1] - previous[1]);
        const crossing = [previous[0] + (point[0] - previous[0]) * t, ground];
        visible.push(crossing);
        if (point[1] > ground) {
          if (visible.length > 1) result.push(visible);
          visible = [];
        }
      }
      if (point[1] <= ground) visible.push(point);
    }
    if (visible.length > 1) result.push(visible);
  }
  return result;
}

function sample_pose(raw, clipId, phase, blend) {
  const clip = DOG_ANIMS_BY_ID[clipId];
  if (!clip) throw new RangeError('Unknown dog animation: ' + clipId);
  if (!Number.isFinite(phase)) throw new TypeError('Animation phase must be finite');
  if (!raw.anim) throw new TypeError('Dog has no animation rig');
  const anim = raw.anim;
  const rest = anim.joints;
  const m = metrics(rest, raw.dna);
  if (anim._clearance === undefined) {
    const torso = anim.groups.body.outlines[0].filter(p => p[0] > rest.nSh[0] + 8 && p[0] < rest.root[0] - 5);
    anim._clearance = max(1, anim.groundY - max(...torso.map(p => p[1])));
  }
  m.clearance = anim._clearance;
  const u = clip.loop ? phase01(phase) : clamp(phase, 0, 1);
  const spec = clip.sample(u, m, rest) || {};
  if (blend && blend.from && blend.weight < 1) {
    const from = blend.from, t = clamp(blend.weight, 0, 1);
    const mix = (a, b) => (a || 0) * (1 - t) + (b || 0) * t;
    for (const key of new Set([...Object.keys(from.rot || {}), ...Object.keys(spec.rot || {})])) {
      spec.rot[key] = mix((from.rot || {})[key], spec.rot[key]);
    }
    spec.root = [mix(from.root[0], spec.root[0]), mix(from.root[1], spec.root[1])];
    for (const key of Object.keys(spec.feet)) {
      spec.feet[key] = spec.feet[key].map((v, i) => mix(from.feet[key][i], v));
    }
  }
  if (spec.rot) {
    spec.rot.neck = clamp(spec.rot.neck || 0, -1.9, 0.4);
    spec.rot.head = clamp(spec.rot.head || 0, -1.2, 1.2);
    spec.rot.ear = clamp(spec.rot.ear || 0, -0.75, 0.75);
  }
  return {...solvePose(anim, spec), targets: spec.feet, phase: u, metrics: m, spec};
}

function clip_duration(raw, clipId) {
  const clip = DOG_ANIMS_BY_ID[clipId];
  if (!clip) throw new RangeError('Unknown dog animation: ' + clipId);
  return clip.duration * metrics(raw.anim.joints, raw.dna).cadence;
}

function pose_dog(raw, clipId, phase, blend) {
  if (!raw.anim || !DOG_ANIMS_BY_ID[clipId]) return raw;
  const anim = raw.anim;
  const rest = anim.joints;
  const { wpos, wrot } = sample_pose(raw, clipId, phase, blend);
  if (!anim._bindCache) anim._bindCache = new WeakMap();
  const cache = anim._bindCache;

  const skinned = {};
  const names = Object.keys(anim.groups);
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    const group = anim.groups[name];
    const bones = GROUP_BONES[name];
    skinned[name] = {
      outlines: skinList(group.outlines, bones, rest, wpos, wrot, cache),
      shading: skinList(group.shading, bones, rest, wpos, wrot, cache),
      coat: skinList(group.coat, bones, rest, wpos, wrot, cache)
    };
  }

  const masks = {};
  const maskNames = Object.keys(anim.masks);
  for (let i = 0; i < maskNames.length; i++) {
    const name = maskNames[i];
    const poly = anim.masks[name];
    masks[name] = poly ? skinPoly(poly, MASK_BONES[name], rest, wpos, wrot, cache) : null;
  }

  const bodyPoly = skinned.body.outlines[0];
  const nForePoly = skinned.nFore.outlines[0];
  const nHindPoly = skinned.nHind.outlines[0];
  const frontBlock = [nForePoly, nHindPoly];
  if (masks.ear) frontBlock.push(masks.ear);
  if (masks.beard) frontBlock.push(masks.beard);
  if (masks.collar) frontBlock.push(masks.collar);

  const bodySil = outside([bodyPoly], frontBlock);
  const creases = skinned.body.outlines.slice(1);
  const nForeOut = outside(skinned.nFore.outlines, [bodyPoly]);
  const nHindOut = outside(skinned.nHind.outlines, [bodyPoly]);
  const fForeOut = outside(skinned.fFore.outlines, [bodyPoly, nForePoly]);
  const fHindOut = outside(skinned.fHind.outlines, [bodyPoly, nHindPoly]);
  const tailOut = outside(skinned.tail.outlines, [bodyPoly, nHindPoly]);

  const shadeBlock = [nForePoly, nHindPoly, masks.eye, masks.nose, masks.beard, masks.collar, masks.ear];
  const bodyShade = outside(skinned.body.shading, shadeBlock);
  const bodyCoat = outside(skinned.body.coat, [nForePoly, nHindPoly, masks.ear, masks.beard]);

  const outlines = aboveGround(keepLines([
    ...fForeOut,
    ...fHindOut,
    ...bodySil,
    ...creases,
    ...tailOut,
    ...nHindOut,
    ...nForeOut,
    ...skinned.collar.outlines,
    ...skinned.beard.outlines,
    ...skinned.ear.outlines,
    ...skinned.head.outlines
  ]), anim.groundY + 0.8);
  const shading = [...skinned.ground.shading, ...aboveGround(keepLines([
    ...outside(skinned.fFore.shading, [bodyPoly, nForePoly]),
    ...outside(skinned.fHind.shading, [bodyPoly, nHindPoly]),
    ...bodyShade,
    ...skinned.nFore.shading,
    ...skinned.nHind.shading,
    ...skinned.tail.shading
  ]), anim.groundY + 0.8)];
  const coat = aboveGround(keepLines([
    ...bodyCoat,
    ...skinned.head.coat,
    ...skinned.nFore.coat,
    ...skinned.nHind.coat,
    ...skinned.tail.coat,
    ...skinned.fFore.coat,
    ...skinned.fHind.coat
  ]), anim.groundY + 0.8);

  const bonePairs = [
    ['root', 'loin'], ['loin', 'back'], ['back', 'withers'], ['withers', 'neck'],
    ['neck', 'head'], ['head', 'nose'], ['earBase', 'ear'],
    ['root', 'tail1'], ['tail1', 'tail2'], ['tail2', 'tail3'], ['tail3', 'tail4'],
    ['withers', 'nSh'], ['nSh', 'nEl'], ['nEl', 'nWr'], ['nWr', 'nPw'],
    ['withers', 'fSh'], ['fSh', 'fEl'], ['fEl', 'fWr'], ['fWr', 'fPw'],
    ['root', 'nHip'], ['nHip', 'nSt'], ['nSt', 'nHo'], ['nHo', 'nAk'], ['nAk', 'nAn'],
    ['root', 'fHip'], ['fHip', 'fSt'], ['fSt', 'fHo'], ['fHo', 'fAk'], ['fAk', 'fAn']
  ];
  const rigBones = bonePairs.map(([a, b]) => [wpos[a], wpos[b]]);
  const rigJoints = bonePairs.map(([a]) => wpos[a]);

  return {
    dna: raw.dna,
    polylines: [...outlines, ...shading, ...coat],
    layers: { outlines, shading, coat, rigBones, rigJoints },
    pose: {joints: wpos, rotations: wrot}
  };
}

function draw_posed_svg(raw, options) {
  const posed = pose_dog(raw, options.clip, options.phase === undefined ? 0 : options.phase, options.blend);
  return DOG.draw_dog_svg(raw.dna, {
    width: options.width,
    height: options.height,
    mode: options.mode,
    seed: options.seed,
    label: options.label,
    raw: posed,
    frozenBbox: options.frozenBbox || null,
    pad: options.pad
  });
}

return { DOG_ANIMS, DOG_ANIMS_BY_ID, pose_dog, sample_pose, clip_duration, draw_posed_svg, animFrameBox, metrics };
})();

if (typeof module !== 'undefined' && module.exports) {
  module.exports = DogAnim;
}
