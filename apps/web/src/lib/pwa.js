import { offlineStatus, showStatus } from './offline.mjs';

// Registers the service worker and offers updates (web/offline-pwa). A new
// release installs in the background and waits; the user chooses when to
// reload into it, so results never change mid-session. The chip says whether
// this browser holds the release yet.
export function registerOffline(prompt, chip) {
  const supported = 'serviceWorker' in navigator && !(location.protocol === 'http:' && location.hostname !== 'localhost');
  const say = (state) => showStatus(chip, offlineStatus({ supported, ...state }));
  if (!supported) return say({});
  let accepted = false;
  const offer = (worker) => {
    if (!worker || !navigator.serviceWorker.controller) return; // the first install needs no prompt
    prompt.hidden = false;
    prompt.querySelector('button.reload').onclick = () => {
      accepted = true;
      worker.postMessage({ type: 'SKIP_WAITING' });
    };
    // "Later" hides it for this page; the new release waits until the next visit.
    prompt.querySelector('button.later').onclick = () => (prompt.hidden = true);
  };
  navigator.serviceWorker.addEventListener('controllerchange', () => {
    if (accepted) location.reload();
    else say({ controlled: true });
  });
  say({ controlled: !!navigator.serviceWorker.controller });
  navigator.serviceWorker.register('/sw.js').then((reg) => {
    offer(reg.waiting);
    if (reg.installing) {
      say({ installing: true });
      reg.installing.addEventListener('statechange', () => say({ controlled: !!navigator.serviceWorker.controller, installing: reg.installing?.state === 'installing' }));
    }
    reg.addEventListener('updatefound', () => {
      const next = reg.installing;
      next?.addEventListener('statechange', () => {
        if (next.state === 'installed') offer(next);
      });
    });
  }, () => {});
  // Installed apps keep offline data: ask the browser not to evict it.
  addEventListener('appinstalled', () => navigator.storage?.persist?.());
}
