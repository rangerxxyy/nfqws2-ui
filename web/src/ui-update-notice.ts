export function uiUpdateNoticePresentation(
  status: { updateAvailable: boolean; latestVersion: string | null },
  dismissedVersion: string | null,
) {
  const version = status.updateAvailable && status.latestVersion && /^v\d+\.\d+\.\d+$/.test(status.latestVersion)
    ? status.latestVersion
    : null;
  return { highlight: version !== null, notify: version !== null && version !== dismissedVersion, version };
}
