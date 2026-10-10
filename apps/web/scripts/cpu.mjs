// CPU throttling calibrated to the host (reference profile "cpu"): the rate
// that makes this machine's Chromium run like a device with the profile's
// target BenchmarkIndex, instead of a fixed multiplier that emulates a faster
// phone on a fast laptop and a slower one on a CI runner. BenchmarkIndex is
// Lighthouse's measure: computeBenchmarkIndex from GoogleChrome/lighthouse
// core/lib/page-functions.js (Apache License 2.0), reproduced below.
function computeBenchmarkIndex() {
  function benchmarkIndexGC() {
    const start = Date.now();
    let iterations = 0;
    while (Date.now() - start < 500) {
      let s = '';
      for (let j = 0; j < 10000; j++) s += 'a';
      if (s.length === 1) throw new Error('will never happen, but prevents compiler optimizations');
      iterations++;
    }
    return Math.round(iterations / 10 / ((Date.now() - start) / 1000));
  }
  function benchmarkIndexNoGC() {
    const arrA = [];
    const arrB = [];
    for (let i = 0; i < 100000; i++) arrA[i] = arrB[i] = i;
    const start = Date.now();
    let iterations = 0;
    while (iterations % 10 !== 0 || Date.now() - start < 500) {
      const src = iterations % 2 === 0 ? arrA : arrB;
      const tgt = iterations % 2 === 0 ? arrB : arrA;
      for (let j = 0; j < src.length; j++) tgt[j] = src[j];
      iterations++;
    }
    return Math.round(iterations / 10 / ((Date.now() - start) / 1000));
  }
  return (benchmarkIndexGC() + benchmarkIndexNoGC()) / 2;
}

/**
 * How fast this host draws, in small filled and stroked polygons per
 * millisecond on a 1280 × 800 canvas. Measured beside BenchmarkIndex and only
 * reported for now: the frame benchmarks draw thousands of canvas paths, and
 * CI runners that score 1.7 times higher on BenchmarkIndex's strings and arrays
 * ran those benchmarks slower once throttled to match, so the two do not move
 * together. This is the number a canvas calibration would be built on.
 */
function computeCanvasIndex() {
  const canvas = document.createElement('canvas');
  canvas.width = 1280;
  canvas.height = 800;
  const g = canvas.getContext('2d');
  g.fillStyle = 'rgba(181, 60, 10, 0.2)';
  g.strokeStyle = 'rgb(181, 60, 10)';
  const start = performance.now();
  let drawn = 0;
  while (performance.now() - start < 500) {
    for (let i = 0; i < 500; i++) {
      const [cx, cy] = [(drawn * 37) % 1270, (drawn * 91) % 790];
      g.beginPath();
      for (let k = 0; k < 6; k++) {
        const a = (k * Math.PI) / 3;
        if (k) g.lineTo(cx + 3 * Math.cos(a), cy + 3 * Math.sin(a));
        else g.moveTo(cx + 3, cy);
      }
      g.closePath();
      g.fill();
      g.stroke();
      drawn++;
    }
    g.getImageData(0, 0, 1, 1); // make the drawing actually finish
  }
  return Math.round(drawn / (performance.now() - start));
}

/** The throttling rate for a host BenchmarkIndex: host ÷ target, held to 1–10. */
export const rateFor = (hostIndex, target) => Math.min(10, Math.max(1, hostIndex / target));

let cached = null;

/**
 * Measures this host once per process (the median of three runs, unthrottled)
 * and returns { hostIndex, rate } for the profile's target. GP_CPU_RATE
 * overrides the rate, for reproducing a report.
 */
export async function cpuRate(browser, profile) {
  if (process.env.GP_CPU_RATE) return { hostIndex: null, canvasIndex: null, rate: Number(process.env.GP_CPU_RATE) };
  if (!cached) {
    cached = (async () => {
      const page = await browser.newPage();
      const runs = [];
      for (let i = 0; i < 3; i++) runs.push(await page.evaluate(computeBenchmarkIndex));
      const draws = [];
      for (let i = 0; i < 3; i++) draws.push(await page.evaluate(computeCanvasIndex));
      await page.close();
      const hostIndex = runs.sort((a, b) => a - b)[1];
      const canvasIndex = draws.sort((a, b) => a - b)[1];
      return { hostIndex, canvasIndex, rate: Math.round(rateFor(hostIndex, profile.cpu.targetBenchmarkIndex) * 100) / 100 };
    })();
  }
  return cached;
}
