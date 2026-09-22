// Shared user-facing messages (app-shell "Error resilience").
export const NO_WASM =
  'This tool needs WebAssembly, which is disabled in this browser. geoprims runs in current Chrome, Edge, Firefox, and Safari (version 15 or newer) with WebAssembly turned on. The documentation and worked example on this page still apply.';

/**
 * What to tell a reader when a tool's reference data is not on this device
 * (web/offline-pwa, "Missing asset offline"): the dataset by its name, not
 * its file; whether they are offline; and what will fix it. The site caches
 * every data file on the first visit online, so the fix is to connect once.
 */
export function assetMessage(error, registry, online = true) {
  const asset = error?.asset ?? {};
  const entry = (registry?.assets ?? []).find((a) => a.id === asset.id);
  const name = entry?.title ?? 'reference data';
  if (error?.code === 'ASSET_INTEGRITY') {
    return {
      message: `The ${name} on this device failed its integrity check, so it was not used.`,
      hint: online ? 'Reload the page to fetch a fresh copy.' : 'Reconnect and reload to fetch a fresh copy.',
    };
  }
  return {
    message: online
      ? `This needs the ${name}, which is still downloading or was cleared from this device.`
      : `This needs the ${name}, which is not available offline on this device.`,
    hint: online
      ? 'Reload the page; it is saved for offline use as soon as it arrives.'
      : 'Connect once and open this page: the site saves its data for offline use on the first visit.',
  };
}
