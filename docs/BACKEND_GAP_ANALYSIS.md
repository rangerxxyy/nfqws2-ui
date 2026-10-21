# nfqws2-ui backend gap analysis

Audit snapshot: 2026-10-21. The repository tree and every frontend action were inspected. Upstream behavior was checked against the `nfqws2-keenetic` README; upstream path information below is not a live-device check. No router connection was available in this session.

Upstream references:

- [zemidala/nfqws2-ui](https://github.com/zemidala/nfqws2-ui)
- [nfqws/nfqws-keenetic-web](https://github.com/nfqws/nfqws-keenetic-web)
- [nfqws/nfqws2-keenetic README](https://github.com/nfqws/nfqws2-keenetic/blob/master/README.md)
- [bol-van/zapret2](https://github.com/bol-van/zapret2)

The older `nfqws-keenetic-web` and `zapret2` projects use different code and configuration paths. Their option names and scripts must not be copied into nfqws2 without checking the installed nfqws2 version and its own documentation.

## UI-to-backend inventory

| Функция | Как сделано у источника | Как сделано у нас | Недостаток | Путь на устройстве | План исправления | Результат теста |
|---|---|---|---|---|---|---|
| Верхний статус, RAM/CPU, версии | Reads the running installation and service state | `GET /api/status`; `GET /api/nfqws2/installation` reports known candidates, permissions and statically referenced list paths | Platform detection is intentionally unknown outside Entware paths; candidates, running binary/cmdline and config references are incomplete | Entware `/opt/etc/init.d/S51nfqws2`; OpenWrt paths need device confirmation | Inspect `/proc` and init/package metadata; confirm supported paths on the target router | Existing Rust status and installation-endpoint tests pass; router not checked |
| Start/stop/restart | Calls the package's supported service controls | `POST /api/service`, closed action enum, direct `Command` invocation | Only the two known init-script paths are supported; no reload action | `/opt/etc/init.d/S51nfqws2` or `/etc/init.d/nfqws2-keenetic` | Resolve actual installed script and verify post-action process state | Existing service tests pass; router not checked |
| Config editor | Reads and writes the detected `nfqws2.conf`, validates and rolls back failed application | `GET/PUT /api/config`; revision, backup, atomic replacement; UI can restart service | No safe parser/validator; saving and service application are separate; rollback after failed restart is missing | Upstream documents `/opt/etc/nfqws2/nfqws2.conf`; OpenWrt commonly `/etc/nfqws2/nfqws2.conf` | Discover active config, preserve unknown syntax, validate before apply, then verify or rollback | Existing file-write tests pass; no nfqws2 validation on device |
| Domain and IP lists | Manages actual lists and their profile references | `GET /api/nfqws2/lists`, `GET/PUT /api/nfqws2/lists/:id`; IDs map to literal active-config flags; UI edits raw content and can remove duplicate entries in a local draft | Shell-variable expansion and multiple distinct references are refused as ambiguous; dedicated add/remove/import/export and automatic service reload are not implemented | Path is read from `--hostlist`, `--hostlist-auto`, `--hostlist-exclude`, `--ipset`, or `--ipset-exclude`; must resolve under the config directory | Add platform discovery, reference map with multiple uses, remaining list operations, and package-confirmed reload/apply behavior | Rust path/revision/backup/read-back tests pass; frontend dedupe tests preserve comments/order; router not checked |
| Site diagnosis | Runs network and policy checks on the router | UI action `runSiteDiagnosis` is explicitly disabled | No diagnosis API or result model | Requires route/DNS/network access on router | Design bounded diagnostic probes using fixed commands and timeouts | Not implemented |
| Strategy scan and profiles | Tests isolated nfqws2 candidates and persists verified profiles | UI scan/apply actions are explicitly disabled | No safe test-process manager, strategy parser, validation or apply/rollback transaction | Depends on actual binary, modules, config and queue support | Inspect installed version and implement isolated validation before profile CRUD | Not implemented; no legacy `--dpi-desync` options are accepted by new code |
| Monitor and Telegram | Uses persistent monitor and notification settings | UI contains display examples; actions are disabled or show placeholder state | No Rust endpoints or persistence model | Paths are installation-specific and not discovered | Add only after platform/path inventory and a separate settings contract | Not implemented |
| Community strategies / ASN | Reads remote community data and router/provider information | UI buttons are disabled; ASN area is a mock display | No API, persistence or network policy | No confirmed device path | Implement read-only provider/ASN lookup first, then explicit safe apply | Not implemented |
| Snapshots | Stores and restores real config/list snapshots | Backend lists, creates, and restores bounded `nfqws2.conf` copies; UI create and restore are wired. Restore saves the current config first and does not restart nfqws2 | No multi-file transaction, list snapshots, export, automatic schedule, or retention policy | Panel backups live under the application data directory at `backups/` | Add multi-file snapshots and retention only after a transactional manager exists | Rust tests cover create and restore mechanics on temporary files; router not checked |
| Logs | Shows live service logs | `GET /api/logs` reads a bounded tail from known candidate paths | Candidate path selection is fixed; debug log path is not discovered | `/opt/var/log/nfqws2.log`, `/opt/var/log/nfqws.log`, `/var/log/nfqws2.log` | Read `LOG_DEBUG_PATH` and active process arguments after config discovery | Bounded-tail test passes; router not checked |
| UI/core updates | Uses the installation's supported package/release method | UI update buttons are disabled | No authenticated, verified update/rollback workflow | Package-specific | Keep disabled until signed/checksummed package workflow and recovery tests exist | Not implemented |

## New list API contract

## Authentication status

The user requested removing authentication completely. The login form, session/login/logout API routes, Argon2 password hash, cookie, CSRF checks/header, and installer password prompt have been removed. The API binds to `0.0.0.0`; all control endpoints are consequently unauthenticated. Restrict access at the router firewall and keep it on a trusted LAN. The installer removes the stale password hash from upgrades.

- `GET /api/nfqws2/lists` returns only existing, allowlisted list files explicitly referenced by supported literal flags in the selected config. The response reports the path and read/write access checks.
- `GET /api/nfqws2/lists/:id` reads one allowlisted file and returns its SHA-256 revision.
- `PUT /api/nfqws2/lists/:id` is unauthenticated per the user's explicit request; it validates domain or IP/CIDR lines, checks revision, creates a backup, atomically replaces the file and compares a subsequent read with the submitted bytes.
- IDs are fixed server-side (`user.list`, `auto.list`, `exclude.list`, `ipset.list`, `ipset_exclude.list`). Missing files are not created. Paths outside the canonical config directory, unresolved shell variables, and symlink components are rejected.
- A successful response means the file was written and read back. It does not mean the running service reloaded or applied the list.

## Confirmed upstream path information

The nfqws2 package README documents these Entware paths: config `/opt/etc/nfqws2/nfqws2.conf`, lists under `/opt/etc/nfqws2/lists/`, init script `/opt/etc/init.d/S51nfqws2`, and log `/opt/var/log/nfqws2.log`. It states OpenWrt files omit the `/opt` prefix. These are source-based references only; the target router's actual paths, rights, package and active command line remain unverified.
