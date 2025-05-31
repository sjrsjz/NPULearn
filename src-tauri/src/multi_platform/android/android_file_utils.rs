// src-tauri/src/android_file_utils.rs
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use chrono::Utc; // For fallback filename
use tauri::{AppHandle, Manager};
use tauri_plugin_fs::FsExt;
use url::Url; // For parsing URI and getting path segments
              // Assuming urlencoding crate is available as it's used in lib.rs for Android URI decoding
              // use urlencoding;

// Helper to get a usable file name from a content URI for the copied file
#[allow(dead_code)]
fn get_file_name_for_copy(uri_string: &str) -> Result<String, String> {
    let parsed_uri =
        Url::parse(uri_string).map_err(|e| format!("Invalid URI to parse for filename: {}", e))?;

    let path_part = parsed_uri.path();

    if let Some(last_segment_encoded) = path_part.split('/').last() {
        if !last_segment_encoded.is_empty() {
            let decoded_name = urlencoding::decode(last_segment_encoded)
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|_| last_segment_encoded.to_string());

            // Sanitize for typical filesystem restrictions: replace problematic chars with '_'
            let sanitized_name: String = decoded_name
                .chars()
                .map(|c| match c {
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                    _ if c.is_control() => '_', // Replace control characters
                    _ => c,
                })
                .collect();

            // Ensure it's not empty after sanitization and not just dots or underscores
            let final_name = if sanitized_name
                .trim_matches(|c: char| c == '.' || c == '_')
                .is_empty()
            {
                format!("file_{}.bin", Utc::now().timestamp_millis())
            } else {
                sanitized_name
            };

            const MAX_FILENAME_LEN: usize = 200; // Common safe length
            if final_name.len() > MAX_FILENAME_LEN {
                // Simple truncation, could be smarter to preserve extension
                return Ok(final_name.chars().take(MAX_FILENAME_LEN).collect());
            }
            return Ok(final_name);
        }
    }

    // Fallback: generate a unique name using a timestamp if no segment found or segment is empty.
    Ok(format!("file_{}.bin", Utc::now().timestamp_millis()))
}

/// Resolves an Android URI to an absolute local file path.
/// For `content://` URIs, this involves copying the file to the app's local data directory.
/// For `file://` URIs, it attempts to convert directly to a path.
#[allow(dead_code)]
pub async fn resolve_uri_to_local_path(
    app_handle: &AppHandle,
    uri_string: &str,
) -> Result<String, String> {
    if uri_string.starts_with("file://") {
        // For file URIs, directly convert to path
        Url::parse(uri_string)
            .map_err(|e| format!("Invalid file URI: {}", e))?
            .to_file_path()
            .map_err(|_| {
                "Failed to convert file URI to path (might be a directory or invalid format)"
                    .to_string()
            })
            .map(|p| p.to_string_lossy().into_owned())
    } else if uri_string.starts_with("content://") {
        // For content URIs, copy the file to app's local data directory
        let file_name_for_copy = get_file_name_for_copy(uri_string)?;

        let local_data_dir = app_handle
            .path()
            .app_local_data_dir()
            .map_err(|_| "Failed to get app local data directory".to_string())?;

        if !local_data_dir.exists() {
            fs::create_dir_all(&local_data_dir).map_err(|e| {
                format!(
                    "Failed to create app local data directory '{}': {}",
                    local_data_dir.display(),
                    e
                )
            })?;
        }

        let destination_path = local_data_dir.join(&file_name_for_copy);

        // Read content from URI using tauri-plugin-fs.
        // This is the critical part that assumes tauri-plugin-fs can handle content:// URIs.
        let uri_url = tauri::Url::parse(uri_string)
            .map_err(|e| format!("Failed to parse URI '{}': {}", uri_string, e))?;

        let file_content = app_handle
            .fs()
            .read(uri_url) // Pass as tauri::Url which implements Into<FilePath>
            .map_err(|e| {
                format!(
                    "Failed to read content from URI '{}' using tauri-plugin-fs: {}. Ensure plugin is configured and URI is accessible.",
                    uri_string, e
                )
            })?;

        // Write content to the destination file
        let mut dest_file = File::create(&destination_path).map_err(|e| {
            format!(
                "Failed to create destination file '{}': {}",
                destination_path.display(),
                e
            )
        })?;
        dest_file.write_all(&file_content).map_err(|e| {
            format!(
                "Failed to write to destination file '{}': {}",
                destination_path.display(),
                e
            )
        })?;

        Ok(destination_path.to_string_lossy().into_owned())
    } else {
        // If it's not file:// or content://, check if it's an absolute path.
        let path = Path::new(uri_string);
        if path.is_absolute() && path.exists() {
            Ok(uri_string.to_string())
        } else if path.is_absolute() && !path.exists() {
            Err(format!(
                "Path '{}' is absolute but does not exist.",
                uri_string
            ))
        } else {
            Err(format!("Unsupported URI scheme or invalid path format: '{}'. Expected 'file://' or 'content://' or an existing absolute path.", uri_string))
        }
    }
}
