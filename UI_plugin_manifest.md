# River UI Plugin Manifest & Architecture Guide

Welcome to River's UI Plugin Architecture! Everything in River is modular and customizable using **KDL** files! You can design the app layout however you want—every panel, button, card, and background can be created, styled, and customized.

## 1. C## 5. Available Built-In Suites
River includes 6 authentic, ultra-custom suites:
1. **Cyberdeck Pro** (`cyberdeck-pro-suite`): Futuristic cyberpunk terminal with matrix rain background and terminal feed cards.
2. **Windows XP** (`windows-xp-suite`): Classic nostalgic Luna blue desktop with gradient background and channel grid.
3. **Samsung One UI** (`one-ui-suite`): Sleek AMOLED dark aura with rounded floating cards and pill navigation.
4. **Apple iOS & macOS** (`iphone-ios-suite`): Minimalist Cupertino design with twinkling starry cosmos background.
5. **Console Plaza** (`console-plaza-suite`): Playful gaming console menu with interactive WaraWara grid background.
6. **Studio Hi-Fi** (`studio-hifi-suite`): Audiophile Winamp/receiver stack with dark brushed metal gradient and LED displays.
ore Philosophy & Dual-Engine Architecture
River provides complete "make it yourself" customization without descriptive text spam:
- **`mode="hotplug"`**: Interprets KDL AST DOM trees dynamically on the fly. You can hot-reload layouts without restarting or re-compiling the Rust binary.
- **`mode="core"`**: Ahead-of-Time (AOT) compiled UI layouts that compile KDL AST trees into native Rust structs for zero-overhead, 60 FPS rendering.

## 2. Multi-Device Multi-Suite Layouts (`target`)
Every theme plugin can specify independent layouts tailored to specific device aspect ratios and form factors:
- **`target="desktop"`**: Optimized for ultrawide and 16:9 widescreen displays (multi-column grids, sidebars, top headers, dock panels).
- **`target="mobile"`**: Optimized for portrait mobile displays (< 1.0 aspect ratio; single-column feeds, compact headers, bottom tab bars).
- **`target="tv"`**: Optimized for 10b-foot living room cinema and TV displays (large typography, 4-column live tiles, D-pad / remote navigation helpers).

River automatically resolves the active target device layout at runtime based on window aspect ratio and device ID!

## 3. Complex Background Styling
You can style application backgrounds beyond simple solid colors by specifying `background-type` in your `style` block:
- **`gradient`**: Smooth vertical color gradient between `fill` and `secondary`.
- **`grid`**: Futuristic cyber-grid lines over `fill` using `secondary` line color.
- **`matrix`**: Animated falling cyber-rain characters with glowing heads (speed controlled by `speed`).
- **`stars`**: Twinkling starry space cosmos with pulsating brightness.
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

## 4. SVG & Image Insertion Anywhere
You can insert vector SVGs or web images anywhere inside your UI panels and layouts using `image` or `svg` nodes:
```kdl
image url="https://raw.githubusercontent.com/egui/egui/master/crates/egui/assets/ferris.svg" width=32.0 height=32.0 rounding=6.0
```
- Supported formats include **SVG**, **PNG**, **JPEG**, **GIF**, and **WEBP**.
- Images automatically scale responsively alongside window dimensions and respect custom rounding and sizing constraints.
