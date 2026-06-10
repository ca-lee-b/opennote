import type { RecordingProcessingStatus } from "@/features/transcription/types";

const ACTIVE_PROCESSING_STATUSES = new Set([
  "queued",
  "chunking",
  "transcribing",
]);

const RESUMEABLE_PROCESSING_STATUSES = new Set([
  "failed",
  "interrupted",
  "partial",
]);

export interface RecordingStatusView {
  canResume: boolean;
  isActive: boolean;
  label: string | null;
}

export function getRecordingStatusView({
  isPartial,
  processingStatus,
}: {
  isPartial: boolean;
  processingStatus?: RecordingProcessingStatus;
}): RecordingStatusView {
  const status = processingStatus?.status;

  if (ACTIVE_PROCESSING_STATUSES.has(status ?? "")) {
    return { canResume: false, isActive: true, label: "Processing" };
  }

  if (RESUMEABLE_PROCESSING_STATUSES.has(status ?? "")) {
    return {
      canResume: true,
      isActive: false,
      label: status === "interrupted" ? "Interrupted" : "Needs retry",
    };
  }

  if (isPartial) {
    return { canResume: false, isActive: false, label: "Incomplete" };
  }

  return { canResume: false, isActive: false, label: null };
}
