# Auto-Fetch Trades from IB on App Startup

## Goal

When the app launches, if the user has configured IB Flex credentials (token + query ID), automatically fetch the latest trades from IB in the background and update the UI without blocking startup.

## Current State

- `flex_fetcher.rs` exists — handles the 2-step Flex Web Service fetch (SendRequest → GetStatement), parses XML, saves CSV to `%LOCALAPPDATA%\TraderRank\imports\ib_flex_import.csv`
- `settings_store.rs` persists `flex_token` and `flex_query_id`
- `data_loader::load_app_state()` reads CSVs from both `Data/Source/` and `%LOCALAPPDATA%\TraderRank\imports\` on startup
- `settings_store.rs` uses `%LOCALAPPDATA%\TraderRank\settings.json` (auto-migrates from legacy path)
- App currently loads data synchronously in `App()` component via `use_context_provider`

## Design

### Startup Flow (Two-Phase Load)

1. **Phase 1 — Instant startup (existing behavior):** Load CSVs from `Data/Source/` (manual) + `%LOCALAPPDATA%\TraderRank\imports\` (fetched). Gives the user immediate data from the last fetch. No delay.

2. **Phase 2 — Background fetch:** If flex credentials are configured, spawn an async task that:
   - Fetches latest trades from IB Flex Web Service
   - Saves to `%LOCALAPPDATA%\TraderRank\imports\ib_flex_import.csv` (overwrites previous)
   - Re-runs the full data pipeline (parse CSVs → analytics → trade matching)
   - Replaces the `AppState` signal with the fresh data
   - Shows a subtle status indicator in the nav bar

### Files to Modify

#### `src/main.rs`
- In `App()` component, after the initial sync load:
  - Read flex credentials from `PersistedSettings`
  - If both token and query_id are non-empty, `spawn` an async task
  - The async task calls `flex_fetcher::fetch_and_save()`, then rebuilds `AppState`
  - On success, update the `Signal<AppState>` with fresh data (all views reactively update)
  - Provide a `Signal<SyncStatus>` via context for the nav bar indicator

#### `src/main.rs` — `AppLayout` component
- Add a small sync status indicator in the nav bar (e.g., spinning icon during fetch, checkmark on success, X on error)
- Fades out after a few seconds on success

#### `src/data_loader.rs`
- Extract the CSV → AppState pipeline into a public function so it can be called from both startup and the async refresh:
  - `pub fn reload_from_csv() -> AppState` — loads CSVs, runs analytics, returns fresh state
- The existing `load_app_state()` calls this internally

#### `src/flex_fetcher.rs`
- Already complete. No changes needed for the core fetch logic.
- Consider adding a `fetch_if_stale()` variant that checks the last fetch timestamp and skips if < N minutes old (avoid hammering IB on rapid restarts). Store `last_flex_fetch` timestamp in `PersistedSettings`.

### New Types

```rust
// In main.rs or a shared module
#[derive(Clone, PartialEq)]
enum SyncStatus {
    Idle,
    Syncing,
    Success { trade_count: usize },
    Error(String),
}
```

### Staleness Check (Optional Enhancement)

To avoid fetching on every single restart:
- Add `last_flex_fetch: String` (ISO timestamp) to `PersistedSettings`
- On startup, check if last fetch was < 30 minutes ago → skip
- Always fetch if > 30 minutes or if no previous fetch recorded
- The manual "Fetch" button in Settings always forces a fetch regardless

### Nav Bar Indicator

Minimal UI — a small text/icon in the nav-right area:
- Syncing: `"Syncing..."` with subtle pulse animation
- Success: `"Synced {N} trades"` — fades after 5s
- Error: `"Sync failed"` — stays visible, clickable to go to Settings
- Idle: hidden

### CSS

```css
.sync-indicator {
    font-size: 12px;
    color: var(--text-muted);
    margin-right: 12px;
    transition: opacity 0.3s;
}
.sync-indicator.syncing { color: var(--accent-primary); }
.sync-indicator.success { color: var(--accent-green); }
.sync-indicator.error { color: var(--accent-red); cursor: pointer; }
```

## Implementation Order

1. Add `SyncStatus` enum and provide as context
2. Extract `reload_from_csv()` in `data_loader.rs`
3. Add spawn logic in `App()` for background fetch + state replacement
4. Add nav bar sync indicator in `AppLayout`
5. Add staleness check with `last_flex_fetch` in settings
6. CSS for indicator

## Edge Cases

- **No credentials configured:** Skip silently. No indicator shown.
- **Network failure:** Show error in nav indicator. App still works with cached CSV data from last successful fetch.
- **IB rate limit (1019):** Already handled in `flex_fetcher.rs` with retries.
- **Concurrent fetch:** If user manually clicks Fetch in Settings while auto-fetch is running, the second write to the same CSV file is fine — last writer wins, and the data is the same.
- **Empty response:** If IB returns 0 trades, don't overwrite existing CSV. Log a warning.
- **Token expired:** IB returns an error code. Show in nav indicator, user goes to Settings to regenerate.

## Testing

1. Start app with no flex credentials → no fetch, no indicator
2. Configure credentials in Settings, restart → auto-fetch runs, indicator shows progress
3. Kill network mid-fetch → error indicator, app still loads cached data
4. Restart within 30 min → staleness check skips fetch
5. Manual Fetch in Settings → always fetches regardless of staleness
