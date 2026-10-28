# v1.11.2 — Более частая проверка статуса обновления ядра

Панель опрашивает лёгкий статус обновления nfqws2 раз в пять минут, как XKeen UI. Кэш каталога релизов остаётся четырёхчасовым: повторные проверки статуса не скачивают релизы из GitHub.

Проверки перед публикацией: frontend suite, TypeScript, production build, Rust suite, fmt, Clippy и cargo audit; release workflow проверит установщик и ARM64 cross-build.
