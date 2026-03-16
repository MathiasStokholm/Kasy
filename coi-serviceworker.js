/**
 * Cross-Origin Isolation service worker.
 *
 * GitHub Pages cannot set HTTP response headers, but Chrome requires
 *   Cross-Origin-Opener-Policy: same-origin
 *   Cross-Origin-Embedder-Policy: require-corp
 * for WebGPU to be available.  This service worker intercepts every
 * same-origin fetch and re-stamps those two headers onto the response,
 * giving the page the cross-origin-isolated context that Chrome needs.
 *
 * The companion script in index.html registers this worker and reloads
 * the page once the worker has taken control so that the first real load
 * already benefits from the added headers.
 */

self.addEventListener("install", () => {
  // Skip the "waiting" phase so the new worker activates immediately.
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  // Claim all open clients (tabs) right away instead of waiting for them
  // to navigate.
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  // Requests with cache mode "only-if-cached" must have mode "same-origin";
  // skip anything that doesn't satisfy that constraint to avoid a TypeError.
  if (
    event.request.cache === "only-if-cached" &&
    event.request.mode !== "same-origin"
  ) {
    return;
  }

  event.respondWith(
    fetch(event.request)
      .then((response) => {
        // Opaque responses (status 0) cannot be reconstructed – pass through.
        if (response.status === 0) {
          return response;
        }

        const headers = new Headers(response.headers);
        headers.set("Cross-Origin-Opener-Policy", "same-origin");
        headers.set("Cross-Origin-Embedder-Policy", "require-corp");

        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        });
      })
      .catch((err) => {
        console.error("[coi-sw] fetch error:", err);
        return new Response("Network error", { status: 408 });
      })
  );
});
