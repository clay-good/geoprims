// Offline pack controls (web/offline-pwa). The service worker owns downloads
// and checksum verification; this module keeps the page accessible and small.

export const formatBytes = (bytes) => {
  if (!Number.isFinite(bytes) || bytes < 0) return 'Unavailable';
  if (bytes < 1_000) return `${bytes} B`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(bytes < 10_000 ? 1 : 0)} KB`;
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
};

export function isSafariTab(nav = navigator, media = matchMedia) {
  const ua = nav.userAgent ?? '';
  const safari = /Safari\//.test(ua) && !/(?:Chrome|Chromium|CriOS|FxiOS|EdgiOS|Android)\//.test(ua);
  const installed = nav.standalone === true || media?.('(display-mode: standalone)')?.matches === true;
  return safari && !installed;
}

export function storageSentence({ usage, quota }) {
  if (!Number.isFinite(usage) || !Number.isFinite(quota) || quota <= 0) return 'Storage use is unavailable in this browser.';
  return `${formatBytes(usage)} used of ${formatBytes(quota)} available to this site.`;
}

function askWorker(type, id, onProgress) {
  return navigator.serviceWorker.ready.then((registration) => new Promise((resolve, reject) => {
    const worker = registration.active ?? navigator.serviceWorker.controller;
    if (!worker) return reject(new Error('Offline support is not ready yet. Reload this page and try again.'));
    const channel = new MessageChannel();
    const timer = setTimeout(() => reject(new Error('The offline pack request timed out.')), 120_000);
    channel.port1.onmessage = ({ data }) => {
      if (data?.type === 'PACK_PROGRESS') return onProgress?.(data);
      if (data?.type !== 'PACK_RESULT') return;
      clearTimeout(timer);
      if (data.ok) resolve(data);
      else reject(Object.assign(new Error(data.error), { packs: data.packs }));
    };
    worker.postMessage({ type, id }, [channel.port2]);
  }));
}

export function wireOfflinePacks(root = document) {
  const cards = [...root.querySelectorAll('[data-pack]')];
  const storage = root.querySelector('[data-storage]');
  const meter = root.querySelector('[data-storage-meter]');
  const safari = root.querySelector('[data-safari-warning]');
  const offline = root.querySelector('[data-offline-now]');
  const persist = root.querySelector('[data-persist]');

  const refreshStorage = async () => {
    const estimate = await navigator.storage?.estimate?.().catch(() => null);
    if (estimate) {
      storage.textContent = storageSentence(estimate);
      meter.max = estimate.quota || 1;
      meter.value = estimate.usage || 0;
      meter.hidden = false;
    }
    const durable = await navigator.storage?.persisted?.().catch(() => false);
    persist.textContent = durable ? 'Storage is protected from automatic cleanup.' : 'Ask this browser to keep offline data';
    persist.disabled = durable;
  };

  const render = (states = []) => {
    for (const card of cards) {
      const state = states.find((item) => item.id === card.dataset.pack);
      const status = card.querySelector('[data-pack-status]');
      const install = card.querySelector('[data-pack-install]');
      const remove = card.querySelector('[data-pack-remove]');
      const progress = card.querySelector('progress');
      const installed = !!state?.installed;
      const installedVersion = state?.installedVersions?.at(-1);
      status.textContent = state?.updateAvailable
        ? `Update available · version ${installedVersion} is downloaded`
        : installed ? `Downloaded · version ${state.version}` : state?.partial ? 'Download incomplete' : 'Not downloaded';
      install.textContent = state?.updateAvailable ? 'Update' : state?.partial ? 'Resume' : installed ? 'Download again' : 'Download';
      remove.hidden = !installed && !state?.installedVersions?.length && !state?.partial;
      progress.hidden = true;
    }
  };

  const setBusy = (card, busy) => {
    for (const button of card.querySelectorAll('button')) button.disabled = busy;
    card.setAttribute('aria-busy', String(busy));
  };

  for (const card of cards) {
    const id = card.dataset.pack;
    const status = card.querySelector('[data-pack-status]');
    const progress = card.querySelector('progress');
    card.querySelector('[data-pack-install]').addEventListener('click', async () => {
      setBusy(card, true);
      progress.hidden = false;
      status.textContent = 'Downloading…';
      try {
        await navigator.storage?.persist?.().catch(() => false);
        const result = await askWorker('PACK_INSTALL', id, ({ done, total }) => {
          progress.max = total;
          progress.value = done;
          status.textContent = `Downloaded ${formatBytes(done)} of ${formatBytes(total)}`;
        });
        render(result.packs);
        if (isSafariTab()) safari.hidden = false;
      } catch (error) {
        status.textContent = error.message;
        progress.hidden = true;
      } finally {
        setBusy(card, false);
        refreshStorage();
      }
    });
    card.querySelector('[data-pack-remove]').addEventListener('click', async () => {
      setBusy(card, true);
      try {
        render((await askWorker('PACK_DELETE', id)).packs);
      } catch (error) {
        status.textContent = error.message;
      } finally {
        setBusy(card, false);
        refreshStorage();
      }
    });
  }

  persist.addEventListener('click', async () => {
    await navigator.storage?.persist?.().catch(() => false);
    refreshStorage();
  });
  const connection = () => (offline.hidden = navigator.onLine);
  addEventListener('online', connection);
  addEventListener('offline', connection);
  connection();
  askWorker('PACK_STATUS').then((result) => render(result.packs), () => {
    for (const card of cards) card.querySelector('[data-pack-status]').textContent = 'Offline support is not ready yet';
  });
  refreshStorage();
}
