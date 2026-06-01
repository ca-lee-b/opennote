#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri_plugin_sql::{Migration, MigrationKind};

mod transcription;

fn main() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create recording_sessions table",
            sql: "CREATE TABLE IF NOT EXISTS recording_sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                date TEXT NOT NULL,
                duration REAL NOT NULL DEFAULT 0,
                audio_path TEXT,
                full_text TEXT NOT NULL DEFAULT '',
                segment_data TEXT NOT NULL DEFAULT '[]',
                model_used TEXT NOT NULL DEFAULT '',
                language TEXT
            )",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "create index on date for sorting",
            sql: "CREATE INDEX IF NOT EXISTS idx_sessions_date ON recording_sessions(date)",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "create normalized recordings and transcript_lines tables",
            sql: "CREATE TABLE IF NOT EXISTS recordings (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                duration REAL NOT NULL DEFAULT 0,
                model_id TEXT NOT NULL DEFAULT '',
                is_partial INTEGER NOT NULL DEFAULT 0,
                audio_path TEXT,
                language TEXT,
                full_text TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS transcript_lines (
                id TEXT PRIMARY KEY,
                recording_id TEXT NOT NULL,
                line_id INTEGER NOT NULL,
                text TEXT NOT NULL DEFAULT '',
                start_time TEXT NOT NULL,
                duration REAL NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL,
                is_final INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
                UNIQUE(recording_id, line_id)
            );

            INSERT OR IGNORE INTO recordings (
                id,
                title,
                created_at,
                duration,
                model_id,
                is_partial,
                audio_path,
                language,
                full_text
            )
            SELECT
                id,
                title,
                date,
                duration,
                model_used,
                0,
                audio_path,
                language,
                full_text
            FROM recording_sessions;",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "index normalized recording tables",
            sql: "CREATE INDEX IF NOT EXISTS idx_recordings_created_at ON recordings(created_at);
            CREATE INDEX IF NOT EXISTS idx_lines_recording_sort ON transcript_lines(recording_id, sort_order);
            CREATE INDEX IF NOT EXISTS idx_lines_recording_line_id ON transcript_lines(recording_id, line_id);",
            kind: MigrationKind::Up,
        },
        Migration {
            version: 5,
            description: "drop legacy recording_sessions table",
            sql: "DROP TABLE IF EXISTS recording_sessions;",
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .manage(transcription::TranscriptionState {
            model_info: std::sync::Mutex::new(transcription::ModelInfoState {
                loaded_model_id: None,
                loaded_model_arch: None,
                loaded_model_path: None,
                is_recording: false,
                started_at: None,
                started_wall_time: None,
            }),
            worker: std::sync::Mutex::new(None),
        })
        .manage(transcription::DownloadState::default())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:opennote.db", migrations)
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            transcription::get_transcription_state,
            transcription::load_transcription_model,
            transcription::start_transcription_recording,
            transcription::stop_transcription_recording,
            transcription::transcribe_recording,
            transcription::delete_audio_file,
            transcription::clear_all_audio_files,
            transcription::get_system_audio_permission,
            transcription::open_system_audio_settings,
            transcription::models::get_downloaded_models,
            transcription::models::download_model,
            transcription::models::delete_model,
            transcription::models::cancel_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
