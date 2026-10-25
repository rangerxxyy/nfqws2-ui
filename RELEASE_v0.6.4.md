# Release v0.6.4

## Changes

- Strategy picking and blockage diagnosis accept a public IPv4 literal as the target.
- Strategy previews scope the candidate to that address with nfqws2's `--ipset-ip` selector.
- IP diagnosis skips DNS, DoH, SNI comparison, and QUIC, and compares the direct connection with the current router route.
- Private/reserved targets and IPv6 are rejected with clear validation messages because the isolated test firewall currently supports IPv4 only.

## Verification

- Backend locked test suite and formatter.
- Web unit tests and production build.
- The router and public network were not used for live probes; firewall isolation and cleanup were only covered by the existing backend tests.
