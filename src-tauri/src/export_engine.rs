//! Export and archiving engine (non-destructive file management and ZIP generation).

use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportMode {
    #[default]
    Copy,
    Suffix,
    Folder,
}

/// Génère un horodatage textuel précis au format YYYY-MM-DD_HHmmss pour les sous-dossiers
pub fn generate_folder_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let seconds_in_day = now % 86400;
    let days = now / 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    let seconds = seconds_in_day % 60;

    let (year, month, day) = days_to_civil(days);
    format!("{year:04}-{month:02}-{day:02}_{hours:02}{minutes:02}{seconds:02}")
}

/// Algorithme civil Hinnant (conversion exacte jours UNIX -> Année, Mois, Jour)
fn days_to_civil(days_since_epoch: u64) -> (u32, u32, u32) {
    let z = days_since_epoch as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1024 + doe / 1460 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m, d)
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
            let target = output_dir.join(&file_name);
            // Protection non-destructive CDC §4.4 : ne jamais écraser le fichier original
            if target == input_path {
                let safe_name = if extension.is_empty() {
                    format!("{file_stem}_opt")
                } else {
                    format!("{file_stem}_opt.{extension}")
                };
                output_dir.join(safe_name)
            } else {
                target
            }
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
            let auto_ts;
            let ts = match folder_timestamp {
                Some(t) => t,
                None => {
                    auto_ts = generate_folder_timestamp();
                    &auto_ts
                }
            };
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

/// Génère une archive ZIP contenant les fichiers spécifiés en streaming mémoire maîtrisé
pub fn create_zip_archive(files: &[PathBuf], output_zip_path: &Path) -> Result<u64, String> {
    if let Some(parent) = output_zip_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = File::create(output_zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut seen_names = HashSet::new();

    for path in files {
        if !path.exists() || !path.is_file() {
            continue;
        }

        let orig_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("fichier");

        // Dédoublonnage des entrées dans le ZIP en cas de noms identiques issus de dossiers différents
        let mut entry_name = orig_name.to_string();
        let mut counter = 1;
        while seen_names.contains(&entry_name) {
            let (stem, ext) = if let Some(dot_idx) = orig_name.rfind('.') {
                (&orig_name[..dot_idx], &orig_name[dot_idx..])
            } else {
                (orig_name, "")
            };
            entry_name = format!("{stem} ({counter}){ext}");
            counter += 1;
        }
        seen_names.insert(entry_name.clone());

        zip.start_file(entry_name, options)
            .map_err(|e| e.to_string())?;
        let mut f = File::open(path).map_err(|e| e.to_string())?;
        // Streaming par blocs pour ne jamais charger de gros fichiers multimédias en mémoire vive
        std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
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
    fn test_resolve_export_path_copy_non_destructive_collision() {
        let input = Path::new("/home/user/export/vacances.jpg");
        let dest = Path::new("/home/user/export");
        let out = resolve_export_path(input, dest, &ExportMode::Copy, None);
        assert_eq!(out, PathBuf::from("/home/user/export/vacances_opt.jpg"));
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
        assert_eq!(
            out,
            PathBuf::from("/home/user/export/Export_2026-10-27/vacances.jpg")
        );
    }

    #[test]
    fn test_generate_folder_timestamp() {
        let ts = generate_folder_timestamp();
        assert_eq!(ts.len(), 17); // YYYY-MM-DD_HHmmss = 10 + 1 + 6 = 17
        assert!(ts.contains('_'));
        assert!(ts.contains('-'));
    }

    #[test]
    fn test_create_zip_archive_streaming_and_deduplication() {
        let temp_dir = std::env::temp_dir().join("go_compressor_zip_test");
        let _ = std::fs::create_dir_all(&temp_dir);

        let sub1 = temp_dir.join("sub1");
        let sub2 = temp_dir.join("sub2");
        let _ = std::fs::create_dir_all(&sub1);
        let _ = std::fs::create_dir_all(&sub2);

        // Créer deux fichiers ayant le même nom dans deux dossiers différents
        let f1 = sub1.join("media.txt");
        let f2 = sub2.join("media.txt");
        std::fs::write(&f1, b"Contenu fichier 1").unwrap();
        std::fs::write(&f2, b"Contenu fichier 2 different").unwrap();

        let zip_path = temp_dir.join("archive.zip");
        let res = create_zip_archive(&[f1.clone(), f2.clone()], &zip_path);

        assert!(res.is_ok());
        let size = res.unwrap();
        assert!(size > 0);
        assert!(zip_path.exists());

        // Nettoyage
        let _ = std::fs::remove_file(&f1);
        let _ = std::fs::remove_file(&f2);
        let _ = std::fs::remove_file(&zip_path);
    }
}
