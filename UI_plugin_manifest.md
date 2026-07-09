# River UI Plugin Manifest & Architecture Guide

Welcome to River's UI Plugin Architecture! Everything in River is modular and customizable using **KDL** files! You can design the app layout however you want—every panel, button, card, and background can be created, styled, and customized.

## 1. Core Philosophy & Dual-Engine Architecture
River provides complete "make it yourself" customization:
- **`mode="hotplug"`**: Interprets KDL AST DOM trees dynamically on the fly. You can hot-reload layouts without restarting or re-compiling the Rust binary.
- **`mode="core"`**: Ahead-of-Time (AOT) compiled UI layouts that compile KDL AST trees into native Rust structs for zero-overhead, 60 FPS rendering.

**The backend is a framework, not a template.** It works like HTML/CSS—providing layout primitives and data bindings—but never dictates what the UI looks like. You have total freedom to compose any interface from scratch.

## 2. Available Primitives

### Panel Nodes (Window-Level Structure)
These create the top-level window layout:
- **`top-panel`**: Fixed panel at the top of the window.
- **`bottom-panel`**: Fixed panel at the bottom.
- **`left-panel`**: Resizable sidebar on the left.
- **`right-panel`**: Resizable sidebar on the right.
- **`central-panel`**: Main content area (fills remaining space).

```kdl
top-panel id="header" height=60.0 fill="#1c1b1fff" {
    // children...
}
```

### Container Nodes (Layout)
- **`row`**: Horizontal wrapping layout. `spacing=` sets horizontal gap.
- **`column`**: Vertical stacking layout. `spacing=` sets vertical gap.
- **`box`**: Styled frame with background `fill`, `padding`, `rounding`, and `border-width`.
- **`scroll`**: Vertical scroll area.
- **Any custom tag**: Unknown tag names automatically become vertical containers—use semantic names freely!

```kdl
row spacing=12.0 {
    column spacing=8.0 {
        box padding=16.0 fill="#1a1a2e" rounding=12.0 {
            label text="Inside a styled box!" color="text"
        }
    }
}
```

### Widget Nodes (UI Elements)
- **`heading`**: Bold header text. `text=`, `color=`, `size=`.
- **`label`**: Body text. `text=`, `color=`, `size=`.
- **`button`**: Clickable button. `text=`, `action=`, `color=`, `size=`, `rounding=`.
- **`nav-item`**: Same as button (alias).
- **`separator`**: Horizontal divider line.
- **`spacer`**: Empty space. `size=` sets the gap in pixels.
- **`image`** / **`svg`**: Display an image or SVG. `url=`, `width=`/`size=`, `height=`, `rounding=`.
- **`theme-switcher`**: Built-in theme picker. `style="vertical"` or `style="horizontal"`.
- **`device-switcher`**: Built-in device target picker (Auto/Desktop/Mobile/TV).

### Data-Bound Nodes (Dynamic Content)
- **`for-each`**: Iterate over a data list and render template children for each item.
- **`if-state`** / **`condition`**: Conditionally show/hide children based on data values.
- **`grid`**: Responsive grid layout with automatic column calculation.

## 3. Data Bindings & Template Interpolation

The backend **exports data tags** that KDL plugins can bind to using `{binding}` syntax in any text or action attribute.

### Available Bindings
| Binding | Type | Description |
|---------|------|-------------|
| `{categories}` | List | All media categories (Video, Music, Manga, Podcasts) |
| `{cat.id}` | String | Category ID (inside `for-each` over categories) |
| `{cat.name}` | String | Category display name |
| `{cat.icon}` | String | Category emoji icon (🎬, 🎵, 📖, 🎙️) |
| `{cat.active}` | Bool | Whether this category is currently selected |
| `{catalog_state}` | String | Current state: `"idle"`, `"loading"`, `"loaded"`, `"error"` |
| `{catalog_error}` | String | Error message (when state is `"error"`) |
| `{catalogs}` | List | Loaded catalog entries |
| `{catalog.id}` | String | Catalog ID (inside for-each) |
| `{catalog.name}` | String | Catalog display name |
| `{catalog.items}` | List | Media items in this catalog |
| `{item.id}` | String | Media item ID |
| `{item.title}` | String | Media item title |
| `{item.description}` | String | Description text |
| `{item.poster_url}` | String | Poster image URL |
| `{item.year}` | String | Release year |
| `{item.rating}` | String | Rating (e.g. "7.5") |
| `{item.genres}` | String | Comma-separated genres |
| `{item.author}` | String | Author/creator name |
| `{active_category}` | String | Currently selected category name |
| `{active_theme}` | String | Currently active theme ID |
| `{themes}` | List | Available themes |
| `{device_id}` | String | Current device identifier |

### Using `for-each`
```kdl
for-each source="{categories}" item="cat" {
    button text="{cat.icon}  {cat.name}" action="SelectCategory:{cat.id}" \
        size=16.0 rounding=14.0
    spacer size=8.0
}
```

### Using `if-state`
```kdl
if-state source="{catalog_state}" equals="loading" {
    row {
        label text="Loading media..." color="secondary"
    }
}
if-state source="{catalog_state}" equals="loaded" {
    for-each source="{catalogs}" item="catalog" {
        heading text="{catalog.name}" color="accent"
        grid columns=3 min-cell-width=190.0 spacing=12.0 {
            for-each source="{catalog.items}" item="item" {
                box rounding=12.0 padding=10.0 fill="#1a1a2e" {
                    label text="{item.title}" color="text"
                    button text="▶ Play" action="AddToLibrary:{item.id}"
                }
            }
        }
    }
}
```

### Using `grid`
```kdl
grid columns=4 min-cell-width=200.0 spacing-x=12.0 spacing-y=12.0 rounding=8.0 {
    // children are laid out in a responsive grid
    box { label text="Cell 1" }
    box { label text="Cell 2" }
    box { label text="Cell 3" }
}
```

## 4. Actions
Button `action=` attributes trigger backend behavior:
- `SelectCategory:Video` / `Music` / `Manga` / `Podcasts` — Switch media category and load catalogs.
- `SwitchTheme:<theme-id>` — Switch to a different KDL theme.
- `SwitchDevice:desktop` / `mobile` / `tv` / `auto` — Override device target layout.
- `AddToLibrary:<item-id>` — Add a media item to the user's library.

## 5. Multi-Device Multi-Suite Layouts (`target`)
Every theme plugin can specify independent layouts tailored to specific device aspect ratios and form factors:
- **`target="desktop"`**: Optimized for ultrawide and 16:9 widescreen displays (multi-column grids, sidebars, top headers, dock panels).
- **`target="mobile"`**: Optimized for portrait mobile displays (< 1.0 aspect ratio; single-column feeds, compact headers, bottom tab bars).
- **`target="tv"`**: Optimized for 10-foot living room cinema and TV displays (large typography, 4-column live tiles, D-pad / remote navigation helpers).

River automatically resolves the active target device layout at runtime based on window aspect ratio and device ID!

## 6. Complex Background Styling
You can style application backgrounds beyond simple solid colors by specifying `background-type` in your `style` block:
- **`gradient`**: Smooth vertical color gradient between `fill` and `secondary`.
- **`grid`**: Futuristic cyber-grid lines over `fill` using `secondary` line color.
- **`matrix`**: Animated falling cyber-rain characters with glowing heads (speed controlled by `speed`).
- **`stars`**: Twinkling starry space cosmos with pulsating brightness.
- **`waves`**: Animated wave lines across the background.
- **`image`** / **`svg`**: Custom background vector SVG or raster image loaded via `url`.

Example:
```kdl
style {
    palette accent="#00f0ff" secondary="#ff0055" text="#ffffff" border="#00f0ff" background="#050811"
    background-type "matrix"
    fill "#00ff66"
    secondary "#002211"
    speed 1.2
}
```

## 7. Animation Effects
Containers and widgets support per-node animation effects:
- **`glow`**: Pulsating border glow cycling between border and accent colors.
- **`pulse`**: Breathing border width animation.
- **`float`**: Gentle vertical floating oscillation.
- **`bounce`**: Vertical bouncing motion.
- **`shimmer`**: Subtle color shimmer between border and secondary colors.

```kdl
box effect="glow" speed=2.0 {
    label text="I'm glowing!" color="accent"
}
```

## 8. SVG & Image Insertion Anywhere
You can insert vector SVGs or web images anywhere inside your UI panels and layouts using `image` or `svg` nodes:
```kdl
image url="https://example.com/logo.svg" width=32.0 height=32.0 rounding=6.0
```
- Supported formats include **SVG**, **PNG**, **JPEG**, **GIF**, and **WEBP**.
- Images automatically scale responsively alongside window dimensions and respect custom rounding and sizing constraints.

## 9. Backward Compatibility: Sugar Widgets
For convenience and backward compatibility, two sugar widgets are available that auto-expand into primitive trees:
- **`menu-bar`**: Renders category navigation buttons. `style=` controls layout (`vertical`, `icons`, `pills`, `brackets`).
- **`catalog-view`**: Renders the media catalog grid with cards. `style=`, `columns=`, `rounding=`, `border-width=`.

These work out of the box but offer less customization than composing your own layouts with `for-each`, `if-state`, and `grid`.

## 10. Built-In Suites
River includes authentic, ultra-custom suites:
1. **Quantum Glass Aurora** (`quantum-glass-suite`): Twinkling starry cosmos with glass panels and pulse animations.
2. **Default Android Style** (`default-android-style`): Material Design 3 dark aura with gradient background.
3. **HyperPulse Cyberpunk** (`hyperpulse-cyber-suite`): Matrix rain cyberdeck terminal with glow effects.
4. **Empty** (`empty`): Minimal blank canvas for building from scratch.
