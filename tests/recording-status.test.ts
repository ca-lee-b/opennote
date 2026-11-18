import { describe, expect, test } from "bun:test";
import { getRecordingStatusView } from "../src/features/library/utils/recording-status";
import type {
  RecordingProcessingState,
  RecordingProcessingStatus,
} from "../src/features/transcription/types";

function processingStatus(
  status: RecordingProcessingState
): RecordingProcessingStatus {
  return {
    completedChunks: 0,
    error: null,
    failedChunks: 0,
    jobId: "job-1",
    recordingId: "recording-1",
    status,
    totalChunks: 0,
    updatedAt: "2026-06-07T10:00:00.000Z",
  };
}

describe("recording status", () => {
  test("marks active processing states", () => {
    expect(
      getRecordingStatusView({
        isPartial: false,
        processingStatus: processingStatus("transcribing"),
      })
    ).toEqual({ canResume: false, isActive: true, label: "Processing" });
  });

  test("offers retry for interrupted recordings", () => {
    expect(
      getRecordingStatusView({
        isPartial: false,
        processingStatus: processingStatus("interrupted"),
      })
    ).toEqual({ canResume: true, isActive: false, label: "Interrupted" });
  });

  test("labels partial recordings without a processing status", () => {
    expect(getRecordingStatusView({ isPartial: true })).toEqual({
      canResume: false,
      isActive: false,
      label: "Incomplete",
    });
  });
});
