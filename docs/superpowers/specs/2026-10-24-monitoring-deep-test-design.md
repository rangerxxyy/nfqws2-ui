# Monitoring Deep Test Design

## Intent

Make manual monitoring report whether a site resolves and completes a real HTTPS request through the router's current route, with the stage timings and bounded body amount visible in the UI.

## Goals

- Keep manual checks and the existing limit of 20 hosts / 10 saved results per host.
- Check DNS, TCP connect, TLS handshake, HTTP response, response transfer, and elapsed timings.
- Bound each HTTPS request to 64 KiB and 8 seconds; pin it to a resolved public IPv4 address.
- Mark an HTTP response followed by a body timeout as a possible transfer interruption, not a proven DPI cause.
- Keep concurrent host checks and save the detailed steps in browser history.

## Non-goals and constraints

- No scheduled checks, notifications, configuration changes, or strategy detection.
- Do not submit a POST body to monitored sites. Zemidala may use an upload probe when the response is small; this panel will avoid that side effect while matching its bounded HTTPS GET and error distinctions.
- Do not follow redirects; report the first HTTP response as the check result.

## Approaches

- Reuse the full site diagnosis: it is comprehensive but slow and also performs direct-route comparisons, QUIC and policy checks that a monitor does not need.
- Add one bounded HTTPS GET per domain and expose its timing milestones: selected because it matches Zemidala's normal monitor request while keeping the manual batch quick.
- Probe TCP, TLS and HTTP independently: this creates redundant connections and can misrepresent the exact request route.

## Decision

Use DNS resolution followed by a curl HTTPS GET pinned to a public IPv4, with IPv4, no redirects, a 64 KiB range/maximum and an 8 second limit. Derive DNS/TCP/TLS/HTTP/transfer steps from the resolver and curl metrics, store them with each result, and render them under a compact disclosure.
