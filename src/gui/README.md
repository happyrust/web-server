# GUI Module - egui Remote Sync UI

## Overview

This module implements a native desktop GUI for the remote sync operations platform using egui 0.33 and eframe.

## Structure

```
gui/
├── mod.rs              # Module exports
├── app.rs              # Main application (EguiRemoteSyncApp)
├── state.rs            # Global state management (AppState)
├── api_client.rs       # HTTP API client
├── theme.rs            # Theme configuration
├── pages/              # Page components
│   ├── mod.rs
│   ├── environment_list.rs      # Environment management
│   ├── topology_canvas.rs       # Visual topology editor
│   ├── monitor_dashboard.rs     # Real-time monitoring
│   ├── log_query.rs             # Log query and filtering
│   └── web_server.rs            # Web server management
├── components/         # Reusable UI components
│   ├── mod.rs
│   ├── toast.rs                 # Toast notifications
│   ├── confirm_dialog.rs        # Confirmation dialogs
│   └── env_form.rs              # Environment form
└── canvas/             # Topology canvas
    ├── mod.rs
    ├── node.rs                  # Node definitions
    ├── edge.rs                  # Edge definitions
    ├── layout.rs                # Layout algorithms
    └── renderer.rs              # Canvas renderer
```

## Key Components

### EguiRemoteSyncApp (app.rs)

Main application structure that implements `eframe::App`. Manages:
- Page navigation
- Global state
- API client
- Theme configuration
- Toast notifications

### AppState (state.rs)

Global application state containing:
- Environments list
- Sites list
- Sync tasks
- Sync logs
- Web server status
- Topology data

### ApiClient (api_client.rs)

HTTP client for backend communication. Provides async methods for:
- Environment CRUD operations
- Site management
- Sync control (start/stop/pause/resume)
- Log queries
- Topology save/load

### Pages

Each page is a self-contained component with its own state and render logic:

- **EnvironmentListPage**: Manage remote sync environments
- **TopologyCanvasPage**: Visual topology configuration
- **MonitorDashboardPage**: Real-time status monitoring
- **LogQueryPage**: Query and filter sync logs
- **WebServerPage**: Manage embedded web server

### Components

Reusable UI components:

- **ToastManager**: Display temporary notifications
- **ConfirmDialog**: Show confirmation dialogs
- **EnvironmentForm**: Form for environment configuration

### Canvas

Topology canvas for visual configuration:

- **TopologyNode**: Node representation (Environment/Site)
- **TopologyEdge**: Connection between nodes
- **TopologyCanvas**: Canvas renderer with zoom/pan
- **Layout**: Auto-layout algorithms

## Usage

### Building

```bash
cargo build --bin egui_remote_sync --features gui
```

### Running

```bash
cargo run --bin egui_remote_sync --features gui
```

### Release Build

```bash
cargo build --bin egui_remote_sync --features gui --release
```

## Dependencies

- `egui` 0.33 - Immediate mode GUI framework
- `eframe` 0.33 - Application framework for egui
- `egui_extras` 0.33 - Additional egui widgets
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `serde_json` - JSON serialization
- `chrono` - Date/time handling
- `rfd` - Native file dialogs
- `csv` - CSV export

## API Integration

The GUI communicates with the backend via REST API:

```
Base URL: http://localhost:3000

Endpoints:
- GET    /api/remote-sync/envs
- POST   /api/remote-sync/envs
- PUT    /api/remote-sync/envs/{id}
- DELETE /api/remote-sync/envs/{id}
- POST   /api/remote-sync/envs/{id}/activate
- GET    /api/remote-sync/sites
- POST   /api/remote-sync/sites
- POST   /api/remote-sync/sites/{id}/test
- POST   /api/sync/start
- POST   /api/sync/stop
- POST   /api/sync/pause
- POST   /api/sync/resume
- GET    /api/sync/status
- POST   /api/sync/queue/clear
- GET    /api/remote-sync/logs
- POST   /api/topology/save
- GET    /api/topology/load
```

## State Management

State is managed through the `AppState` struct, which is passed to pages via mutable reference. Pages can update state directly, and changes are reflected across the application.

## Async Operations

All API calls are performed asynchronously using `tokio::spawn`. Results are communicated back to the UI thread through channels or by updating shared state.

## Theme Support

The application supports light and dark themes, configurable through the Settings page. Theme preferences are persisted across sessions.

## Configuration Persistence

Window layout, current page, and theme settings are automatically saved and restored using eframe's storage API.

## Adding New Pages

1. Create a new file in `pages/` directory
2. Implement the page struct with a `render()` method
3. Add the page to `pages/mod.rs`
4. Add a new variant to the `Page` enum in `app.rs`
5. Update navigation and rendering logic in `app.rs`

## Adding New Components

1. Create a new file in `components/` directory
2. Implement the component struct with a `render()` method
3. Add the component to `components/mod.rs`
4. Use the component in pages as needed

## Testing

Currently, the GUI module does not have automated tests. Manual testing is performed by running the application and verifying functionality.

## Future Enhancements

- [ ] Site configuration management page
- [ ] Parse task management page
- [ ] Model generation configuration page
- [ ] Quick deploy page
- [ ] WebSocket real-time updates
- [ ] Multi-language support
- [ ] Performance charts and statistics
- [ ] Plugin system

## Contributing

When contributing to the GUI module:

1. Follow the existing code structure
2. Use consistent naming conventions
3. Add documentation for new components
4. Test thoroughly before submitting
5. Update this README if adding new features

## License

Same as the main project.
