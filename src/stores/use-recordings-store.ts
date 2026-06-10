import Database from "@tauri-apps/plugin-sql";
import { create } from "zustand";
import {
  type CreateTranscriptLineInput,
  type FinalizeTranscriptLineInput,
  RecordingsRepository,
} from "@/features/library/api/recordings-repository";
import {
  enqueueRecordingTranscription as enqueueRecordingTranscriptionCommand,
  importAudioForTranscription as importAudioForTranscriptionCommand,
  listRecordingProcessingStatuses,
  resumeRecordingProcessing as resumeRecordingProcessingCommand,
} from "@/features/transcription/api/transcription-service";
import type {
  EnqueueRecordingTranscriptionRequest,
  EnqueueRecordingTranscriptionResult,
  RecordingProcessingStatus,
} from "@/features/transcription/types";
import { getAppPreferences } from "@/lib/app-preferences";
import {
  markRecordingAsRecent,
  reconcileRecentRecordingIds,
} from "@/lib/recent-recordings";
import type { Recording, TranscriptLine } from "@/types/recording";
import { generateDefaultTitle } from "@/types/recording";

/** Module-level repository — not serializable state, so it lives outside Zustand. */
let repository: RecordingsRepository | null = null;
const AUDIO_EXTENSION_PATTERN = /\.[^.]+$/;
const PATH_SEPARATOR_PATTERN = /[/\\]/;

function titleFromAudioPath(path: string): string {
  const filename = path.split(PATH_SEPARATOR_PATTERN).pop() ?? path;
  return (
    filename.replace(AUDIO_EXTENSION_PATTERN, "").trim() || "Imported Audio"
  );
}

function arraysMatch(left: string[], right: string[]) {
  return (
    left.length === right.length &&
    left.every((id, index) => id === right[index])
  );
}

interface RecordingsState {
  createRecording: (partial?: Partial<Recording>) => Promise<Recording>;
  deleteRecording: (id: string) => Promise<void>;
  deleteRecordings: (ids: string[]) => Promise<void>;
  enqueueRecordingTranscription: (
    input: Omit<EnqueueRecordingTranscriptionRequest, "title"> & {
      title?: string;
    }
  ) => Promise<EnqueueRecordingTranscriptionResult>;
  finalizeLine: (line: FinalizeTranscriptLineInput) => Promise<void>;
  importAudioFile: (
    sourceAudioPath: string
  ) => Promise<EnqueueRecordingTranscriptionResult>;
  initialize: () => Promise<void>;
  insertLine: (line: CreateTranscriptLineInput) => Promise<TranscriptLine>;
  isImportingAudio: boolean;
  isLoading: boolean;
  linesByRecordingId: Record<string, TranscriptLine[]>;
  loadProcessingStatuses: () => Promise<RecordingProcessingStatus[]>;
  loadRecordings: (options?: { selectDefault?: boolean }) => Promise<void>;
  loadRecordingWithLines: (id: string) => Promise<void>;
  processingStatusesByRecordingId: Record<string, RecordingProcessingStatus>;
  recentRecordingIds: string[];
  recordings: Recording[];
  renameRecording: (id: string, title: string) => Promise<void>;
  resumeRecordingProcessing: (recordingId: string) => Promise<void>;
  searchText: string;
  selectedRecordingId: string | null;
  selectedRecordingIds: string[];
  selectRecording: (id: string | null) => void;
  selectRecordings: (ids: string[], primaryId?: string) => void;
  setPartial: (id: string, isPartial: boolean) => Promise<void>;
  setSearchText: (text: string) => void;
  updateDuration: (id: string, duration: number) => Promise<void>;
  updateLineText: (lineId: string, text: string) => Promise<void>;
}

function reconcileRecordingSelection({
  recordings,
  selectDefault,
  selectedRecordingId,
  selectedRecordingIds,
  selectRecording,
  selectRecordings,
  setSelectedRecordingIds,
}: {
  recordings: Recording[];
  selectDefault?: boolean;
  selectedRecordingId: string | null;
  selectedRecordingIds: string[];
  selectRecording: RecordingsState["selectRecording"];
  selectRecordings: RecordingsState["selectRecordings"];
  setSelectedRecordingIds: (ids: string[]) => void;
}) {
  const existingIds = new Set(recordings.map((recording) => recording.id));
  const reconciledSelectedIds = [...new Set(selectedRecordingIds)].filter(
    (id) => existingIds.has(id)
  );

  if (selectedRecordingId && existingIds.has(selectedRecordingId)) {
    const nextSelectedIds = reconciledSelectedIds.includes(selectedRecordingId)
      ? reconciledSelectedIds
      : [selectedRecordingId, ...reconciledSelectedIds];
    if (!arraysMatch(nextSelectedIds, selectedRecordingIds)) {
      setSelectedRecordingIds(nextSelectedIds);
    }
    return;
  }

  if (reconciledSelectedIds.length > 0) {
    selectRecordings(reconciledSelectedIds, reconciledSelectedIds[0]);
    return;
  }

  if (
    recordings.length > 0 &&
    (selectDefault || selectedRecordingId || selectedRecordingIds.length > 0)
  ) {
    selectRecording(recordings[0].id);
    return;
  }

  if (recordings.length === 0) {
    selectRecording(null);
  }
}

export const useRecordingsStore = create<RecordingsState>((set, get) => ({
  recordings: [],
  processingStatusesByRecordingId: {},
  recentRecordingIds: [],
  linesByRecordingId: {},
  selectedRecordingId: null,
  selectedRecordingIds: [],
  searchText: "",
  isLoading: true,
  isImportingAudio: false,

  initialize: async () => {
    try {
      const db = await Database.load("sqlite:opennote.db");
      repository = new RecordingsRepository(db);
      await repository.initialize();
      await get().loadRecordings({ selectDefault: true });
      await get().loadProcessingStatuses();
    } catch (error) {
      console.error("Failed to initialize database:", error);
      set({ isLoading: false });
    }
  },

  loadRecordings: async (options) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    try {
      const recordings = await repository.listRecordings();
      const recentRecordingIds = reconcileRecentRecordingIds(
        recordings.map((recording) => recording.id)
      );
      set({ recordings, recentRecordingIds, isLoading: false });

      const { selectedRecordingId, selectedRecordingIds } = get();
      reconcileRecordingSelection({
        recordings,
        selectDefault: options?.selectDefault,
        selectedRecordingId,
        selectedRecordingIds,
        selectRecording: get().selectRecording,
        selectRecordings: get().selectRecordings,
        setSelectedRecordingIds: (ids) => set({ selectedRecordingIds: ids }),
      });
    } catch (error) {
      console.error("Failed to load recordings:", error);
      set({ isLoading: false });
    }
  },

  selectRecording: (id) => {
    set({
      recentRecordingIds: id
        ? markRecordingAsRecent(id)
        : get().recentRecordingIds,
      selectedRecordingId: id,
      selectedRecordingIds: id ? [id] : [],
    });
    if (id) {
      get()
        .loadRecordingWithLines(id)
        .catch((error) => {
          console.error("Failed to load transcript lines:", error);
        });
    }
  },

  selectRecordings: (ids, primaryId) => {
    const selectedRecordingIds = [...new Set(ids)];
    const selectedRecordingId =
      primaryId && selectedRecordingIds.includes(primaryId)
        ? primaryId
        : (selectedRecordingIds[0] ?? null);

    set({
      recentRecordingIds: selectedRecordingId
        ? markRecordingAsRecent(selectedRecordingId)
        : get().recentRecordingIds,
      selectedRecordingId,
      selectedRecordingIds,
    });

    if (selectedRecordingId) {
      get()
        .loadRecordingWithLines(selectedRecordingId)
        .catch((error) => {
          console.error("Failed to load transcript lines:", error);
        });
    }
  },

  setSearchText: (text) => {
    set({ searchText: text });
  },

  createRecording: async (partial = {}) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    const now = new Date();
    const recording = await repository.createRecording({
      id: partial.id,
      title: partial.title ?? generateDefaultTitle(now),
      createdAt: partial.createdAt ?? now.toISOString(),
      duration: partial.duration ?? 0,
      audioPath: partial.audioPath ?? null,
      fullText: partial.fullText ?? "",
      modelId: partial.modelId ?? "",
      isPartial: partial.isPartial ?? false,
      language: partial.language ?? null,
    });

    await get().loadRecordings();
    get().selectRecording(recording.id);
    return recording;
  },

  enqueueRecordingTranscription: async (input) => {
    const startedAt = input.startedAt ? new Date(input.startedAt) : new Date();
    const result = await enqueueRecordingTranscriptionCommand({
      ...input,
      title: input.title ?? generateDefaultTitle(startedAt),
    });

    await get().loadRecordings();
    get().selectRecording(result.recordingId);
    await get().loadProcessingStatuses();
    return result;
  },

  importAudioFile: async (sourceAudioPath) => {
    set({ isImportingAudio: true });
    try {
      const selectedModelId = getAppPreferences().selectedModelId;
      if (!selectedModelId) {
        throw new Error(
          "Select and download a transcription model in Settings before importing audio."
        );
      }
      const result = await importAudioForTranscriptionCommand({
        modelId: selectedModelId,
        sourceAudioPath,
        title: titleFromAudioPath(sourceAudioPath),
      });

      await get().loadRecordings();
      get().selectRecording(result.recordingId);
      await get().loadProcessingStatuses();
      return result;
    } finally {
      set({ isImportingAudio: false });
    }
  },

  deleteRecording: async (id) => {
    await get().deleteRecordings([id]);
  },

  deleteRecordings: async (ids) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    const uniqueIds = [...new Set(ids)];
    if (uniqueIds.length === 0) {
      return;
    }

    const recordingsRepository = repository;
    await Promise.all(
      uniqueIds.map((id) => recordingsRepository.deleteRecording(id))
    );
    set((state) => {
      const remainingLines = { ...state.linesByRecordingId };
      for (const id of uniqueIds) {
        delete remainingLines[id];
      }
      const selectedRecordingIds = state.selectedRecordingIds.filter(
        (id) => !uniqueIds.includes(id)
      );
      const selectedRecordingId =
        state.selectedRecordingId &&
        uniqueIds.includes(state.selectedRecordingId)
          ? null
          : state.selectedRecordingId;
      return {
        linesByRecordingId: remainingLines,
        selectedRecordingIds,
        selectedRecordingId,
      };
    });
    await get().loadRecordings();
    await get().loadProcessingStatuses();
  },

  loadProcessingStatuses: async () => {
    const statuses = await listRecordingProcessingStatuses();
    const statusesByRecordingId = Object.fromEntries(
      statuses.map((status) => [status.recordingId, status])
    );
    set({ processingStatusesByRecordingId: statusesByRecordingId });
    return statuses;
  },

  finalizeLine: async (line) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    await repository.finalizeLine(line);
    await get().loadRecordingWithLines(line.recordingId);
    await get().loadRecordings();
  },

  insertLine: async (line) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    const savedLine = await repository.insertLine(line);
    await get().loadRecordingWithLines(line.recordingId);
    await get().loadRecordings();
    return savedLine;
  },

  loadRecordingWithLines: async (id) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    const [recording, lines] = await Promise.all([
      repository.getRecording(id),
      repository.getLines(id),
    ]);

    set((state) => ({
      linesByRecordingId: { ...state.linesByRecordingId, [id]: lines },
      recordings: recording
        ? state.recordings.map((item) => (item.id === id ? recording : item))
        : state.recordings,
    }));
  },

  renameRecording: async (id, title) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    await repository.renameRecording(id, title);

    set((state) => ({
      recordings: state.recordings.map((recording) =>
        recording.id === id ? { ...recording, title } : recording
      ),
    }));
  },

  resumeRecordingProcessing: async (recordingId) => {
    await resumeRecordingProcessingCommand(recordingId);
    await get().loadProcessingStatuses();
    await get().loadRecordings();
  },

  setPartial: async (id, isPartial) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    await repository.setPartial(id, isPartial);
    await get().loadRecordings();
  },

  updateDuration: async (id, duration) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    await repository.updateDuration(id, duration);
    await get().loadRecordings();
  },

  updateLineText: async (lineId, text) => {
    if (!repository) {
      throw new Error("Database not initialized");
    }

    await repository.updateLineText(lineId, text);

    const recordingId = Object.entries(get().linesByRecordingId).find(
      ([, lines]) => lines.some((line) => line.id === lineId)
    )?.[0];

    if (recordingId) {
      await get().loadRecordingWithLines(recordingId);
    }
    await get().loadRecordings();
  },
}));
