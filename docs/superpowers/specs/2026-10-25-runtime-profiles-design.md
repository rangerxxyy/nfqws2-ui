# nfqws2 runtime profiles

## Contract

The Profiles page edits the groups of nfqws2 arguments that the package turns
into runtime profiles. It does not store or switch complete copies of
`nfqws2.conf`. In particular, custom groups are sourced only from the literal
`NFQWS_ARGS_CUSTOM` assignment; package supplied groups (`NFQWS_ARGS_UDP`,
`NFQWS_ARGS_QUIC`, `NFQWS_ARGS_IPSET`, `NFQWS_ARGS`, and `NFQWS_EXTRA_ARGS`)
are shown read-only. This follows the reference's `expectedProfiles()` and
`splitNew()` behavior, ported from PHP to Rust and TypeScript.

Each item displays its source, profile name, filters, protocol/payload rules,
lists, and ordered `--lua-desync` steps. The first custom group is the literal
argument sequence before the first `--new`; subsequent groups begin at each
`--new` / `--new=name`. `--new=name` is represented in the group as
`--name=name`, matching nfqws2 argv semantics.

Mutations affect only `NFQWS_ARGS_CUSTOM`, preserve all other config bytes,
flatten only the parser-supported static/self-appended assignment forms,
validate the resulting config, and use the existing transactional apply path
(backup, write, restart, verify, rollback on failure). Adding, editing,
deleting, or reordering custom profiles is therefore an explicit apply to the
nfqws2 service. If the config contains shell expansion or duplicate assignment
for the editable variable, the editor must refuse mutation and direct the user
to resolve it in the raw config editor.

## UI

The page follows the supplied Zemidala reference: compact profile order list at
left, selected profile editor at right, clear lock treatment for managed
profiles, custom-profile actions, and text/configuration views. The TS UI uses
the existing panel design tokens and backend APIs; no shell/PHP is introduced.
