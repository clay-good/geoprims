// The preview deploy only. It serves the same static build as production and
// adds one header: the workers.dev URL must never be indexed while the product
// is still unreleased. Production (geoprims.com) does not run this.
export default {
  async fetch(request, env) {
    const response = await env.ASSETS.fetch(request);
    const headers = new Headers(response.headers);
    headers.set('X-Robots-Tag', 'noindex, nofollow, noarchive');
    return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
  },
};
