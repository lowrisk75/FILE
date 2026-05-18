# AORATA App Icons

## Structure

```
Assets/
├── AppIcon.appiconset/       # Xcode-ready app icons
│   ├── icon-1024.png         # App Store (1024×1024)
│   ├── icon-180.png          # iPhone @3x (60×60)
│   ├── icon-120.png          # iPhone @2x (60×60)
│   ├── icon-167.png          # iPad Pro @2x (83.5×83.5)
│   ├── icon-152.png          # iPad @2x (76×76)
│   ├── icon-76.png           # iPad @1x (76×76)
│   ├── icon-60.png           # Spotlight @3x (20×20)
│   ├── icon-40.png           # Spotlight @2x (20×20)
│   ├── icon-29.png           # Settings @1x (29×29)
│   ├── icon-512.png          # macOS Retina
│   ├── icon-256.png          # macOS
│   ├── icon-128.png          # macOS
│   ├── icon-64.png           # macOS
│   ├── icon-32.png           # macOS
│   ├── icon-16.png           # macOS menu bar
│   └── Contents.json         # Xcode asset catalog metadata
│
├── Variants/                 # Status indicator variants
│   ├── icon-1024-connected.png      # Green dot (active)
│   ├── icon-1024-disconnected.png   # Grayscale + red dot (inactive)
│   └── icon-1024-alert.png          # Orange glow (warning)
│
└── Web/                      # Website favicons & meta images
    ├── favicon.ico           # Multi-resolution .ico (16/32/64)
    ├── favicon-16.png        # 16×16 PNG
    ├── favicon-32.png        # 32×32 PNG
    ├── apple-touch-icon.png  # 180×180 Apple devices
    ├── og-image.png          # 1200×630 Open Graph
    ├── html-snippet.html     # Copy-paste meta tags
    └── site.webmanifest      # PWA manifest
```

## Design Concept

**"Invisible Hexagon"** — representing zero open ports and invisible networking.

- **Colors**: Electric violet (#7C3AED), dark background (#1A1A1A), slate gray (#64748B)
- **Style**: Glassmorphism, 3D depth, modern iOS design
- **Symbolism**: Concentric hexagons fading to invisible at center

## Status Variants

### Connected (Green)
- Use: Active connection, authenticated, secure
- Indicator: Green dot (#10B981) bottom-right
- Visual: Full color, hexagon fully visible

### Disconnected (Red)
- Use: No connection, inactive, logged out
- Indicator: Red dot (#EF4444) bottom-right
- Visual: Desaturated (grayscale)

### Alert (Orange)
- Use: Warning, threat detected, action required
- Indicator: Orange dot (#F59E0B) bottom-right
- Visual: Orange glow overlay

## Usage in Xcode

1. Drag `AppIcon.appiconset` folder into Xcode Asset Catalog
2. Select as app icon in target settings
3. Use variants for menu bar / dock badge states

## Usage in macOS Menu Bar

For menu bar status icons (16×16, 32×32):
```swift
let connectedIcon = NSImage(named: "icon-connected")
let disconnectedIcon = NSImage(named: "icon-disconnected")
let alertIcon = NSImage(named: "icon-alert")
```

## Usage on Website

Copy files from `Web/` to your web root:
```bash
cp Web/favicon.ico /path/to/website/public/
cp Web/favicon-*.png /path/to/website/public/
cp Web/apple-touch-icon.png /path/to/website/public/
cp Web/og-image.png /path/to/website/public/
cp Web/site.webmanifest /path/to/website/public/
```

Add meta tags from `Web/html-snippet.html` to your `<head>`:
```html
<link rel="icon" type="image/x-icon" href="/favicon.ico">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<meta property="og:image" content="/og-image.png">
<!-- ... see html-snippet.html for complete list -->
```

## Regenerating Sizes

If you update the base 1024×1024 icon:
```bash
cd AppIcon.appiconset
sips -z 180 180 icon-1024.png --out icon-180.png
sips -z 120 120 icon-1024.png --out icon-120.png
# ... (repeat for all sizes)
```

## Original Source

Generated from prompt in `/AORATA/ICON_DESIGN_PROMPT.md`
Base icon: "Invisible Hexagon" concept (Version 1)
