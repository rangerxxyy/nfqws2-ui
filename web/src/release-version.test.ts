import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const backendManifest = readFileSync(new URL('../../backend/Cargo.toml', import.meta.url), 'utf8');
const backendLock = readFileSync(new URL('../../backend/Cargo.lock', import.meta.url), 'utf8');
const frontendManifest = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as { version: string };
const backendVersion = backendManifest.match(/^version = "([^"]+)"/m)?.[1];
if (!backendVersion) throw new Error('Cargo.toml has no package version');
const releaseNotesUrl = new URL(`../../RELEASE_v${frontendManifest.version}.md`, import.meta.url);
const releaseNotes = existsSync(releaseNotesUrl) ? readFileSync(releaseNotesUrl, 'utf8') : '';
const releaseWorkflow = readFileSync(new URL('../../.github/workflows/release.yml', import.meta.url), 'utf8');
const changelog = readFileSync(new URL('../../CHANGELOG.md', import.meta.url), 'utf8');

describe('release metadata', () => {
  it('keeps Rust, web, lockfile, and release notes on the same version', () => {
    const lockedVersion = backendLock.match(/name = "nfqws2-ui"\s+version = "([^"]+)"/)?.[1];

    expect(backendVersion).toMatch(/^\d+\.\d+\.\d+$/);
    expect(frontendManifest.version).toBe(backendVersion);
    expect(lockedVersion).toBe(backendVersion);
    expect(releaseNotes).toMatch(new RegExp(`^# v${backendVersion.replaceAll('.', '\\.')}\\b`, 'm'));
  });

  it('uses the matching release note as the GitHub release body', () => {
    expect(releaseWorkflow).toContain('body_path: RELEASE_v${{ steps.release-meta.outputs.version }}.md');
    expect(releaseWorkflow).not.toContain('Убрана глобальная синяя подложка');
  });

  it('keeps the newest release first in the changelog', () => {
    const changelogVersions = [...changelog.matchAll(/^## \[(\d+\.\d+\.\d+)\]/gm)].map((match) => match[1]);
    expect(changelogVersions[0]).toBe(frontendManifest.version);
    expect(changelogVersions.length).toBeGreaterThan(1);
    expect(new Set(changelogVersions).size).toBe(changelogVersions.length);
  });
});
