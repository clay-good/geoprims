import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const web = fileURLToPath(new URL('../..', import.meta.url));

export async function serveBuiltSite(t) {
  const server = spawn(process.execPath, ['scripts/serve.mjs', '0'], { cwd: web });
  t.after(() => server.kill());
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.once('exit', (code) => reject(new Error(`Site server exited: ${code}`)));
    server.stdout.on('data', (chunk) => {
      const match = /localhost:(\d+)/.exec(chunk.toString());
      if (match) resolve(`http://127.0.0.1:${match[1]}`);
    });
  });
}

export async function captureCsp(context) {
  const violations = [];
  await context.exposeBinding('__recordCspViolation', (source, detail) => {
    violations.push({ page: source.frame.url(), ...detail });
  });
  await context.addInitScript(() => {
    document.addEventListener('securitypolicyviolation', (event) => {
      window.__recordCspViolation({ directive: event.violatedDirective, blocked: event.blockedURI });
    });
  });
  return violations;
}
