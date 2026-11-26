//! Durable application settings and their serialized record types.

mod capabilities;
mod keybindings;
mod prompts;
mod records;
mod shared;
mod store;
mod trusted_repository;
mod vocabulary;

pub use capabilities::Capabilities;
pub use keybindings::{KeybindingSettings, ShortcutModifiers};
pub use prompts::{Prompt, StartupPrompt};
pub use records::{
    ActivationMode, BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, SavedPullRequest,
    SavedWindowBounds, ViewMode, Workspace, WorkspaceUiState,
};
pub use shared::SharedSettings;
pub use store::{PromptReferences, Settings, detect_source_base_folder};
pub use trusted_repository::TrustedRepository;
pub use vocabulary::{
    AiProvider, AppearanceMode, AutopilotAction, CostTier, LayoutMode, UnknownVariant,
};
