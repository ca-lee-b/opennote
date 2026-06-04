import { invoke } from "@tauri-apps/api/core";
import type Database from "@tauri-apps/plugin-sql";
import type { TranscriptionSegment } from "@/features/transcription/types";
import type {
  Recording,
  RecordingRow,
  TranscriptLine,
  TranscriptLineRow,
} from "@/types/recording";
import { buildFullText, rowToLine, rowToRecording } from "@/types/recording";

export interface CreateRecordingInput {
  audioPath?: string | null;
  createdAt?: string;
  duration?: number;
  fullText?: string;
  id?: string;
  isPartial?: boolean;
  language?: string | null;
  modelId?: string;
  title: string;
}

export interface CreateTranscriptLineInput {
  duration?: number;
  endTimeSecs?: number;
  id?: string;
  isFinal?: boolean;
  lineId: number;
  recordingId: string;
  sortOrder: number;
  startTime?: string;
  startTimeSecs?: number;
  text: string;
}

export interface FinalizeTranscriptLineInput {
  duration?: number;
  endTimeSecs?: number;
  lineId: number;
  recordingId: string;
  startTime?: string;
  startTimeSecs?: number;
  text: string;
}

function normalizeRecordingWrite(input: CreateRecordingInput): Recording {
  return {
    id: input.id ?? crypto.randomUUID(),
    title: input.title,
    createdAt: input.createdAt ?? new Date().toISOString(),
    duration: input.duration ?? 0,
    audioPath: input.audioPath ?? null,
    fullText: input.fullText ?? "",
    modelId: input.modelId ?? "",
    isPartial: input.isPartial ?? false,
    language: input.language ?? null,
  };
}

function normalizeTranscriptLineWrite(
  input: CreateTranscriptLineInput
): TranscriptLine {
  return {
    id: input.id ?? crypto.randomUUID(),
    recordingId: input.recordingId,
    lineId: input.lineId,
    text: input.text,
    startTime: input.startTime ?? "0:00",
    startTimeSecs: input.startTimeSecs ?? 0,
    endTimeSecs: input.endTimeSecs ?? input.duration ?? 0,
    duration: input.duration ?? 0,
    sortOrder: input.sortOrder,
    isFinal: input.isFinal ?? false,
  };
}

export class RecordingsRepository {
  private readonly db: Database;

  constructor(db: Database) {
    this.db = db;
  }

  async initialize(): Promise<void> {
    await this.db.execute("PRAGMA foreign_keys = ON");
  }

  async createRecording(input: CreateRecordingInput): Promise<Recording> {
    const recording = normalizeRecordingWrite(input);

    await this.db.execute(
      `INSERT INTO recordings (id, title, created_at, duration, model_id, is_partial, audio_path, language, full_text)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
      [
        recording.id,
        recording.title,
        recording.createdAt,
        recording.duration,
        recording.modelId,
        Number(recording.isPartial),
        recording.audioPath,
        recording.language,
        recording.fullText,
      ]
    );

    return recording;
  }

  async createRecordingWithSegments(
    input: Omit<CreateRecordingInput, "fullText">,
    segments: TranscriptionSegment[]
  ): Promise<Recording> {
    const recording = normalizeRecordingWrite(input);
    const result = await invoke<{ fullText: string }>(
      "create_recording_with_segments",
      { request: { ...recording, segments } }
    );

    return { ...recording, fullText: result.fullText };
  }

  async deleteRecording(id: string): Promise<void> {
    await this.db.execute("DELETE FROM recordings WHERE id = $1", [id]);
  }

  async finalizeLine(input: FinalizeTranscriptLineInput): Promise<void> {
    await this.db.execute(
      `INSERT INTO transcript_lines (id, recording_id, line_id, text, start_time, start_time_secs, end_time_secs, duration, sort_order, is_final)
       VALUES (
         $1,
         $2,
         $3,
         $4,
         COALESCE($5, datetime('now')),
         COALESCE($6, 0),
         COALESCE($7, 0),
         COALESCE($8, 0),
         COALESCE((SELECT MAX(sort_order) + 1 FROM transcript_lines WHERE recording_id = $2), 0),
         1
       )
       ON CONFLICT(recording_id, line_id) DO UPDATE SET
         text = excluded.text,
         start_time = excluded.start_time,
         start_time_secs = excluded.start_time_secs,
         end_time_secs = excluded.end_time_secs,
         duration = excluded.duration,
         is_final = 1`,
      [
        crypto.randomUUID(),
        input.recordingId,
        input.lineId,
        input.text,
        input.startTime ?? null,
        input.startTimeSecs ?? null,
        input.endTimeSecs ?? null,
        input.duration ?? null,
      ]
    );
    await this.rebuildFullText(input.recordingId);
  }

  async getLines(recordingId: string): Promise<TranscriptLine[]> {
    const rows = await this.db.select<TranscriptLineRow[]>(
      `SELECT * FROM transcript_lines
       WHERE recording_id = $1
       ORDER BY sort_order ASC`,
      [recordingId]
    );
    return rows.map(rowToLine);
  }

  async getRecording(id: string): Promise<Recording | null> {
    const rows = await this.db.select<RecordingRow[]>(
      "SELECT * FROM recordings WHERE id = $1 LIMIT 1",
      [id]
    );
    return rows[0] ? rowToRecording(rows[0]) : null;
  }

  async insertLine(input: CreateTranscriptLineInput): Promise<TranscriptLine> {
    const line = normalizeTranscriptLineWrite(input);

    await this.insertTranscriptLine(line);

    await this.rebuildFullText(input.recordingId);
    return line;
  }

  async listRecordings(): Promise<Recording[]> {
    const rows = await this.db.select<RecordingRow[]>(
      "SELECT * FROM recordings ORDER BY created_at DESC"
    );
    return rows.map(rowToRecording);
  }

  async rebuildFullText(recordingId: string): Promise<void> {
    const lines = await this.getLines(recordingId);
    await this.db.execute(
      "UPDATE recordings SET full_text = $1 WHERE id = $2",
      [buildFullText(lines), recordingId]
    );
  }

  async renameRecording(id: string, title: string): Promise<void> {
    await this.db.execute("UPDATE recordings SET title = $1 WHERE id = $2", [
      title,
      id,
    ]);
  }

  async setPartial(id: string, isPartial: boolean): Promise<void> {
    await this.db.execute(
      "UPDATE recordings SET is_partial = $1 WHERE id = $2",
      [Number(isPartial), id]
    );
  }

  async updateDuration(id: string, duration: number): Promise<void> {
    await this.db.execute("UPDATE recordings SET duration = $1 WHERE id = $2", [
      duration,
      id,
    ]);
  }

  async updateLineText(lineId: string, text: string): Promise<void> {
    const rows = await this.db.select<Array<{ recording_id: string }>>(
      "SELECT recording_id FROM transcript_lines WHERE id = $1 LIMIT 1",
      [lineId]
    );
    const recordingId = rows[0]?.recording_id;

    await this.db.execute(
      "UPDATE transcript_lines SET text = $1 WHERE id = $2",
      [text, lineId]
    );

    if (recordingId) {
      await this.rebuildFullText(recordingId);
    }
  }

  private async insertTranscriptLine(line: TranscriptLine): Promise<void> {
    await this.db.execute(
      `INSERT INTO transcript_lines (id, recording_id, line_id, text, start_time, start_time_secs, end_time_secs, duration, sort_order, is_final)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
       ON CONFLICT(recording_id, line_id) DO UPDATE SET
         text = excluded.text,
         start_time = excluded.start_time,
         start_time_secs = excluded.start_time_secs,
         end_time_secs = excluded.end_time_secs,
         duration = excluded.duration,
         sort_order = excluded.sort_order,
         is_final = excluded.is_final`,
      [
        line.id,
        line.recordingId,
        line.lineId,
        line.text,
        line.startTime,
        line.startTimeSecs,
        line.endTimeSecs,
        line.duration,
        line.sortOrder,
        Number(line.isFinal),
      ]
    );
  }
}
