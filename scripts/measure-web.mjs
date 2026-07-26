// Drive the trunk-served WebGPU build in real Chrome, run the ramp benchmark,
// and read the REAL measured numbers back out of the DOM (#fps/#bench/#result).
//
// Usage: node scripts/measure-web.mjs <baseUrl> <screenshotPath>
import { chromium } from 'playwright';

const base = process.argv[2] || 'http://127.0.0.1:8088';
const shot = process.argv[3] || 'web-shot.png';

const GPU_FLAGS = [
  '--enable-unsafe-webgpu',
  '--enable-features=Vulkan,WebGPU',
  '--ignore-gpu-blocklist',
  '--use-angle=d3d11',
];

async function run(headless) {
  const browser = await chromium.launch({
    channel: 'chrome',
    headless,
    args: GPU_FLAGS,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(`${base}/?bench`, { waitUntil: 'load', timeout: 30000 });

  // let the wasm boot + the WebGPU probe resolve
  await page.waitForTimeout(2500);
  const gpu = (await page.textContent('#gpu'))?.trim();
  const adapterOk = gpu && gpu.toLowerCase().includes('ok');

  if (!adapterOk && headless) {
    await browser.close();
    return { retryHeaded: true };
  }

  // wait for the ramp to finish (#result gets populated) — up to 150s
  let result = '';
  const deadline = Date.now() + 150000;
  while (Date.now() < deadline) {
    result = (await page.textContent('#result'))?.trim() || '';
    if (result) break;
    await page.waitForTimeout(500);
  }

  const bench = (await page.textContent('#bench'))?.trim() || '';
  const fps = (await page.textContent('#fps'))?.trim() || '';
  await page.waitForTimeout(400);
  await page.screenshot({ path: shot });

  await browser.close();
  return { gpu, adapterOk, fps, bench, result, errors, headless };
}

let out = await run(true);
if (out.retryHeaded) {
  console.error('[measure] headless WebGPU adapter missing — retrying headed');
  out = await run(false);
}
console.log(JSON.stringify(out, null, 2));
