export interface Recording {
  audioPath: string | null;
  createdAt: string;
  duration: number;
  fullText: string;
  id: string;
  isPartial: boolean;
  language: string | null;
  modelId: string;
  title: string;
}

export interface TranscriptLine {
  duration: number;
  endTimeSecs: number;
  id: string;
  isFinal: boolean;
  lineId: number;
  recordingId: string;
  sortOrder: number;
  startTime: string;
  startTimeSecs: number;
  text: string;
}

export interface TranscriptLineUpdate {
  duration?: number;
  endTimeSecs?: number;
  kind: "started" | "textChanged" | "completed" | "interrupted";
  lineId: number;
  startTime?: string;
  startTimeSecs?: number;
  text: string;
}

export interface RecordingRow {
  audio_path: string | null;
  created_at: string;
  duration: number;
  full_text: string;
  id: string;
  is_partial: number;
  language: string | null;
  model_id: string;
  title: string;
}

export interface TranscriptLineRow {
  duration: number;
  end_time_secs: number;
  id: string;
  is_final: number;
  line_id: number;
  recording_id: string;
  sort_order: number;
  start_time: string;
  start_time_secs: number;
  text: string;
}

/** Convert a SQLite row to a Recording entity. */
export function rowToRecording(row: RecordingRow): Recording {
  return {
    id: row.id,
    title: row.title,
    createdAt: row.created_at,
    duration: row.duration,
    audioPath: row.audio_path,
    fullText: row.full_text,
    modelId: row.model_id,
    isPartial: Boolean(row.is_partial),
    language: row.language,
  };
}

export function rowToLine(row: TranscriptLineRow): TranscriptLine {
  return {
    id: row.id,
    recordingId: row.recording_id,
    lineId: row.line_id,
    text: row.text,
    startTime: row.start_time,
    startTimeSecs: row.start_time_secs,
    endTimeSecs: row.end_time_secs,
    duration: row.duration,
    sortOrder: row.sort_order,
    isFinal: Boolean(row.is_final),
  };
}

export function buildFullText(lines: Pick<TranscriptLine, "text">[]): string {
  return lines
    .map((line) => line.text.trim())
    .filter(Boolean)
    .join("\n\n");
}

/** Format duration in seconds to "M:SS" */
export function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

/** Generate a default title like "Recording – May 26, 2026 3:45 PM" */
export function generateDefaultTitle(date: Date = new Date()): string {
  return `Recording – ${date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  })}`;
}
