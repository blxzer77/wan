const assert = require('node:assert/strict');
const { test } = require('node:test');
const { BREED_CATALOG, generate_dog } = require('./dogdraw');
const A = require('./doganim');

function joints(pose) {
  return pose.layers.rigBones.flat();
}

function distance(a, b) {
  return Math.max(...a.map((p, i) => Math.hypot(p[0] - b[i][0], p[1] - b[i][1])));
}

test('loop seams close for every breed and looping clip', () => {
  for (const breed of BREED_CATALOG) {
    const raw = generate_dog(breed);
    for (const clip of A.DOG_ANIMS.filter(c => c.loop)) {
      const first = joints(A.pose_dog(raw, clip.id, 0));
      const last = joints(A.pose_dog(raw, clip.id, 1 - 1e-6));
      assert.ok(distance(first, last) < 0.02, `${breed.id}/${clip.id}: ${distance(first, last)}`);
    }
  }
});

test('sit and lie keep their endpoint beyond the clip duration', () => {
  const raw = generate_dog(BREED_CATALOG[0]);
  for (const clip of ['sit', 'lie']) {
    assert.ok(distance(joints(A.pose_dog(raw, clip, 1)), joints(A.pose_dog(raw, clip, 1.1))) < 1e-8, clip);
  }
});

test('jump takeoff is continuous', () => {
  for (const breed of BREED_CATALOG) {
    const raw = generate_dog(breed);
    const before = joints(A.pose_dog(raw, 'jump', 0.24 - 1e-6));
    const after = joints(A.pose_dog(raw, 'jump', 0.24 + 1e-6));
    assert.ok(distance(before, after) < 0.02, `${breed.id}: ${distance(before, after)}`);
  }
});

test('all bones retain length and all four paws reach their targets across the library', () => {
  for (const breed of BREED_CATALOG) {
    const raw = generate_dog(breed);
    const rest = raw.anim.joints;
    for (const clip of A.DOG_ANIMS) {
      for (let frame = 0; frame <= 120; frame++) {
        const pose = A.sample_pose(raw, clip.id, frame / 120);
        for (const [name, parent] of Object.entries(raw.anim.parents)) {
          const before = Math.hypot(rest[name][0] - rest[parent][0], rest[name][1] - rest[parent][1]);
          const after = Math.hypot(pose.wpos[name][0] - pose.wpos[parent][0], pose.wpos[name][1] - pose.wpos[parent][1]);
          assert.ok(Number.isFinite(after) && Math.abs(after - before) < 1e-7, `${breed.id}/${clip.id}/${name}`);
        }
        for (const [name, target] of Object.entries(pose.targets)) {
          assert.ok(Math.hypot(pose.wpos[name][0] - target[0], pose.wpos[name][1] - target[1]) < 1e-7, `${breed.id}/${clip.id}/${frame}/${name}`);
        }
        for (const side of ['n', 'f']) {
          const [st, ho, ak] = ['St', 'Ho', 'Ak'].map(k => pose.wpos[side + k]);
          const a = [st[0] - ho[0], st[1] - ho[1]], b = [ak[0] - ho[0], ak[1] - ho[1]];
          const cosine = (a[0] * b[0] + a[1] * b[1]) / Math.hypot(...a) / Math.hypot(...b);
          assert.ok(cosine <= Math.cos(Math.PI / 6) + 1e-7, `${breed.id}/${clip.id}/${frame}: collapsed ${side} hock`);
        }
      }
    }
  }
});

test('rendered paths stay finite and inside the fixed animation frame', () => {
  for (const breed of BREED_CATALOG) {
    const raw = generate_dog(breed);
    const box = A.animFrameBox(raw);
    for (const clip of A.DOG_ANIMS) {
      for (const phase of [0, .25, .5, .75, 1]) {
        const pose = A.pose_dog(raw, clip.id, phase);
        for (const line of pose.polylines) for (const [x, y] of line) {
          assert.ok(Number.isFinite(x) && Number.isFinite(y), `${breed.id}/${clip.id}: non-finite vertex`);
          assert.ok(x >= box.x && x <= box.x + box.w && y >= box.y && y <= box.y + box.h, `${breed.id}/${clip.id}: clipped vertex ${x},${y}`);
        }
      }
    }
  }
});

test('clip blends preserve both endpoints and invalid phases are rejected', () => {
  const raw = generate_dog(BREED_CATALOG[0]);
  const from = A.sample_pose(raw, 'sit', .7);
  const to = A.sample_pose(raw, 'walk', .3);
  const first = A.sample_pose(raw, 'walk', .3, {from: from.spec, weight: 0});
  const last = A.sample_pose(raw, 'walk', .3, {from: from.spec, weight: 1});
  assert.ok(distance(Object.values(from.wpos), Object.values(first.wpos)) < 1e-8);
  assert.ok(distance(Object.values(to.wpos), Object.values(last.wpos)) < 1e-8);
  assert.throws(() => A.sample_pose(raw, 'walk', NaN), /phase must be finite/);
  assert.throws(() => A.sample_pose(raw, 'unknown', 0), /Unknown dog animation/);
});
