// Preloaded by the network audit: reports any attempt to open a connection
// or resolve a name, so the audit fails even where the sandbox is silent.
import dns from 'node:dns';
import net from 'node:net';

const report = (what) => process.stderr.write(`NETWORK ATTEMPT: ${what}\n`);
const connect = net.Socket.prototype.connect;
net.Socket.prototype.connect = function (...args) {
  // stdio pipes are not network: only report TCP and named-socket connects.
  report(`connect ${JSON.stringify(args[0]?.host ?? args[1] ?? args[0]?.path ?? args[0])}`);
  return connect.apply(this, args);
};
const lookup = dns.lookup;
dns.lookup = (host, ...rest) => {
  report(`dns ${host}`);
  return lookup(host, ...rest);
};
const realFetch = globalThis.fetch;
globalThis.fetch = (url, ...rest) => {
  report(`fetch ${url}`);
  return realFetch(url, ...rest);
};
