const HAS_COMPLETED_ONBOARDING_KEY = "hasCompletedOnboarding";
const LIVE_TRANSCRIPTION_PREVIEW_ENABLED_KEY =
  "liveTranscriptionPreviewEnabled";
const SELECTED_MODEL_ID_KEY = "selectedModelId";

export interface AppPreferences {
  hasCompletedOnboarding: boolean;
  liveTranscriptionPreviewEnabled: boolean;
  selectedModelId: string | null;
}

export function getAppPreferences(): AppPreferences {
  const savedLivePreviewValue = localStorage.getItem(
    LIVE_TRANSCRIPTION_PREVIEW_ENABLED_KEY
  );

  return {
    hasCompletedOnboarding:
      localStorage.getItem(HAS_COMPLETED_ONBOARDING_KEY) === "true",
    liveTranscriptionPreviewEnabled: savedLivePreviewValue
      ? savedLivePreviewValue === "true"
      : false,
    selectedModelId: localStorage.getItem(SELECTED_MODEL_ID_KEY),
  };
}

export function setHasCompletedOnboarding(value: boolean): void {
  localStorage.setItem(HAS_COMPLETED_ONBOARDING_KEY, String(value));
}

export function setLiveTranscriptionPreviewEnabled(value: boolean): void {
  localStorage.setItem(LIVE_TRANSCRIPTION_PREVIEW_ENABLED_KEY, String(value));
}

export function setSelectedModelId(modelId: string | null): void {
  if (modelId) {
    localStorage.setItem(SELECTED_MODEL_ID_KEY, modelId);
    return;
  }

  localStorage.removeItem(SELECTED_MODEL_ID_KEY);
}
