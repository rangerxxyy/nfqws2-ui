# Резервные копии, оформление и сведения о панели — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Добавить три системных страницы с управлением снимками конфига, тремя темами и сведениями о версии и командах nfqws2-UI.

**Architecture:** Страницы добавляются в существующий `App.tsx`/`panel.ts` и роутер. Полные tar.gz-снимки и NAS-передача реализуются в Rust backend, сохраняя совместимость старых config backup endpoints. Отдельный модуль управляет темой и компактной навигацией через localStorage; страница о программе использует существующий API релизов.

**Tech Stack:** React TSX как шаблон, TypeScript, Tailwind/CSS, Axum/Rust backend API.

**Spec:** `docs/superpowers/specs/2026-10-24-backups-appearance-about.md`

## Global Constraints

- Текущая тёмная тема остаётся начальным значением.
- Темы сохраняются только в localStorage браузера.
- Снимки ограничены `nfqws2.conf`, списками, blobs и whitelisted UI state; приватный NAS key всегда исключён.
- Восстановление архивов не допускает traversal, ссылки и пути вне разрешённого корня.
- Compact mode всегда оставляет доступ к странице оформления.

## Review Focus

- Повреждённое или недоступное localStorage: применить стандартные тему и вид, не ломать запуск.
- Пустой список резервных копий: показать понятное пустое состояние и доступное действие создания.
- Ошибка API при восстановлении/проверке версии: сохранить страницу и показать причину.
- Список резервных копий с недопустимыми ID: передавать только ID от API и применять существующую серверную валидацию.
- Узкий экран: новые страницы должны переносить кнопки и команды без горизонтального переполнения.

---

### Task 1: Добавить полные снимки и управление архивами в backend

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/panel.ts`
- Modify: `web/src/panel-template.test.ts` (обновить устаревшее ожидание, что раздел снимков отсутствует)
- Modify: `backend/Cargo.toml`
- Modify: `backend/Cargo.lock`

**Interfaces:**
- Produces: `/api/snapshots` metadata/settings, create/list/archive/upload-restore/pin/retention endpoints; safe archive helpers; NAS settings/key/test/push endpoints; daily scheduler.

- [ ] Добавить архивирование конфига, list-файлов, blobs и whitelisted UI state; никогда не архивировать закрытый NAS ключ.
- [ ] Добавить индекс с причиной/заметкой/размером/закреплением/локальным и NAS состоянием, pruning по сроку и количеству, с минимумом пяти локальных снимков.
- [ ] Добавить ежедневный запуск в 04:05 и снимки до изменений через интерфейс.
- [ ] Добавить скачивание актуального архива/копии, upload `.tar.gz` с ограничением размера и восстановление после строгой проверки путей.
- [ ] Добавить ограниченную NAS-команду, сохранение настроек, генерацию ключа, тест связи, ручную отправку и фоновую отправку новых снимков.
- [ ] Сохранить обратную совместимость текущих `/api/backups` routes.

### Task 2: Добавить страницу резервных копий

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/panel.ts`
- Modify: `web/src/api.ts`
- Modify: `web/src/panel-template.test.ts` (обновить устаревшее ожидание, что раздел снимков отсутствует)

**Interfaces:**
- Consumes: API routes from Task 1.
- Produces: `snapshots` page matching supplied screenshot, including retention, download/import, NAS setup and snapshot history.

- [ ] Добавить карточки «На роутере», «Вне роутера», «Копии на NAS» и таблицу снимков.
- [ ] Добавить загрузку списка, создание снимка с подписью, скачивание, загрузку и восстановление архива, закрепление и понятный результат.
- [ ] Добавить настройки NAS, создание/копирование открытого ключа и безопасной команды, сохранение, тест подключения и отправку сейчас.
- [ ] Показывать местоположение архива, ошибки и состояние очереди передачи.

### Task 3: Добавить темы и компактную навигацию

**Files:**
- Create: `web/src/appearance.ts`
- Modify: `web/src/App.tsx`
- Modify: `web/src/panel.ts`
- Modify: `web/src/panel.css`

**Interfaces:**
- Produces: `loadAppearance(storage): AppearanceSettings`, `saveAppearance(storage, settings): AppearanceSettings`, `applyAppearance(settings, root): void`.
- `AppearanceSettings` uses `theme: 'dark' | 'light' | 'neon'` and `layout: 'standard' | 'compact'`.

- [ ] Добавить навигацию и панель «Оформление UI» с тремя темами и вариантами стандартной/компактной навигации.
- [ ] Добавить отдельные CSS переменные и читаемые overrides для карточек, полей, боковой панели, верхней панели, текста и состояний каждой темы.
- [ ] Компактная навигация скрывает только редко используемые технические пункты; разделы конфигурации, списков, проверки, стратегий, мониторинга и все системные страницы остаются доступны.
- [ ] Инициализировать оформление до `bootPanel()` и корректно переживать недоступный или повреждённый localStorage.

### Task 4: Добавить страницу «О nfqws2-UI»

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/panel.ts`

**Interfaces:**
- Consumes: `nfqwsUpdates.uiStatus(): Promise<UiUpdateStatus>`.
- Produces: route `about`, status renderer and `checkUiUpdate` action.

- [ ] Добавить страницу со ссылкой на GitHub репозиторий, текущей версией, состоянием доступной версии и действием проверки.
- [ ] Показать все пять запрошенных команд с краткими объяснениями и отдельными копируемыми блоками.
- [ ] Обработать ошибку проверки, оставляя текущую версию и команды видимыми.

### Task 5: Финальная сверка интерфейса и проекта

**Files:**
- Review: `web/src/App.tsx`, `web/src/panel.ts`, `web/src/panel.css`, `web/src/appearance.ts`

- [ ] Проверить вручную diff на ошибки маршрутов, повторяющиеся ID, контраст и сохранение текущей темы по умолчанию.
- [ ] Из-за текущего ограничения не добавлять и не запускать проверки; сообщить об этом явно.
