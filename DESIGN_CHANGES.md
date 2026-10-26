# v0.6.0 visual audit

## Журнал: прокрутка при обновлении

- При первом получении журнала область теперь прокручивается к последним строкам, чтобы пользователь сразу видел актуальное состояние службы.
- Автообновление запоминает, находился ли читатель у нижней границы: если да, новые строки остаются видимыми; если пользователь прокрутил вверх, его позиция не сбрасывается.
- Порог следования за концом — 24 px, чтобы небольшое расхождение из-за округления высоты строк не отключало автопрокрутку.
- Поведение покрыто unit-тестами для первой загрузки, чтения конца и чтения старых строк.

## Штатные профили в конструкторе

- Конструктор теперь доступен для пяти штатных групп nfqws2: UDP, QUIC по IP, QUIC по сайтам, HTTP/HTTPS по IP и HTTP/HTTPS по сайтам.
- Backend выдаёт все пять штатных профилей даже при пустых переменных конфигурации: новый транспорт можно настроить прямо из конструктора, не переходя сначала в сырой конфиг.
- Изменения фильтров и шагов записываются в их исходные переменные (`NFQWS_ARGS_UDP`, `NFQWS_ARGS_QUIC`, `NFQWS_ARGS`); связанные списки остаются отдельными компонентами (`NFQWS_ARGS_IPSET`, `NFQWS_EXTRA_ARGS`). Общие переменные помечены, чтобы было видно влияние на парный профиль.
- При открытии конструктора связанные списки и дополнительные переменные теперь раскрыты сразу. Для `NFQWS_ARGS_IPSET` и `NFQWS_EXTRA_ARGS` интерфейс называет обе группы профилей, которые используют эти переменные.
- В связанных IP- и доменных списках штатного профиля существующие аргументы теперь видны отдельными полями: их можно изменить или удалить по одному. Текстовый режим и режим только чтения для `MODE_*` сохраняют исходные значения.
- Выбор списков направляется в связанную переменную; UDP, не имеющий связанного компонента, сохраняет списки в собственной группе. Ссылки `NFQWS_EXTRA_ARGS` на общий режим доступны только для чтения, чтобы не потерять связь с `MODE_LIST`/`MODE_AUTO`/`MODE_ALL`.
- Backend по-прежнему валидирует переменные, выполняет проверку через `nfqws2 --dry-run`, создаёт резервную копию и применяет транзакционно.

## Следующая итерация конструктора профилей

- Длинный каталог параметров доступен с клавиатуры: стрелки перемещают фокус по вариантам с переходом через края, Home и End ведут к началу и концу; поле поиска переводит стрелкой в каталог.
- Варианты выбора остаются сгруппированы по назначению, а группы и список имеют явные подписи для вспомогательных технологий. После фильтрации порядок фокуса пересобирается вместе с результатами.

## v1.2.1 — загрузки и проверка службы

- Скрипты панели выбирают `/opt/bin/wget`, если он установлен, прежде чем искать `wget` в `PATH`. На роутерах BusyBox `wget` может первым разрешаться в `PATH` и аварийно завершаться; явный путь к Entware-клиенту сохраняет загрузку релизов и локальную HTTP-проверку доступности панели.
- Установщик, обновление панели, обновление nfqws2 и команда `status` используют единый выбор клиента загрузки.

## Reference and existing UI

The XKeen UI source uses the variable Inter family for interface text and JetBrains Mono for code. This panel already packages both fonts and uses them in its Tailwind sans/mono stacks, so the audit keeps the same local font files and does not add a network font request. The panel also already has a shared keyboard accessible select, visible focus rings, light-theme select colors, panel transitions, and a reduced-motion mode.

## v0.6.0 adjustments

- Set the local Inter family and native dark/light color scheme on the document root so menus and controls follow the selected theme consistently.
- Set a comfortable shared line height and a 12 px minimum for explanatory and tabular text in every main panel. Small screens receive a little more line spacing.
- Give the existing custom select menu a short entrance transition and translucent depth while retaining its current keyboard, pointer, and selection behavior.
- Disable the select entrance animation when the user requests reduced motion.

## Panel audit

| Area | Existing pattern retained | v0.6.0 check |
| --- | --- | --- |
| Header and service state | Inter labels, semantic status colors, accessible action controls | Start/stop text and icon use a shared state presentation |
| Configuration and list editors | JetBrains Mono, line gutters, syntax coloring | Long values wrap or scroll within their editor; theme contrast remains explicit |
| Diagnosis and strategy picker | Ordered result cards and status tones | Explanations meet the shared minimum text size |
| Monitoring and Telegram | Responsive cards and status bars | Field labels and history remain readable on narrow screens |
| Appearance and backups | Theme/layout cards and grouped settings | Existing light/neon overrides stay scoped to their theme |
| About and release controls | Monospace commands and version pills | Focus, hover, and select states use the same shared interaction language |

## v1.0.5 interface pass — 2026-10-25

- Updated the shared custom select to match the compact dark menu reference: selected values keep a blue highlight, readable white text and a checkmark; hover/focus use the same blue family. Opening and closing animate, and reduced-motion disables the motion.
- Lifecycle toasts now remain deduplicated across their enter, visible and exit periods. The stack is capped at four, accounts for mobile safe areas and can scroll if several long notices need more room. Dismissal is idempotent and animated.
- Toast overflow now counts only toast cards, preserving persistent diagnosis progress messages in the shared notification host. Old cards leave through the same exit animation instead of disappearing abruptly.
- The log viewer now lets readers pause its five-second polling while retaining manual refresh. Background tabs do not poll, and overlapping reads are suppressed so a delayed response cannot replace the currently selected source.
- The backups page now has one persistent switch for scheduled and pre-edit automatic snapshots; manual snapshots remain available, and restoring a snapshot retains its safety copy. Backup rows and actions use a denser two-line layout.
- On phone-sized screens, the document itself now owns vertical scrolling so a finger swipe works across the header, navigation and page content instead of requiring a nested scroll area. Inner code editors keep their own scroll behavior.
- Successful service-control responses now return a concise lifecycle message after backend verification; init-script stdout/stderr remains available to error handling, not normal success text.
- Delayed action tooltips keep the existing two-second delay, skip touch-only pointers, and clear when the page scrolls or resizes so a tooltip cannot linger over a different control.
- Release notes retain safe Markdown links and formatting; strong text is white in the dark theme and follows the foreground in the light theme. Version action icons use a 3 px alignment adjustment.
- Site diagnosis now includes active host/IP lists used by the running process, in addition to standard lists. Only files that resolve inside the configuration directory are read; symlinks and paths outside it are ignored.
- The panel continues to use its bundled Inter Variable and JetBrains Mono files. No remote font dependency or new UI framework was added.

### Verification

- Frontend: 29 test files, 241 tests passed; TypeScript production build completed.
- Backend: Rust formatting and the full Cargo test suite completed successfully.
- Chromium local preview reached the designed “Нет связи с панелью” state. The router backend was not available locally, so populated pages and their interactive flows were not visible in that session.
- The GitHub release workflow completed the `aarch64-unknown-linux-musl` cross-test/build and package preflight for v1.0.5. No physical router was available for deployment checks.

## Next interface iteration

- While the site diagnostic request is running, one queued stage now receives a moving active highlight. All other stages remain pending until the router returns the report, so the animation does not claim unconfirmed checks succeeded. The result view then walks through the returned per-stage statuses.
- A closing dropdown is removed from the accessibility tree immediately while its visual exit animation finishes; reopening restores the listbox state.
- Appearance palette cards now honor reduced-motion preferences consistently: selecting an accent palette does not retain hover movement or transitions when motion is disabled.
- The release-note renderer now preserves GitHub table structure, column alignment, inline emphasis, code and safe links. Tables scroll horizontally inside the changelog panel on narrow screens, with matching dark and light theme surfaces.
- The shared select menu now chooses the side with usable viewport space and repositions after page scroll or resize; its upward opening and closing motion follows the same reduced-motion setting as the downward menu.
- Action tooltips now dismiss on activation and when their focused or hovered button is removed during a view change, preventing a stale label from floating over a different panel.
- Select options now use the screenshot-matched hierarchy: the selected row stays dark with a white checkmark, while pointer hover and keyboard focus use XKeen's restrained 15% blue highlight and blue text. Row height, type size and maximum menu height follow the compact timezone-list reference.
- Opening a select with the pointer leaves the selected row neutral, as in the timezone reference; keyboard navigation sets an active row and receives the blue focus treatment.
- Long select menus can use up to 34rem of available viewport height; the TypeScript positioner no longer clamps every menu to 256px.
- The release picker now offers an in-place retry for both an empty eligible-version list and a failed release request, matching the recovery affordance in the reference updater.
- About-panel command rows now reserve an intrinsic copy-button column and wrap long command text in the remaining space, following the upstream narrow-screen overlap fix.
- Select menus now render in the document overlay layer and use fixed viewport positioning, so long timezone and settings lists remain visible above scrollable panels. Their dark surface and border use the same color tokens as XKeen UI; options keep the compact 14 px/30 px rhythm, neutral selected row with a white vector check, and 15% blue hover/keyboard highlight.
- The select menu remains in front of editor panels but below confirmation dialogs, while outside-click detection continues to distinguish the portaled menu from unrelated page clicks.

## Diagnosis and list editor — 2026-10-25

- Diagnosis queue rows now use compact divider lines, a neutral pending dot, and success/error markers instead of visible step numbers. The ordered list remains a semantic list with an explicit list role and accessible row status.
- Per-step results remain visible long enough to read before the next stage activates. The final QUIC row summarizes each route with its result, HTTP code, and latency; backend detail paragraphs are no longer duplicated in the interface. The separate note states only whether HTTP/3 was checked or advertised.
- The `.list` editor no longer reserves a full row for search. Its text/list toggle now sits alongside deduplication and apply actions, with file import/export and selected-row removal on a compact utility row.
- Clearing a list edits only the active draft and requires an in-panel confirmation. A shared toolbar button expands any active `.list` into the animated near-fullscreen editor; Escape restores the panel to its original location.
- Text mode now uses a 13.5 px editor font with a 24 px line rhythm. Row mode uses a slightly larger readable font, clearer hover feedback, and the existing selection/accessibility behavior.
- Backend config apply transactions now persist an atomic recovery journal before changing the router config. At startup the journal path is checked against the detected allowlisted config path; the original file is restored only while the current revision is the expected candidate, and the service restarts before the journal is cleared. A third, externally edited revision stops recovery to prevent overwriting it.

## Профили и сверка референсов — 2026-10-26

- В разделе «Профили» действие «Копия» доступно и для пользовательских, и для штатных групп. Копия становится отдельным редактируемым профилем в `NFQWS_ARGS_CUSTOM`; исходный профиль и его группа конфигурации не меняются.
- При копировании имя исходного профиля не переносится, чтобы новая копия получила своё имя по умолчанию. Порядок аргументов сохраняется; перед добавлением проверяются лимиты 64 профиля и 8192 аргумента.
- Ограничение имени профиля выровнено с upstream-редактором: до 32 букв/цифр/точек/дефисов/подчёркиваний. Frontend и Rust отклоняют более длинное имя до сохранения.
- У `zxc-rv/XKeen-UI` просмотр версий построен как список раскрываемых карточек с выделенной выбранной версией и прокручиваемым Markdown-описанием. Наш селектор сохраняет тот же основной ритм, но использует локальный безопасный рендер Markdown и получает релизы через Rust backend.
- В актуальном `zemidala/nfqws2-ui` интерфейс профилей редактирует модель аргументов nfqws2 непосредственно. Здесь профильные группы выводятся из конфигурации и сохраняют происхождение; штатные группы остаются защищёнными, а их копия редактируется отдельно. Это сохраняет точку отката и не переписывает неожиданные переменные конфигурации.
- После детальной сверки редактор пользовательского профиля получил отдельные секции «Когда срабатывает», «Для кого» и «Что делает»: TCP/UDP и прочие фильтры, протоколы/payload, доменные/IP-списки и упорядоченные Lua desync-шаги больше не смешаны в одну строку. Имя можно менять прямо в конструкторе, а аргументы условий и списков — редактировать по одному с удалением.
- Шаги стратегии теперь можно добавлять из списка известных функций, менять функцию, править двоеточие-разделённые параметры, переставлять вверх/вниз и удалять. Для функций показана краткая подсказка; исходные неизвестные параметры остаются в разделе «Другие параметры», а точная текстовая форма сохраняется во вкладке «Текст».
- Порядок payload/range/Lua-шагов можно менять и перетаскиванием за ручку, как в Zemidala; стрелки остаются как точное клавиатурное управление. Перемещение меняет только позиции упорядоченных шагов, сохраняя расположение фильтров и прочих аргументов.
- В шагах Lua параметры отображаются отдельно: имя ключа, редактируемое значение/флаг и удаление. «Для чата» копирует подписанный профиль; вставка принимает комментарии, обычные кавычки и переносы строк, но отвергает переменные и shell-операторы, затем разбирает `--new` и shorthand Lua-функций.
- В модели конструктора payload и входящий/исходящий диапазоны входят в ту же последовательность, что и `--lua-desync`; их можно менять местами, потому что порядок меняет, к каким пакетам применяются следующие шаги. Фильтры `--filter-l7` показаны отдельно в секции условий.
- Все изменения остаются черновиком до явного «Применить изменения». Rust endpoint проверяет revision и аргументы, затем собирает все группы конфигурации и прогоняет установленный `nfqws2 --dry-run` до записи и перезапуска. Автосписки для проверки изолируются во временном каталоге; после успешной preflight-проверки конфигурация сохраняется транзакцией с резервной копией и откатом. Штатные группы можно копировать в пользовательские, редактировать и сохранять без прямой модификации исходной группы.
- Каталог Lua-функций теперь читает Rust backend из установленных `.lua` и `.lua.gz` файлов с ограничением размера; комментарии дают описание, аргументы функции, ключи всего Lua-файла и группы `standard args`, а сигнатура и тело функции используются для обнаружения обязательных ключей. Справка шага показывает эту документацию, а конструктор и проверка аргументов опираются на установленные скрипты.
- В шаге Lua добавлена сворачиваемая справка, собранная из описания установленной функции, её аргументов и указанных ею общих групп. Обязательные ключи выделены отдельно, а отсутствующие группы не подменяются выдуманной справкой.
- Проверка параметров шага Lua теперь знает аргументы конкретной функции, общие ключи её файла и перечисленные стандартные группы. Она помечает ключи, которых нет в документации текущей функции, и неизвестные ключи; когда инвентаря параметров нет, сомнительные предупреждения не создаются.
- Профили с неизвестной Lua-функцией или отсутствующим обязательным параметром получают предупреждения в общем списке, заголовке и у соответствующего шага. Если API каталога недоступен, интерфейс не делает неподтверждённых выводов и не показывает ложные ошибки.
- Каталог покрывает документацию Lua-функций, стандартные группы аргументов и обязательные параметры. Текстовый режим остаётся для точного редактирования нестандартных конфигураций; полная диагностика неизвестных Lua-ключей с автоматическими исправлениями пока отличается от движка lint в Zemidala.
- Сводка профиля теперь включает `--filter-l7`, payload и диапазоны пакетов в исходном порядке, чтобы не скрывать часть условий и шагов.
- Сверка команд с локальным checkout Zemidala v1.22.3 выявила общий путь `~/.nfqws-ui-store.sh` в генераторе SSH-команды NAS-бэкапов. Наш helper теперь называется `~/.nfqws2-ui-snapshot-store.sh`; повторная настройка удаляет только старые строки принудительной команды, содержащие публичный ключ nfqws2-UI и старое имя helper, затем устанавливает новую строку; сам старый файл и строки других ключей не меняются.

## Журнал — 2026-10-26

- Добавлена очистка выбранного журнала nfqws2-UI или nfqws2 через Rust API. В интерфейсе требуется подтверждение с объяснением, что файл обнуляется, а служба продолжает работу и записывать новые строки.
- Поиск файла учитывает каталог журналов обнаруженной установки (включая `/var/log` на OpenWrt и `/opt/var/log` в Entware), а настроенный путь отладки nfqws2 принимается только внутри этого каталога.
- Перед чтением или очисткой кандидат проверяется как обычный файл без финального симлинка и после canonicalize должен оставаться внутри разрешённого каталога. Очистка выполняется через `O_NOFOLLOW` на Unix.

- Конструктор профиля разделяет выбор на условия транспорта/протокола, списки доменов/IP и упорядоченные действия; списки берутся из Rust API. Повторные Lua-шаги разрешены, поскольку один профиль может содержать несколько этапов desync.
- В выборе профиля добавлена короткая сводка условий и области действия (например, TCP 443 · TLS · домены), чтобы было проще отличать похожие профили до открытия редактора.
- В редактор добавлена осторожная проверка порядка: она предупреждает, если более раннее правило без включающих списков пересекается с последующим по транспортному протоколу, портам и L7-фильтру. Несовпадающие порты и правила с включающим списком не помечаются как перехват.
- Список Lua-параметров в конструкторе объединяет документацию функции, параметры файла-скрипта и разрешённые стандартные группы; одинаковые ключи показываются один раз.
- Сводка Rust API разделяет фильтр протокола L7 (`--filter-l7`) и тип пакета (`--payload`); в деталях профиля эти значения показываются под разными заголовками.
- Транспортные фильтры отделены от L7 в сводке: один аргумент больше не дублируется в двух строках редактора.
- В конструкторе условия совпадения, списки целевого трафика и последовательность действий получили отдельные сдержанные цветовые маркеры. Номера шагов отображаются в бирюзовых обведённых кружках; payload, диапазоны и Lua desync имеют разные цветовые акценты, а широкие поля выбора легче читать.
- Выпадающие меню функции и параметров Lua в динамическом редакторе используют общий компонент панели. При повторном рендере отсоединённые меню и наблюдатели очищаются.
- В списке профилей убран значок замка у штатных профилей; статус «только просмотр» остаётся доступен скринридеру и виден в карточке профиля.
- Диагностика Lua знает совместимые параметры nfqws2 `altorder`, `oob`, `oob_data`, `nofake1`–`nofake4` для соответствующих fake-splitting функций, даже если комментарии некоторых установленных Lua-файлов неполны. Эти ключи доступны в списке добавления параметров; опечатки и ключи для неподдерживающей функции по-прежнему предупреждаются.
- Rust-каталог Lua дополнительно извлекает прямые чтения `desync.arg.<ключ>` из тела каждой установленной функции и сканирует до следующего верхнеуровневого определения функции, а не до первого вложенного `end`. Вспомогательная функция не загрязняет список предыдущей функции; список выбора содержит реальные ключи конкретной версии nfqws2.
- Список выбора параметров шага теперь сохраняет пункт `strategy` из редактора Zemidala, а при неполном Lua-каталоге всё равно предлагает совместимые ключи `fakedsplit`/`fakeddisorder`/`hostfakesplit`; пользовательские параметры вынесены в раскрываемый блок.
- Сверка с `allowedKeys()` Zemidala: варианты собираются в том же порядке из параметров функции, её стандартных групп, общих ключей Lua-файла и `strategy`; выбранные ключи повторно не предлагаются. Rust-парсер принимает те же допустимые пробелы в сигнатурах, комментариях `arg` и `standard args`, а также пояснения стандартных параметров через `—`.
- Каталог остаётся привязан к Lua-файлам конкретной установки nfqws2: он объединяет документированные параметры и прямые обращения `desync.arg`, поэтому варианты зависят от установленных скриптов. На узком экране меню ограничивает ширину областью viewport; длинные параметры получают отдельную минимальную ширину для чтения.
- TCP, UDP и L7-фильтры редактируются отдельными цветными значениями: их можно добавлять, менять и удалять по одному, не переписывая всю строку аргумента.
- Каталог конструктора сверяется с `helpInfo()` Zemidala: Rust запускает установленный `nfqws2 --help` с лимитом ожидания 5 секунд и возвращает опции, значения L7 и все документированные payload. Профильный выбор показывает локальные опции и скрывает глобальные параметры запуска, которые нельзя назначать отдельному профилю; неизвестный/неподдержанный вывод не блокирует остальные редакторы.
- Выбор «Добавить» использует payload-типы из установленной версии бинарника вместо фиксированных TLS/QUIC двух пунктов. Профильный параметр-пикер также предлагает все найденные не-глобальные параметры nfqws2 и найденные протоколы; версии с расширенными опциями автоматически отображают их без изменения фронтенда.
- Сверка с роутером по `nfqws2 --help` (core v1.0.6): pipe-вывод содержит 16 значений `filter-l7` и 31 payload; полный список попадает в каталог. Интерактивная SSH-обёртка сужает строки и добавляет ANSI-перемещения курсора, поэтому парсер очищает терминальные коды; backend вызывает процесс без TTY.
- Повторная сверка на роутере с nfqws2 v1.3.2 подтвердила полный каталог: 63 CLI-опции, 16 протоколов, 31 тип пакетов, 44 Lua-функции и 7 стандартных групп. Синтаксис необязательного значения `key[=value]` теперь корректно разбирается; например, `tcp_md5[=hex]` доступен в выборе и больше не порождает ложное предупреждение.
- Структурные `--new` и `--name` не предлагаются как общие параметры профиля: первое значение разделяет группы и не допускается в теле профиля, второе задаётся отдельным полем имени. Остальные локальные multi-strategy опции (например, `--skip`, `--template`, `--cookie`, `--import`) остаются доступными.
# Profile editor usability update

- Removed the duplicate profile summary rows (filters, protocols, payload types, lists, and the flattened ordered-step command) from above the editor. The profile header and warnings remain, followed directly by the editable builder or text view.

- Removed the introductory copy above the profile workspace so the profile list and current editor lead the page.
- Replaced inline Lua source comments and English parameter descriptions with Russian summaries generated from the installed nfqws2 catalog. The contextual reference stays beside the editor on wide screens and moves after it on narrow screens.
- Added a searchable, grouped parameter chooser for profile-scoped nfqws2 options and Lua functions. Selecting an item updates the active profile draft through the existing profile editing flow; global options and the profile separator remain excluded.
- The reference follows focused Lua steps and CLI arguments, showing their syntax and Russian purpose. Parameter names and command syntax remain unchanged for accuracy.
- Lua validation accepts nfqws2's `repeats` parameter for the `fake` function when older installed Lua comments omit it; this follows Zemidala's shipped strategy examples and avoids a false warning without widening the key to unrelated functions.
- Lua validation recognizes `pos` for `fakeddisorder` when older installed Lua comments omit it. The constructor offers the documented `midsld` starter value and still warns on unrelated unknown keys.
- The profile reference moves after the editor when the editor column itself is narrow, including desktop layouts where the fixed navigation consumes much of the viewport. This prevents the contextual guide from compressing or covering profile fields before the viewport-level mobile breakpoint.

## Profile constructor navigation — 2026-10-26

- The profile selector stays pinned while the profile content scrolls in its own touch-enabled region. The editor keeps the simpler v1.2.3 page layout; only the strategy content receives a viewport-based maximum height. Profile names and summaries wrap instead of being truncated.
- The profile list now has compact rename and visual reorder controls for every built-in and custom profile. Those display names and list order are stored in this browser only; they do not alter nfqws2 configuration or its processing order.
- Because custom profile API IDs are positional (`custom-N`), deleting a profile now removes its display preferences and shifts the remaining names and visual order with the surviving profiles.
- Removed the verbose successful profile/catalog summary from the workspace and the old “order of checking” caption. Load errors remain visible, while the visual-only reorder note is a small amber badge beside “Профили”.
- Kept the profile list pinned on narrow screens so switching profiles does not require scrolling back to the top. Removed only the built-in “HTTP/HTTPS по IP” constructor entry by its source variable; a user profile with the same display name remains visible. Its shared configuration source remains available through the HTTP/HTTPS by sites editor. Visible profile numbering is recalculated after filtering so list numbers and editor headings stay aligned.
- Moved list row deletion, import, and export into a compact “Ещё” menu within the primary toolbar. This removes the extra toolbar strip while preserving the actions and gives the list editor more vertical space.

## Profile source editing and multiline config — 2026-10-26

- The Zemidala comparison confirmed that built-in UDP and HTTP/HTTPS profile groups are editable in the reference. The Rust API now returns each managed profile's exact source variables; the UI edits those variables instead of flattening the composed profile into one unrelated config group.
- UDP settings map to `NFQWS_ARGS_UDP`; HTTP/HTTPS by IP maps to `NFQWS_ARGS` plus `NFQWS_ARGS_IPSET`; HTTP/HTTPS by sites maps to `NFQWS_ARGS` plus `NFQWS_EXTRA_ARGS`. Shared source variables are shown explicitly because a change can affect multiple logical profile views. The reference combines HTTP and HTTPS into these two profiles; it does not expose separate HTTP-only and HTTPS-only groups.
- Managed profile writes use an allowlisted Rust endpoint, reject stale config revisions and shell syntax, validate the proposed config, run the nfqws2 dry-run, then use the normal backup/restart/rollback transaction.
- Custom profile saves and strategy-picker additions preserve one argument per line in `NFQWS_ARGS_CUSTOM`; a newly appended `--new` profile gets a blank-line separator. Newlines are argument whitespace to nfqws2, so this changes readability without changing profile semantics.
# Конструктор стратегий в разделе «Конфигурация»

Навигация профилей nfqws2 объединена с разделом «Конфигурация»: в обычной боковой навигации конструктор показан вложенным пунктом, а в упрощённой — второй страницей группы «Конфигурация». Внутренний маршрут `profiles` и API оставлены прежними, чтобы не менять загрузку, редактирование и применение профилей.

Справочник параметров остаётся справа от редактора на широком экране и свёрнут при первом открытии профилей. Нативный раскрываемый заголовок доступен с клавиатуры; выбранная функция и параметр обновляют содержимое справки, а раскрытое состояние сохраняется при перерисовке редактора. На узком экране справочник располагается после редактора.

## Имена профилей после удаления

ID пользовательских профилей в текущем API строится по их позиции (`custom-1`, `custom-2` и т. д.). При удалении позиции меняются, поэтому браузерные имена и визуальная сортировка тоже должны перенумеровываться. Перед удалением конструктор удаляет настройки удалённой позиции и сдвигает последующие ID; изменения не затрагивают конфигурацию nfqws2 и штатные профили.

## v1.2.6 — возврат конструктора к v1.2.3

- По просьбе пользователя код раздела «Конструктор стратегий» в `App.tsx`, `panel.ts` и `profile-editor.ts` возвращён к тегу v1.2.3. Это отменяет добавленные в v1.2.4–v1.2.5 визуальное переименование/перестановку профилей и отдельную прокручиваемую область конструктора.
- Сохранено отдельное требование полностью показывать названия профилей: scoped CSS переопределяет усечение строками, не меняя логику конструктора из v1.2.3.
- Мобильная прокрутка страницы и изменения редактора `.list` не откатывались.

## v1.2.7 — компактный список профилей

- Удалена строка-сводка под названием каждого профиля. Оставлены номер, название и индикатор предупреждений.
- Заголовок списка теперь «Профили» без пояснения о порядке проверки и с чуть меньшим размером шрифта.
- Штатный профиль `NFQWS_ARGS_IPSET` скрыт только из списка конструктора; связанные параметры конфигурации сохранены, а общая переменная IP-set остаётся доступна из конструктора QUIC. Встроенная карточка `NFQWS_ARGS_QUIC_IPSET` отображается как «QUIC». Пользовательские профили с совпадающими именами не затрагиваются.
- Слой toast-уведомлений опущен ниже диалогов подтверждения, чтобы не закрывать важные кнопки.

## Аудит после v1.2.7

- Для `focusin` подсказка теперь появляется только при `:focus-visible`. Это сохраняет подсказки для клавиатурной навигации и предотвращает их появление от касания, когда браузер оставляет кнопку в фокусе на мобильном устройстве.
- Правила отображения встроенных профилей вынесены в `prepareVisibleRuntimeProfiles` и проверяются отдельно: штатный `NFQWS_ARGS_IPSET` скрывается, `NFQWS_ARGS_QUIC_IPSET` получает короткую подпись, а пользовательские профили и их имена сохраняются.

## Конструктор профилей: фильтры UDP и протоколов

- В строке «UDP-порты» появилась команда добавления фильтра; она открывает поле порта/диапазона, проверяет границы 1–65535 и добавляет фильтр только после ввода корректного значения.
- Текстовая кнопка добавления UDP-фильтра и её подтверждение больше не используют квадратный CSS-класс маленькой кнопки «+». Ширина теперь подстраивается под надпись, поэтому она не выходит за пределы кнопки.
- Протоколы выбираются из каталога `cli.l7`, который приходит от backend nfqws2. Уже выбранные значения скрываются из меню; выбор дописывается в существующий `--filter-l7`/`--filter-7` или создаёт фильтр, если его ещё нет.
- Меню остаются HTML-select, чтобы применялась общая стилизованная реализация выпадающих окон nfqws2-ui и её клавиатурная доступность.

## Диагностика объёмной передачи: компактный итог и проба короткого ответа

- Сверка с Zemidala показала, что его fallback для коротких страниц отправляет POST с телом 64 КБ. Для введённых пользователем сайтов добавлен отдельный выключенный по умолчанию флажок такой пробы. Она выполняется только для этих пользовательских доменов, по прямому и текущему маршруту; тела ограничены 64 КБ, запросы не следуют перенаправлениям и используют отдельный случайный URL-параметр.
- Тайм-аут после TLS при POST отображается как «POST не завершился». Он не считается доказанным обрывом передачи 16 КБ и сам по себе не запускает подбор стратегии, поскольку сервер может не поддерживать POST или обработку такого тела. Ранее разрешённые статические контрольные загрузки сохраняют прежнее поведение.
- Результаты диагностики сведены к сети, IP/ASN и одному статусу. Замеры напрямую/через nfqws2 и подробности стратегии доступны в раскрываемом блоке.
- Если оба маршрута ответили, статус теперь говорит «Ответ получен», не утверждая, что короткая страница полностью проверила передачу 16–20 КБ.

## После v1.3.0 — отмена UDP-фильтра

- В строке добавления UDP-портов добавлена явная кнопка-крестик. Отмена сбрасывает только незаписанный черновик, восстанавливает кнопку добавления и возвращает ей фокус; нажатие Escape делает то же самое.
- Сохранение профиля не вызывается при отмене, а ввод подтверждается только кнопкой «Добавить» или Enter.
- Статус справки профилей теперь показывает рабочее действие «Повторить», если `nfqws2 --help` остаётся неполным или недоступным после backend-повтора с увеличенным тайм-аутом.

## Очередь диагностики сайта

- Backend возвращает весь `SiteDiagnosticReport` одним ответом и не передаёт промежуточный статус каждого шага.
- Убран таймер, который циклически назначал этап активным во время единого запроса. До ответа этапы остаются ожидающими; после ответа они раскрываются по одному, с фактическими статусами из отчёта.

## Прокрутка конструктора профилей после v1.3.0

- По просьбе вернуть исходный конструктор версии 1.2.3 внутреннюю прокрутку редактора и закрепление списка профилей отложил. Сейчас на телефоне прокручивается вся страница обычным жестом; список и редактор идут в общем потоке.
- Текущий мобильный режим проверяется в `panel-template.test.ts`: у `body`, `#root`, основного layout и `main` нет вложенной вертикальной прокрутки, перехватывающей жест.

## Сверка с актуальными upstream-проектами перед v1.3.1 — 2026-10-26

- Повторная сверка выполнена по актуальным `main` без изменения локальных checkout-файлов: `zxc-rv/XKeen-UI` `ba04efa` (2026-10-26 16:34 +03:00) и `zemidala/nfqws2-ui` `ab4a8ad`, версия 1.23.1 (2026-10-26 01:23 +03:00). Предыдущие локальные копии были устаревшими.
- В текущем XKeen-UI просмотрены обновления `ConfigPanel`, редактора конфигурации и общих `context-menu`/`tooltip`: актуальные паттерны используют единый компонентный стиль, доступные состояния фокуса и popover/context-menu. В нашем React/TypeScript интерфейсе применены те же принципы через общие токены, семантические варианты кнопок и доступные UI-диалоги; компоненты shadcn/Radix напрямую не переносились, поскольку стек и разметка отличаются.
- В Zemidala 1.23.1 повторное чтение `nfqws2 --help` увеличено с 5 до 15 секунд, когда первая справка неполная. Rust backend уже делает ту же ограниченную пару попыток (5 и 15 секунд), возвращает признак полноты каталога, а frontend не показывает все отсутствующие параметры как неизвестные и предлагает повторную загрузку. Эту защиту сверили по `load_nfqws_cli_catalog`, предупреждениям валидации и тестам неполного каталога; новую логику добавлять не потребовалось.
- Обе внешние реализации имеют собственные модели серверной раздачи/компонентов и не дают совместимого API для переиспользования кода. Сверялись поведение и UX-паттерны, а не переносились их PHP/React-компоненты в наш Rust + TypeScript.
