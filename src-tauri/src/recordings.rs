mod write;

use tauri::AppHandle;

#[tauri::command]
pub async fn create_recording_with_segments(
    app: AppHandle,
    request: write::CreateRecordingWithSegmentsRequest,
) -> Result<write::CreateRecordingWithSegmentsResult, String> {
    write::create_recording_with_segments(app, request).await
}
