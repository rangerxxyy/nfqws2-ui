# QUIC strategy picking

## Intent

Add QUIC (HTTP/3, UDP 443) to the existing single-site strategy picker. The picker should test a small compatible set of QUIC strategies through a temporary isolated nfqws2 process, show repeat-confirmed results, and allow a user to preview and explicitly apply one result to that domain.

## Goals

- Reuse the existing QUIC Initial generator, UDP response correlation, strategy catalog, operation persistence, cancellation, and isolated NFQUEUE infrastructure.
- Verify a direct baseline first and stop when QUIC already responds or the target cannot be checked safely.
- Run candidates sequentially with bounded retries; no parallel batches, multi-site sweeps, or refinement phase for QUIC.
- Require a domain name for SNI. Reject literal IP targets for QUIC.
- Apply only a repeat-confirmed result after an explicit preview and confirmation. Store a domain-scoped UDP profile in `NFQWS_ARGS_CUSTOM`.
- Keep QUIC separate from community TLS strategy candidates and TCP-only refinements.

## Non-goals

- Full HTTP/3 transfer or TLS handshake validation; a correlated QUIC Initial response is the reachability signal, matching the existing router implementation.
- Multi-site QUIC picker, large blockcheck2 catalogs, parallel candidate lanes, or automatic config writes.
- Changing existing HTTP/HTTPS picker behavior.

## Constraints and safety

- Use existing Rust dependencies and `backend/src/quic_initial.lua`; do not port the PHP implementation or add crypto dependencies.
- Candidate tests must use the current temporary test NFQUEUE and UDP-specific source-port-scoped firewall rules. Always remove owned rules and the test process, including cancellation and error paths.
- Generate fresh connection IDs for independent candidates; only the bounded retries for one candidate may reuse that candidate's packet.
- Keep existing repeat count limits (2–5). Restrict QUIC to the short standard/config/history/custom candidates, capped by the existing picker limit; force serial execution.
- Never change the live configuration during search. The confirmed result preview and existing apply transaction remain the only write path.

## Approaches

1. Port Zemidala's PHP QUIC packet and runner. Rejected because this app already has a Rust Initial generator, response correlation, temporary firewall rule ownership, operation state and cancellation.
2. Extend the Rust isolated site-trace runner with a QUIC probe mode, then route the existing strategy picker through it. Chosen: it reuses the app's cleanup/queue safety boundaries and avoids a second test runner.
3. Run the candidate against the live configuration without a temporary queue. Rejected because it cannot isolate candidate effects and could alter unrelated router traffic.

## User-facing behavior

- The transport selector offers `QUIC · HTTP/3 · UDP 443` only for one-site mode. Selecting QUIC switches to the bounded quick mode and explains that QUIC needs a domain and returns a correlated Initial response, not a full HTTP/3 page.
- The baseline and candidate results use the existing progress, cancellation, history and result cards. A direct QUIC response ends the search as already reachable.
- Preview explains that the candidate creates a `NFQWS_ARGS_CUSTOM` profile for the selected domain only. The preview shows the actual UDP profile and keeps the existing explicit confirmation, backup, dry-run, revision check, apply and rollback transaction.

## Builder decisions

- For a domain, build the profile with `--filter-udp=443 --filter-l7=quic --hostlist-domains=<domain> --payload=quic_initial` followed by validated steps.
- Keep candidate probes strictly sequential, even if HTTP/TLS full-catalog logic later uses batches.
- Do not treat a missing HTTP `Alt-Svc` advertisement as proof that QUIC is unavailable. A correlated direct QUIC Initial response ends the search early; otherwise run the bounded probe because some endpoints or proxies may accept QUIC without advertising it over HTTP.
- After applying a QUIC profile, verify the selected domain using a correlated QUIC Initial response. Do not use unrelated HTTP monitoring sites as the success criterion for a UDP-only profile.
- Treat a valid correlated Initial response as successful reachability; silence is an inconclusive failed attempt, not proof of DPI blocking.

## Acceptance criteria

- QUIC can be selected for a single domain, and literal IP, multi-site QUIC and unsupported full-catalog combinations are rejected or disabled with clear copy.
- Baseline and candidate UDP packets go through owned temporary rules; all rules/processes are cleaned up on success, failure, and cancellation.
- Only candidates successful on every configured repeat can be previewed/applied.
- The preview targets only the requested domain and QUIC transport; no unrelated profile or base arguments change.
- Frontend tests, Rust tests, typecheck, production build, formatting, clippy, and the release CI workflow pass.
