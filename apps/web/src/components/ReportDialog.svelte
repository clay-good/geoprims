<script>
  // The problem-report dialog (feedback/problem-reports). Loaded only when the
  // user clicks "Report a problem". It shows exactly what will be sent, loads
  // the bot check only now, posts once, and never retries on its own.
  import { onMount } from 'svelte';
  import { buildPayload, openState, reportText, sendState, tokenIsFresh, ISSUE_URL, LIMITS, viewportClass } from '../lib/report.js';

  let { tool, args, result, onclose } = $props();

  let dialog;
  let status = $state('loading'); // loading | ready | paused | offline | sending | sent | failed
  let includeInputs = $state(true);
  let note = $state('');
  let kind = $state('wrong-result');
  let copied = $state(false);
  let widget = null;
  let token = '';
  let tokenAt = 0;
  let tokenWaiters = [];

  const theme = () =>
    document.documentElement.dataset.theme ?? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  const payload = $derived(
    buildPayload({
      tool,
      args,
      result,
      includeInputs,
      note,
      kind,
      display: { theme: theme(), unitProfile: 'default', viewportClass: viewportClass(innerWidth) },
      pagePath: location.pathname + location.hash,
    }),
  );
  const preview = $derived(JSON.stringify({ ...payload, token: '(added when you send)' }, null, 2));

  function loadTurnstile() {
    if (window.turnstile) return Promise.resolve();
    return new Promise((resolve, reject) => {
      const s = document.createElement('script');
      s.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit';
      s.async = true;
      s.onload = resolve;
      s.onerror = reject;
      document.head.append(s);
    });
  }

  onMount(async () => {
    dialog.showModal();
    status = await open();
  });

  async function open() {
    if (!navigator.onLine) return 'offline';
    try {
      const r = await fetch('/api/reports/config', { credentials: 'omit' });
      const config = r.ok ? await r.json() : null;
      const next = openState({ online: navigator.onLine, config });
      if (next !== 'ready') return next;
      await loadTurnstile();
      widget = window.turnstile.render('#report-check', {
        sitekey: config.sitekey,
        action: 'problem-report',
        appearance: 'interaction-only',
        'refresh-expired': 'auto',
        callback: (t) => {
          token = t;
          tokenAt = Date.now();
          tokenWaiters.splice(0).forEach((w) => w(t));
        },
      });
      return 'ready';
    } catch {
      return openState({ online: navigator.onLine, config: null });
    }
  }

  /** A bot-check token no older than the shared maximum (contracts/report-api). */
  function freshToken() {
    if (tokenIsFresh(token, tokenAt, Date.now())) return Promise.resolve(token);
    token = '';
    window.turnstile.reset(widget);
    return new Promise((resolve) => tokenWaiters.push(resolve));
  }

  async function send() {
    status = 'sending';
    try {
      const t = await freshToken();
      const r = await fetch('/api/reports', {
        method: 'POST',
        credentials: 'omit',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ...payload, token: t }),
      });
      status = sendState(r.status);
    } catch {
      status = 'failed';
    }
  }

  async function copyReport() {
    await navigator.clipboard.writeText(reportText(payload));
    copied = true;
  }

  function close() {
    dialog.close();
    onclose();
  }
</script>

<dialog bind:this={dialog} class="report" aria-labelledby="report-title" onclose={onclose}>
  <h2 id="report-title">Report a problem</h2>
  <p>
    This sends the tool, its version, the inputs and results shown below, and your note to geoprims. Nothing else: no
    account, no address, no device details. A Cloudflare bot check runs only while this dialog is open.
  </p>

  {#if status === 'paused'}
    <p role="status">Reporting is paused right now. You can <a href={ISSUE_URL}>open a "Wrong answer" issue</a> instead, or copy the report.</p>
    <div class="actions"><button type="button" onclick={copyReport}>{copied ? 'Copied' : 'Copy report'}</button><button type="button" onclick={close}>Close</button></div>
  {:else if status === 'offline'}
    <p role="status">Reporting needs a connection. Copy the report now and send it later; nothing is queued.</p>
    <div class="actions"><button type="button" onclick={copyReport}>{copied ? 'Copied' : 'Copy report'}</button><button type="button" onclick={close}>Close</button></div>
  {:else if status === 'sent'}
    <p role="status">Report sent. Thanks. Updates appear on the <a href="/known-issues/">known-issues page</a>.</p>
    <div class="actions"><button type="button" onclick={close}>Close</button></div>
  {:else}
    <form onsubmit={(e) => { e.preventDefault(); send(); }}>
      <label>
        What kind of problem?
        <select bind:value={kind}>
          <option value="wrong-result">Wrong result</option>
          <option value="broken">Something is broken</option>
          <option value="confusing">Confusing</option>
          <option value="other">Other</option>
        </select>
      </label>
      <label>
        Note (optional)
        <textarea bind:value={note} maxlength={LIMITS.noteChars} rows="3" placeholder="Expected 7,900 ft per the POH chart"></textarea>
        <span class="help" aria-live="polite">{note.length} / {LIMITS.noteChars}</span>
      </label>
      <label class="check"><input type="checkbox" bind:checked={includeInputs} /> Include my inputs and results</label>
      <details open>
        <summary>Exactly what will be sent</summary>
        <pre class="payload">{preview}</pre>
      </details>
      <div id="report-check"></div>
      <p role="status" aria-live="polite">
        {#if status === 'loading'}Loading the bot check…{:else if status === 'sending'}Sending…{:else if status === 'failed'}Couldn't send. Copy the report instead?{/if}
      </p>
      <div class="actions">
        <button type="submit" disabled={status !== 'ready' && status !== 'failed'}>Send report</button>
        {#if status === 'failed'}<button type="button" onclick={copyReport}>{copied ? 'Copied' : 'Copy report'}</button>{/if}
        <button type="button" onclick={close}>Cancel</button>
      </div>
    </form>
  {/if}
</dialog>
