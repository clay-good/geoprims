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

/** The throttling rate for a host BenchmarkIndex: host ÷ target, held to 1–10. */
export const rateFor = (hostIndex, target) => Math.min(10, Math.max(1, hostIndex / target));

let cached = null;

/**
 * Measures this host once per process (the median of three runs, unthrottled)
 * and returns { hostIndex, rate } for the profile's target. GP_CPU_RATE
 * overrides the rate, for reproducing a report.
 */
export async function cpuRate(browser, profile) {
  if (process.env.GP_CPU_RATE) return { hostIndex: null, rate: Number(process.env.GP_CPU_RATE) };
  if (!cached) {
    cached = (async () => {
      const page = await browser.newPage();
      const runs = [];
      for (let i = 0; i < 3; i++) runs.push(await page.evaluate(computeBenchmarkIndex));
      await page.close();
      const hostIndex = runs.sort((a, b) => a - b)[1];
      return { hostIndex, rate: Math.round(rateFor(hostIndex, profile.cpu.targetBenchmarkIndex) * 100) / 100 };
    })();
  }
  return cached;
}
