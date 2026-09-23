// www.geoprims.com → geoprims.com. One permanent redirect, path and query
// kept, so a typed or linked www address lands on the canonical page. It
// reads nothing about the visitor and sets nothing.
export default {
  fetch(request) {
    const url = new URL(request.url);
    return Response.redirect(`https://geoprims.com${url.pathname}${url.search}`, 301);
  },
};
