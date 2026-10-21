# Changelog

## [Unreleased]

- Перенесены разметка, компоновка и встроенные стили из приложенного `nfqws2_ui_web_management_panel.html` в React/TSX-компонент `web/src/App.tsx`; Rust backend и TypeScript API-клиент остаются отдельными слоями.
- Внешние CDN-скрипты и inline JavaScript из HTML не загружаются: обработчики переведены в делегирование событий TypeScript, а существующие API-операции подключены к конфигурации, спискам, службе и журналам.
- Удалены встроенные демонстрационные значения и фиктивные действия; отсутствующая интеграция остаётся явно отключённой. По запросу пользователя полностью удалены вход, пароль, сессия и CSRF из панели и Rust API.
- Установщик больше не запрашивает пароль, удаляет устаревший файл пароля и после health-check печатает кликабельные ссылки с обнаруженными IPv4-адресами роутера и выбранным портом.
- Добавлены allowlist API для чтения и записи существующих списков с проверкой ревизии, валидацией, резервной копией, атомарной записью и повторным чтением.
- По активным literal-параметрам nfqws2 определяется фактический путь списка; неоднозначные ссылки не угадываются. Дедупликация сохраняет комментарии, порядок и первое вхождение, изменяя только локальный черновик до явного сохранения.
- Добавлен диагностический `GET /api/nfqws2/installation`; кнопка применения конфига доступна только после подтверждения backend, что файл доступен для записи.
- UI/API/Rust unit и integration tests, typecheck, production build и clippy проходят. Проверки на целевом роутере не выполнены; публичный релиз не создавался.

## [0.1.3] — перенос интерфейса из HTML-макета

- Перенесена структура предоставленного HTML-макета; старый React-интерфейс удалён, контроллер панели написан на TypeScript без runtime React.
- Удалена блокирующая стартовая авторизация; вход находится в отдельном пункте бокового меню и оформлен в стиле панели.
- Tailwind CSS и шрифты Inter/JetBrains Mono собираются локально, без CDN, чтобы интерфейс работал при строгом CSP роутера.
- Переведены inline-обработчики в делегирование событий; исправлены параллельные input/scroll действия полей конфигурации и списков.
- Добавлены проверки структуры меню, отдельного входа, отсутствия inline-обработчиков/CDN и сопоставления действий интерфейса с контроллером.
- Установщик теперь проходит актуальный ShellCheck для косвенно вызываемых trap-функций.
- Макетные функции без безопасного backend endpoint явно сообщают о недоступности и не изображают выполненную операцию.

## [0.1.2] — редизайн по HTML-макету

- Перестроена общая шапка по приложенному макету: достоверный статус nfqws2, RAM/CPU, версии, обновление статуса и реальные кнопки перезапуска/остановки/запуска.
- Сгруппирована боковая навигация; оставлены только существующие разделы.
- Обзор приведён к карточному расположению прототипа; значения и действия поступают из backend.
- Подогнаны тёмная палитра, контраст, полноэкранная компоновка и мобильный вид; сохранены светлая тема и reduced motion.
- Исправлен расчёт индикатора занятой RAM при неизвестных и некорректных значениях.

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
