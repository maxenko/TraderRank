# TraderRankDesktop - Theme & CSS Plan

## Theme System

CSS custom properties on `[data-theme]` attribute. Theme signal in Dioxus context toggles the attribute.

### Dark Theme (default)
```css
--bg-primary: #0b1019
--bg-secondary: #111827
--bg-card: rgba(17, 24, 39, 0.9)
--text-primary: #e2e8f0
--text-secondary: #94a3b8
--accent-green: #22c55e (profit)
--accent-red: #ef4444 (loss)
--accent-blue: #3b82f6 (highlights)
```

### Light Theme
```css
--bg-primary: #f1f5f9
--bg-secondary: #ffffff
--bg-card: rgba(255, 255, 255, 0.95)
--text-primary: #0f172a
--text-secondary: #475569
```

## Design Aesthetic
- Professional trading terminal look
- Glassmorphism cards: backdrop-filter blur, semi-transparent backgrounds
- Subtle borders with low-opacity colors
- Smooth 0.3s transitions for theme switching
- Gradient bar charts (green/red with transparency)
- Hover effects on cards (lift + glow)
- Custom scrollbars matching theme

## Layout
- Top navigation bar (52px height, fixed)
- Main content area (scrollable, max-width 1200px centered)
- KPI grid: 3 columns (responsive to 2 on narrow)
- Tables: sticky headers, alternating hover rows
