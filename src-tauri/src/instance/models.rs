use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocalInstanceKind {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
    Quilt,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInstance {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub kind: LocalInstanceKind,
    pub game_dir: String,
    pub icon: Option<String>,
    pub last_played_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocalInstanceRequest {
    pub name: String,
    pub game_version: String,
    pub kind: Option<LocalInstanceKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLocalInstanceRequest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub game_version: Option<String>,
    #[serde(default)]
    pub kind: Option<LocalInstanceKind>,
    /// `Some(Some(v))` to set icon; `Some(None)` to clear it; `None` to leave unchanged.
    #[serde(default)]
    pub icon: Option<Option<String>>,
}
