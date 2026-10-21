# Changelog

## [0.1.1] — исправление установщика

- Исправлена подстановка версии в опубликованный установщик: шаблон проверки версии больше не переписывается при сборке.
- Релизная сборка запускает preflight на уже сгенерированном установщике и проверяет, что защита от незаполненной версии осталась на месте.
- В разделе конфигурации добавлен режим предварительного просмотра с явным переходом к редактированию и отменой несохранённых изменений.

## [0.1.0] — first public build

- Rebuilt the panel from a clean repository with React/TypeScript UI and a Rust local API.
- Added authenticated sessions with Argon2 password hashes, CSRF protection, rate limiting, security headers, and conservative service status.
- Added nfqws2 start/stop/restart via the detected init script; status requires init confirmation and a matching PID/cmdline.
- Added a save-and-apply configuration action that uses only a confirmed service state and the supported init start/restart command.
- Marked conflicting init and live-process status as an error instead of reporting a false stopped state; CPU sampling excludes guest counters and rounds to one decimal place.
- Bounded config writes and restores to 512 KiB before modifying files, and made temporary replacement files unique.
- Limited log reads to the final 256 KiB before decoding, so a large router log cannot consume unbounded memory.
- Added RAM/CPU status, config editing with SHA revision checks and backups, backup restore, bounded logs, and responsive RU/EN/ZH/TR UI with light/dark themes.
- Added an ARM64 Entware installer with archive checksum/path validation, explicit `sha256sum` prerequisite (no implicit Entware package changes), automatic port selection from 1002, health check, update rollback, plus a panel-only uninstaller.
- Real Keenetic service actions are not yet claimed as tested because SSH authentication to the available router failed.
