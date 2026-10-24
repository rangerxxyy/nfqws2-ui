# Monitoring and strategy update plan

1. Keep baseline connectivity as diagnostic information and always proceed to candidate tests after a completed baseline, including HTTP 301.
2. Move monitor settings, site list, results history, and Telegram configuration to backend JSON storage with API routes; mask stored bot token in reads.
3. Add backend interval worker (30m, 1h, 2h, custom) and transition alerts; implement Telegram test message and chat discovery.
4. Add separate Monitoring and Telegram sections using existing panel styles; persist settings through API and refresh state.
5. Increment version to 0.5.2, run build and focused verification, publish through the existing release workflow, and inspect its result.

