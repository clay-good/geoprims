// The offline status chip (ux/mobile-and-field, "Offline affordances in the
// field"). The service worker caches the whole release on install — every
// page, module, and data asset — so there is one pack, not one per domain,
// and the only question a chip can honestly answer is whether this browser
// has it yet.

/**
 * What to show for a registration state.
 * `controlled` is true once a worker serves this page; `installing` is true
 * while one is caching the release.
 */
export function offlineStatus({ supported = true, controlled = false, installing = false } = {}) {
  if (!supported) return { state: 'unavailable', text: 'Needs the network' };
  if (controlled) return { state: 'ready', text: 'Works offline' };
  if (installing) return { state: 'saving', text: 'Saving for offline' };
  return { state: 'pending', text: 'Needs the network' };
}

/** Puts the status on a chip element, or hides it when there is nothing to say. */
export function showStatus(chip, status) {
  if (!chip) return;
  chip.textContent = status.text;
  chip.dataset.state = status.state;
  chip.hidden = false;
}
