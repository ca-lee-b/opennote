const HAS_COMPLETED_ONBOARDING_KEY = "hasCompletedOnboarding";
const SELECTED_MODEL_ID_KEY = "selectedModelId";

export interface AppPreferences {
  hasCompletedOnboarding: boolean;
  selectedModelId: string | null;
}

export function getAppPreferences(): AppPreferences {
  return {
    hasCompletedOnboarding:
      localStorage.getItem(HAS_COMPLETED_ONBOARDING_KEY) === "true",
    selectedModelId: localStorage.getItem(SELECTED_MODEL_ID_KEY),
  };
}

export function setHasCompletedOnboarding(value: boolean): void {
  localStorage.setItem(HAS_COMPLETED_ONBOARDING_KEY, String(value));
}

export function setSelectedModelId(modelId: string | null): void {
  if (modelId) {
    localStorage.setItem(SELECTED_MODEL_ID_KEY, modelId);
    return;
  }

  localStorage.removeItem(SELECTED_MODEL_ID_KEY);
}
