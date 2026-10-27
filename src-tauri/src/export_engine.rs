//! Export and archiving engine (non-destructive file management and ZIP generation).

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportMode {
    #[default]
    Copy,
    Suffix,
    Folder,
}

/// Résout le chemin de destination pour un fichier selon le mode d'export choisi
pub fn resolve_export_path(
    input_path: &Path,
    output_dir: &Path,
    mode: &ExportMode,
    folder_timestamp: Option<&str>,
) -> PathBuf {
    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match mode {
        ExportMode::Copy => {
            let file_name = if extension.is_empty() {
                file_stem.to_string()
            } else {
                format!("{file_stem}.{extension}")
            };
            output_dir.join(file_name)
        }
        ExportMode::Suffix => {
            let parent = input_path.parent().unwrap_or(output_dir);
            let file_name = if extension.is_empty() {
                format!("{file_stem}_opt")
            } else {
                format!("{file_stem}_opt.{extension}")
            };
            parent.join(file_name)
        }
        ExportMode::Folder => {
            let ts = folder_timestamp.unwrap_or("export");
            let target_subfolder = output_dir.join(format!("Export_{ts}"));
            let file_name = if extension.is_empty() {
                file_stem.to_string()
            } else {
                format!("{file_stem}.{extension}")
            };
            target_subfolder.join(file_name)
        }
    }
}

/// Génère une archive ZIP contenant les fichiers spécifiés
pub fn create_zip_archive(files: &[PathBuf], output_zip_path: &Path) -> Result<u64, String> {
    if let Some(parent) = output_zip_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = File::create(output_zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();

    for path in files {
        if !path.exists() || !path.is_file() {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("fichier");

        zip.start_file(name, options).map_err(|e| e.to_string())?;
        let mut f = File::open(path).map_err(|e| e.to_string())?;
        f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        zip.write_all(&buffer).map_err(|e| e.to_string())?;
        buffer.clear();
    }

    zip.finish().map_err(|e| e.to_string())?;

    let meta = std::fs::metadata(output_zip_path).map_err(|e| e.to_string())?;
    Ok(meta.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_export_path_copy() {
        let input = Path::new("/home/user/photos/vacances.jpg");
        let dest = Path::new("/home/user/export");
        let out = resolve_export_path(input, dest, &ExportMode::Copy, None);
        assert_eq!(out, PathBuf::from("/home/user/export/vacances.jpg"));
    }

    #[test]
    fn test_resolve_export_path_suffix() {
        let input = Path::new("/home/user/photos/vacances.jpg");
        let dest = Path::new("/home/user/export");
        let out = resolve_export_path(input, dest, &ExportMode::Suffix, None);
        assert_eq!(out, PathBuf::from("/home/user/photos/vacances_opt.jpg"));
    }

    #[test]
    fn test_resolve_export_path_folder() {
        let input = Path::new("/home/user/photos/vacances.jpg");
        let dest = Path::new("/home/user/export");
        let out = resolve_export_path(input, dest, &ExportMode::Folder, Some("2026-10-27"));
        assert_eq!(out, PathBuf::from("/home/user/export/Export_2026-10-27/vacances.jpg"));
    }
}
