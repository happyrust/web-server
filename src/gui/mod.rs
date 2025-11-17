// GUI module for egui-based remote sync UI

pub mod app;
pub mod state;
pub mod api_client;
pub mod pages;
pub mod components;
pub mod canvas;
pub mod theme;

pub use app::EguiRemoteSyncApp;
pub use state::AppState;
pub use api_client::ApiClient;
