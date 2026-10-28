export type SnapshotFreeSpaceState = 'unknown' | 'low' | 'sufficient';

export function snapshotFreeSpaceState(availableBytes: number | null, maxArchiveBytes: number): SnapshotFreeSpaceState {
  if (availableBytes === null || !Number.isFinite(availableBytes) || availableBytes < 0) return 'unknown';
  return availableBytes < maxArchiveBytes ? 'low' : 'sufficient';
}
