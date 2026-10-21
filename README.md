# nfqws2-ui

Локальная панель управления службой nfqws2 для Keenetic с Entware на TypeScript и Rust. Shell используется только для установки, удаления и init-интеграции.

## Установка

Первый релиз предназначен для 64-битных ARM-устройств Entware. Нужны `wget`, `tar` и `sha256sum` (Entware: `coreutils-sha256sum`). Если `sha256sum` отсутствует, установите зависимость отдельно командой `opkg update && opkg install coreutils-sha256sum`. Установщик не меняет состояние пакетов Entware. Он задаёт пароль панели в интерактивном терминале, скачивает архив релиза, сверяет SHA-256, сохраняет конфигурацию и проверяет HTTP health endpoint.

```sh
wget -qO- https://github.com/rangerxxyy/nfqws2-ui/releases/latest/download/install.sh | sh
```

После установки откройте `http://<LAN-IP-маршрутизатора>:1002/`. Если порт 1002 занят, backend попробует следующий свободный порт до 1100; выбранный порт хранится в `/opt/var/run/nfqws2-ui.port`.

Обновление выполняется той же командой. Пароль и резервные копии сохраняются. При ошибке запуска/HTTP-проверки установщик возвращает предыдущую версию. Удаление панели: `/opt/share/nfqws2-ui/current/uninstall.sh`; `--purge` также удаляет пароль и резервные копии панели. Конфигурация nfqws2 не удаляется.

## Сборка и проверки

Требуются Rust stable, Node.js 22 и pnpm 10.

```sh
cargo fmt --manifest-path backend/Cargo.toml --all -- --check
cargo test --manifest-path backend/Cargo.toml --locked
cargo clippy --manifest-path backend/Cargo.toml --all-targets --locked -- -D warnings
pnpm --dir web install --frozen-lockfile
pnpm --dir web test
pnpm --dir web build
```

Для статического ARM64 Linux-артефакта используется `cross` с target `aarch64-unknown-linux-musl`. Публичный релиз публикуется GitHub Actions только после прохождения Rust, frontend и target-тестов.

## Безопасность и ограничения

- Панель слушает `0.0.0.0`, защищает API паролем (Argon2), сессией HttpOnly/SameSite и CSRF-токеном. Ограничьте доступ LAN/firewall роутера.
- Действия сервиса вызывают только штатный init-скрипт nfqws2; произвольные shell-команды из API не принимаются.
- Для конфигурации разрешён только `nfqws2.conf`; перед записью создаётся резервная копия, проверяется ревизия, файл заменяется атомарно.
- Данные панели: пароль `/opt/etc/nfqws2-ui/password.hash`, backups `/opt/var/lib/nfqws2-ui/backups`, журнал панели `/opt/var/log/nfqws2-ui.log`.
- Первый релиз предназначен для ARM64 Entware. OpenWrt/procd и другие архитектуры не объявляются поддержанными до отдельных сборочных и аппаратных проверок.
- В текущей среде не удалось пройти SSH-аутентификацию на тестовый роутер; действия start/stop и установленная совместимость на реальном Keenetic пока не подтверждены.

## Лицензия

MIT; см. [LICENSE](LICENSE).
