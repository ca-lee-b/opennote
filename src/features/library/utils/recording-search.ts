import type { Recording, TranscriptLine } from "@/types/recording";

export interface HighlightSegment {
  isActive?: boolean;
  isMatch: boolean;
  start: number;
  text: string;
}

export interface TranscriptSnippet {
  hasLeadingEllipsis: boolean;
  hasTrailingEllipsis: boolean;
  segments: HighlightSegment[];
}

export interface RecordingSearchResult {
  matchCount: number;
  recording: Recording;
  snippet: TranscriptSnippet | null;
  titleSegments: HighlightSegment[];
}

export interface TranscriptLineSearchResult {
  line: TranscriptLine;
  segments: HighlightSegment[];
}

export interface TranscriptSearchMatch {
  lineId: string;
  matchIndex: number;
}

export interface TranscriptSearchState {
  activeMatchIndex: number;
  lines: TranscriptLineSearchResult[];
  matches: TranscriptSearchMatch[];
  query: string;
}

const SNIPPET_CONTEXT_LENGTH = 40;

function normalizeQuery(query: string): string {
  return query.trim();
}

function countMatches(text: string, query: string): number {
  if (!query) {
    return 0;
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  let count = 0;
  let index = lowerText.indexOf(lowerQuery);

  while (index !== -1) {
    count += 1;
    index = lowerText.indexOf(lowerQuery, index + lowerQuery.length);
  }

  return count;
}

function createTranscriptLineHighlightSegments({
  activeMatchIndex,
  firstMatchIndex,
  query,
  text,
}: {
  activeMatchIndex: number;
  firstMatchIndex: number;
  query: string;
  text: string;
}): HighlightSegment[] {
  const segments = createHighlightSegments(text, query);
  let nextMatchIndex = firstMatchIndex;

  return segments.map((segment) => {
    if (!segment.isMatch) {
      return segment;
    }

    const matchIndex = nextMatchIndex;
    nextMatchIndex += 1;
    return {
      ...segment,
      ...(matchIndex === activeMatchIndex ? { isActive: true } : {}),
    };
  });
}

export function createHighlightSegments(
  text: string,
  query: string
): HighlightSegment[] {
  const normalizedQuery = normalizeQuery(query);
  if (!normalizedQuery) {
    return [{ text, isMatch: false, start: 0 }];
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = normalizedQuery.toLowerCase();
  const segments: HighlightSegment[] = [];
  let cursor = 0;
  let matchIndex = lowerText.indexOf(lowerQuery);

  while (matchIndex !== -1) {
    if (matchIndex > cursor) {
      segments.push({
        text: text.slice(cursor, matchIndex),
        isMatch: false,
        start: cursor,
      });
    }

    const matchEnd = matchIndex + normalizedQuery.length;
    segments.push({
      text: text.slice(matchIndex, matchEnd),
      isMatch: true,
      start: matchIndex,
    });
    cursor = matchEnd;
    matchIndex = lowerText.indexOf(lowerQuery, cursor);
  }

  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor), isMatch: false, start: cursor });
  }

  return segments.length > 0 ? segments : [{ text, isMatch: false, start: 0 }];
}

function createTranscriptSnippet(
  text: string,
  query: string
): TranscriptSnippet | null {
  const normalizedQuery = normalizeQuery(query);
  if (!normalizedQuery) {
    return null;
  }

  const matchIndex = text.toLowerCase().indexOf(normalizedQuery.toLowerCase());
  if (matchIndex === -1) {
    return null;
  }

  const rawStart = Math.max(0, matchIndex - SNIPPET_CONTEXT_LENGTH);
  const rawEnd = Math.min(
    text.length,
    matchIndex + normalizedQuery.length + SNIPPET_CONTEXT_LENGTH
  );
  const nextSpaceAfterStart = text.indexOf(" ", rawStart);
  const start =
    rawStart === 0 || nextSpaceAfterStart === -1
      ? rawStart
      : Math.min(matchIndex, nextSpaceAfterStart + 1);
  const end =
    rawEnd === text.length
      ? rawEnd
      : Math.max(
          matchIndex + normalizedQuery.length,
          text.lastIndexOf(" ", rawEnd)
        );
  const snippetText = text.slice(start, end).trim();

  return {
    hasLeadingEllipsis: start > 0,
    hasTrailingEllipsis: end < text.length,
    segments: createHighlightSegments(snippetText, normalizedQuery),
  };
}

export function searchRecordings(
  recordings: Recording[],
  query: string
): RecordingSearchResult[] {
  const normalizedQuery = normalizeQuery(query);

  return recordings.flatMap((recording) => {
    if (!normalizedQuery) {
      return [
        {
          recording,
          titleSegments: createHighlightSegments(recording.title, ""),
          snippet: null,
          matchCount: 0,
        },
      ];
    }

    const titleMatchCount = countMatches(recording.title, normalizedQuery);
    const transcriptMatchCount = countMatches(
      recording.fullText,
      normalizedQuery
    );
    const matchCount = titleMatchCount + transcriptMatchCount;

    if (matchCount === 0) {
      return [];
    }

    return [
      {
        recording,
        titleSegments: createHighlightSegments(
          recording.title,
          normalizedQuery
        ),
        snippet: createTranscriptSnippet(recording.fullText, normalizedQuery),
        matchCount,
      },
    ];
  });
}

export function createTranscriptSearchState(
  lines: TranscriptLine[],
  query: string,
  activeMatchIndex: number
): TranscriptSearchState {
  const normalizedQuery = normalizeQuery(query);
  const matches: TranscriptSearchMatch[] = [];
  const matchCountsByLineId = new Map<string, number>();

  for (const line of lines) {
    const lineMatchCount = countMatches(line.text, normalizedQuery);
    matchCountsByLineId.set(line.id, lineMatchCount);
    for (let matchIndex = 0; matchIndex < lineMatchCount; matchIndex += 1) {
      matches.push({ lineId: line.id, matchIndex });
    }
  }

  const clampedActiveMatchIndex =
    matches.length > 0
      ? Math.min(Math.max(activeMatchIndex, 0), matches.length - 1)
      : 0;
  let nextMatchIndex = 0;
  const searchLines = lines.map((line) => {
    const lineMatchCount = matchCountsByLineId.get(line.id) ?? 0;

    const segments = createTranscriptLineHighlightSegments({
      activeMatchIndex: clampedActiveMatchIndex,
      firstMatchIndex: nextMatchIndex,
      query: normalizedQuery,
      text: line.text,
    });
    nextMatchIndex += lineMatchCount;

    return {
      line,
      segments,
    };
  });

  return {
    activeMatchIndex: clampedActiveMatchIndex,
    lines: searchLines,
    matches,
    query: normalizedQuery,
  };
}
