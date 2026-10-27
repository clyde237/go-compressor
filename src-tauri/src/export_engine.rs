//! Export and archiving engine (non-destructive file management and ZIP generation).

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportMode {
    #[default]
    Copy,
    Suffix,
    Folder,
}
