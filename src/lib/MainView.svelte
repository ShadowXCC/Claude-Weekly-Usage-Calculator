<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl } from '@tauri-apps/plugin-opener'

  interface UsageInfo {
    suggested_pct: number
    time_until_reset: string
    reset_weekday: number
    reset_hour: number
    reset_minute: number
    last_refreshed: string
  }

  const WEEKDAY_NAMES = ['Monday','Tuesday','Wednesday','Thursday','Friday','Saturday','Sunday']

  let { refreshTick }: { refreshTick: number } = $props()

  let info: UsageInfo | null = $state(null)
  let error: string | null = $state(null)

  async function fetchInfo() {
    try {
      info = await invoke<UsageInfo>('get_usage_info')
      error = null
    } catch (e) {
      error = String(e)
    }
  }

  // Fetch on mount and whenever refreshTick changes
  $effect(() => {
    void refreshTick // depend on it
    fetchInfo()
  })

  function formatPct(pct: number): string {
    return pct.toFixed(1)
  }

  function resetLabel(info: UsageInfo): string {
    const day = WEEKDAY_NAMES[info.reset_weekday] ?? 'Unknown'
    const h = info.reset_hour
    const m = info.reset_minute
    const ampm = h >= 12 ? 'PM' : 'AM'
    const h12 = h % 12 === 0 ? 12 : h % 12
    const mStr = m.toString().padStart(2, '0')
    return `${day}s at ${h12}:${mStr} ${ampm}`
  }

  // The arc SVG for the big display circle
  function arcPath(pct: number, r: number, cx: number, cy: number): string {
    const frac = Math.min(Math.max(pct / 100, 0), 1)
    if (frac >= 0.9999) {
      // Full circle
      return `M ${cx} ${cy - r} A ${r} ${r} 0 1 1 ${cx - 0.001} ${cy - r} Z`
    }
    const angle = frac * 2 * Math.PI - Math.PI / 2  // clockwise from top
    const startAngle = -Math.PI / 2
    const x1 = cx + r * Math.cos(startAngle)
    const y1 = cy + r * Math.sin(startAngle)
    const x2 = cx + r * Math.cos(angle)
    const y2 = cy + r * Math.sin(angle)
    const largeArc = frac > 0.5 ? 1 : 0
    return `M ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2}`
  }

  async function openUsagePage() {
    await openUrl('https://claude.ai/settings/usage')
  }
</script>

<div class="main-view">
  {#if error}
    <div class="error-card">
      <p>Failed to load usage info:</p>
      <code>{error}</code>
    </div>
  {:else if info}
    <!-- Big circular gauge -->
    <div class="gauge-section">
      <div class="gauge-wrap">
        <svg class="gauge-svg" viewBox="0 0 120 120" width="180" height="180" aria-hidden="true">
          <!-- Track ring -->
          <circle
            cx="60" cy="60" r="50"
            fill="none"
            stroke="var(--claude-track)"
            stroke-width="10"
          />
          <!-- Filled arc -->
          <path
            d={arcPath(info.suggested_pct, 50, 60, 60)}
            fill="none"
            stroke="var(--claude-orange)"
            stroke-width="10"
            stroke-linecap="round"
          />
          <!-- Center label -->
          <text x="60" y="56" text-anchor="middle" class="gauge-pct">{formatPct(info.suggested_pct)}%</text>
          <text x="60" y="72" text-anchor="middle" class="gauge-sub">suggested</text>
        </svg>
      </div>
      <p class="gauge-caption">
        You should be <strong>at or above</strong> this usage to stay on track for 100% weekly usage.
      </p>
    </div>

    <!-- Progress bar section -->
    <div class="bar-section">
      <div class="bar-header">
        <span class="bar-label">Weekly Usage Progress</span>
        <span class="bar-pct">{formatPct(info.suggested_pct)}%</span>
      </div>

      <div class="bar-track" role="progressbar" aria-valuenow={info.suggested_pct} aria-valuemin={0} aria-valuemax={100}>
        <div class="bar-fill" style="width: {Math.min(info.suggested_pct, 100)}%"></div>
        <div class="bar-marker" style="left: {Math.min(info.suggested_pct, 100)}%">
          <div class="bar-marker-line"></div>
          <div class="bar-marker-label">at or above</div>
        </div>
      </div>

      <div class="bar-scale">
        <span>0%</span>
        <span>25%</span>
        <span>50%</span>
        <span>75%</span>
        <span>100%</span>
      </div>
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
        <div class="info-card-label">Last Updated</div>
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

    <!-- Future: manual usage input placeholder -->
    <!-- TODO: Add manual current usage input (slider + text box).
         When manually entered, display alongside suggested but do not use for any math.
         See future-ideas.txt for automatic usage tracking ideas. -->

  {:else}
    <div class="loading">Loading usage data…</div>
  {/if}
</div>

<style>
  .main-view {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding: 28px 20px 32px;
  }

  /* ── Error ─────────────────────────────────────────────── */
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

  /* ── Gauge ─────────────────────────────────────────────── */
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

  /* ── Bar ──────────────────────────────────────────────── */
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

  /* ── Info cards ────────────────────────────────────────── */
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

  /* ── Actions ──────────────────────────────────────────── */
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

  /* ── Loading ──────────────────────────────────────────── */
  .loading {
    text-align: center;
    padding: 60px 0;
    color: var(--claude-text-muted);
    font-size: 14px;
  }


</style>
