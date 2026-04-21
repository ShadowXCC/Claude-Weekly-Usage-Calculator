<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { arcPath, formatPct } from './gauge'

  interface UsageInfo {
    suggested_pct: number
    remaining_pct: number
    now_unix: number
    next_reset_unix: number
    tz_display: string
  }

  interface SimResult {
    suggested_pct: number
    remaining_pct: number
    target_label: string
  }

  let {
    displayInverse,
    refreshTick,
  }: {
    displayInverse: boolean
    refreshTick: number
  } = $props()

  let info = $state<UsageInfo | null>(null)
  let sim = $state<SimResult | null>(null)
  let error = $state<string | null>(null)
  let loading: boolean = $state(false)

  // Offset in minutes from `now_unix` (keeps single-integer truth for both inputs).
  let offsetMinutes: number = $state(0)

  let maxOffsetMinutes = $derived(
    info ? Math.max(0, Math.floor((info.next_reset_unix - info.now_unix) / 60)) : 0,
  )
  let targetUnix = $derived(info ? info.now_unix + offsetMinutes * 60 : 0)

  // ISO local datetime (YYYY-MM-DDTHH:MM) for the datetime-local input.
  // Note: value reflects the user's wall clock; we convert to unix via Date parsing.
  let targetIso = $derived(unixToLocalIso(targetUnix))

  let nowIso = $derived(info ? unixToLocalIso(info.now_unix) : '')
  let maxIso = $derived(info ? unixToLocalIso(info.next_reset_unix) : '')

  function unixToLocalIso(u: number): string {
    if (!u) return ''
    const d = new Date(u * 1000)
    const pad = (n: number) => n.toString().padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
  }

  function localIsoToUnix(iso: string): number {
    const t = new Date(iso).getTime()
    return Number.isFinite(t) ? Math.floor(t / 1000) : 0
  }

  async function loadInfo() {
    try {
      info = await invoke<UsageInfo>('get_usage_info')
      error = null
      // Clamp offset if max shrank (e.g. close to reset).
      if (offsetMinutes > maxOffsetMinutes) offsetMinutes = maxOffsetMinutes
      await runSim()
    } catch (e) {
      error = String(e)
    }
  }

  let simDebounce: ReturnType<typeof setTimeout> | undefined
  async function runSim() {
    if (!info) return
    if (simDebounce) clearTimeout(simDebounce)
    simDebounce = setTimeout(async () => {
      loading = true
      try {
        sim = await invoke<SimResult>('simulate_at', { unixSeconds: targetUnix })
        error = null
      } catch (e) {
        error = String(e)
      } finally {
        loading = false
      }
    }, 80)
  }

  $effect(() => {
    void refreshTick
    loadInfo()
  })

  // Re-run sim whenever the offset changes (after info is loaded).
  $effect(() => {
    void offsetMinutes
    if (info) runSim()
  })

  function onDatetimeInput(e: Event) {
    const val = (e.target as HTMLInputElement).value
    if (!val || !info) return
    const unix = localIsoToUnix(val)
    if (!unix) return
    const diffMin = Math.round((unix - info.now_unix) / 60)
    offsetMinutes = Math.max(0, Math.min(maxOffsetMinutes, diffMin))
  }

  function onSliderInput(e: Event) {
    offsetMinutes = Number((e.target as HTMLInputElement).value)
  }

  function offsetLabel(mins: number): string {
    if (mins === 0) return 'now'
    const h = Math.floor(mins / 60)
    const m = mins % 60
    if (h === 0) return `+${m}m`
    if (m === 0) return `+${h}h`
    return `+${h}h ${m}m`
  }

  let shownPct = $derived(sim ? (displayInverse ? sim.remaining_pct : sim.suggested_pct) : 0)
  let headlineLabel = $derived(displayInverse ? 'remaining' : 'used')
</script>

<div class="sim-view">
  {#if error}
    <div class="error-card" role="alert">
      <p>Simulator error:</p>
      <code>{error}</code>
    </div>
  {:else if !info}
    <div class="loading">Loading simulator…</div>
  {:else}
    <div class="intro">
      <h2 class="section-title">Look-ahead</h2>
      <p class="section-desc">
        Preview where the gauge will be at a future moment. Purely read-only —
        nothing here changes your live state.
      </p>
    </div>

    <div class="controls">
      <label class="control">
        <span class="control-label">Offset from now</span>
        <input
          type="range"
          min="0"
          max={maxOffsetMinutes}
          step="15"
          value={offsetMinutes}
          oninput={onSliderInput}
          aria-label="Offset from now, in minutes"
        />
        <span class="control-value">{offsetLabel(offsetMinutes)}</span>
      </label>

      <label class="control">
        <span class="control-label">Target time ({info.tz_display})</span>
        <input
          type="datetime-local"
          value={targetIso}
          min={nowIso}
          max={maxIso}
          oninput={onDatetimeInput}
          aria-label="Target datetime for simulation"
        />
      </label>
    </div>

    <div class="preview-section" aria-live="polite">
      <div class="preview-watermark">PREVIEW</div>
      <div class="gauge-wrap">
        <svg class="gauge-svg" viewBox="0 0 120 120" width="160" height="160" aria-hidden="true">
          <circle cx="60" cy="60" r="50" fill="none" stroke="var(--claude-track)" stroke-width="10" />
          <path
            d={arcPath(displayInverse ? (sim?.remaining_pct ?? 0) : (sim?.suggested_pct ?? 0), 50, 60, 60)}
            fill="none"
            stroke="var(--claude-orange)"
            stroke-width="10"
            stroke-linecap="round"
          />
          <text x="60" y="56" text-anchor="middle" class="gauge-pct">{formatPct(shownPct)}%</text>
          <text x="60" y="72" text-anchor="middle" class="gauge-sub">{headlineLabel}</text>
        </svg>
      </div>
      {#if sim}
        <p class="preview-caption">
          By <strong>{sim.target_label}</strong>, you will be at
          <strong>{formatPct(sim.suggested_pct)}%</strong>
          (<strong>{formatPct(sim.remaining_pct)}%</strong> remaining).
        </p>
      {/if}
      {#if loading}
        <span class="loading-chip">calculating…</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .sim-view {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px 20px 32px;
  }

  .section-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--claude-text);
    margin-bottom: 6px;
  }

  .section-desc {
    font-size: 13px;
    color: var(--claude-text-muted);
    line-height: 1.5;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--claude-surface);
    border: 1.5px solid var(--claude-border);
    border-radius: var(--radius-md);
    padding: 14px 16px;
  }

  .control {
    display: grid;
    grid-template-columns: 130px 1fr 70px;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }

  .control-label {
    color: var(--claude-text-muted);
    font-weight: 600;
  }

  .control input[type="range"] {
    width: 100%;
    accent-color: var(--claude-orange);
  }

  .control input[type="datetime-local"] {
    grid-column: 2 / 4;
    background: var(--claude-surface);
    border: 1.5px solid var(--claude-border);
    border-radius: var(--radius-sm);
    color: var(--claude-text);
    padding: 6px 10px;
    font: inherit;
  }

  .control-value {
    text-align: right;
    font-weight: 600;
    color: var(--claude-orange-deep);
    font-variant-numeric: tabular-nums;
  }

  .preview-section {
    position: relative;
    background: var(--claude-surface);
    border: 1.5px solid var(--claude-border);
    border-radius: var(--radius-md);
    padding: 20px 16px 18px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    opacity: 0.92;
  }

  .preview-watermark {
    position: absolute;
    top: 10px;
    right: 14px;
    font-size: 10px;
    letter-spacing: 1.2px;
    color: var(--claude-text-muted);
    font-weight: 700;
    opacity: 0.6;
  }

  .gauge-wrap {
    border-radius: 50%;
    padding: 6px;
    background: repeating-linear-gradient(
      45deg,
      transparent 0 6px,
      rgba(0, 0, 0, 0.04) 6px 8px
    );
  }

  :global(.sim-view .gauge-pct) {
    font-size: 20px;
    font-weight: 700;
    fill: var(--claude-text);
  }
  :global(.sim-view .gauge-sub) {
    font-size: 9px;
    fill: var(--claude-text-muted);
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .preview-caption {
    font-size: 13px;
    color: var(--claude-text-muted);
    text-align: center;
    line-height: 1.5;
    max-width: 320px;
  }
  .preview-caption strong {
    color: var(--claude-orange-deep);
    font-weight: 600;
  }

  .loading-chip {
    font-size: 11px;
    color: var(--claude-text-muted);
    font-style: italic;
  }

  .loading {
    text-align: center;
    padding: 60px 0;
    color: var(--claude-text-muted);
    font-size: 14px;
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
  }
</style>
