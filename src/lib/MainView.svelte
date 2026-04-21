<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { arcPath, formatPct } from './gauge'
  import WeekBarSegmented from './WeekBarSegmented.svelte'

  interface UsageInfo {
    suggested_pct: number
    remaining_pct: number
    time_until_reset: string
    reset_weekday: number
    reset_hour: number
    reset_minute: number
    last_refreshed: string
    tz: string
    tz_display: string
  }

  const WEEKDAY_NAMES = ['Monday','Tuesday','Wednesday','Thursday','Friday','Saturday','Sunday']

  let {
    refreshTick,
    displayInverse,
    weekBarSegmented,
    a11yAnnounce,
    latestThreshold,
  }: {
    refreshTick: number
    displayInverse: boolean
    weekBarSegmented: boolean
    a11yAnnounce: boolean
    latestThreshold: number | null
  } = $props()

  let info = $state<UsageInfo | null>(null)
  let error = $state<string | null>(null)

  async function fetchInfo() {
    try {
      info = await invoke<UsageInfo>('get_usage_info')
      error = null
    } catch (e) {
      error = String(e)
    }
  }

  $effect(() => {
    void refreshTick
    fetchInfo()
  })

  let shownPct = $derived(info ? (displayInverse ? info.remaining_pct : info.suggested_pct) : 0)
  // Gauge arc stays aligned with the center number (fuel-gauge metaphor).
  let gaugeArcPct = $derived(info ? (displayInverse ? info.remaining_pct : info.suggested_pct) : 0)
  // Linear bar always represents time elapsed through the week (progress metaphor).
  let barFillPct = $derived(info ? info.suggested_pct : 0)

  // Gauge label/aria strings flip with the inverse toggle.
  let headlineLabel = $derived(displayInverse ? 'remaining' : 'suggested')
  let gaugeAriaLabel = $derived(
    info
      ? displayInverse
        ? `${formatPct(info.remaining_pct)} percent of the weekly budget remaining`
        : `${formatPct(info.suggested_pct)} percent used (suggested)`
      : 'Loading',
  )

  function resetLabel(info: UsageInfo): string {
    const day = WEEKDAY_NAMES[info.reset_weekday] ?? 'Unknown'
    const h = info.reset_hour
    const m = info.reset_minute
    const ampm = h >= 12 ? 'PM' : 'AM'
    const h12 = h % 12 === 0 ? 12 : h % 12
    const mStr = m.toString().padStart(2, '0')
    return `${day}s at ${h12}:${mStr} ${ampm}`
  }

  async function openUsagePage() {
    await openUrl('https://claude.ai/settings/usage')
  }

  // ── Threshold live-region announcement ─────────────────────
  // We keep a local, user-friendly message and update it on each new event.
  let announce = $state('')
  $effect(() => {
    if (!a11yAnnounce || latestThreshold === null) return
    const suffix = displayInverse
      ? `${100 - latestThreshold}% of your weekly budget remains`
      : `${latestThreshold}% of your weekly budget has been used`
    announce = suffix
  })
</script>

<div class="main-view">
  {#if error}
    <div class="error-card" role="alert">
      <p>Failed to load usage info:</p>
      <code>{error}</code>
    </div>
  {:else if info}
    <!-- Big circular gauge -->
    <div class="gauge-section">
      <div
        class="gauge-wrap"
        role="progressbar"
        aria-valuenow={Math.round(shownPct)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={gaugeAriaLabel}
      >
        <svg class="gauge-svg" viewBox="0 0 120 120" width="180" height="180" aria-hidden="true">
          <circle
            cx="60" cy="60" r="50"
            fill="none"
            stroke="var(--claude-track)"
            stroke-width="10"
          />
          <path
            d={arcPath(gaugeArcPct, 50, 60, 60)}
            fill="none"
            stroke="var(--claude-orange)"
            stroke-width="10"
            stroke-linecap="round"
          />
          <text x="60" y="56" text-anchor="middle" class="gauge-pct">{formatPct(shownPct)}%</text>
          <text x="60" y="72" text-anchor="middle" class="gauge-sub">{headlineLabel}</text>
        </svg>
      </div>
      <p class="gauge-caption">
        {#if displayInverse}
          You have <strong>{formatPct(shownPct)}%</strong> of the week remaining.
        {:else}
          You should be <strong>at or above</strong> this usage to stay on track for 100% weekly usage.
        {/if}
      </p>
    </div>

    <!-- Progress bar section -->
    <div class="bar-section">
      <div class="bar-header">
        <span class="bar-label">{displayInverse ? 'Weekly Budget Remaining' : 'Weekly Usage Progress'}</span>
        <span class="bar-pct">{formatPct(shownPct)}%</span>
      </div>

      {#if weekBarSegmented}
        <WeekBarSegmented fillPct={barFillPct} resetWeekday={info.reset_weekday} />
      {:else}
        <div class="bar-track">
          <div class="bar-fill" style="width: {Math.min(barFillPct, 100)}%"></div>
          {#if !displayInverse}
            <div class="bar-marker" style="left: {Math.min(barFillPct, 100)}%">
              <div class="bar-marker-line"></div>
              <div class="bar-marker-label">at or above</div>
            </div>
          {/if}
        </div>

        <div class="bar-scale">
          <span>0%</span>
          <span>25%</span>
          <span>50%</span>
          <span>75%</span>
          <span>100%</span>
        </div>
      {/if}
    </div>

    <!-- Info cards row -->
    <div class="info-row">
      <div class="info-card">
        <div class="info-card-label">Reset In</div>
        <div class="info-card-value">{info.time_until_reset}</div>
      </div>
      <div class="info-card">
        <div class="info-card-label">Reset Schedule</div>
        <div class="info-card-value">{resetLabel(info)}</div>
      </div>
      <div class="info-card info-card--full">
        <div class="info-card-label">Last Updated · {info.tz_display}</div>
        <div class="info-card-value">{info.last_refreshed}</div>
      </div>
    </div>

    <!-- Usage page shortcut -->
    <div class="actions">
      <button class="action-btn primary" onclick={openUsagePage}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
          <polyline points="15 3 21 3 21 9"/>
          <line x1="10" y1="14" x2="21" y2="3"/>
        </svg>
        Open Claude Usage Page
      </button>
      <button class="action-btn secondary" onclick={fetchInfo}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="23 4 23 10 17 10"/>
          <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
        </svg>
        Refresh
      </button>
    </div>
  {:else}
    <div class="loading">Loading usage data…</div>
  {/if}
</div>

<!-- Live region for threshold announcements (screen readers only). -->
<div role="status" aria-live="polite" class="sr-only">{announce}</div>

<style>
  .main-view {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding: 28px 20px 32px;
  }

  .error-card {
    background: var(--claude-error-bg);
    border: 1.5px solid var(--claude-error-border);
    border-radius: var(--radius-md);
    padding: 16px;
    font-size: 13px;
    color: var(--claude-error-text);
  }
  .error-card code {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    word-break: break-all;
  }

  .gauge-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .gauge-wrap {
    background: var(--claude-surface);
    border-radius: 50%;
    padding: 8px;
    box-shadow: 0 2px 12px rgba(0,0,0,0.18);
  }

  :global(.gauge-pct) {
    font-size: 20px;
    font-weight: 700;
    fill: var(--claude-text);
    font-family: inherit;
  }

  :global(.gauge-sub) {
    font-size: 9px;
    fill: var(--claude-text-muted);
    font-family: inherit;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .gauge-caption {
    font-size: 13px;
    color: var(--claude-text-muted);
    text-align: center;
    max-width: 320px;
    line-height: 1.5;
  }

  .gauge-caption strong {
    color: var(--claude-orange);
    font-weight: 600;
  }

  .bar-section {
    background: var(--claude-surface);
    border-radius: var(--radius-md);
    padding: 16px 18px 14px;
    border: 1.5px solid var(--claude-border);
  }

  .bar-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 10px;
  }

  .bar-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--claude-text);
  }

  .bar-pct {
    font-size: 22px;
    font-weight: 700;
    color: var(--claude-orange);
    letter-spacing: -0.5px;
  }

  .bar-track {
    position: relative;
    height: 18px;
    background: var(--claude-track);
    border-radius: 9px;
    overflow: visible;
    margin-bottom: 6px;
  }

  .bar-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--claude-orange-deep), var(--claude-orange));
    border-radius: 9px;
    transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    min-width: 0;
    max-width: 100%;
  }

  .bar-marker {
    position: absolute;
    top: -4px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    pointer-events: none;
  }

  .bar-marker-line {
    width: 2px;
    height: 26px;
    background: var(--claude-orange-deep);
    border-radius: 1px;
  }

  .bar-marker-label {
    font-size: 10px;
    color: var(--claude-orange-deep);
    font-weight: 600;
    white-space: nowrap;
    letter-spacing: 0.3px;
    margin-top: 2px;
  }

  .bar-scale {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--claude-text-muted);
    padding: 0 2px;
    margin-top: 18px;
  }

  .info-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .info-card--full {
    grid-column: 1 / -1;
  }

  .info-card {
    background: var(--claude-surface);
    border: 1.5px solid var(--claude-border);
    border-radius: var(--radius-md);
    padding: 14px 16px;
  }

  .info-card-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--claude-text-muted);
    margin-bottom: 4px;
    font-weight: 600;
  }

  .info-card-value {
    font-size: 15px;
    font-weight: 600;
    color: var(--claude-text);
  }

  .actions {
    display: flex;
    gap: 10px;
  }

  .action-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    font-weight: 500;
    font-size: 13px;
  }

  .action-btn.primary {
    background: var(--claude-orange);
    color: #fff;
  }

  .action-btn.primary:hover {
    background: var(--claude-orange-deep);
  }

  .action-btn.secondary {
    background: var(--claude-surface);
    color: var(--claude-text);
    border: 1.5px solid var(--claude-border);
  }

  .action-btn.secondary:hover {
    background: var(--claude-cream-dark);
    border-color: var(--claude-orange);
    color: var(--claude-orange);
  }

  .loading {
    text-align: center;
    padding: 60px 0;
    color: var(--claude-text-muted);
    font-size: 14px;
  }
</style>
