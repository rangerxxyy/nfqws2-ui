# Последующая сверка с Zemidala — перетаскивание на мобильных экранах

## После v1.4.0 — полосы прокрутки по теме и акценту

- В актуальном Zemidala v1.23.2 (2026-10-27) полосы прокрутки приведены к выбранной теме. В нашей панели в v1.4.0 WebKit scrollbar сохранял фиксированные синие/серые цвета, а Firefox не получал общих theme-токенов.
- Добавлены `--scrollbar-track`, `--scrollbar-thumb` и hover-token на основе акцентного цвета; тёмная, светлая и neon-темы получают согласованные фон и ползунок. Gecko получает `scrollbar-color`, WebKit — те же переменные через псевдоэлементы. Толщина и геометрия не меняются.
- CSS-состояния покрыты тестом для акцентного цвета, hover, светлой и neon-тем. Визуальную проверку прокрутки на широком и узком viewport пока не подтверждаю: in-app preview не позволяет менять viewport, а Firefox headless не сохранил скриншот и оставался запущен. API dev-server сообщает v1.2.2, поэтому новые backend-функции через него не проверялись.

## После v1.4.0 — выбор Lua-функции общим popup-селектом

- В конструкторе Lua-функция исключалась из `installStyledSelects` через `data-native-select`, поэтому показывала браузерный список вместо общего popup каталога параметров.
- Убрано исключение, выровнены размеры, текст и ширина меню функции с выбором параметра из каталога. Сохранён `change`-обработчик, который заменяет только имя функции и оставляет аргументы шага.
- В браузере проверен выбор `fake` и обратный выбор `hostfakesplit`; оба обновили профильный черновик и пересобрали редактор. Конфигурацию не применял. Автоматические проверки: 339 frontend-тестов и production build/typecheck.

## Сверка Lua-каталога — не учитывать комментарии и строковые литералы

- Последний upstream Zemidala `v1.23.2` проверен отдельно: фильтрации комментариев в его опубликованном Lua-каталоге нет. Та же идея реализована в contribution branch `fix/recognized-nfqws2-lua-params` (commit `0e9586c`), но ветка не входит в upstream `main`; взял этот приём как точечный образец, не приписывая его опубликованному релизу.
- Наша версия просматривала сырой текст, поэтому комментарий `-- desync.arg.example` мог ошибочно добавить параметр в каталог; длинная/обычная строка давала такой же ложный результат.
- Сканер теперь пропускает комментарии, короткие и long-string литералы, поддерживает пробелы вокруг обращения и dot/bracket формы ключа. Проверка обязательных аргументов также пропускает комментарии и long strings, сохраняя обычные строки ошибок nfqws2.
- Регрессионный тест сначала подтвердил ложный параметр из комментария, после исправления проверяет игнорирование line/block-комментариев и строк, распознавание реальных ключей и сохранение обязательного ключа из сообщения ошибки.

## Сверка с XKeen UI — состояние кнопки при успешном обновлении

- В XKeen UI commit `ba04efa` установка интерфейса сохраняет состояние «обновление идёт» до перезагрузки страницы. В нашем обновляющем модальном окне `finally` безусловно снимал блокировку сразу после успеха, хотя перезагрузка была запланирована через 1,2 секунды: кнопку можно было нажать повторно в это окно.
- При успешном обновлении теперь оставляем `updatePolling` и кнопку установки заблокированными до перезагрузки; при ошибке, откате или истечении ожидания состояние по-прежнему сбрасывается для повтора.
- Добавлен regression-check на успешный путь и `finally`; проверка сначала падала на отсутствующем флаге `reloading`.
- Общая проверка текущих незарелизенных изменений: 340 frontend-тестов, TypeScript/Vite production build, 347 Rust unit/integration tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release --locked` и `git diff --check` прошли. ARM64 cross-build и роутер не запускал; пользователь проверит релиз на роутере сам. Живой preview использует API v1.2.2, поэтому новые backend-функции через него не проверялись.

## v1.4.0 — мониторинг и рабочие списки

- Выбор основной Lua-функции стратегии оставлен на нативном `<select>`: кастомная обёртка ранее перехватывала этот конкретный picker; теперь изменение вызывает обработчик `change` и обновляет аргументы профиля.
- Панель редактора списка использует стилизованный select для режима «Текстом / Списком». Действие «Удалить дубли» перенесено в меню `⋯`, рядом с импортом и экспортом.
- Мониторинг уплотнён: карточки показывают домен, итог и действия в одной строке, компактную историю; технические этапы сохранены в раскрываемом блоке. Проверка нового домена запускается после успешного сохранения.
- Сверка с Zemidala: обе реализации проверяют HTTPS GET до 64 КиБ и отправляют контрольный POST 64 КиБ при коротком ответе. В Rust уже оставались проверка POST, статусы DNS/TCP/TLS/HTTP/передачи и защита публичного IPv4 от SSRF. Отличие — backend закреплял только первый DNS IPv4. Теперь после неуспешного/неполного ответа повторяется проба на одном альтернативном разрешённом IPv4; ограничение до двух адресов сохраняет предсказуемую нагрузку роутера.
- Сохранение и удаление сайтов сохраняет текущий интервал с формы, а сетевые ошибки не теряются в необработанном Promise и показываются сообщением и toast.
- Проверено: targeted frontend tests, полный frontend-набор, typecheck, production build, Rust unit/integration tests, fmt, clippy и diff check. Роутер и живой UI не проверялись: локальная API-среда сообщает старую версию и не годится для проверки нового backend.

## v1.3.6 — обнаружение и идентификация релиза

- Сверка опубликованных GitHub-релизов выявила единственный разрыв формата: выпуск с заголовком «nfqws2-ui v1.3.6» опубликован под тегом `1.3.6`, без обязательного префикса `v`. Панель принимает только теги `vX.Y.Z` и сверяет их с именем ARM64-архива, поэтому корректно отбрасывала этот релиз.
- Метаданные Cargo, lockfile и web-пакета согласованы на `1.3.6`. Корректная публикация использует тег `v1.3.6` и стандартные для предыдущих релизов asset names.
- Сверка GitHub Releases API: прежние стабильные релизы используют префикс `v`, и имя архива соответствует тегу; исключение — публикация с тегом `1.3.6` без префикса.

- Удалён `cursor:grab`/`cursor:grabbing` и `touch-action:none` у ручки: в Zemidala курсор остаётся обычным, а управление перетаскиванием начинается по свободному месту карточки.
- На узкой ширине ручка скрывается; сетка карточки перестраивается с двух колонок на номер и содержимое, а кнопки действий остаются справа внутри второй колонки. Это освобождает ширину и не мешает вертикальному свайпу по ручке.
- Проверено: frontend-тесты (336/336), TypeScript typecheck, production build и `git diff --check`. Изменение относится к следующей итерации и не включено в опубликованный v1.3.4.

## v1.3.5 — Мобильный drag-and-drop и списки выбора

- Сверка с доступным поведением select в XKeen UI выявила, что собственный список не закрывался при Tab/focusout: обработчики закрывали его по Escape и указателю вне меню, но не при переводе фокуса клавиатурой.
- Добавлен `shouldCloseStyledSelectOnFocusOut` и обработчик `focusout`, который закрывает открытое меню только если следующий фокус вне триггера и меню.
- Проверено: unit-тесты для открытого/закрытого меню и фокуса внутри/снаружи, общий frontend-набор и typecheck.

# v1.3.4 — Редактирование профиля и полировка конструктора

- Курсор `grab`/`grabbing` снят с карточки стратегии и ограничен ручкой перетаскивания; карточкой по-прежнему можно управлять через имеющуюся ручку, а поля параметров используют обычный курсор.
- В чипе параметра ключ и значение получили отдельные фоны и тонкие обводки. Оба сегмента имеют одинаковую высоту; значение больше не смещается из-за ручного `=` и собственного левого отступа.
- Переименование профиля переведено с карандаша и модального окна на inline-поле «Имя». Существующая валидация (непустое имя из букв/цифр/`_.-`, максимум 32 символа) сохранена; пользовательские имена записываются в конфигурационный черновик, штатные — в `localStorage`.
- Проверено: точечные frontend-тесты для курсора, сегментов и inline-переименования. Полные проверки и результаты GitHub Actions фиксируются в `RELEASE_v1.3.4.md`.

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

## Доработки после v1.3.1 — проверка списков и интерфейса провайдера

- Сверка диагностики сайта с Zemidala 1.23.1 выявила, что у нас в сверку попадали стандартные файлы списков, даже если nfqws2 их не использовал. Теперь проверяются только списки из активных аргументов nfqws2; для отсутствия подключённых списков показывается отдельное пояснение. Добавлен тест на исключение неактивного `user.list`.
- В настройках `ISP_INTERFACE` оставлен один основной селектор. Он показывает обнаруженные интерфейсы, сохраняет пометку для настроенного, но не обнаруженного имени и не исчезает, если обнаружение не вернуло вариантов. Ручное поле открывается отдельным пунктом селектора и проходит ту же проверку имени, что и раньше. По Zemidala сверено наличие выбора обнаруженного сетевого интерфейса и ручного варианта.
- В заголовке конструктора сохранён только карандаш переименования; подпись «Редактор профиля» и отдельная строка имени убраны. Переключатель «Конструктор / Текст» перенесён в заголовок и получил анимированный индикатор выбранного режима.
- Вкладки списков показывают короткие имена файлов; пояснение для стандартных и пользовательских списков появляется через 3 секунды наведения. Читаемость вкладок, редактора строк, кнопок панели и текущего пути увеличена, в том числе на узком экране.
- Проверено: `pnpm test` (330 тестов), `pnpm typecheck`, `pnpm build`, `git diff --check`. Проверка на роутере не выполнялась.

## v1.3.3 — компактность и перестановка шагов конструктора

- Чипы параметров получили согласованные мягкие фон и границу, а ключ и значение окрашены по-разному. Разделитель `=` имеет отдельную фиксированную позицию, поэтому значение не смещается при разной длине ключей.
- Сетка шагов выровнена, размеры контролов унифицированы, длинные параметры переносятся внутри доступной ширины. Убрана панель справочника, занимающая место рядом с редактором.
- Карточки стратегий можно перетаскивать за свободное место; поля, кнопки и другие интерактивные зоны исключены из начала drag. Новая позиция сохраняется через тот же draft-путь, что и обычное редактирование, и карточки анимируют перестановку.
- Нумерация пропускает типы шагов, отличные от `lua-desync`, и пересчитывается в порядке сериализации после добавления, удаления и перестановки.
- Перед релизом проверены 335 frontend-тестов, TypeScript typecheck/build, 127 backend unit-тестов и все backend integration tests, `cargo clippy --all-targets -D warnings`, Rust formatting. Проверки router target запускает GitHub Actions; live-router проверка не выполнялась.

## v1.3.5-r1 — редактирование параметров без запуска перетаскивания

- Исправлен конфликт `draggable`-карточки стратегии с полем значения: решение о начале перемещения теперь принимается по элементу, на котором начался `pointerdown`, а не только по `event.target` браузерного `dragstart` (которым может быть сама карточка). Интерактивные элементы, включая поля значений, не начинают drag; свободная часть карточки сохраняет перетаскивание.
- Исправлен выбор основной Lua-функции шага: селектор напрямую обновляет выбранную функцию, не полагаясь на делегирование события.
- Добавлены проверки регрессии на блокировку drag при старте жеста в интерактивном поле и на применение выбора функции.
- Для патч-ревизии `-rN` добавлена корректная проверка тегов и сравнение версий обновления панели; управление релизами ядра nfqws2 сохраняет строгий числовой формат.
- Проверено: 338 frontend-тестов, TypeScript typecheck/build и `git diff --check`. Добавлены Rust и shell preflight тесты; физический роутер не проверялся.

## После v1.4.1 — выбор Lua-функции мышью

- Общий popup-select мог терять фокус при нажатии на нефокусируемый пункт меню: обработчик `focusout` закрывал список до завершения клика. При нажатии на пункт теперь подавляется стандартный перевод фокуса и учитывается незавершённый выбор.
- Добавлена регрессия на сохранение открытого меню во время выбора мышью.
- Проверено: 341 frontend-тест, TypeScript build и `git diff --check`. В браузере backend отдаёт старую панель 1.2.2, поэтому сквозную проверку с актуальным профилем нужно повторить на установленной версии.

## После v1.4.2 — подсветка функции и завершение pointer-жеста

- Возвращён прежний оттенок у селектора Lua-функции шага: цвет текста и тонкая граница берутся из цвета номера стратегии. Общее popup-меню и его палитра остаются едиными; фокусная обводка также следует оттенку шага.
- Устранён крайний случай popup-select: если указатель начинал выбор на пункте, но отпускался за пределами меню, локальный обработчик не получал `pointerup` и оставлял флаг незавершённого выбора. Теперь завершение/отмена отслеживается на `document` и сбрасывается только для того же `pointerId`.
- Для touch не отменяется стандартное поведение браузера; это сохраняет прокрутку длинного списка пальцем. Отдельные тесты закрепляют сохранение и сброс состояния указателя.
- В браузере найдено сжатие содержимого стратегии в узком контейнере редактора: карточка сохраняла ширину, но описание и параметры сворачивались в узкую grid-колонку. Для контейнера до 640 px шаг теперь использует flex-wrap: селектор и параметры занимают оставшуюся ширину, кнопки действий переносятся на отдельную строку; настольная сетка не меняется.
- Повторно сверены актуальные upstream-ревизии: Zemidala `abcf1ec` (v1.23.2, 2026-10-27) и XKeen-UI `ba04efa` (2026-10-26). В v1.23.2 Zemidala исправил проверку подключения правил перехвата на Keenetic при перестроении firewall и добавил scrollbar, окрашенный темой. В нашей панели нет соответствующей проверки firewall-цепочек, поэтому этот Keenetic-специфичный false positive к текущему статусу не применим и не требует параллельного исправления. Объединение проверенной сети с совместимым существующим профилем появилось раньше, в v1.23.0 (`2ced88b`), и требует отдельной backend-транзакции с проверкой соответствия стратегии. Окраска scrollbar по теме/акценту в текущей панели уже соответствует паттерну Zemidala и оставлена без изменения толщины.
- Добавлен поток предпросмотра и объединения сети в уже существующий профиль по мотивам Zemidala v1.23.0. Панель предлагает его только для явно введённого домена с публичным IPv4, подтверждённым ASN, найденным обрывом и свежей стратегией 3/3 без повторного использования; backend повторно проверяет все условия, принадлежность адреса префиксам RIPEstat и однозначность IP-only профиля. Стратегия сопоставляется после сортировки параметров и игнорирует `strategy=N`, как в Zemidala. Перед применением показывается diff; подтверждённая транзакция эксклюзивно создаёт список `ipset_as<ASN>.list`, проверяет конфигурацию, сверяет ревизию/активный процесс, создаёт резервную копию и откатывает конфиг при ошибке. В отличие от одноразового снапшота Zemidala, зарегистрированный список обновляется раз в сутки из фиксированного RIPEstat endpoint. Обновление допускается только пока конфиг ссылается на список, файл совпадает с последним UI-управляемым хэшем, новый ответ валиден и не меньше трети сохранённого числа префиксов; ручная правка ставит feed на паузу и не перезаписывается. Замена файла атомарна, прежнее содержимое сохраняется в ограниченной приватной резервной копии, а метаданные записываются до изменения для восстановления после сбоя. Работающий nfqws2 получает `SIGHUP` для перечитывания списков без полного перезапуска; если повторный reload после перезапуска панели не проходит, прежний файл возвращается из резервной копии. Раздел списков показывает дату последнего обновления и причины паузы/ошибки.
- Мониторинг сверён с Zemidala `abcf1ec`: обе реализации проверяют публичный IPv4 через HTTPS и ограничивают объём; у нашей UI более подробные этапы DNS/TCP/TLS/HTTP/передачи, а контрольная upload-проба используется только при коротком ответе. Обнаружена разница по нагрузке: Zemidala запускает проверки последовательно, наша версия могла запустить одновременно все 20 сайтов. Ручной API и автоматический цикл используют общий лимит в три одновременных сайта; регрессионный async-тест подтвердил максимум 3 одновременно на списке из 10.
- Для выбора Lua-функции восстановлено заметное состояние текущего выбора: выбранный пункт подсвечивается текущим акцентным цветом и галочкой, включая светлую тему; стрелочная/клавиатурная подсветка активного пункта сохраняет отдельное состояние.
- Проверено в локальном frontend preview: изменена Lua-функция через popup, список закрылся, функция и связанные предупреждения обновились; затем выбор возвращён к исходному значению без сохранения. На узкой ширине редактора параметры выстроились в строки без вертикального сжатия. Проверены тематические scrollbar-стили, селектор, pointerId и адаптивная раскладка.
- Проверки кандидата v1.4.3: 343 frontend-теста, TypeScript typecheck/build; 129 Rust library-тестов, 166 binary-тестов, 14 freeze-diagnostic, 3 service-status, 20 strategy-picker и 16 transfer-diagnostic интеграционных тестов; `cargo fmt`, `cargo clippy -- -D warnings`, `cargo audit`, `pnpm audit --prod`. Installer/updater/nfqws2-ui-ctl preflight и `git diff --check` прошли. BusyBox-сценарий не запускался локально (BusyBox отсутствует), его выполняет GitHub release workflow. Роутер не проверялся.

## v1.5.0 — История изменений, ASN и стратегии сообщества

- История изменений перенесена на backend: конфиг и все `.list` сохраняются в ограниченном приватном хранилище версий, фиксируются изменения из UI и внешние правки, отображается diff. Групповой откат сверяет текущий хэш каждого файла, отмена последней операции и возврат к состоянию установки проверяют перезапуск и сохраняют созданные позднее файлы.
- Для ASN-диагностики добавлены повторные пробы передачи в неоднозначном диапазоне, объединение подтверждённой сети с существующим совместимым профилем и безопасное обновление управляемых ASN-списков. Мониторинг ограничен тремя одновременными сайтами.
- Внедрена компактная база стратегий сообщества Zemidala: автоматическое/ручное определение ASN, 24-часовой успешный кэш, 10-минутный кэш ошибки, фильтрация устаревших и провалившихся вариантов, отдельная тестовая операция и заполненная GitHub-форма для подтверждений/неудач. Публикация остаётся действием пользователя; кандидат тестируется на установленной версии nfqws2 и не применяется вслепую. Одиночный и общий многосайтовый подбор могут включать эти варианты отдельным флажком.
- Перенесены улучшения состояния Lua-селектора, тематических scrollbar и компактного узкого редактора.
- Проверки перед выпуском: полный Rust/frontend suites, форматирование, Clippy, TypeScript, production build и dependency audit; smoke-test на роутере будет внесён после выполнения на опубликованной ARM64-сборке.

## После v1.5.0 — компактный экран стратегий сообщества

- При первом открытии раздела автоматически запрашивается определение ASN, только если пользователь ещё не сохранил ASN. Существующий ручной выбор не перезаписывается; явная кнопка «Определить автоматически» остаётся доступна, а имя отображается вместе с номером в формате `AS35807 Название провайдера`, как в Zemidala.
- Убраны пустые карточки `HTTPS / TLS · 0` и `HTTP · 0`: отображаются только протоколы, в которых действительно есть стратегии. Если подходящих вариантов нет, выводится одна компактная причина для выбранного провайдера или просьба определить ASN.
- Блок «Что это значит и как это устроено» перенесён под результаты и оформлен читаемой адаптивной сеткой с краткими пояснениями о проверке, подтверждении и публикации. Поля провайдера скрываются после выбора; их можно открыть кнопкой «Изменить».
- Проверено: 355 frontend-тестов, TypeScript typecheck, Vite production build, полный Rust-набор и локальный браузерный preview на desktop/narrow ширине без горизонтального переполнения. Роутер не проверялся; `v1.5.1` не выпускался.

## После v1.5.0 — читаемость «Истории изменений»

- По скриншотам сравнения с Zemidala выявлено, что наша история была ограничена шириной `max-w-5xl`, а панель действий оставалась выровненной по всему экрану. Из-за этого кнопки выглядели оторванными от списка и текст истории был слишком мелким.
- Убрано ограничение ширины карточки; верхние действия получили отдельную адаптивную группу, увеличенную высоту и размер шрифта. Увеличены также кнопки «Что изменилось» и «Откатить» в каждой записи. Отключённые действия остаются различимыми, но сохраняют disabled-состояние. В записях увеличены дата, название файла, метаданные и diff; на узком экране верхние действия переносятся и занимают доступную ширину.
- Добавлены regression-проверки на ширину контейнера и размеры элементов истории. Desktop/narrow preview показал связанную с историей панель действий, раскрывающийся diff и отсутствие горизонтального переполнения; браузерная консоль была без ошибок. Проверено также 355 frontend-тестами, TypeScript typecheck, Vite production build и `git diff --check`; релиз не создавался.

## После v1.5.0 — проверка группового отката истории

- Добавлен регрессионный сценарий для двухфайлового отката, где предыдущая версия второго списка содержит некорректные домены. До исправления API успевал восстановить первый список и только потом отклонял второй, оставляя операцию частично применённой.
- `undo-last` теперь заранее загружает и проверяет все предыдущие версии группы: допустимость пути, формат конфигурации, лимиты и содержимое `.list`. Ошибка в любом элементе останавливает операцию до первой записи. Для одиночного отката списка проверка актуального хэша теперь выполняется под общей блокировкой записи, чтобы не использовать устаревшее чтение.
- Проверено: регрессионный тест воспроизводил частичный откат до исправления и проходит после; полный `cargo test` (393 теста), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` и `git diff --check` прошли. Релиз не создавался.
- Повторный фронтенд-прогон: 355 тестов, typecheck и production build прошли; `cargo audit` и `pnpm audit --prod` не нашли уязвимостей. Полноценный ARM64/router cross-build не выполнен из-за отсутствия `cross` и Docker/Podman. BusyBox test и ShellCheck не запускались из-за отсутствия соответствующих программ. Физический роутер не проверялся.

## После v1.5.0 — целостность объектов истории

- В сжатом хранилище версия файла адресуется SHA-256, но при чтении раньше проверялась только корректность имени digest и распаковки. Повреждённый либо подменённый gzip-объект мог вернуть другое валидное UTF-8-содержимое под чужим хэшем и попасть в diff/откат.
- Чтение объекта теперь повторно считает SHA-256 распакованных байтов и отклоняет несовпадение до преобразования в текст. Регрессионный тест подменяет содержимое объекта, сохраняя старое имя по digest; до исправления он проходил с подменёнными байтами, после исправления корректно получает ошибку.
- Целевой тест сначала воспроизвёл принятие подмены; после проверки SHA-256 прошёл. Полный Rust-набор из 393 тестов, `cargo fmt --check`, Clippy с `-D warnings` и `git diff --check` повторно прошли. Релиз не создавался.
- Release-preflight обновлён: `cargo check --target aarch64-unknown-linux-musl --locked` прошёл, что проверяет компиляцию под ARM64/musl, но не линковку итогового router-бинарника. Через Git for Windows `dash -n` прошёл для установочных скриптов и доступных preflight-тестов; прошли `installer-preflight.sh`, `update-helper-preflight.sh` и `nfqws2-ui-ctl-preflight.sh`. BusyBox test и ShellCheck не запускались, так как эти программы отсутствуют; полноценный cross-build и физический роутер не проверялись.

## После v1.5.0 — состояния checkbox по теме и акценту

- В тёмной, светлой и neon-теме checkbox «включён» теперь использует выбранный акцент для заливки и обводки, а фокус — акцентный цвет рамки. Нейтральная светлая заливка и фиолетовая обводка применяются только к выключенному checkbox, поэтому не перекрывают checked-состояние.
- Перед исправлением браузерная проверка показала, что светлая тема оставляла белый фон, а neon — фиолетовую обводку при cyan/amber-акценте. CSS теперь ограничивает эти цвета состоянием `:not(:checked)`; добавлен регрессионный тест на переменные акцента и тему.
- Проверено в Chrome: 9 сочетаний трёх тем (dark/light/neon) и трёх акцентов (cyan/amber/violet) совпадают по заливке и обводке с переменными интерфейса; клавиатурный focus-visible в светлой теме использует amber-рамку, ошибок страницы нет. Полный frontend suite (355 тестов), typecheck, production build и `git diff --check` прошли. Релиз не создавался.
- Состояние опубликованного релиза сверено через GitHub API: `v1.5.0` не является draft, installer, ARM64-архив и checksum приложены; release workflow завершился успешно. Текущие изменения после этого тега не публиковались.

## После v1.5.0 — атомарная отмена группы изменений

- Групповая команда «Отменить последнее» теперь держит общий `config_write_lock` от проверки актуальной истории до записи результата, поэтому параллельное сохранение из UI не может вклиниться между файлами одной отмены.
- Перед первым изменением создаётся страховочный архив управляемой конфигурации и списков. Если любой файл не удаётся отменить либо историю не удаётся атомарно пометить как отменённую, архив восстанавливается целиком; при возврате конфигурации выполняется перезапуск службы. Метки `undo_of` для всех файлов сохраняются одним обновлением истории после успеха всей группы.
- Рефакторинг транзакции конфигурации разделил блокирующую обёртку и внутреннюю функцию под уже удерживаемой блокировкой, чтобы групповой откат не вызывал повторный захват mutex.
- Добавлен регрессионный API-тест с искусственным отказом записи второго списка: он подтверждает возврат первого списка к состоянию до отмены, отсутствие частичного результата и сохранение возможности повторить отмену. Тест группового отката с двумя списками и атомарности отметок истории также проходит.
- Проверено: полный `cargo test --locked` (395 тестов: 164 library, 178 binary, 14 + 3 + 20 + 16 integration), `cargo clippy --all-targets --locked -- -D warnings`, `cargo fmt --check`, frontend 355 тестов, `pnpm typecheck`, `pnpm build`, `git diff --check` и `cargo check --target aarch64-unknown-linux-musl --locked`.
- В GitHub Actions вручную выполнен полный workflow для коммита-кандидата `278543d`: CI завершился `success`, включая `cross test`, статический ARM64/musl release build, shell/preflight checks, аудиты, упаковку и checksum. Артефакт SHA-256 `1724ff721ed85d3e6eaaf898638cf73017f8a2e117babe9ad2a1e9ffc7dccc3f` совпал локально.
- На Keenetic по SSH подтверждены `aarch64`, работающий процесс v1.5.0, успешные `GET /api/health` и главная страница. Собранный бинарник v1.5.1 проверен на самом роутере только запуском `--version` в изолированном временном каталоге; он вывел `nfqws2-ui 1.5.1`. Архив и бинарник удалены, установленный сервис не изменялся. Сетевые проверки функций приложения на кандидате не выполнялись.
- `v1.5.1` опубликован 2026-10-27: https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v1.5.1. Tagged release workflow `40513091271` завершился `success`; на странице релиза подтверждены installer, ARM64/musl архив и SHA-256. Архив повторно скачан, локальный SHA-256 совпал с опубликованным checksum.
- После проверки кандидатного бинарника роутер по-прежнему запущен со штатного `/opt/share/nfqws2-ui/current/bin/nfqws2-ui`; `/api/health` отвечает успешно. Оба временных пути теста отсутствуют.

## После v1.5.1 — повторное использование каталога параметров nfqws2

- При сверке с Zemidala `nfqws2-ui` (`abcf1ec`, v1.23.2) обнаружено, что справка `nfqws2 --help` может быть неполной при коротком тайм-ауте. Наш backend уже повторяет запрос с увеличенным тайм-аутом, однако повторно выполнял его при каждом открытии раздела профилей; кроме того, перечитывал и разбирал все Lua-скрипты каталога.
- Добавлен backend-кэш полного каталога CLI. Ключ учитывает канонический путь бинарника, размер и время модификации, поэтому замена установленного `nfqws2` инвалидирует запись. Неполный ответ в кэш не попадает; явный `forceRefresh` обходит его, а UI-кнопка «Повторить» использует этот путь.
- Добавлен кэш разобранного каталога Lua с зависимостями по именам, размеру и времени модификации `.lua`/`.gz` файлов. Посторонние файлы не сбрасывают кэш, обновление Lua-скриптов сбрасывает. Принудительное обновление обходит оба кэша.
- Сверка парсера с `helpParse()` Zemidala выявила слишком строгое условие полноты: мы требовали ещё непустые списки протоколов и payload, хотя upstream определяет полноту по обязательным параметрам CLI. Убрано лишнее условие; добавлена регрессия на полный набор опций без необязательных value-списков.
- Кнопка повтора каталога обновляет только справку CLI и Lua и перерисовывает редактор из текущего черновика, а не повторно загружает профили с backend. Это исключает потерю несохранённых изменений; при сетевой ошибке кнопку можно нажать снова.
- Frontend-тест закрепляет передачу принудительного обновления; backend-тесты проверяют CLI и Lua-кэши, инвалидацию при изменении соответствующих файлов и отказ от неполных/устаревших CLI-записей.
- Проверено: 356 frontend-тестов, `pnpm typecheck`, `pnpm build`, 180 Rust-тестов, `cargo fmt --check`, `cargo clippy --bin nfqws2-ui -- -D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked`.
- Работа над следующей версией продолжается. Эти изменения после v1.5.1 пока не опубликованы и не устанавливались на роутер.

## Навигационное свечение следует выбранному акценту

- Исправлены базовые стили активного пункта навигации и его светящегося индикатора: цвет берётся из `--accent-color`/`--accent-border`, поэтому при смене акцентной палитры не остаётся синее свечение.
- Общее состояние `:focus-visible` и мягкое свечение сфокусированных полей также используют текущий акцент темы.
- Добавлена статическая регресс-проверка, что навигационный индикатор и focus-обводки подключены к accent-токенам.
- Проверки: 357 frontend-тестов, TypeScript typecheck и production build прошли; `git diff --check` прошёл.
- Ограничение: визуальную проверку переключения разделов с живым backend выполнить не удалось — локальный preview не подключён к панели. Проверил каскад и привязку CSS к токенам по исходникам.

## После v1.5.1 — статусы списка стратегий сообщества

- Сверил страницу и логику сообщества с текущим `zemidala/nfqws2-ui` `main` (`abcf1ec`, API и страницу раздела «Стратегии сообщества»). Уже реализованные определение ASN, загрузка базы, выбор цели, тестирование стратегии, копирование и кнопки обратной связи в результатах подбора сохранены.
- На карточках базы теперь отображаются статусы, соответствующие правилам Zemidala: «Подбор пропускает» для стратегии с преобладающими неудачами и «Давно не подтверждалась» для записей старше 60 дней. Это делает причины порядка и отбора заметными до запуска подбора.
- Если целей у стратегии больше шести, строка показывает количество скрытых целей, как таблица upstream, при этом карточка остаётся компактной.
- Добавлены локальные цветовые токены карточек сообщества для тёмной и светлой темы: заголовки, подписи, блок кода, рамки и поверхности сохраняют контраст; служебные статусы получили отдельные читаемые варианты для светлой темы.
- Копирование использует общий `writeTextToClipboard`: работает через Clipboard API в secure context и через `execCommand('copy')` как fallback, вместо ошибки при недоступности `navigator.clipboard`.
- Регрессионная проверка закрепляет оба статуса, счётчик целей и clipboard fallback. Проверки после изменений: frontend 357 тестов, typecheck и production build прошли; полный `cargo test --locked`, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` и `cargo check --target aarch64-unknown-linux-musl --locked` прошли. `git diff --check` прошёл.
- Эти правки после v1.5.1 не публиковались.

## Предупреждения конфигурации и подсветка групп

- В «Настройки и оформление UI → Оформление UI» добавлен переключатель показа предупреждений конфигурации. По умолчанию включён; настройка сохраняется в браузере и на роутере, старые сохранённые настройки получают безопасное значение по умолчанию.
- В разделе «Конфигурация → Конфигурация» отображаются предупреждения backend-валидатора для текущего черновика. Проверка выполняется при загрузке и с задержкой после ввода, устаревшие ответы игнорируются; для замечаний с номером строки есть кнопка перехода к ней. Перед применением действует существующая проверка ошибок и подтверждение предупреждений.
- В редакторы групп NFQWS_EXTRA_ARGS/NFQWS_BASE_ARGS/NFQWS_ARGS и остальных групп добавлена та же посимвольная подсветка токенов, что и в полном редакторе: комментарии, переменные, флаги, Lua-параметры, значения, строки и числа. Синхронизированы прокрутка, перенос строк и строки-нумерация; ввод и редактирование текста остаются обычными.
- Подсветка фокуса редакторов групп теперь берёт цвет из выбранной accent-темы; счётчик предупреждений использует общую русскую форму склонения.
- Проверено: 359 frontend-тестов, TypeScript typecheck, production build, 180 backend unit-тестов и 14 + 3 + 20 + 16 integration-тестов, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked` и `git diff --check`. В Chromium-предпросмотре с локальным mock API на 1280×720 и 390×844 визуально проверены список предупреждений, переключатель, переход «К строке» и палитра/выравнивание редакторов групп; при ширине 390 px горизонтального переполнения нет, ошибок консоли нет. Mock API остановлен, временный скрипт удалён, viewport сброшен. Это подтверждает фронтенд-отрисовку, но не реальный backend или роутер. Эти изменения остаются после v1.5.1 и не опубликованы.
- Дополнительная визуальная сверка: в Chromium с изолированным mock API сопоставил полную конфигурацию и редакторы групп в активной фиолетовой теме. В обоих местах отображаются одни и те же токен-цвета для флагов, Lua-режимов, ключей, значений и переменных; все семь групп доступны, ширина колонок не ломает textarea. После проверки mock остановлен, его временный скрипт удалён, вкладка закрыта.

## После v1.5.1 — select использует выбранный акцент

- В аудите нашёл оставшиеся жёстко заданные синие цвета в открытом триггере и активных пунктах общего кастомного select. Из-за них меню выглядело синим при акценте «Аметист», несмотря на тематизацию остальных активных элементов.
- Обводка и мягкое свечение открытого select, клавиатурный фокус и фон/текст активного пункта теперь используют `--accent-color`, `--accent-border` и `--accent-soft`. Светлая тема смешивает оттенок с белым фоном для контраста; выбор профиля сохраняет существующий стиль.
- Добавлена регрессионная проверка обеих тем и accent-токенов. В Chromium с mock API проверил раскрытый список и навигацию клавишей вниз при акценте «Аметист» на тёмной и светлой темах; восстановил исходную тему «Фиолетово-неоновая». Mock остановлен, скрипт и тестовая вкладка удалены.
- Проверено: 362 frontend-теста, TypeScript typecheck, production build и `git diff --check`. Backend и роутер для этой CSS-правки не менялись. Изменения остаются после v1.5.1 и не опубликованы.

## После v1.5.1 — уведомление об обновлении nfqws2 и белая галочка

- Для бинарного ядра nfqws2 добавлена плашка обновления в общей зоне уведомлений с плашкой nfqws2-UI. Она использует существующий список подходящих GitHub Releases и установленную версию из backend; версия в шапке подсвечивается и получает доступную подпись, когда latest отличается от установленной. Нажатие на плашку открывает штатный выбор версии ядра.
- Закрытие плашки nfqws2 запоминается в localStorage для текущего тега; новая версия снова отображается. Проверка ядра выполняется при старте и далее не чаще раза в 30 минут, поскольку текущий endpoint делает запрос к GitHub от роутера. Возврат из фоновой вкладки не запускает внеплановый запрос до истечения интервала; ошибки фоновой проверки не мешают панели. Порядок ответа backend сохранён как порядок GitHub Releases (новейшие первыми).
- Все стандартные успешные toast-уведомления, включая «nfqws2 перезапущен», теперь используют белую SVG-галочку с нейтральной рамкой/подложкой вместо зелёной; это оставляет ошибки красными и не меняет отдельные индикаторы состояния/диагностики.
- Добавлены тесты отображения/скрытия плашки ядра (включая нормализацию тега пакета `-rN`), 30-минутного throttle и шаблонные проверки общего контейнера и белого success-icon. Проверено: 368 frontend-тестов, TypeScript typecheck, production build, `cargo fmt --check`, все suites `cargo test --locked`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked` и `git diff --check`. Реальный роутер и live-запрос обновления не проверялись; релиз не выпускался, правки остаются после v1.5.1.
- В Chromium на `127.0.0.1:5177` с локальным mock API визуально проверил обе плашки одновременно, подсветку кнопок и белые иконки на desktop viewport 1280×720; доступ к браузерной консоли через этот browser provider отсутствовал. Mock-процесс остановлен, созданный для него временный файл удалён, четыре временные вкладки закрыты. Mock-данные не являются реальным релизом.

## После v1.5.1 — кэш проверки ядра по модели Zemidala

- Сверка `origin/main` upstream Zemidala на commit `abcf1ec` (v1.23.2, 2026-10-27) показала, что GitHub проверяется не чаще раза в 12 часов при 10-минутном локальном опросе. Предыдущая реализация UI дергала с роутера полный GitHub Releases API каждые 30 минут и передавала весь каталог релизов в фоне.
- Фоновый запрос теперь идёт к отдельному лёгкому `/api/nfqws2/updates/status`; backend кэширует подходящий каталог на 12 часов и возвращает установленную/последнюю версии. При ошибке GitHub использует ранее успешный кэш; явное открытие списка релизов оставляет прямую свежую проверку. Это снижает фоновые обращения к GitHub примерно с 48 до максимум 2 за сутки при обычной работе.
- Добавлены проверки кэширования по сроку/архитектуре и определения наличия обновления; добавлен frontend API-тест и проверка связности route/UI. Проверено: 369 frontend-тестов, TypeScript typecheck, production build, `cargo fmt --check`, полный `cargo test --locked`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked` и `git diff --check` прошли.
- В Chromium с локальным mock API проверил одновременный показ плашек nfqws2 и UI, номер версии и переход к штатному выбору версии ядра. Предыдущий полный скриншотный прогон подтвердил белый success-icon; мок завершён и временная вкладка закрыта. Реальный роутер и live-запрос к GitHub не проверялись. Изменения остаются после v1.5.1; релиз не выпускался.

- Продолжая аудит, устранил повторный запрос GitHub при перезапуске backend: успешный кэш каталога теперь атомарно сохраняется в каталоге данных UI (`core-releases-cache.json`) с ограничением размера и приватными правами файла на Unix. Кэш привязан к архитектуре; ошибки проверки получают 10-минутный backoff, сохраняя последний успешный каталог. При отсутствии успешной истории статус остаётся недоступным до следующей попытки, а ручное открытие выбора версии по-прежнему делает свежий запрос.
- Добавлены тесты срока жизни 12 часов, архитектурной изоляции, 10-минутного backoff и сериализации/чтения файла. Проверено: 372 frontend-теста, TypeScript typecheck, production build, 403 Rust-теста, `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, ARM64/musl `cargo check` и `git diff --check` прошли. Реальный роутер и GitHub пока не проверялись; релиз не выпускался.

## После v1.5.1 — совместимость отключённых списков

- Сверил `zemidala/nfqws2-ui` `main` на commit `abcf1ec` (v1.23.2): стандартные списки, не подключённые к текущей конфигурации nfqws2, доступны сторонним инструментам и не должны принудительно преобразовываться к формату hostlist/ipset.
- Backend собирает все ссылки каждого поддерживаемого флага, учитывает argv работающего процесса и включает строгую проверку при подключённом, динамическом, повреждённом или неоднозначном значении. Для штатного списка без ссылок проверка доменов/CIDR пропускается; неизвестные/пользовательские пути остаются fail-closed.
- Лимит размера, запрет NUL, проверка ревизии, snapshot и история записи сохранены. При отмене истории сначала проверяются текущая конфигурация и целевая конфигурация операции; если список используется одной из них или запущенным процессом, сторонний формат не принимается. Весь batch валидируется до восстановления файлов.
- Добавлены тесты нескольких ссылок и динамических значений, записи стороннего формата в отключённый список, отказа для подключённого списка, восстановления из истории, а также атомарного отказа при текущей или целевой конфигурации, подключающей список.
- Для последнего пункта проверено: 372 frontend-теста, TypeScript `--noEmit`, production build; 410 backend-тестов, `cargo fmt --check`, Clippy с `-D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked` и `git diff --check`. На роутере не проверялось; изменения после v1.5.1 не выпускались.
- Запрошенная плашка доступного обновления `nfqws2` уже находится рядом с плашкой `nfqws2-UI`; кнопка ядра получает подсветку при новой версии. Все успешные toast-подтверждения, в том числе «nfqws2 перезапущен», отображают белую SVG-галочку. Повторно проверены frontend-тесты уведомлений и тестовая сборка.

## После v1.5.1 — предупреждение о маршруте Podkop в подборе

- Повторная сверка свежего `origin/main` Zemidala (`abcf1ec`, v1.23.2) показала, что подбор стратегии, проверка нескольких сайтов и сравнение чужого конфига предупреждают, когда Podkop подменяет DNS служебным FakeIP. XKeen UI сверена по последнему локальному тегу v1.4.2 (`ba04efa`); в этой задаче изменений XKeen не выявлено.
- Реализована та же проверка маршрута: только при наличии `/etc/init.d/podkop` backend спрашивает resolver Podkop `127.0.0.42` через `nslookup` (с fallback на `busybox nslookup`), распознаёт диапазон `198.18.0.0/15` и ограничивает каждую lookup двумя секундами. Вызовы сделаны без shell; параллельны только для уже ограниченного числа проверяемых доменов. Тесты стратегии не блокируются и не меняются — UI поясняет, что результат может показывать туннель, а не nfqws2.
- Список обнаруженных доменов сохраняется в операциях одиночного/многосайтового подбора и проверки чужих конфигов, так что предупреждение остаётся в результате и после повторного чтения операции. Старые JSON-операции продолжают читаться; пустое поле не сериализуется.
- Добавлены Rust-тесты диапазона и разбора ответа `nslookup`, проверки совместимости старых JSON-операций, frontend-тесты текста предупреждения и подключения к обоим экранам. Проверено: 372 frontend-теста, TypeScript typecheck, production build, `cargo fmt --check`, все Rust-наборы (401 тест), `cargo clippy --all-targets --locked -- -D warnings`, ARM64/musl `cargo check` и `git diff --check` прошли. Chromium-предпросмотр с локальным mock API показал предупреждение над результатами в desktop viewport; мок и вкладка закрыты, временный скрипт удалён. Консоль браузера этим provider недоступна. Реальный роутер и работающий Podkop не тестировались; релиз не выпускался.

## Диагностика сайта — совпадения в активных списках

- Вынес чтение и сопоставление runtime-списков из `diagnose_site` в отдельный асинхронный helper. Сохранил лимит чтения 512 КиБ на файл и список непрочитанных/слишком больших списков в результате диагностики.
- Добавлен тест с активными пользовательскими hostlist и ipset: домен и IP/CIDR попадают в результат, список соседнего инструмента не читается. Существующая проверка подтверждает дедупликацию стандартного файла, если тот же путь передан процессу явно.
- Проверено: 411 backend-тестов, `cargo fmt --check` и Clippy `-D warnings`. Роутер и реальные сетевые проверки не запускались; релиз не выпускался.

## Единая подсветка аргументов в группах конфигурации

- Вынес общий подсветчик строк конфигурации в `web/src/config-syntax.ts`; обычный редактор и редакторы `NFQWS_*_ARGS` используют один и тот же renderer и классы палитры.
- Исправлена классификация Lua-параметров: после `--lua-desync=<режим>` имя параметра (`fails`, `time`, `seqovl_pattern`) получает цвет ключа, а числовые и текстовые значения — цвет значения. Имя режима и флаги сохраняют собственные цвета.
- Добавлены unit-тесты на различение ключей/значений, одинаковую подсветку отдельных аргументов и строки в присваивании конфигурации, а также HTML-экранирование пользовательского текста.
- Проверено: 375 frontend-тестов, TypeScript `--noEmit`, production build и `git diff --check`. Весь UI backend-набор тестов также проверен отдельно в этой ветке; физический роутер не затрагивался. Релиз не выпускался.

## Время шагов диагностики и аудит Telegram/сервиса

- Шаги DNS, TCP-соединения, чтения маршрута и сверки активных списков теперь возвращают длительность фактически выполненной проверки; время неуспешного TCP-подключения также сохраняется. Пропущенные шаги остаются без фиктивной длительности.
- Очередь диагностики остаётся нейтральной, пока не получен полный ответ backend. После ответа анимация использует реальные статусы шагов, поэтому промежуточное отображение не подтверждает несуществующий успех.
- Подтверждено: экран Telegram загружает сохранённый транспорт; поля интерфейса и прокси видимы только при выборе соответствующего транспорта. Добавлен round-trip тест настроек direct/interface/proxy через файл monitoring store.
- Сверка со свежим локальным checkout Zemidala подтвердила, что имена `nfqws2-ui-ctl`, `S99nfqws2-ui` и портовый диапазон 1002–1100 являются намеренными отличиями этого проекта; daemon nfqws2 остаётся штатным `S51nfqws2`. Исправлений по этому пункту не требуется.
- Проверено: `cargo fmt --check`, полный `cargo test`, тесты Telegram store и queue диагностики. На роутере выполнен один read-only `GET /api/health` (HTTP 200); нагрузочные и диагностические сетевые пробы не выполнялись.
- Релиз v1.5.2 опубликован: https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v1.5.2. Tagged workflow `40534286385` завершился успешно; installer, ARM64/musl archive и SHA-256 доступны. Опубликованный архив скачан, checksum совпал: `3c04edceb05f44d63ba0165a3c66d921973b4c43c43ef5d5a6aa2ff1cca41eee`.

## ISP_INTERFACE в быстрых параметрах

- Добавлена видимая подпись `ISP_INTERFACE` и метка `WAN` в карточку выбора интерфейса; селект выровнен по вертикальному отступу с остальными быстрыми параметрами.
- Поведение выбора обнаруженных интерфейсов и ручного ввода не менялось.
- Добавлен структурный тест карточки. Проверено: 376 frontend-тестов, TypeScript typecheck, production build и `git diff --check` прошли. Релиз не выпускался.

## Плашка обновления ядра — учитывать точное решение backend

- Проверка логики плашки показала, что frontend игнорировал передаваемый backend флаг `updateAvailable` и самостоятельно отбрасывал суффикс после дефиса. Из-за этого новую пакетную ревизию, например `v1.3.2-r2` при установленной `1.3.2-r1`, можно было не показать.
- Плашка и подсветка кнопки ядра теперь используют backend-решение и сохраняют полный tag. Добавлены регрессионные тесты для пакетной ревизии и для случая, когда backend явно сообщает, что обновления нет.
- Проверка RED подтвердила воспроизведение дефекта; после исправления прошли все 379 frontend-тестов и TypeScript typecheck. Production build и полный релизный gate запускаются после завершения текущего набора правок. Релиз не выпускался.

## Проверка подключённых цепочек перехвата

- Сверка со свежим Zemidala `main` (`abcf1ec`, v1.23.2) выявила важное отличие статуса: раньше панель показывала только состояние процесса. Теперь read-only проверка подтверждает, что цепочки `nfqws_post` и `nfqws_pre` действительно связаны с `POSTROUTING` и `PREROUTING`.
- Проверка использует ограниченный набор `iptables -S` для двух цепочек и двух hook-направлений, общий timeout 2 секунды и кэш результата на 10 секунд. Отсутствующие переходы должны наблюдаться не менее 20 секунд: это повторяет защиту актуального Zemidala от ложного предупреждения во время перестройки firewall на Keenetic. Неизвестное наблюдение сбрасывает таймер. Firewall не меняется; при остановленном сервисе, отсутствующей утилите, ошибке, timeout или неизвестной схеме UI не показывает ложную ошибку.
- При подтверждённом отсутствующем переходе статус процесса остаётся фактическим, но подпись становится короткой янтарной («Перехват частичный» / «Перехват отключён»); подсказка перечисляет направления и советует перезапустить службу и проверить журнал. Статус-кнопка не меняет семантику.
- Добавлены Rust-тесты корректных/отсутствующих переходов, похожих имён, неизвестных снимков и 20-секундной grace-проверки, API-регрессия и frontend-регрессия. Chromium с локальным mock API проверен на 1440×900 и 390×844: предупреждение читаемое, компактное и не перекрывает кнопки; mock остановлен.
- Лёгкая read-only проверка роутера сначала не обнаружила переходы, а повторная проверка позже увидела оба hook подключёнными — это подтверждает необходимость grace-периода. Конфигурация, firewall и файлы на устройстве не менялись; релиз не выпускался.
- Проверено перед следующим этапом: 377 frontend-тестов, TypeScript typecheck, production build; все 421 Rust-тесты, `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked` и `git diff --check` прошли.

## Мониторинг при политике доступа Keenetic

- Сверил актуальный Zemidala `main` (`abcf1ec`, v1.23.2): при активном `POLICY_NAME` исходящие проверки самого роутера нужно помечать runtime fwmark из `nfqws_post`, иначе мониторинг не проверяет тот же маршрут, что клиентские устройства.
- Для подтверждённого режима `only` используется выделенный короткоживущий диапазон TCP-портов 41000–41099. Временная цепочка `mangle OUTPUT` маркирует только соединения этого диапазона; GET и условная контрольная отправка закреплены за ним. Если чтение firewall/config завершается ошибкой, проверка получает `unavailable` и не отправляется. Если активного policy-only mark нет, мониторинг остаётся на стандартном пути, как в Zemidala.
- Временная цепочка удаляется после проверки; при отмене действует RAII-очистка. Дополнительно guard очистки теперь регистрируется **до** `iptables -N`: если команда завершилась тайм-аутом после принятия правила ядром, код всё равно пытается удалить цепочку и сообщает, если очистка не подтверждена.
- Fallback на следующий IP выполняется только после сетевой ошибки/предупреждения; результат `unavailable` (например, не удалось подготовить policy-route) не маскируется повторной попыткой.
- Проверено относительно API/логики Zemidala: правило в `mangle OUTPUT`, mark только из активного policy-only правила `nfqws_post`, те же исходные порты и явная очистка временных правил. Добавлены тесты для разборки mark, игнорирования служебных меток, диапазона GET/POST и cleanup guard; RED подтверждён на ошибочной трактовке `POLICY_NAME` без подходящего runtime mark и на отсутствующем guard.
- Read-only сверка роутера: `POLICY_NAME="nfqws"`, но в `nfqws_post` присутствует служебное `0x40000000/0x40000000`, а не policy-only mark с маской `0x0fffffff`, которую использует Zemidala. Поэтому эту метку нельзя ошибочно применять к проверочным запросам; текущая реализация оставляет стандартный маршрут. `ip rule` просмотрен без изменений. Активные firewall-правила не менялись, сетевые пробы не запускались.
- После правки прошли 424 backend-теста, `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo check --target aarch64-unknown-linux-musl --locked`; frontend: 379 тестов и `tsc -b && vite build`. `git diff --check` прошёл. Релиз не выпускался.

## Не хранить старые версии панели после обновления

- Причина накопления файлов: штатный `install.sh` удалял старые релизы после успешного запуска, а `update-release.sh`, используемый обновлением из UI, оставлял их все.
- Оба сценария вызывают внутреннюю команду `nfqws2-ui-ctl prune-releases`: после прохождения health-check и проверки перезапуска nfqws2 остаётся только установленный релиз, а прошлые каталоги удаляются. До успешной проверки предыдущая версия сохраняется временно, чтобы откатить неудачное обновление; после успешного завершения резервная копия релиза не хранится.
- Добавлена изолированная проверка временных каталогов: текущий релиз остаётся, все старые удаляются, путь вне каталога релизов отклоняется. Скрипт включён в CI и архив релиза.
- Изменение подготовлено для следующего релиза; текущий опубликованный установщик на роутере пока не менялся и сам по себе настройку не применит.
# Локальное сохранение переноса строк и компактный экран проверки 16 КБ

- Переключатель «Перенос строк» в редакторе конфигурации сохраняет своё состояние в настройках браузера. При запуске оно применяется к самому полю, подсветке и положению чекбокса; отключённый пользователем перенос не включается повторно.
- Локальная настройка не отправляется в API оформления роутера. Синхронизация остальных настроек с роутером сохраняет выбранное состояние переноса строк.
- Экран «Обрыв на 16 КБ» уплотнён по образцу раскладки Zemidala: уменьшены отступы карточки, заголовка, поля доменов и кнопок, а строки результатов занимают меньше высоты на широком и узком экранах. Содержимое и поведение сетевой проверки не менялись.
- Проверки: сначала добавленный тест воспроизвёл потерю значения `false`; после исправления прошли все 380 frontend-тестов, TypeScript typecheck, production build и `git diff --check`. В Chromium на локальном mock API проверил экран диагностики и подтвердил, что выключенный перенос строк остаётся выключенным после перезагрузки. Mock API остановлен; релиз не выпускался.

## Компактность диагностики «Обрыв на 16 КБ» — 27.10.2026

- Сверил страницу Zemidala: у него сетевые результаты подаются плотными строками, с короткими вертикальными отступами и мобильными карточками.
- В нашей странице оставил раздельные поля сети, подтверждённого IP/ASN и результата, включая раскрываемые замеры, но уменьшил вертикальные интервалы, размеры поля запуска и кнопок, высоту таблицы и внутренние отступы мобильных карточек.
- Обновил шаблонный UI-чек, чтобы он описывал новые классы компактной разметки.
