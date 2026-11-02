const fs = require('fs');
const path = require('path');
const { BREED_CATALOG, draw_dog_svg } = require('./dogdraw.js');

// Regenerate standalone SVG files in ./svgs/
const svgDir = path.join(__dirname, 'svgs');
fs.mkdirSync(svgDir, { recursive: true });
for (const name of fs.readdirSync(svgDir)) {
  if (name.endsWith('.svg')) fs.unlinkSync(path.join(svgDir, name));
}
BREED_CATALOG.forEach((b, idx) => {
  const num = String(idx + 1).padStart(2, '0');
  const svg = draw_dog_svg(b, { width: 520, height: 340, mode: 'plotter', seed: b.name, label: b.name });
  fs.writeFileSync(path.join(svgDir, `${num}_${b.id}.svg`), svg, 'utf8');
});

// Also write a dedicated jack_russell.svg at the root of ./svgs/ for quick access
fs.writeFileSync(
  path.join(svgDir, 'jack_russell.svg'),
  draw_dog_svg(BREED_CATALOG[0], { width: 520, height: 340, mode: 'plotter', seed: 'JACK RUSSELL TERRIER', label: 'JACK RUSSELL TERRIER' }),
  'utf8'
);

const dogdrawCode = fs.readFileSync(path.join(__dirname, 'dogdraw.js'), 'utf8')
  .replace(/if \(typeof module[\s\S]*$/, '');
const doganimCode = fs.readFileSync(path.join(__dirname, 'doganim.js'), 'utf8')
  .replace(/if \(typeof module[\s\S]*$/, '');

const html = `<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>dogdraw.js v2.2 — 21 Cins, Yandan Siluet ve 10 Animasyon</title>
<style>
  *, *::before, *::after { box-sizing: border-box; }
  body {
    margin: 0;
    padding: 0;
    font-family: var(--font-sans, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif);
    color: var(--foreground, #e5e5e5);
  }
  .wrap {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  /* Interactive Hero Studio */
  .studio {
    background: var(--card, #18181b);
    border: 1px solid var(--border, #27272a);
    border-radius: var(--radius, 10px);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .studio-top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .studio-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .studio-title h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .studio-title span {
    font-size: 12px;
    color: var(--muted-foreground, #a1a1aa);
  }
  .controls-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .btn {
    background: var(--secondary, #27272a);
    color: var(--secondary-foreground, #fafafa);
    border: 1px solid var(--border, #3f3f46);
    border-radius: 6px;
    padding: 6px 11px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .btn:hover {
    border-color: var(--ring, #71717a);
  }
  .btn.active {
    background: var(--primary, #fafafa);
    color: var(--primary-foreground, #18181b);
    border-color: var(--primary, #fafafa);
  }
  .seed-input {
    background: var(--input, #121214);
    color: var(--foreground, #fafafa);
    border: 1px solid var(--border, #3f3f46);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    width: 175px;
  }
  .variant-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding-top: 2px;
  }
  #animButtons { display: contents; }
  .variant-label {
    font-size: 11px;
    color: var(--muted-foreground, #a1a1aa);
    margin-right: 4px;
  }
  .hero-stage {
    display: grid;
    grid-template-columns: 1.38fr 1fr;
    gap: 14px;
    align-items: stretch;
  }
  @media (max-width: 600px) {
    .hero-stage { grid-template-columns: 1fr; }
  }
  .hero-canvas-box {
    background: #fbf8f1;
    border-radius: 8px;
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    min-height: 230px;
  }
  .hero-canvas-box svg {
    width: 100%;
    height: auto;
    display: block;
  }
  .dna-panel {
    background: var(--muted, #202024);
    border: 1px solid var(--border, #2e2e34);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 8px;
  }
  .dna-header {
    font-size: 12px;
    font-weight: 600;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border, #2e2e34);
    padding-bottom: 6px;
  }
  .dna-badge {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--accent-surface, rgba(59,130,246,0.15));
    color: var(--accent-surface-foreground, #60a5fa);
  }
  .sliders {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 10px;
  }
  .slider-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .slider-row label {
    font-size: 10px;
    color: var(--muted-foreground, #a1a1aa);
    display: flex;
    justify-content: space-between;
  }
  .slider-row input[type="range"] {
    width: 100%;
    accent-color: var(--primary, #60a5fa);
    height: 4px;
    margin: 2px 0;
  }
  .legend-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-size: 11px;
    color: var(--muted-foreground, #a1a1aa);
    padding-top: 4px;
    border-top: 1px solid var(--border, #2e2e34);
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }
  /* Breed Grid */
  .grid-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }
  .grid-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .grid-header span {
    font-size: 11px;
    color: var(--muted-foreground, #a1a1aa);
  }
  .breed-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  @media (max-width: 640px) {
    .breed-grid { grid-template-columns: repeat(2, 1fr); }
  }
  .breed-card {
    background: var(--card, #18181b);
    border: 1px solid var(--border, #27272a);
    border-radius: 8px;
    padding: 8px;
    cursor: pointer;
    transition: transform 0.12s ease, border-color 0.12s ease;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .breed-card:hover {
    border-color: var(--ring, #71717a);
    transform: translateY(-1px);
  }
  .breed-card.selected {
    border-color: var(--primary, #60a5fa);
    box-shadow: 0 0 0 1px var(--primary, #60a5fa);
  }
  .card-svg-wrap {
    background: #fbf8f1;
    border-radius: 5px;
    overflow: hidden;
    aspect-ratio: 520 / 340;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .card-svg-wrap svg {
    width: 100%;
    height: 100%;
    display: block;
  }
  .card-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 4px;
  }
  .card-name {
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card-num {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    color: var(--muted-foreground, #a1a1aa);
  }
</style>
</head>
<body>
<div class="wrap">
  <div class="studio">
    <div class="studio-top">
      <div class="studio-title">
        <h2>Köpek gravür stüdyosu — yandan siluet ve animasyon paketi</h2>
        <span>AKC/FCI yan profil oranları · Muybridge yürüyüş, tırıs, dörtnal · 10 klip aynı rig üzerinde</span>
      </div>
      <div class="controls-bar">
        <input id="seedInput" class="seed-input" type="text" value="JACK RUSSELL TERRIER" placeholder="İsim / Seed girin..." />
        <button id="modePlotterBtn" class="btn active" onclick="setMode('plotter')">Gravür Mürekkep</button>
        <button id="modeBlueprintBtn" class="btn" onclick="setMode('blueprint')">Anatomik İskelet</button>
        <button class="btn" onclick="downloadCurrentSvg()">SVG İndir</button>
      </div>
    </div>

    <div class="variant-bar">
      <span class="variant-label">Jack Russell Kürk Varyantı:</span>
      <button id="jrtSmoothBtn" class="btn active" onclick="applyJackRussellVariant('smooth')">Smooth Coat (Kısa Kürk)</button>
      <button id="jrtRoughBtn" class="btn" onclick="applyJackRussellVariant('rough')">Rough / Broken Coat (Telli & Sakallı)</button>
      <button id="jrtParsonBtn" class="btn" onclick="applyJackRussellVariant('parson')">Parson Russell (Uzun Bacaklı)</button>
    </div>

    <div class="variant-bar" id="animBar">
      <span class="variant-label">Animasyon:</span>
      <button id="animPlayBtn" class="btn" onclick="togglePlay()">Duraklat</button>
      <button class="btn" onclick="stopClip()">Duruş</button>
      <span id="animButtons"></span>
      <input id="animScrub" type="range" min="0" max="1" step="0.001" value="0" oninput="onScrub(this.value)" style="width:130px" />
      <select id="animSpeed" class="seed-input" style="width:68px" onchange="setSpeed(this.value)">
        <option value="0.5">0.5×</option>
        <option value="1" selected>1×</option>
        <option value="1.5">1.5×</option>
      </select>
    </div>

    <div class="hero-stage">
      <div class="hero-canvas-box" id="heroCanvas"></div>
      <div class="dna-panel">
        <div>
          <div class="dna-header">
            <span id="activeBreedTitle">1. Jack Russell Terrier</span>
            <span class="dna-badge" id="activeBreedGroup">Terrier · smooth</span>
          </div>
          <div class="sliders" style="margin-top:8px;">
            <div class="slider-row">
              <label><span>Burun Uzunluğu (muzzle)</span><b id="v_muzzle_len">25</b></label>
              <input type="range" id="s_muzzle_len" min="8" max="46" value="25" oninput="onSliderChange()" />
            </div>
            <div class="slider-row">
              <label><span>Alın Çukuru (stop)</span><b id="v_stop_depth">6.5</b></label>
              <input type="range" id="s_stop_depth" min="-5" max="18" step="0.5" value="6.5" oninput="onSliderChange()" />
            </div>
            <div class="slider-row">
              <label><span>Gövde Uzunluğu (body)</span><b id="v_body_len">122</b></label>
              <input type="range" id="s_body_len" min="90" max="185" value="122" oninput="onSliderChange()" />
            </div>
            <div class="slider-row">
              <label><span>Bacak Boyu (leg)</span><b id="v_leg_len">54</b></label>
              <input type="range" id="s_leg_len" min="32" max="108" value="54" oninput="onSliderChange()" />
            </div>
            <div class="slider-row">
              <label><span>Karın Çekikliği (tuck-up)</span><b id="v_waist_tuck">13</b></label>
              <input type="range" id="s_waist_tuck" min="2" max="38" value="13" oninput="onSliderChange()" />
            </div>
            <div class="slider-row">
              <label><span>Boyun Uzunluğu (neck)</span><b id="v_neck_len">36</b></label>
              <input type="range" id="s_neck_len" min="20" max="62" value="36" oninput="onSliderChange()" />
            </div>
          </div>
        </div>
        <div class="legend-bar">
          <span class="legend-item"><i class="dot" style="background:#141210"></i>1.12px Ana Kontur & Pençe/Göz</span>
          <span class="legend-item"><i class="dot" style="background:#457b9d"></i>0.52px Cross-Contour Hacim Taraması</span>
          <span class="legend-item"><i class="dot" style="background:#2a9d8f"></i>0.66px Pafta Yamaları (patches) & Kürk</span>
          <span class="legend-item"><i class="dot" style="background:#e63946"></i>Osteolojik Eklemler</span>
        </div>
      </div>
    </div>
  </div>

  <div class="grid-header">
    <h3>Jack Russell Terrier + 20 Köpek Cinsi Koleksiyonu (Tek <code>generate_dog(dna)</code> Fonksiyonu)</h3>
    <span>Bir cinse tıklayarak DNA parametrelerini yukarıda canlı değiştirebilirsiniz</span>
  </div>

  <div class="breed-grid" id="breedGrid"></div>
</div>

<script>
${dogdrawCode}
${doganimCode}

let currentMode = 'plotter';
let selectedIndex = 0;
let currentDna = Object.assign({}, BREED_CATALOG[0]);

function syncSlidersFromDna(dna) {
  const keys = ['muzzle_len', 'stop_depth', 'body_len', 'leg_len', 'waist_tuck', 'neck_len'];
  keys.forEach(k => {
    const el = document.getElementById('s_' + k);
    const valEl = document.getElementById('v_' + k);
    if (el && valEl) {
      el.value = dna[k];
      valEl.textContent = dna[k];
    }
  });
}

let activeClip = null;
let playing = false;
let clipStarted = 0;
let currentPhase = 0;
let animSpeed = 1;
let rafId = 0;
let poseCache = null;
let transition = null;
let lastPoseSpec = null;

function dnaKey() {
  const seedVal = document.getElementById('seedInput').value;
  return seedVal + '|' + JSON.stringify(currentDna);
}

function getRaw() {
  const key = dnaKey();
  if (!poseCache || poseCache.key !== key) {
    const seedVal = document.getElementById('seedInput').value.trim() || currentDna.name;
    const raw = generate_dog(currentDna, seedVal);
    poseCache = { key: key, raw: raw, box: DogAnim.animFrameBox(raw), seed: seedVal };
  }
  return poseCache;
}

function renderStill() {
  const seedVal = document.getElementById('seedInput').value.trim() || currentDna.name;
  const svg = draw_dog_svg(currentDna, {
    width: 520,
    height: 320,
    mode: currentMode,
    seed: seedVal,
    label: seedVal
  });
  document.getElementById('heroCanvas').innerHTML = svg;
}

function renderPosed(phase) {
  const cached = getRaw();
  const clip = DogAnim.DOG_ANIMS_BY_ID[activeClip];
  let blend;
  if (transition) {
    const t = Math.min(1, (performance.now() - transition.started) / 180);
    blend = {from: transition.spec, weight: t * t * (3 - 2 * t)};
    if (t === 1) transition = null;
  }
  lastPoseSpec = DogAnim.sample_pose(cached.raw, activeClip, phase, blend).spec;
  const svg = DogAnim.draw_posed_svg(cached.raw, {
    clip: activeClip,
    phase: phase,
    blend: blend,
    width: 520,
    height: 320,
    mode: currentMode,
    seed: cached.seed,
    label: clip ? clip.name.toUpperCase() : cached.seed,
    frozenBbox: cached.box
  });
  const canvas = document.getElementById('heroCanvas');
  const paths = Array.from(svg.matchAll(/<path d="([^"]*)"/g), m => m[1]);
  const circles = Array.from(svg.matchAll(/<circle cx="([^"]*)" cy="([^"]*)"/g));
  const oldPaths = canvas.querySelectorAll('path');
  const oldCircles = canvas.querySelectorAll('circle');
  if (canvas.dataset.mode === currentMode && oldPaths.length === paths.length && oldCircles.length === circles.length) {
    oldPaths.forEach((path, i) => path.setAttribute('d', paths[i]));
    oldCircles.forEach((circle, i) => { circle.setAttribute('cx', circles[i][1]); circle.setAttribute('cy', circles[i][2]); });
  } else {
    canvas.innerHTML = svg;
    canvas.dataset.mode = currentMode;
  }
  const scrub = document.getElementById('animScrub');
  if (scrub && document.activeElement !== scrub) scrub.value = String(phase);
}

function renderHero() {
  poseCache = null;
  transition = null;
  lastPoseSpec = null;
  if (activeClip) renderPosed(currentPhase);
  else renderStill();
  if (activeClip && playing) clipStarted = performance.now() - currentPhase * DogAnim.clip_duration(getRaw().raw, activeClip) / animSpeed * 1000;
}

function setPlayLabel() {
  const btn = document.getElementById('animPlayBtn');
  if (btn) btn.textContent = playing ? 'Duraklat' : 'Oynat';
}

function tick(now) {
  const clip = DogAnim.DOG_ANIMS_BY_ID[activeClip];
  if (!clip || !playing) return;
  const elapsed = ((now - clipStarted) / 1000) * animSpeed;
  const duration = DogAnim.clip_duration(getRaw().raw, activeClip);
  const phase = clip.loop ? (elapsed / duration) % 1 : Math.min(1, elapsed / duration);
  currentPhase = phase;
  renderPosed(phase);
  if (!clip.loop && phase >= 1) {
    playing = false;
    setPlayLabel();
    return;
  }
  rafId = requestAnimationFrame(tick);
}

function startClip(id) {
  transition = lastPoseSpec ? {spec: lastPoseSpec, started: performance.now()} : null;
  activeClip = id;
  playing = true;
  currentPhase = 0;
  clipStarted = performance.now();
  renderPosed(0);
  document.querySelectorAll('[data-clip]').forEach(function (btn) {
    btn.classList.toggle('active', btn.getAttribute('data-clip') === id);
  });
  setPlayLabel();
  if (rafId) cancelAnimationFrame(rafId);
  rafId = requestAnimationFrame(tick);
}

function togglePlay() {
  if (!activeClip) { startClip('idle'); return; }
  const clip = DogAnim.DOG_ANIMS_BY_ID[activeClip];
  playing = !playing;
  if (playing) {
    if (!clip.loop && currentPhase >= 1) currentPhase = 0;
    clipStarted = performance.now() - (currentPhase * DogAnim.clip_duration(getRaw().raw, activeClip) / animSpeed) * 1000;
    rafId = requestAnimationFrame(tick);
  } else if (rafId) {
    cancelAnimationFrame(rafId);
  }
  setPlayLabel();
}

function stopClip() {
  activeClip = null;
  playing = false;
  currentPhase = 0;
  transition = null;
  lastPoseSpec = null;
  if (rafId) cancelAnimationFrame(rafId);
  document.querySelectorAll('[data-clip]').forEach(function (btn) { btn.classList.remove('active'); });
  const scrub = document.getElementById('animScrub');
  if (scrub) scrub.value = '0';
  setPlayLabel();
  renderStill();
}

function onScrub(v) {
  playing = false;
  transition = null;
  if (rafId) cancelAnimationFrame(rafId);
  currentPhase = Number(v);
  if (!activeClip) activeClip = 'idle';
  setPlayLabel();
  renderPosed(currentPhase);
}

function setSpeed(v) {
  const clip = DogAnim.DOG_ANIMS_BY_ID[activeClip];
  animSpeed = Number(v);
  if (clip && playing) {
    clipStarted = performance.now() - (currentPhase * DogAnim.clip_duration(getRaw().raw, activeClip) / animSpeed) * 1000;
  }
}

function buildAnimButtons() {
  const bar = document.getElementById('animButtons');
  DogAnim.DOG_ANIMS.forEach(function (clip) {
    const b = document.createElement('button');
    b.className = 'btn';
    b.setAttribute('data-clip', clip.id);
    b.textContent = clip.tr;
    b.onclick = function () { startClip(clip.id); };
    bar.appendChild(b);
  });
}

function updateVariantButtons(variant) {
  ['Smooth', 'Rough', 'Parson'].forEach(v => {
    const btn = document.getElementById('jrt' + v + 'Btn');
    if (btn) btn.classList.toggle('active', v.toLowerCase() === variant);
  });
}

function applyJackRussellVariant(variant) {
  selectedIndex = 0;
  currentDna = Object.assign({}, BREED_CATALOG[0]);
  if (variant === 'rough') {
    currentDna.coat_type = 'wire';
    currentDna.fur_len = 8.5;
    currentDna.beard = 0.65;
    document.getElementById('seedInput').value = 'JACK RUSSELL (ROUGH)';
  } else if (variant === 'parson') {
    currentDna.coat_type = 'smooth';
    currentDna.leg_len = 74;
    currentDna.body_len = 108;
    currentDna.chest_depth = 46;
    currentDna.beard = 0;
    document.getElementById('seedInput').value = 'PARSON RUSSELL';
  } else {
    currentDna.coat_type = 'smooth';
    currentDna.fur_len = 6;
    currentDna.beard = 0;
    document.getElementById('seedInput').value = 'JACK RUSSELL TERRIER';
  }
  updateVariantButtons(variant);
  document.getElementById('activeBreedTitle').textContent = '1. ' + currentDna.trName;
  document.getElementById('activeBreedGroup').textContent = currentDna.group + ' · ' + currentDna.coat_type;
  syncSlidersFromDna(currentDna);
  renderHero();
  document.querySelectorAll('.breed-card').forEach((c, i) => {
    c.classList.toggle('selected', i === 0);
  });
}

function selectBreed(idx) {
  selectedIndex = idx;
  currentDna = Object.assign({}, BREED_CATALOG[idx]);
  document.getElementById('activeBreedTitle').textContent = (idx + 1) + '. ' + currentDna.trName;
  document.getElementById('activeBreedGroup').textContent = currentDna.group + ' · ' + currentDna.coat_type;
  document.getElementById('seedInput').value = currentDna.name.toUpperCase();
  updateVariantButtons(idx === 0 ? 'smooth' : '');
  syncSlidersFromDna(currentDna);
  renderHero();
  document.querySelectorAll('.breed-card').forEach((c, i) => {
    c.classList.toggle('selected', i === idx);
  });
}

function onSliderChange() {
  const keys = ['muzzle_len', 'stop_depth', 'body_len', 'leg_len', 'waist_tuck', 'neck_len'];
  keys.forEach(k => {
    const v = Number(document.getElementById('s_' + k).value);
    currentDna[k] = v;
    document.getElementById('v_' + k).textContent = v;
  });
  renderHero();
}

function setMode(m) {
  currentMode = m;
  document.getElementById('modePlotterBtn').classList.toggle('active', m === 'plotter');
  document.getElementById('modeBlueprintBtn').classList.toggle('active', m === 'blueprint');
  renderHero();
  renderGrid();
}

function renderGrid() {
  const grid = document.getElementById('breedGrid');
  grid.innerHTML = '';
  BREED_CATALOG.forEach((b, i) => {
    const card = document.createElement('div');
    card.className = 'breed-card' + (i === selectedIndex ? ' selected' : '');
    card.onclick = () => selectBreed(i);
    const svg = draw_dog_svg(b, { width: 520, height: 340, mode: currentMode, seed: b.name, label: b.name });
    card.innerHTML = \`
      <div class="card-svg-wrap">\${svg}</div>
      <div class="card-meta">
        <span class="card-name">\${b.trName}</span>
        <span class="card-num">#\${String(i + 1).padStart(2, '0')}</span>
      </div>
    \`;
    grid.appendChild(card);
  });
}

function downloadCurrentSvg() {
  const svgEl = document.querySelector('#heroCanvas svg');
  if (!svgEl) return;
  const blob = new Blob([svgEl.outerHTML], { type: 'image/svg+xml' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = (currentDna.id || 'dog') + '.svg';
  a.click();
}

document.getElementById('seedInput').addEventListener('input', () => renderHero());

buildAnimButtons();
selectBreed(0);
renderGrid();
startClip('idle');
console.log('Rendered ' + BREED_CATALOG.length + ' breeds and ' + DogAnim.DOG_ANIMS.length + ' clips.');
</script>
</body>
</html>`;

fs.writeFileSync(path.join(__dirname, 'index.html'), html);
console.log('Wrote index.html and SVGs, byte length:', Buffer.byteLength(html));
