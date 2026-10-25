# План релиза v0.6.1

1. Автоподбор: уплотнить плитки, встроить поля своей стратегии в плитку режима и добавить копирование с обратной связью.
2. Конфигурация: выровнять действия копирования и применения с навигацией разделов.
3. Оформление: сделать всю площадь каждой карточки надежной областью выбора и сохранить клавиатурный доступ.
4. ASN: сверить формат RIPEstat и таймауты с zemidala, исправить обработку данных и покрыть регрессии тестами.
5. Обновить версию, журнал изменений, собрать и проверить пакет, затем опубликовать релиз GitHub v0.6.1.

## Коммиты

- `fix: improve strategy picker controls`
- `fix: align configuration actions and appearance cards`
- `fix: align RIPE ASN lookup with reference behavior`
- `release: publish v0.6.1`

## Проверки

- Web unit/template tests, typecheck и production build.
- Rust tests, formatting и lint checks.
- Проверка diff и release package metadata.
