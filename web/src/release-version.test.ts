import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const backendManifest = readFileSync(new URL('../../backend/Cargo.toml', import.meta.url), 'utf8');
const backendLock = readFileSync(new URL('../../backend/Cargo.lock', import.meta.url), 'utf8');
const frontendManifest = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')) as { version: string };
const releaseNotesUrl = new URL('../../RELEASE_v0.9.3.md', import.meta.url);
const releaseNotes = existsSync(releaseNotesUrl) ? readFileSync(releaseNotesUrl, 'utf8') : '';
const releaseWorkflow = readFileSync(new URL('../../.github/workflows/release.yml', import.meta.url), 'utf8');
const changelog = readFileSync(new URL('../../CHANGELOG.md', import.meta.url), 'utf8');

describe('v0.9.3 release metadata', () => {
  it('keeps Rust, web, lockfile, and release notes on the same version', () => {
    const backendVersion = backendManifest.match(/^version = "([^"]+)"/m)?.[1];
    const lockedVersion = backendLock.match(/name = "nfqws2-ui"\s+version = "([^"]+)"/)?.[1];

    expect(backendVersion).toBe('0.9.3');
    expect(frontendManifest.version).toBe(backendVersion);
    expect(lockedVersion).toBe(backendVersion);
    expect(releaseNotes).toMatch(/^# v0\.9\.3\b/);
  });

  it('uses the matching release note as the GitHub release body', () => {
    expect(releaseWorkflow).toContain('body_path: RELEASE_v${{ steps.release-meta.outputs.version }}.md');
    expect(releaseWorkflow).not.toContain('Убрана глобальная синяя подложка');
  });

  it('keeps the newest release first in the changelog', () => {
    const v093 = changelog.indexOf('## [0.9.3]');
    const v092 = changelog.indexOf('## [0.9.2]');
    const v091 = changelog.indexOf('## [0.9.1]');

    expect(v093).toBeGreaterThanOrEqual(0);
    expect(v092).toBeGreaterThan(v093);
    expect(v091).toBeGreaterThan(v092);
  });
});
