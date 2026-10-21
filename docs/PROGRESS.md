# Журнал прогресса

## 2026-10-21 — чистый перезапуск

- Подтверждено: удалённый репозиторий не восстанавливать; новая папка проекта пустая и независимая.
- Проверены справочные upstream-клоны и предоставленный HTML-дизайн; краткие выводы сохранены в `UPSTREAM_ANALYSIS.md` и `DESIGN_ANALYSIS.md`.
- SSH к роутеру не прошёл publickey/password аутентификацию; безопасные проверки на устройстве пока недоступны, никаких изменений на роутере не сделано.
- Реализованы TypeScript/React UI, Rust API, auth+CSRF, статус и метрики, allowlisted config/backups, логи, port selection/persistence, обновляемый install/uninstall и Entware init integration.
- Первый установочный пакет ограничен ARM64 Entware; под `zemidala/nfqws2-ui` у текущего аккаунта только чтение, создан будет независимый новый репозиторий пользователя.
- Локальная проверка пройдена: Rust 14 тестов, Clippy, TypeScript 5 тестов и prod build, HTTP smoke; `dash -n` прошёл.
- Cross-target `cargo check` сначала нашёл Unix-only async compile issue; исправлено, повторный `aarch64-unknown-linux-musl` check прошёл.
- Реальный роутер пока не тестировался: SSH key auth отказал; не запускались/не останавливались nfqws2 и не изменялись роутерные файлы.
- GitHub CI прошёл: Rust tests/Clippy/cargo-audit, TypeScript tests/build/pnpm audit, dash syntax и ShellCheck.
- Добавлены регрессионные проверки расхождения init/PID и точности CPU; CPU-метрика исключает guest-счётчики Linux.
- Запись/восстановление конфигурации ограничены 512 КБ, временные файлы уникальны и создаются только после этих проверок.
- Добавлена команда «Сохранить и применить» для конфигурации; применяется только при подтверждённом состоянии службы и использует штатный start/restart.
- Запущен release dry-run: cross-test/build ARM64, проверка static binary, упаковки, версии установщика и SHA-256; ручной workflow не публикует релиз.
- Далее: завершить dry-run, проверить итоговые assets и CI, затем создать публичный v0.1.0; реальный Keenetic smoke-test остаётся непроверенным.
- Final review fixes: log endpoint now reads only a bounded tail; installer shows the exact checksum dependency command instead of running `opkg`; installer preflight regression coverage added.
