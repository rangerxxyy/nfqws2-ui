import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const backendManifest = readFileSync(new URL('../../backend/Cargo.toml', import.meta.url), 'utf8');
const backendLock = readFileSync(new URL('../../backend/Cargo.lock', import.meta.url), 'utf8');
const frontendManifest = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as { version: string };
const releaseNotesUrl = new URL('../../RELEASE_v1.0.3.md', import.meta.url);
const releaseNotes = existsSync(releaseNotesUrl) ? readFileSync(releaseNotesUrl, 'utf8') : '';
const releaseWorkflow = readFileSync(new URL('../../.github/workflows/release.yml', import.meta.url), 'utf8');
const changelog = readFileSync(new URL('../../CHANGELOG.md', import.meta.url), 'utf8');

describe('v1.0.3 release metadata', () => {
  it('keeps Rust, web, lockfile, and release notes on the same version', () => {
    const backendVersion = backendManifest.match(/^version = "([^"]+)"/m)?.[1];
    const lockedVersion = backendLock.match(/name = "nfqws2-ui"\s+version = "([^"]+)"/)?.[1];

    expect(backendVersion).toBe('1.0.3');
    expect(frontendManifest.version).toBe(backendVersion);
    expect(lockedVersion).toBe(backendVersion);
    expect(releaseNotes).toMatch(/^# v1\.0\.3\b/);
  });

  it('uses the matching release note as the GitHub release body', () => {
    expect(releaseWorkflow).toContain('body_path: RELEASE_v${{ steps.release-meta.outputs.version }}.md');
    expect(releaseWorkflow).not.toContain('Убрана глобальная синяя подложка');
  });

  it('keeps the newest release first in the changelog', () => {
    const v103 = changelog.indexOf('## [1.0.3]');
    const v102 = changelog.indexOf('## [1.0.2]');
    const v101 = changelog.indexOf('## [1.0.1]');
    const v100 = changelog.indexOf('## [1.0.0]');
    const v098 = changelog.indexOf('## [0.9.8]');
    const v097 = changelog.indexOf('## [0.9.7]');
    const v096 = changelog.indexOf('## [0.9.6]');
    const v095 = changelog.indexOf('## [0.9.5]');
    const v094 = changelog.indexOf('## [0.9.4]');
    const v093 = changelog.indexOf('## [0.9.3]');
    const v092 = changelog.indexOf('## [0.9.2]');
    const v091 = changelog.indexOf('## [0.9.1]');

    expect(v095).toBeGreaterThanOrEqual(0);
    expect(v103).toBeGreaterThanOrEqual(0);
    expect(v102).toBeGreaterThanOrEqual(0);
    expect(v096).toBeGreaterThanOrEqual(0);
    expect(v101).toBeGreaterThanOrEqual(0);
    expect(v100).toBeGreaterThanOrEqual(0);
    expect(v101).toBeLessThan(v100);
    expect(v103).toBeLessThan(v102);
    expect(v102).toBeLessThan(v101);
    expect(v100).toBeLessThan(v098);
    expect(v098).toBeLessThan(v097);
    expect(v095).toBeGreaterThan(v096);
    expect(v094).toBeGreaterThan(v095);
    expect(v093).toBeGreaterThan(v094);
    expect(v092).toBeGreaterThan(v093);
    expect(v091).toBeGreaterThan(v092);
  });
});
