# Frontend Layout Fixes

## 1. Dark Mode & Color Contrast (`style.css`)
**Issue**: High-contrast text color overrides were applied globally using `!important`, which made text unreadable in dark mode (dark text on dark background).
**Fix**:
- Scoped high-contrast text rules to `[data-theme="light"]`.
- Removed `!important` where possible to allow Tailwind's dark mode utilities (`dark:text-*`) to work if needed (though the theme switch logic handles the root attribute).
- Ensured text colors adapt correctly to the active theme.

## 2. Mobile Navigation (`App.vue`)
**Issue**: The sidebar was hidden on mobile (`hidden lg:flex`), making the app unnavigable on small screens.
**Fix**:
- Added a "Hamburger" menu button in the header (visible only on mobile).
- Implemented a slide-in drawer sidebar for mobile:
  - Uses `fixed` positioning and `transform` for smooth transitions.
  - Added a backdrop overlay to close the menu when clicking outside.
  - Automatically closes the menu when a link is clicked.
- Preserved the static sidebar layout for desktop (`lg` screens).

## 3. Responsive Topology Manager (`TopologyManager.vue`)
**Issue**: The layout used a fixed split (`w-2/5` and `flex-1`) which was too cramped on tablets and mobile.
**Fix**:
- Refactored to use a flex column layout on mobile (`flex-col`) and row layout on desktop (`lg:flex-row`).
- Added `w-full` for mobile panels and restored `lg:w-2/5` for desktop.
- Added `min-height` to panels on mobile to ensure visibility.

## 4. Responsive Archives Manager (`ArchivesManager.vue`)
**Issue**: The top bar elements (title and filters) overlapped or crowded on small screens.
**Fix**:
- Changed the header container to `flex-col sm:flex-row`.
- Allowed filter controls to wrap (`flex-wrap`).

## 5. Responsive MQTT Message Viewer (`MqttMessageViewer.vue`)
**Issue**: The filter bar was a single row that overflowed on mobile.
**Fix**:
- Refactored the filter bar to stack vertically on mobile (`flex-col`) and align horizontally on desktop (`lg:flex-row`).
- Made filter select inputs full-width on mobile for better touch usability.
