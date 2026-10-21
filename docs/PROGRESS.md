# Журнал прогресса

## 2026-10-21 — чистый перезапуск

- Подтверждено: удалённый репозиторий не восстанавливать; новая папка проекта пустая и независимая.
- Проверены справочные upstream-клоны и предоставленный HTML-дизайн; краткие выводы сохранены в `UPSTREAM_ANALYSIS.md` и `DESIGN_ANALYSIS.md`.
- SSH к роутеру не прошёл publickey/password аутентификацию; безопасные проверки на устройстве пока недоступны, никаких изменений на роутере не сделано.
- Реализованы TypeScript/React UI, Rust API, auth+CSRF, статус и метрики, allowlisted config/backups, логи, port selection/persistence, обновляемый install/uninstall и Entware init integration.
- Первый установочный пакет ограничен ARM64 Entware; под `zemidala/nfqws2-ui` у текущего аккаунта только чтение, создан будет независимый новый репозиторий пользователя.
- Локальная проверка пройдена: Rust 8 тестов, Clippy, TypeScript 3 теста и prod build, HTTP smoke; `dash -n` прошёл.
- Cross-target `cargo check` сначала нашёл Unix-only async compile issue; исправлено, повторный `aarch64-unknown-linux-musl` check прошёл.
- Реальный роутер пока не тестировался: SSH key auth отказал; не запускались/не останавливались nfqws2 и не изменялись роутерные файлы.
- Далее: добавить CI/release workflow, прогнать все gates на новом GitHub repo, собрать ARM64 артефакт, проверить install asset и публиковать только если проверки зелёные.
