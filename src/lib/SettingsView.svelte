<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'

  const WEEKDAYS = [
    { value: 0, label: 'Monday' },
    { value: 1, label: 'Tuesday' },
    { value: 2, label: 'Wednesday' },
    { value: 3, label: 'Thursday' },
    { value: 4, label: 'Friday' },
    { value: 5, label: 'Saturday' },
    { value: 6, label: 'Sunday' },
  ]

  let { onSaved, theme, onThemeChange }: {
    onSaved: () => void
    theme: 'light' | 'dark'
    onThemeChange: (t: 'light' | 'dark') => void
  } = $props()

  let weekday: number = $state(3)
  let hour: number = $state(19)
  let minute: number = $state(0)
  let runOnStartup: boolean = $state(true)
  let saving: boolean = $state(false)
  let saveError: string | null = $state(null)
  let saved: boolean = $state(false)

  // Hour display helpers
  function hourOptions() {
    return Array.from({ length: 24 }, (_, i) => ({
      value: i,
      label: formatHour(i),
    }))
  }

  function minuteOptions() {
    // Offer every minute for accuracy but show 5-minute increments + current value
    return Array.from({ length: 60 }, (_, i) => ({
      value: i,
      label: i.toString().padStart(2, '0'),
    }))
  }

  function formatHour(h: number): string {
    const ampm = h >= 12 ? 'PM' : 'AM'
    const h12 = h % 12 === 0 ? 12 : h % 12
    return `${h12}:00 ${ampm}`
  }

  onMount(async () => {
    // Load current config from backend
    try {
      const info = await invoke<{
        suggested_pct: number
        time_until_reset: string
        reset_weekday: number
        reset_hour: number
        reset_minute: number
      }>('get_usage_info')
      weekday = info.reset_weekday
      hour = info.reset_hour
      minute = info.reset_minute
    } catch {}

    try {
      runOnStartup = await invoke<boolean>('get_startup_enabled')
    } catch {}
  })

  async function save() {
    saving = true
    saveError = null
    saved = false
    try {
      await invoke('save_config', {
        weekday,
        hour,
        minute,
        runOnStartup,
      })
      saved = true
      setTimeout(() => {
        saved = false
        onSaved()
      }, 800)
    } catch (e) {
      saveError = String(e)
    } finally {
      saving = false
    }
  }

  // Preview of reset time label
  let resetPreview = $derived(() => {
    const day = WEEKDAYS.find(d => d.value === weekday)?.label ?? 'Unknown'
    const ampm = hour >= 12 ? 'PM' : 'AM'
    const h12 = hour % 12 === 0 ? 12 : hour % 12
    const mStr = minute.toString().padStart(2, '0')
    return `${day}s at ${h12}:${mStr} ${ampm}`
  })
</script>

<div class="settings-view">
  <div class="section">
    <h2 class="section-title">Reset Schedule</h2>
    <p class="section-desc">
      Set the day and time your Claude weekly usage resets. This is shown on your
      <a href="https://claude.ai/settings/usage" target="_blank" rel="noopener noreferrer">Claude usage page</a>.
    </p>

    <div class="field-group">
      <label class="field-label" for="reset-day">Reset day</label>
      <select id="reset-day" bind:value={weekday}>
        {#each WEEKDAYS as day}
          <option value={day.value}>{day.label}</option>
        {/each}
      </select>
    </div>

    <div class="field-row">
      <div class="field-group">
        <label class="field-label" for="reset-hour">Hour</label>
        <select id="reset-hour" bind:value={hour}>
          {#each hourOptions() as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div class="field-group">
        <label class="field-label" for="reset-minute">Minute</label>
        <select id="reset-minute" bind:value={minute}>
          {#each minuteOptions() as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="preview-chip">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/>
        <polyline points="12 6 12 12 16 14"/>
      </svg>
      Resets: {resetPreview()}
    </div>
  </div>

  <div class="divider"></div>

  <div class="section">
    <h2 class="section-title">Startup</h2>
    <label class="toggle-row">
      <input type="checkbox" bind:checked={runOnStartup} />
      <div class="toggle-text">
        <span class="toggle-title">Launch at login</span>
        <span class="toggle-desc">Start Claude Usage Tracker automatically when you log in</span>
      </div>
    </label>
  </div>

  <div class="divider"></div>

  <div class="section">
    <h2 class="section-title">Appearance</h2>
    <div class="theme-toggle">
      <button
        class="theme-btn"
        class:active={theme === 'light'}
        onclick={() => onThemeChange('light')}
        title="Light mode"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="5"/>
          <line x1="12" y1="1" x2="12" y2="3"/>
          <line x1="12" y1="21" x2="12" y2="23"/>
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
          <line x1="1" y1="12" x2="3" y2="12"/>
          <line x1="21" y1="12" x2="23" y2="12"/>
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
        </svg>
        Light
      </button>
      <button
        class="theme-btn"
        class:active={theme === 'dark'}
        onclick={() => onThemeChange('dark')}
        title="Dark mode"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
        Dark
      </button>
    </div>
  </div>

  <div class="divider"></div>

  <!-- Save footer -->
  <div class="save-footer">
    {#if saveError}
      <p class="save-error">{saveError}</p>
    {/if}
    <button
      class="save-btn"
      class:success={saved}
      disabled={saving || saved}
      onclick={save}
    >
      {#if saving}
        Saving…
      {:else if saved}
        ✓ Saved
      {:else}
        Save Settings
      {/if}
    </button>
  </div>
</div>

<style>
  .settings-view {
    display: flex;
    flex-direction: column;
    padding: 0 0 24px;
  }

  /* ── Sections ──────────────────────────────────────────── */
  .section {
    padding: 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--claude-text);
    letter-spacing: -0.1px;
  }

  .section-desc {
    font-size: 13px;
    color: var(--claude-text-muted);
    line-height: 1.6;
  }

  .divider {
    height: 1.5px;
    background: var(--claude-border);
    flex-shrink: 0;
  }

  /* ── Fields ────────────────────────────────────────────── */
  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--claude-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  select {
    width: 100%;
  }

  /* ── Preview chip ──────────────────────────────────────── */
  .preview-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--claude-orange-glow);
    color: var(--claude-orange-deep);
    border: 1.5px solid rgba(218, 119, 86, 0.3);
    border-radius: 20px;
    padding: 5px 12px;
    font-size: 12px;
    font-weight: 600;
    width: fit-content;
  }

  /* ── Toggle row ────────────────────────────────────────── */
  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    cursor: pointer;
  }

  .toggle-row input {
    margin-top: 2px;
    flex-shrink: 0;
  }

  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .toggle-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--claude-text);
  }

  .toggle-desc {
    font-size: 12px;
    color: var(--claude-text-muted);
    line-height: 1.5;
  }

  /* ── Save footer ───────────────────────────────────────── */
  .save-footer {
    padding: 0 20px;
    margin-top: 4px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .save-error {
    font-size: 12px;
    color: var(--claude-error-text);
    background: var(--claude-error-bg);
    border: 1px solid var(--claude-error-border);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
  }

  .save-btn {
    width: 100%;
    padding: 11px;
    background: var(--claude-orange);
    color: #fff;
    border-radius: var(--radius-sm);
    font-weight: 600;
    font-size: 14px;
    letter-spacing: 0.1px;
    transition: background 0.15s, transform 0.1s;
  }

  .save-btn:hover:not(:disabled) {
    background: var(--claude-orange-deep);
  }

  .save-btn:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .save-btn.success {
    background: var(--claude-success);
  }

  /* ── Theme toggle ──────────────────────────────────────────── */
  .theme-toggle {
    display: flex;
    gap: 0;
    background: var(--claude-track);
    border-radius: var(--radius-sm);
    padding: 3px;
    width: fit-content;
  }

  .theme-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 16px;
    border-radius: calc(var(--radius-sm) - 2px);
    font-size: 13px;
    font-weight: 500;
    color: var(--claude-text-muted);
    background: transparent;
    transition: background 0.15s, color 0.15s, box-shadow 0.15s;
  }

  .theme-btn:hover:not(.active) {
    color: var(--claude-text);
  }

  .theme-btn.active {
    background: var(--claude-surface);
    color: var(--claude-text);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
  }
</style>
