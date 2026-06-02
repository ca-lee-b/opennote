const RECENT_RECORDINGS_KEY = "opennote.recentRecordings";
const RECENT_RECORDINGS_LIMIT = 12;
const RECENT_RECORDINGS_VERSION = 1;

interface StoredRecentRecordings {
  ids: string[];
  version: typeof RECENT_RECORDINGS_VERSION;
}

let cachedRecentRecordingIds: string[] | null = null;

function sanitizeIds(ids: unknown): string[] {
  if (!Array.isArray(ids)) {
    return [];
  }

  return [...new Set(ids.filter((id): id is string => typeof id === "string"))]
    .filter(Boolean)
    .slice(0, RECENT_RECORDINGS_LIMIT);
}

function saveRecentRecordingIds(ids: string[]): string[] {
  const sanitizedIds = sanitizeIds(ids);
  cachedRecentRecordingIds = sanitizedIds;

  try {
    const payload: StoredRecentRecordings = {
      ids: sanitizedIds,
      version: RECENT_RECORDINGS_VERSION,
    };
    localStorage.setItem(RECENT_RECORDINGS_KEY, JSON.stringify(payload));
  } catch {
    // Recents are optional UI state. SQLite remains the recording source of truth.
  }

  return [...sanitizedIds];
}

export function getRecentRecordingIds(): string[] {
  if (cachedRecentRecordingIds) {
    return [...cachedRecentRecordingIds];
  }

  try {
    const value = localStorage.getItem(RECENT_RECORDINGS_KEY);
    if (!value) {
      cachedRecentRecordingIds = [];
      return [];
    }

    const payload = JSON.parse(value) as Partial<StoredRecentRecordings>;
    cachedRecentRecordingIds =
      payload.version === RECENT_RECORDINGS_VERSION
        ? sanitizeIds(payload.ids)
        : [];
  } catch {
    cachedRecentRecordingIds = [];
  }

  return [...cachedRecentRecordingIds];
}

export function markRecordingAsRecent(id: string): string[] {
  return saveRecentRecordingIds([
    id,
    ...getRecentRecordingIds().filter((recentId) => recentId !== id),
  ]);
}

export function reconcileRecentRecordingIds(recordingIds: string[]): string[] {
  const existingIds = new Set(recordingIds);
  const recentIds = getRecentRecordingIds();
  const reconciledIds = recentIds.filter((id) => existingIds.has(id));

  if (
    recentIds.length === reconciledIds.length &&
    recentIds.every((id, index) => id === reconciledIds[index])
  ) {
    return recentIds;
  }

  return saveRecentRecordingIds(reconciledIds);
}
