# CareerCraft Agent Design System

> A calm, document-first career workspace inspired by Notion's clarity and warmth. This is an original CareerCraft system, not an official Notion design system and not a pixel-for-pixel copy.

## 1. Design Direction

CareerCraft helps people turn scattered work history into reusable career assets. The interface should feel like a well-organized personal notebook: quiet enough for long-form writing, structured enough for job decisions, and reassuring when AI is working in the background.

Design parameters:

- `DESIGN_VARIANCE: 5`
- `MOTION_INTENSITY: 3`
- `VISUAL_DENSITY: 5`
- Theme: light only for the first implementation
- Accent: Career Blue only
- Geometry: 6px controls, 10px panels, pills only for status and filters

The design borrows the content-first hierarchy, warm neutrals, compact sidebar, restrained borders, and editorial rhythm associated with Notion. It does not borrow Notion logos, proprietary illustrations, product screenshots, or branded copy.

## 2. Product Principles

1. **The content is the interface.** Experiences, resumes, job descriptions, and learning notes receive more visual weight than containers.
2. **Progress should feel calm.** AI actions explain what is happening without theatrical animation or glowing effects.
3. **Structure before decoration.** Use headings, spacing, indentation, dividers, and background shifts before adding cards or shadows.
4. **One clear next action.** Each page has one primary action. Secondary actions remain quiet.
5. **Career data feels personal.** Use human language such as “添加一段经历” instead of system language such as “创建记录”.
6. **Local-first trust is visible.** When relevant, state that data is stored locally in short, factual copy.

## 3. Color System

Use warm neutral grays throughout. Do not mix them with cool slate grays.

### Core tokens

| Token | Value | Role |
|---|---:|---|
| `--canvas` | `#FFFFFF` | Main document canvas |
| `--sidebar` | `#F7F6F3` | Sidebar and quiet regions |
| `--surface` | `#F1F0ED` | Hover, selected rows, grouped controls |
| `--surface-raised` | `#FFFFFF` | Dialogs and true overlays |
| `--ink` | `#2F2E2B` | Primary text |
| `--ink-strong` | `#191918` | Page titles and primary buttons |
| `--ink-secondary` | `#6F6D67` | Supporting copy |
| `--ink-muted` | `#9B9992` | Placeholders and metadata |
| `--border` | `#E5E3DE` | Default dividers and borders |
| `--border-strong` | `#CFCCC5` | Inputs and emphasized boundaries |
| `--accent` | `#2463A6` | Primary action, focus, active link |
| `--accent-hover` | `#1D528B` | Hover and pressed primary action |
| `--accent-soft` | `#E8F0F8` | Selected item and informational tint |
| `--success` | `#2F7D4A` | Completed and strong match |
| `--success-soft` | `#E7F3EA` | Success background |
| `--warning` | `#9A6718` | Skill gaps and attention states |
| `--warning-soft` | `#F8EED7` | Warning background |
| `--danger` | `#B33A3A` | Destructive actions and errors |
| `--danger-soft` | `#F8E7E7` | Error background |

### Color rules

- Career Blue is the only interactive accent.
- Semantic colors communicate actual state, never decoration.
- Body text uses `--ink`, not pure black.
- A selected navigation item uses `--surface` plus stronger text. Do not add a glowing side rail.
- Large colored feature-card grids are not part of the product UI.
- Minimum contrast is WCAG AA: 4.5:1 for body text and 3:1 for large text and controls.

## 4. Typography

CareerCraft is Chinese-first and document-heavy. Use a system sans stack for stable rendering inside PySide6 WebView.

```css
font-family: "Noto Sans SC", "PingFang SC", "Microsoft YaHei",
  -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

Use `JetBrains Mono`, `Cascadia Mono`, or the system monospace stack only for identifiers, scores requiring tabular alignment, and technical configuration.

| Style | Size | Weight | Line height | Use |
|---|---:|---:|---:|---|
| Display | 36px | 650 | 1.18 | Welcome headline only |
| H1 | 30px | 650 | 1.25 | Page title |
| H2 | 24px | 620 | 1.30 | Major section title |
| H3 | 18px | 600 | 1.40 | Subsection and panel title |
| Body | 15px | 400 | 1.70 | Long-form content |
| Body strong | 15px | 550 | 1.60 | Emphasis and labels |
| Small | 13px | 400 | 1.55 | Metadata and helper text |
| Micro | 12px | 500 | 1.45 | Status and compact controls |
| Button | 14px | 550 | 1.20 | Buttons |

Rules:

- Chinese headings do not use negative letter spacing.
- Keep reading columns between 680px and 760px.
- Avoid uppercase navigation labels and wide tracking.
- Use no more than three font weights on one screen.
- Use bold sparingly inside long-form content.

## 5. Spacing and Layout

Base unit: 4px. Primary rhythm: 8px.

```text
4   micro gap
8   icon gap, inline gap
12  compact control gap
16  default component gap
20  panel padding on compact screens
24  panel padding
32  section gap
48  major content break
64  page top and bottom breathing room
```

### Desktop shell

- Sidebar: 236px fixed width, `--sidebar` background, 1px right divider.
- Top bar: 52px, optional per page, white background, 1px bottom divider.
- Main canvas: fluid width with a maximum content width of 1120px.
- Document column: 720px target width.
- Page padding: 40px to 56px desktop, 24px tablet, 16px mobile.
- Dialog width: 480px normal, 720px for complex editing.

### Responsive behavior

- Below 900px, collapse the sidebar to 60px and hide text labels.
- Below 640px, replace the sidebar with a compact top bar or drawer.
- Stack split panes vertically on small screens.
- Keep touch targets at least 44px high.
- Never use `h-screen`; use `min-height: 100dvh`.
- Long tables become labeled rows or horizontally scroll within a clearly bounded region.

## 6. Shape and Depth

```text
4px   tags and compact inline elements
6px   buttons, inputs, navigation items
10px  dialogs, panels, meaningful cards
999px status dots, filter chips, avatars only
```

- Default content sections are flat and separated by whitespace or a single divider.
- Do not put every section inside a card.
- Standard panels use a 1px `--border` border and no shadow.
- Menus and dialogs may use `0 8px 24px rgba(47, 46, 43, 0.12)`.
- Selected items use background color, not elevation.

## 7. Navigation

The existing information architecture stays intact:

- 首页
- 经历库
- 角色档案
- 简历
- 岗位匹配
- 技能图谱
- 学习路径
- 设置

Sidebar behavior:

- Brand area is compact and text-led.
- Navigation rows are 36px high with 8px horizontal padding.
- Icons are 18px, visually consistent, and use one icon family.
- Default items use `--ink-secondary`.
- Hover uses `--surface`.
- Active uses `--surface`, `--ink-strong`, and font weight 550.
- Do not use gradients, neon accents, or a colored active rail.
- Section labels are optional. If used, write normal Chinese labels without uppercase styling.

## 8. Components

### Buttons

Primary button:

- Background `--ink-strong` for ordinary actions.
- Background `--accent` when the action invokes the core CareerCraft workflow, such as generating a resume or starting a match.
- White label, 6px radius, 10px 16px padding, minimum 40px height.
- Hover darkens the fill. Active translates down 1px.

Secondary button:

- White background, `--ink` label, 1px `--border-strong` border.
- No shadow.

Ghost button:

- Transparent background and `--ink-secondary` label.
- Hover uses `--surface`.

Destructive button:

- Use danger color only in the confirmation context.
- List-level delete actions remain quiet until hover or focus.

### Inputs

- Height 40px for compact fields, 44px for primary forms.
- White background, 1px `--border-strong`, 6px radius.
- Focus uses a 2px `--accent` ring with no glow.
- Labels sit above fields. Placeholder text never replaces a label.
- Validation appears inline beneath the field.
- Long experience input uses a generous editor surface, not a small textarea.

### Content rows

Use rows for experiences, jobs, learning resources, and generated resumes when users need to scan more than five items.

- 12px to 16px vertical padding.
- One subtle divider between rows, not a border around every row.
- Title first, one line of meaningful metadata second.
- Actions appear on hover, focus, or in an overflow menu.
- Selected state uses `--accent-soft`.

### Cards

Cards are reserved for:

- A distinct resume preview.
- A job-match summary.
- An empty-state onboarding block.
- A dialog-like task that needs clear containment.

Cards use 10px radius, 1px border, and 24px padding. Do not create three equal promotional cards by default.

### Tags and status

- Skills use soft neutral tags with 4px radius.
- Status uses compact tinted labels or a semantic icon plus text.
- Match score is shown as a large number plus a plain-language explanation.
- Do not use decorative progress bars, score rings, or radar charts.

### Dialogs

- Clear title and one-sentence explanation.
- Form content is left aligned.
- Footer places secondary action before the primary action.
- Escape closes when safe. Focus is trapped inside.
- Destructive confirmation names the item being deleted.

## 9. Page Patterns

### 首页

Use a left-aligned welcome document, not a dashboard of metrics.

1. Personal greeting and one short sentence describing the next useful step.
2. One primary action, such as “添加第一段经历” or “继续完善经历”.
3. Recent work as a compact list.
4. Optional progress summary expressed in plain language.

The empty state should feel like the first page of a notebook. Avoid emoji as the primary illustration.

### 经历库

- Document-style list grouped by work, project, and education.
- Search and filters remain in a quiet toolbar.
- Selecting an experience opens a right-side detail pane on wide screens.
- Editing prioritizes the original description, achievements, skills, and measurable results.

### 角色档案

- Each persona reads like a short profile page.
- Show target role, positioning statement, preferred evidence, and selected experiences.
- Compare personas with tabs or a select control, not side-by-side card walls.

### 简历

- Use a two-pane editor on desktop: controls on the left, document preview on the right.
- Preview uses a true white paper surface with a restrained shadow.
- Export and generate are the only visually strong actions.
- Template selection uses thumbnail previews with names, not abstract color swatches.

### 岗位匹配

- Show the job title and company first.
- Present score as evidence, not spectacle.
- Follow with “已匹配”, “需要补强”, and “可用于证明的经历”.
- Keep the original job description accessible in a collapsible document section.

### 技能图谱

- Use a structured list or simple graph only when relationships add meaning.
- Every skill exposes evidence from actual experiences.
- Color never acts as the only signal.

### 学习路径

- Use a reading-list pattern ordered by relevance or date.
- Show source, expected effort, target skill, and completion state.
- Avoid course-marketplace card styling.

### 设置

- Use a narrow settings document with grouped sections and dividers.
- Explain provider and privacy choices in direct language.
- Secret values are masked and never echoed in notifications.

## 10. AI Interaction States

### Loading

- Preserve layout with content-shaped skeletons.
- For operations longer than two seconds, show a short stage label such as “正在整理经历” or “正在比对岗位要求”.
- Never fabricate an exact percentage unless progress is measurable.

### Empty

- State what is missing.
- Explain why adding it helps.
- Provide one action.

Example:

```text
还没有经历记录
添加工作、项目或教育经历后，CareerCraft 才能生成有依据的简历。
[添加经历]
```

### Error

- Explain what failed in user language.
- Preserve the user's input.
- Offer retry when retry is safe.
- Put field errors next to fields. Use a toast only for transient global feedback.

### Success

- Prefer an inline state change or concise confirmation.
- Do not use confetti or full-screen celebration.

## 11. Motion

- Page transitions: 120ms fade, optional 4px vertical movement.
- Hover and focus transitions: 100ms to 150ms.
- Dialog entrance: 160ms opacity and scale from 0.98.
- Right detail pane: 180ms horizontal reveal.
- No springy navigation, parallax, animated gradients, or looping decoration.
- Respect `prefers-reduced-motion` and remove nonessential transforms.
- Motion exists only for hierarchy, feedback, or state continuity.

## 12. Accessibility

- All interactive elements are keyboard reachable.
- Use a visible 2px focus ring in `--accent`.
- Icon-only controls have accessible names and tooltips.
- Do not rely on hover for required information.
- Do not rely on color alone for match status, warnings, or errors.
- Preserve browser zoom and text scaling.
- Use semantic headings in order.
- Modal dialogs trap focus and restore it when closed.
- Minimum pointer target is 44 by 44px on touch layouts.

## 13. Copy Style

- Calm, direct, and specific.
- Prefer verbs: 添加经历、生成简历、分析岗位、保存修改.
- Avoid hype such as “一键逆袭”, “颠覆求职”, or “AI 魔法”.
- State uncertainty honestly: “根据现有经历推测” rather than “你一定适合”.
- Explain destructive effects before confirmation.
- Use Chinese punctuation consistently.
- Do not expose internal model, database, or exception terminology unless the user opens technical details.

## 14. Do and Do Not

### Do

- Let documents and evidence dominate the page.
- Use warm neutral surfaces and a single blue accent.
- Keep navigation compact and familiar.
- Prefer lists, dividers, and whitespace over card grids.
- Preserve user input during errors and AI retries.
- Show where recommendations came from.

### Do not

- Do not copy Notion branding or proprietary assets.
- Do not use the current purple gradient brand mark.
- Do not default to a dark theme.
- Do not put every block in a rounded card.
- Do not use glassmorphism, neon glow, or AI-purple gradients.
- Do not use decorative score rings, radar charts, or progress bars.
- Do not mix multiple icon families.
- Do not animate for decoration.

## 15. Implementation Mapping

The first visual migration should change tokens before restructuring pages:

```css
:root {
  --bg-canvas: #ffffff;
  --bg-panel: #f7f6f3;
  --bg-surface: #f1f0ed;
  --bg-elevated: #ffffff;
  --text-primary: #2f2e2b;
  --text-secondary: #6f6d67;
  --text-tertiary: #9b9992;
  --text-quaternary: #b6b3ac;
  --accent-brand: #2463a6;
  --accent-hover: #1d528b;
  --border-subtle: #eceae5;
  --border-standard: #e5e3de;
  --border-solid: #cfccc5;
  --success: #2f7d4a;
  --warning: #9a6718;
  --danger: #b33a3a;
  --radius-sm: 4px;
  --radius-md: 6px;
  --radius-lg: 10px;
  --radius-pill: 9999px;
  --shadow-elevated: 0 8px 24px rgba(47, 46, 43, 0.12);
}
```

Migration order:

1. Replace global color, type, radius, and shadow tokens.
2. Simplify the sidebar active state and brand mark.
3. Convert repeated bordered cards into document rows and sections.
4. Rework empty, loading, error, and success states.
5. Refine each major page pattern without renaming routes or navigation labels.
6. Verify keyboard navigation, contrast, Chinese text wrapping, and WebView rendering.

## 16. Agent Prompt Guide

When implementing UI work, use this instruction:

```text
Read DESIGN.md before editing the interface. Preserve existing routes, navigation labels,
form fields, behavior, QWebChannel bindings, and user data flows. Apply the CareerCraft
document-first visual language using warm neutral surfaces and Career Blue as the only
interactive accent. Prefer whitespace, dividers, and content rows over generic cards.
Implement loading, empty, error, success, focus, hover, active, disabled, and reduced-motion
states. Do not copy Notion branding or assets.
```

