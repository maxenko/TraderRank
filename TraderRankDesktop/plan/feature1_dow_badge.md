# Feature 1 — Day-of-Week Badge as First Column in Timeline Table

## Goal

Add a small colored badge as the **first column** of the Timeline table that shows the day of week (Mon/Tue/Wed/Thu/Fri) for Daily rows, the Monday label for Weekly rows, and the month abbreviation for Monthly rows. The column is sortable and styled consistently with the rest of the app's neutral chip aesthetic.

## Files to modify

- `D:\GitHub\TraderRank\TraderRankDesktop\src\views\timeline.rs`
- `D:\GitHub\TraderRank\TraderRankDesktop\assets\main.css`

No new files. No model changes. No persisted-settings changes (the new column reuses `"period"` sorting semantics).

## Detailed steps

### 1. Extend `SortableRow` with badge fields

In `src/views/timeline.rs`, edit the `SortableRow` struct (around line 36) to carry a precomputed badge label and an explicit weekday sort key:

```rust
#[derive(Clone)]
#[allow(dead_code)]
struct SortableRow {
    /// 3-letter badge label: "Mon".."Fri" for daily, "Mon" (week start) for weekly,
    /// 3-letter month abbrev ("Jan".."Dec") for monthly.
    badge_label: String,
    /// Badge variant CSS class suffix: "mon", "tue", "wed", "thu", "fri", "week", "month".
    badge_variant: &'static str,
    /// Sort key for the badge column. For daily/weekly: weekday num (Mon=0..Fri=4).
    /// For monthly: month number 1..12. Sorts in natural calendar order.
    badge_sort_key: i32,

    period: String,
    sort_date: chrono::NaiveDate,
    sort_period_num: i64,
    realized_pnl: Decimal,
    r_mult: Decimal,
    win_rate: f64,
    total_trades: u32,
    winning_trades: u32,
    losing_trades: u32,
    total_commission: Decimal,
    is_positive: bool,
}
```

### 2. Add a helper to compute badge fields from a weekday

Add this free function at the top of `src/views/timeline.rs` (above `Timeline()`):

```rust
/// Map a chrono Weekday to (3-letter label, css variant suffix, sort key 0..6).
/// Sat/Sun are mapped but should never appear in trading data.
fn weekday_badge(wd: chrono::Weekday) -> (&'static str, &'static str, i32) {
    match wd {
        chrono::Weekday::Mon => ("Mon", "mon", 0),
        chrono::Weekday::Tue => ("Tue", "tue", 1),
        chrono::Weekday::Wed => ("Wed", "wed", 2),
        chrono::Weekday::Thu => ("Thu", "thu", 3),
        chrono::Weekday::Fri => ("Fri", "fri", 4),
        chrono::Weekday::Sat => ("Sat", "sat", 5),
        chrono::Weekday::Sun => ("Sun", "sun", 6),
    }
}

/// 3-letter month abbreviation for monthly badge.
fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "—",
    }
}
```

### 3. Populate badge fields in each mode branch

In the `Daily` arm of `match current_mode` (around line 71), inside the `.map(|d| ...)` closure, compute the weekday from `d.date`:

```rust
let wd = d.date.weekday();
let (badge_label, badge_variant, badge_sort_key) = weekday_badge(wd);
```

…and pass `badge_label: badge_label.to_string(), badge_variant, badge_sort_key,` into the `SortableRow { ... }` literal.

In the `Weekly` arm (around line 99), every weekly row's `start_date` is a Monday, so:

```rust
let badge_label = "Mon".to_string();
let badge_variant = "week";
let badge_sort_key = 0;
```

Add those three fields to the `SortableRow { ... }` literal.

In the `Monthly` arm (around line 136), use `month_abbrev`:

```rust
let badge_label = month_abbrev(m.month).to_string();
let badge_variant = "month";
let badge_sort_key = m.month as i32;
```

Add the three fields to the `Some(SortableRow { ... })` literal.

### 4. Add `"dow"` to the sort match

In the `rows.sort_by(...)` call (around line 176), add a new arm before the default:

```rust
"dow" => a.badge_sort_key.cmp(&b.badge_sort_key)
    .then_with(|| a.sort_date.cmp(&b.sort_date)),
```

The `.then_with` tiebreaker ensures rows with the same weekday/month sort by date as a secondary key (so when sorted by Mon, the earliest Monday is first).

### 5. Add the new `<th>` as the first header cell

Inside `thead { tr { ... } }` (around line 273), insert a new `th` BEFORE the existing `"Period"` th:

```rust
th {
    class: header_class("dow"),
    style: "width: 70px;",
    onclick: move |_| {
        let col = sort_col.read().clone();
        if col == "dow" {
            let cur = *sort_asc.read();
            sort_asc.set(!cur);
        } else {
            sort_col.set("dow".to_string());
            sort_asc.set(true);
        }
        let sc = sort_col.read().clone();
        let sa = *sort_asc.read();
        settings_store::update(|s| { s.timeline_sort_col = sc; s.timeline_sort_asc = sa; });
    },
    "Day{sort_indicator(\"dow\")}"
}
```

Note: defaults to **ascending** when first clicked (Mon → Fri / Jan → Dec feels natural).

### 6. Add the new `<td>` as the first body cell

Inside the `tbody { for row in rows.iter().take(...) { ... } }` block (around line 396), inside the `rsx! { tr { ... } }`, add the badge cell BEFORE the existing `td { "{period}" }`:

```rust
let badge_label = row.badge_label.clone();
let badge_class = format!("dow-badge dow-{}", row.badge_variant);
```

(declare these alongside the existing `let period = ...` lines), then in the `tr`:

```rust
tr { class: "{row_class}",
    td { class: "dow-cell",
        span { class: "{badge_class}", "{badge_label}" }
    }
    td { "{period}" }
    td { class: "pnl", "{pnl}" }
    // ... rest unchanged
}
```

### 7. Add CSS for the badge

Append to `D:\GitHub\TraderRank\TraderRankDesktop\assets\main.css` (after the existing `.timeline-row.negative` block around line 720, before the `/* Trade table specifics */` comment):

```css
/* Day-of-Week / Period badge in Timeline first column */
.timeline-table td.dow-cell {
    width: 70px;
    padding: 8px 12px;
}

.dow-badge {
    display: inline-block;
    min-width: 42px;
    text-align: center;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.5px;
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
    line-height: 1.2;
    font-variant: small-caps;
}

/* Per-weekday subtle accent on left border so the column scans visually */
.dow-badge.dow-mon { border-left: 3px solid var(--accent-primary); }
.dow-badge.dow-tue { border-left: 3px solid var(--accent-cyan); }
.dow-badge.dow-wed { border-left: 3px solid var(--accent-green); }
.dow-badge.dow-thu { border-left: 3px solid var(--accent-yellow); }
.dow-badge.dow-fri { border-left: 3px solid var(--accent-orange); }
.dow-badge.dow-sat,
.dow-badge.dow-sun { border-left: 3px solid var(--text-muted); opacity: 0.6; }

/* Weekly mode: every row shows "Mon" badge — use neutral primary accent */
.dow-badge.dow-week {
    border-left: 3px solid var(--accent-primary);
    background: var(--accent-primary-glow);
    color: var(--text-heading);
}

/* Monthly mode: month abbrev — neutral cyan tint */
.dow-badge.dow-month {
    border-left: 3px solid var(--accent-cyan);
    color: var(--text-heading);
}
```

## Edge cases & decisions

- **Weekly rows:** All weekly rows literally start on Monday (per `WeeklySummary.start_date`). Showing "Mon" on every row is uninformative repetition, but keeps the column structurally present and visually consistent. **Decision: render "Mon" badge with the dedicated `dow-week` style** (slightly tinted to read as "week start"). An alternative — leaving the cell blank in Weekly mode — was rejected because empty leading cells make the table feel broken.
- **Monthly rows:** No weekday applies. **Decision: render the 3-letter month abbreviation** (Jan, Feb, …) with the `dow-month` variant. This keeps the column meaningful in all three modes.
- **Weekend days (Sat/Sun):** Trading is M-F. Mapped defensively in `weekday_badge` so a stray weekend trade still renders rather than crashing, but styled at 60% opacity in muted color so it's visually flagged as anomalous.
- **Sort order:** When the Day column is sorted ascending, daily rows go Mon → Fri (calendar order, NOT alphabetical). Monthly rows go Jan → Dec. This is achieved via the explicit `badge_sort_key` (0..4 for weekdays, 1..12 for months) — never alphabetical. **Default sort direction on first click is ascending** (most natural for chronological labels).
- **Sort tiebreaker:** When grouping by weekday (e.g. all Mondays together), sort by date within that group via `.then_with(|| a.sort_date.cmp(&b.sort_date))`.
- **Persisted settings:** Reuses existing `timeline_sort_col` / `timeline_sort_asc` fields. No `PersistedSettings` changes required. If a user had `timeline_sort_col == "dow"` saved from a future session and downgrades, the sort match falls through to the default `period` arm safely.
- **Column width:** Fixed 70px keeps the badge aligned and prevents the rest of the table from reflowing as data changes.
- **Badge component:** Inline `span` with class — no separate Dioxus `#[component]` (a 4-line span doesn't justify the indirection per project KISS preference).

## Verification

```bash
cargo check
cargo build
cargo run
```

Manual visual checks after `cargo run`:
1. Switch to **Daily** mode — first column shows colored Mon/Tue/Wed/Thu/Fri badges, one per row.
2. Click the **"Day"** header — rows reorder by weekday Mon→Fri (ascending). Click again — Fri→Mon (descending).
3. Switch to **Weekly** — every row shows a primary-tinted "Mon" badge.
4. Switch to **Monthly** — each row shows a cyan-tinted 3-letter month (Jan, Feb, …). Sorting by Day puts them in calendar order Jan→Dec.
5. Toggle Light/Dark theme — badges remain readable in both (all colors are CSS variables).
6. Restart the app — sort column persists across sessions.
