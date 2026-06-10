import { describe, expect, test } from "bun:test";
import {
  createTranscriptSearchState,
  searchRecordings,
} from "../src/features/library/utils/recording-search";
import type { Recording, TranscriptLine } from "../src/types/recording";

const baseRecording: Recording = {
  id: "recording-1",
  title: "Planning Review",
  createdAt: "2026-06-07T10:00:00.000Z",
  duration: 180,
  audioPath: null,
  fullText:
    "We discussed the library search interface and transcript navigation before moving into a longer implementation review.",
  modelId: "whisper",
  isPartial: false,
  language: "en",
};

describe("recording search", () => {
  test("returns highlighted recording matches with transcript context", () => {
    const results = searchRecordings([baseRecording], "search");

    expect(results).toHaveLength(1);
    expect(results[0].titleSegments).toEqual([
      { text: "Planning Review", isMatch: false, start: 0 },
    ]);
    expect(results[0].snippet).toEqual({
      hasLeadingEllipsis: false,
      hasTrailingEllipsis: true,
      segments: [
        { text: "We discussed the library ", isMatch: false, start: 0 },
        { text: "search", isMatch: true, start: 25 },
        {
          text: " interface and transcript navigation",
          isMatch: false,
          start: 31,
        },
      ],
    });
  });

  test("keeps empty queries as an unfiltered recording list", () => {
    const results = searchRecordings([baseRecording], " ");

    expect(results).toHaveLength(1);
    expect(results[0].snippet).toBeNull();
    expect(results[0].matchCount).toBe(0);
  });
});

describe("transcript search", () => {
  const lines: TranscriptLine[] = [
    {
      id: "line-1",
      recordingId: "recording-1",
      lineId: 1,
      text: "Search starts in the sidebar.",
      startTime: "0:00",
      startTimeSecs: 0,
      endTimeSecs: 4,
      duration: 4,
      sortOrder: 0,
      isFinal: true,
    },
    {
      id: "line-2",
      recordingId: "recording-1",
      lineId: 2,
      text: "Then search jumps between transcript lines.",
      startTime: "0:04",
      startTimeSecs: 4,
      endTimeSecs: 8,
      duration: 4,
      sortOrder: 1,
      isFinal: true,
    },
  ];

  test("builds match-level active state and clamps the active index", () => {
    const state = createTranscriptSearchState(
      [
        {
          ...lines[0],
          text: "Search starts in the sidebar search field.",
        },
        lines[1],
      ],
      "search",
      1
    );

    expect(state.activeMatchIndex).toBe(1);
    expect(state.matches.map((match) => match.lineId)).toEqual([
      "line-1",
      "line-1",
      "line-2",
    ]);
    expect(state.lines[0].segments).toEqual([
      { text: "Search", isMatch: true, start: 0 },
      { text: " starts in the sidebar ", isMatch: false, start: 6 },
      { text: "search", isActive: true, isMatch: true, start: 29 },
      { text: " field.", isMatch: false, start: 35 },
    ]);
  });
});
