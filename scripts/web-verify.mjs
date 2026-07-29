#!/usr/bin/env node
// Headless proof that the wasm build in dist/ actually boots in a browser:
// serves dist/ over HTTP, loads it in Chromium with a software WebGPU adapter,
// and reports what the page says about itself (adapter, FPS, HUD status) plus
// every console error the wasm threw. Screenshots the canvas as evidence.
//
// Usage: node scripts/web-verify.mjs [--query "?bench&grid=4"] [--wait 25000]
//                                    [--shot web-verify.png] [--headed]
// Exit 0 = wasm booted, WebGPU adapter acquired, frames rendered.
import { createServer } from 'node:http';
import { readFile, stat, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const require = createRequire(import.meta.url);
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DIST = path.join(ROOT, 'dist');

// playwright isn't a project dependency — take whichever copy is already on the
// machine (project → global playwright-core → $VOXELFORGE_PLAYWRIGHT) instead of
// pulling one per verify run.
function loadChromium() {
  const candidates = [
    process.env.VOXELFORGE_PLAYWRIGHT,
    'playwright',
    'playwright-core',
    'C:/Users/BagIdea/AppData/Roaming/npm/node_modules/openclaw/node_modules/playwright-core',
  ].filter(Boolean);
  for (const c of candidates) {
    try { return require(c).chromium; } catch { /* try the next one */ }
  }
  throw new Error(`no playwright found — tried: ${candidates.join(', ')}`);
}
// Empty = let playwright pick its own browser; set to a chrome.exe to reuse a
// browser cache that this playwright build didn't download itself.
const CHROME = process.env.VOXELFORGE_CHROME
  || 'C:/Users/BagIdea/AppData/Local/ms-playwright/chromium-1228/chrome-win64/chrome.exe';

const argv = process.argv.slice(2);
const arg = (name, fallback) => {
  const i = argv.indexOf(name);
  return i >= 0 && argv[i + 1] ? argv[i + 1] : fallback;
};
const QUERY = arg('--query', '');
const WAIT_MS = Number(arg('--wait', '25000'));
const SHOT = path.resolve(ROOT, arg('--shot', 'web-verify.png'));
const SETTLE_MS = Number(arg('--settle', '8000'));
const BROWSER_LOG = `${SHOT}.browser.log`;
const HEADED = argv.includes('--headed');

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm', // wrong type here = instantiateStreaming fails
  '.json': 'application/json',
  '.png': 'image/png',
  '.css': 'text/css',
  '.ico': 'image/x-icon',
};

const notFound = [];

async function serveDist() {
  const server = createServer(async (req, res) => {
    const rel = decodeURIComponent(req.url.split('?')[0]).replace(/^\/+/, '') || 'index.html';
    const file = path.join(DIST, rel);
    if (!file.startsWith(DIST)) { res.writeHead(403).end(); return; }
    try {
      const info = await stat(file);
      const target = info.isDirectory() ? path.join(file, 'index.html') : file;
      const body = await readFile(target);
      res.writeHead(200, {
        'content-type': MIME[path.extname(target)] || 'application/octet-stream',
        'content-length': body.length,
      });
      res.end(body);
    } catch {
      // Record the path: the browser console only ever says "Failed to load
      // resource: 404" with no url, which hides missing game assets.
      notFound.push(rel);
      res.writeHead(404, { 'content-type': 'text/plain' }).end('not found');
    }
  });
  await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
  return { server, port: server.address().port };
}

const { server, port } = await serveDist();
const url = `http://127.0.0.1:${port}/${QUERY}`;

const chromium = loadChromium();
const browser = await chromium.launch({
  ...(CHROME ? { executablePath: CHROME } : {}),
  headless: !HEADED,
  args: [
    // Headless Chromium DOES reach the real GPU here (nvidia/pascal) — the only
    // flag needed is --enable-unsafe-webgpu.
    //
    // Do NOT add --use-angle=swiftshader: it only swaps the WebGL backend, but
    // it forces the GPU process to software ANGLE, after which Dawn enumerates
    // zero adapters and requestAdapter() resolves null — the page then reports
    // "WebGPU present but no adapter" and Bevy panics with "Unable to find a
    // GPU!". If a software adapter is ever genuinely wanted, the correct flags
    // are --use-webgpu-adapter=swiftshader --enable-unsafe-swiftshader.
    '--enable-unsafe-webgpu',
    '--ignore-gpu-blocklist',
    '--disable-gpu-sandbox',
    // Dawn reports a failed WGSL compile to JS only as "[Invalid ShaderModule]
    // is invalid due to a previous error" — the actual compiler diagnostic goes
    // to Chrome's own log, so route that to a file we can read afterwards.
    // (playwright-core here exposes no browser.process(), so --log-file it is.)
    '--enable-logging',
    `--log-file=${BROWSER_LOG}`,
    '--v=1',
  ],
});

const consoleErrors = [];
const consoleLines = [];
const httpErrors = [];
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  page.on('console', (m) => {
    consoleLines.push(`[${m.type()}] ${m.text()}`);
    if (m.type() === 'error') consoleErrors.push(m.text());
  });
  page.on('pageerror', (e) => consoleErrors.push(`pageerror: ${e.message}`));
  // "Failed to load resource: 404" on the console never says WHICH url — name it,
  // so a missing asset can't hide behind a generic console error.
  page.on('response', (r) => {
    if (r.status() >= 400) httpErrors.push(`${r.status()} ${r.url()}`);
  });
  page.on('requestfailed', (r) => {
    httpErrors.push(`FAILED ${r.url()} — ${r.failure()?.errorText ?? 'unknown'}`);
  });

  // A failed WGSL compile only reaches JS through GPUShaderModule.getCompilationInfo(),
  // which wgpu never calls — so Dawn's real diagnostic is invisible and you get the
  // useless "[Invalid ShaderModule] is invalid due to a previous error" instead.
  // Wrap createShaderModule before the wasm boots and print what Tint actually said.
  await page.addInitScript(() => {
    if (!window.GPUDevice) return;
    const orig = GPUDevice.prototype.createShaderModule;
    GPUDevice.prototype.createShaderModule = function (desc) {
      const mod = orig.call(this, desc);
      mod.getCompilationInfo?.().then((info) => {
        for (const m of info.messages) {
          if (m.type !== 'error') continue;
          console.log(`WGSL-ERROR ${desc.label ?? '(unlabeled)'} @line ${m.lineNum}:${m.linePos} — ${m.message}`);
          const lines = String(desc.code ?? '').split('\n');
          const from = Math.max(0, m.lineNum - 4);
          const to = Math.min(lines.length, m.lineNum + 3);
          for (let i = from; i < to; i++) console.log(`WGSL-SRC ${i + 1}| ${lines[i]}`);
        }
      }).catch(() => {});
      return mod;
    };
  });

  await page.goto(url, { waitUntil: 'load', timeout: 60_000 });

  // The HUD's FPS counter is written by the wasm every frame — a real number
  // there is the proof that Bevy booted, got a surface, and is rendering.
  const rendered = await page
    .waitForFunction(
      () => {
        const fps = document.getElementById('fps')?.textContent?.trim();
        return !!fps && fps !== '-' && fps !== '–' && Number.parseFloat(fps) > 0;
      },
      { timeout: WAIT_MS },
    )
    .then(() => true)
    .catch(() => false);

  // The first frame with a nonzero FPS is not necessarily a frame with terrain
  // in it — chunks still mesh and upload for a while after. Let it settle so the
  // screenshot shows the world rather than an empty clear colour.
  await page.waitForTimeout(SETTLE_MS);

  const hud = await page.evaluate(() => ({
    gpu: document.getElementById('gpu')?.textContent ?? '',
    fps: document.getElementById('fps')?.textContent ?? '',
    status: document.getElementById('status')?.textContent ?? '',
    bench: document.getElementById('bench')?.textContent ?? '',
    result: document.getElementById('result')?.textContent ?? '',
    // Bevy may attach to its own canvas rather than #bevy, so report every one:
    // a canvas still at the 300x150 default means nothing ever drew into it.
    canvases: [...document.querySelectorAll('canvas')].map((c) => ({
      id: c.id || '(none)', w: c.width, h: c.height,
    })),
  }));

  // Ask the adapter directly. NOTE: this (and the `#gpu` HUD line, which runs the
  // same call) only proves the BROWSER exposes a WebGPU adapter — it says nothing
  // about which backend wgpu opened. A wasm built without `webgpu` still runs on
  // WebGL2 on this exact page while `navigator.gpu.requestAdapter()` resolves fine.
  // Diagnostic only; the gate is `bevyBackend` below.
  const adapter = await page.evaluate(async () => {
    if (!('gpu' in navigator)) return { present: false };
    const a = await navigator.gpu.requestAdapter().catch(() => null);
    return a ? { present: true, ...(a.info ?? {}) } : { present: true, adapter: null };
  });

  // THE authoritative backend read: bevy_render logs the AdapterInfo it actually
  // opened, and `backend:` in it is wgpu's own answer — BrowserWebGpu (WebGPU/Dawn)
  // vs Gl (WebGL2/ANGLE). A dist built from an index.html whose feature attributes
  // trunk silently ignored boots on Gl and looks healthy everywhere else; this is
  // the only line that catches it, so it is a hard gate.
  const adapterLine = consoleLines.find((l) => l.includes('AdapterInfo {')) ?? '';
  const bevyBackend = adapterLine.match(/backend:\s*(\w+)/)?.[1] ?? null;
  const bevyAdapterName = adapterLine.match(/name:\s*"([^"]*)"/)?.[1] ?? null;
  const bevyDriverInfo = adapterLine.match(/driver_info:\s*"([^"]*)"/)?.[1] ?? null;
  const onWebGpu = bevyBackend === 'BrowserWebGpu';

  // Shared pixel histogram — `data` is RGBA bytes from a 2D context.
  const MEASURE = `(data) => {
    let sum = 0, alpha = 0, nonBlack = 0;
    const seen = new Set();
    for (let i = 0; i < data.length; i += 4) {
      const lum = (data[i] + data[i + 1] + data[i + 2]) / 3;
      sum += lum;
      alpha += data[i + 3];
      if (lum > 8) nonBlack++;
      if (seen.size < 40) seen.add(data[i] + ',' + data[i + 1] + ',' + data[i + 2]);
    }
    const px = data.length / 4;
    return {
      meanLuma: +(sum / px).toFixed(2),
      meanAlpha: +(alpha / px).toFixed(2),
      nonBlackPct: +((nonBlack / px) * 100).toFixed(2),
      sampleColors: [...seen].slice(0, 8),
    };
  }`;

  // Read the canvas from inside the page, bypassing the screenshot pipeline.
  // DIAGNOSTIC ONLY — do NOT gate on this. A WebGPU canvas has no preserved
  // drawing buffer: once the frame is presented the texture is expired, so
  // drawImage() snapshots an empty surface and reports meanLuma 0 / meanAlpha 0
  // even while the compositor is showing a fully lit scene. meanAlpha is the
  // tell: 0 = "nothing was readable here", not "the scene is black".
  const canvasReadback = await page.evaluate((measureSrc) => {
    const c = document.getElementById('bevy');
    if (!c) return { error: 'no #bevy canvas' };
    const off = document.createElement('canvas');
    off.width = c.width; off.height = c.height;
    const ctx = off.getContext('2d');
    try { ctx.drawImage(c, 0, 0); } catch (e) { return { error: `drawImage: ${e}` }; }
    return (0, eval)(measureSrc)(ctx.getImageData(0, 0, off.width, off.height).data);
  }, MEASURE);

  // The authoritative "is the frame black?" read: measure the bytes that were
  // actually captured. Decoding happens back inside the page (no image decoder
  // in node here) — it is the same PNG that lands on disk as evidence.
  const shotBuffer = await page.screenshot({ path: SHOT });
  const framePixels = await page.evaluate(async ([b64, measureSrc]) => {
    const bytes = Uint8Array.from(atob(b64), (ch) => ch.charCodeAt(0));
    const bmp = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    const off = document.createElement('canvas');
    off.width = bmp.width; off.height = bmp.height;
    const ctx = off.getContext('2d');
    ctx.drawImage(bmp, 0, 0);
    return (0, eval)(measureSrc)(ctx.getImageData(0, 0, off.width, off.height).data);
  }, [shotBuffer.toString('base64'), MEASURE]);

  await writeFile(`${SHOT}.console.txt`, consoleLines.join('\n'), 'utf8');

  // A missing favicon is the browser asking for a file the game never shipped —
  // it surfaces as a bare "Failed to load resource: 404" with no url, which must
  // not be allowed to fail the gate. Any OTHER 404 (a real game asset) still does.
  const missingAssets = notFound.filter((p) => p !== 'favicon.ico');
  const realErrors = consoleErrors.filter(
    (e) => !(/Failed to load resource/i.test(e) && missingAssets.length === 0),
  );
  // A black frame is a HUD on top of a cleared canvas: the ~1% of lit pixels is
  // the overlay text. A drawn scene fills the viewport, so demand a real margin.
  const NON_BLACK_MIN = 20;
  const frameDrawn = (framePixels.nonBlackPct ?? 0) >= NON_BLACK_MIN;

  // WGSL diagnostics only exist on the WebGPU path (the GL backend goes naga→GLSL
  // and never calls createShaderModule), so "no WGSL-ERROR" is evidence of a clean
  // shader compile ONLY when onWebGpu is true. Count them explicitly rather than
  // inferring silence.
  const wgslErrors = consoleLines.filter((l) => l.includes('WGSL-ERROR'));

  const ok = rendered && onWebGpu && realErrors.length === 0 && frameDrawn;
  console.log(JSON.stringify(
    {
      url, rendered,
      bevyBackend, onWebGpu, bevyAdapterName, bevyDriverInfo,
      adapter, hud, frameDrawn, nonBlackMin: NON_BLACK_MIN,
      framePixels, canvasReadback, wgslErrors,
      consoleErrors: realErrors, httpErrors,
      notFound, missingAssets, shot: SHOT, verdict: ok ? 'PASS' : 'FAIL',
      ...(onWebGpu ? {} : {
        backendFailure: bevyBackend
          ? `bevy opened backend "${bevyBackend}", not BrowserWebGpu — the served dist was `
            + 'built without the webgpu feature (check that index.html uses '
            + 'data-cargo-features="webgpu", NOT the silently-ignored data-features)'
          : 'no AdapterInfo line in the console — could not confirm the wgpu backend',
      }),
    },
    null, 2,
  ));
  process.exitCode = ok ? 0 : 1;
} finally {
  await browser.close();
  server.close();
}
