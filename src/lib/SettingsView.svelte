<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { DEFAULT_ACCENT } from './accent'

  type ThemeMode = 'light' | 'dark' | 'auto'
  type Skin =
    | 'default'
    | 'crt'
    | 'lcd'
    | 'mac'
    | 'solarized-dark'
    | 'solarized-light'
    | 'nord'
    | 'dracula'
    | 'catppuccin-mocha'
    | 'catppuccin-latte'
    | 'github-light'
    | 'sepia'
    | 'rose-pine-dawn'
  type TrayMode = 'ring' | 'number' | 'emoji'

  const WEEKDAYS = [
    { value: 0, label: 'Monday' },
    { value: 1, label: 'Tuesday' },
    { value: 2, label: 'Wednesday' },
    { value: 3, label: 'Thursday' },
    { value: 4, label: 'Friday' },
    { value: 5, label: 'Saturday' },
    { value: 6, label: 'Sunday' },
  ]

  const SKIN_OPTIONS: { value: Skin; label: string }[] = [
    { value: 'default', label: 'Default' },
    // Retro
    { value: 'crt', label: 'CRT Green' },
    { value: 'lcd', label: 'LCD Seven-Seg' },
    { value: 'mac', label: 'Early Mac' },
    // Modern dark
    { value: 'solarized-dark', label: 'Solarized Dark' },
    { value: 'nord', label: 'Nord' },
    { value: 'dracula', label: 'Dracula' },
    { value: 'catppuccin-mocha', label: 'Catppuccin Mocha' },
    // Modern light
    { value: 'solarized-light', label: 'Solarized Light' },
    { value: 'catppuccin-latte', label: 'Catppuccin Latte' },
    { value: 'github-light', label: 'GitHub Light' },
    { value: 'sepia', label: 'Sepia' },
    { value: 'rose-pine-dawn', label: 'Rose Pine Dawn' },
  ]

  let {
    onSaved,
    themeMode,
    resolvedTheme,
    onThemeModeChange,
    skin,
    onSkinChange,
    displayInverse,
    trayMode,
    accentHex,
    a11yAnnounce,
    weekBarSegmented,
    onWeekBarSegmentedChange,
    onDisplayPrefsChanged,
  }: {
    onSaved: () => void
    themeMode: ThemeMode
    resolvedTheme: 'light' | 'dark'
    onThemeModeChange: (t: ThemeMode) => void
    skin: Skin
    onSkinChange: (s: Skin) => void
    displayInverse: boolean
    trayMode: TrayMode
    accentHex: string
    a11yAnnounce: boolean
    weekBarSegmented: boolean
    onWeekBarSegmentedChange: (v: boolean) => void
    onDisplayPrefsChanged: (p: {
      displayInverse: boolean
      trayMode: TrayMode
      accentHex: string
      a11yAnnounce: boolean
    }) => void
  } = $props()

  // ── Reset schedule / sleep / tz (saved via explicit Save Settings button) ─
  let weekday: number = $state(3)
  let hour: number = $state(19)
  let minute: number = $state(0)
  let runOnStartup: boolean = $state(true)
  let sleepStartHour: number = $state(23)
  let sleepStartMinute: number = $state(0)
  let sleepEndHour: number = $state(7)
  let sleepEndMinute: number = $state(0)
  let sleepAdjustmentEnabled: boolean = $state(false)
  let timezone: string = $state('auto')
  let tzSearch: string = $state('')
  let timezones: string[] = $state([])

  let saving: boolean = $state(false)
  let saveError: string | null = $state(null)
  let saved: boolean = $state(false)

  // ── Display prefs (live, auto-saved with debounce) ──────────────
  // Initialize from props; the $effect mirrors below keep them synced on prop changes.
  // svelte-ignore state_referenced_locally
  let liveInverse = $state(displayInverse)
  // svelte-ignore state_referenced_locally
  let liveTrayMode = $state<TrayMode>(trayMode)
  // svelte-ignore state_referenced_locally
  let liveAccent = $state(accentHex)
  // svelte-ignore state_referenced_locally
  let liveA11y = $state(a11yAnnounce)

  // Keep local mirrors in sync if parent changes them (e.g. after background refresh).
  // `untrack` on the write side stops the compiler warning about the state-write
  // being read by the same effect — we only want the prop reads to be tracked.
  $effect(() => {
    const v = displayInverse
    untrack(() => { liveInverse = v })
  })
  $effect(() => {
    const v = trayMode
    untrack(() => { liveTrayMode = v })
  })
  $effect(() => {
    const v = accentHex
    untrack(() => { liveAccent = v })
  })
  $effect(() => {
    const v = a11yAnnounce
    untrack(() => { liveA11y = v })
  })

  function hourOptions() {
    return Array.from({ length: 24 }, (_, i) => ({ value: i, label: formatHour(i) }))
  }
  function minuteOptions() {
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
    try {
      const info = await invoke<{
        reset_weekday: number
        reset_hour: number
        reset_minute: number
        sleep_start_hour: number
        sleep_start_minute: number
        sleep_end_hour: number
        sleep_end_minute: number
        sleep_adjustment_enabled: boolean
        tz: string
      }>('get_usage_info')
      weekday = info.reset_weekday
      hour = info.reset_hour
      minute = info.reset_minute
      sleepStartHour = info.sleep_start_hour
      sleepStartMinute = info.sleep_start_minute
      sleepEndHour = info.sleep_end_hour
      sleepEndMinute = info.sleep_end_minute
      sleepAdjustmentEnabled = info.sleep_adjustment_enabled
      timezone = info.tz || 'auto'
    } catch {}

    try {
      runOnStartup = await invoke<boolean>('get_startup_enabled')
    } catch {}

    try {
      timezones = await invoke<string[]>('get_timezones')
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
        sleepStartHour,
        sleepStartMinute,
        sleepEndHour,
        sleepEndMinute,
        sleepAdjustmentEnabled,
        timezone: timezone === 'auto' ? null : timezone,
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

  // ── Debounced auto-save for display prefs ──────────────────
  let prefDebounce: ReturnType<typeof setTimeout> | undefined
  let prefError: string | null = $state(null)

  function schedulePrefSave() {
    if (prefDebounce) clearTimeout(prefDebounce)
    prefDebounce = setTimeout(async () => {
      try {
        await invoke('update_display_prefs', {
          prefs: {
            display_inverse: liveInverse,
            tray_mode: liveTrayMode,
            accent_hex: liveAccent,
            a11y_announce_thresholds: liveA11y,
          },
        })
        prefError = null
        onDisplayPrefsChanged({
          displayInverse: liveInverse,
          trayMode: liveTrayMode,
          accentHex: liveAccent,
          a11yAnnounce: liveA11y,
        })
      } catch (e) {
        prefError = String(e)
      }
    }, 500)
  }

  function onInverseToggle(e: Event) {
    liveInverse = (e.target as HTMLInputElement).checked
    schedulePrefSave()
  }
  function onA11yToggle(e: Event) {
    liveA11y = (e.target as HTMLInputElement).checked
    schedulePrefSave()
  }
  function onTrayModeChange(mode: TrayMode) {
    liveTrayMode = mode
    schedulePrefSave()
  }
  function onAccentInput(e: Event) {
    liveAccent = (e.target as HTMLInputElement).value
    schedulePrefSave()
  }
  function resetAccent() {
    liveAccent = DEFAULT_ACCENT
    schedulePrefSave()
  }

  function onSegmentedToggle(e: Event) {
    onWeekBarSegmentedChange((e.target as HTMLInputElement).checked)
  }

  // Filtered tz list (debounced implicitly via $derived over tzSearch).
  let filteredTimezones = $derived(
    tzSearch.trim() === ''
      ? timezones
      : timezones.filter((z) =>
          z.toLowerCase().includes(tzSearch.toLowerCase()),
        ),
  )

  // Reset time preview
  let resetPreview = $derived(() => {
    const day = WEEKDAYS.find(d => d.value === weekday)?.label ?? 'Unknown'
    const ampm = hour >= 12 ? 'PM' : 'AM'
    const h12 = hour % 12 === 0 ? 12 : hour % 12
    const mStr = minute.toString().padStart(2, '0')
    return `${day}s at ${h12}:${mStr} ${ampm}`
  })

  let dailySleepHours = $derived(() => {
    const startMin = sleepStartHour * 60 + sleepStartMinute
    const endMin = sleepEndHour * 60 + sleepEndMinute
    if (startMin === endMin) return 0
    const diffMin = endMin > startMin ? endMin - startMin : endMin + 24 * 60 - startMin
    return diffMin / 60
  })
</script>

<div class="settings-view">
  <!-- ── Display ────────────────────────────────────────────── -->
  <div class="section">
    <h2 class="section-title">Display</h2>
    <p class="section-desc">
      Choose how the gauge and tray icon present your weekly budget. Changes here
      save automatically.
    </p>

    <label class="toggle-row">
      <input type="checkbox" checked={liveInverse} onchange={onInverseToggle} />
      <div class="toggle-text">
        <span class="toggle-title">Show budget remaining</span>
        <span class="toggle-desc">
          Flip the gauge, tray icon, and CLI to show what's left instead of what's used.
        </span>
      </div>
    </label>

    <label class="toggle-row">
      <input
        type="checkbox"
        checked={weekBarSegmented}
        onchange={onSegmentedToggle}
      />
      <div class="toggle-text">
        <span class="toggle-title">Seven-segment week bar</span>
        <span class="toggle-desc">
          Split the weekly progress bar into 7 day-sized blocks.
        </span>
      </div>
    </label>

    <label class="toggle-row">
      <input type="checkbox" checked={liveA11y} onchange={onA11yToggle} />
      <div class="toggle-text">
        <span class="toggle-title">Announce threshold crossings</span>
        <span class="toggle-desc">
          Screen-reader announcement at 50%, 75%, 90%, and 100%.
        </span>
      </div>
    </label>

    <div class="field-group">
      <span class="field-label">Tray icon mode</span>
      <div class="radio-row" role="radiogroup" aria-label="Tray icon mode">
        {#each [{ v: 'ring', l: 'Ring', d: 'Circular gauge' }, { v: 'number', l: 'Number', d: 'Two-digit %' }, { v: 'emoji', l: 'Emoji', d: '🟢🟡🔴 dot' }] as opt}
          <button
            type="button"
            class="radio-card"
            class:active={liveTrayMode === opt.v}
            role="radio"
            aria-checked={liveTrayMode === opt.v}
            onclick={() => onTrayModeChange(opt.v as TrayMode)}
          >
            <span class="radio-title">{opt.l}</span>
            <span class="radio-desc">{opt.d}</span>
          </button>
        {/each}
      </div>
    </div>

    {#if prefError}
      <p class="save-error inline">{prefError}</p>
    {/if}
  </div>

  <div class="divider"></div>

  <!-- ── Appearance ─────────────────────────────────────────── -->
  <div class="section">
    <h2 class="section-title">Appearance</h2>

    <div class="field-group">
      <span class="field-label">Theme</span>
      <div class="theme-toggle">
        <button
          class="theme-btn"
          class:active={themeMode === 'light'}
          onclick={() => onThemeModeChange('light')}
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
          class:active={themeMode === 'dark'}
          onclick={() => onThemeModeChange('dark')}
          title="Dark mode"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
          </svg>
          Dark
        </button>
        <button
          class="theme-btn auto"
          class:active={themeMode === 'auto'}
          onclick={() => onThemeModeChange('auto')}
          title="Follow system"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="4" width="18" height="12" rx="2"/>
            <line x1="8" y1="20" x2="16" y2="20"/>
            <line x1="12" y1="16" x2="12" y2="20"/>
          </svg>
          <span>Auto <span class="sublabel">({resolvedTheme})</span></span>
        </button>
      </div>
    </div>

    <div class="field-group">
      <span class="field-label">Accent color</span>
      <div class="accent-row">
        <input
          type="color"
          class="color-picker"
          value={liveAccent}
          oninput={onAccentInput}
          aria-label="Accent color"
        />
        <span class="accent-hex">{liveAccent.toUpperCase()}</span>
        <button type="button" class="reset-chip" onclick={resetAccent}>
          Reset to default
        </button>
      </div>
    </div>

    <div class="field-group">
      <label class="field-label" for="skin-select">Skin</label>
      <select
        id="skin-select"
        value={skin}
        onchange={(e) => onSkinChange((e.target as HTMLSelectElement).value as Skin)}
      >
        {#each SKIN_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="divider"></div>

  <!-- ── Reset schedule ─────────────────────────────────────── -->
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

  <!-- ── Timezone ───────────────────────────────────────────── -->
  <div class="section">
    <h2 class="section-title">Timezone</h2>
    <p class="section-desc">
      Reset time is anchored to this timezone. Set to Auto to follow the system clock,
      or pin a specific zone so the gauge doesn't jump when traveling.
    </p>

    <div class="field-group">
      <label class="field-label" for="tz-search">Search zone</label>
      <input
        id="tz-search"
        type="text"
        placeholder="e.g. New_York, Berlin, Tokyo"
        bind:value={tzSearch}
        autocomplete="off"
      />
    </div>

    <div class="field-group">
      <label class="field-label" for="tz-select">Zone</label>
      <select id="tz-select" bind:value={timezone} size={6}>
        <option value="auto">Auto (follow system)</option>
        {#each filteredTimezones as z}
          <option value={z}>{z}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="divider"></div>

  <!-- ── Startup ────────────────────────────────────────────── -->
  <div class="section">
    <h2 class="section-title">Startup</h2>
    <label class="toggle-row">
      <input type="checkbox" bind:checked={runOnStartup} />
      <div class="toggle-text">
        <span class="toggle-title">Launch at login</span>
        <span class="toggle-desc">Start Claude Weekly Usage Calculator automatically when you log in</span>
      </div>
    </label>
  </div>

  <div class="divider"></div>

  <!-- ── Sleep Adjustment ───────────────────────────────────── -->
  <div class="section">
    <h2 class="section-title">Sleep Adjustment</h2>
    <p class="section-desc">
      Pause the suggested usage % during your sleep hours. The gauge ticks only while you're
      awake and still reaches 100% at the weekly reset.
    </p>
    <label class="toggle-row">
      <input type="checkbox" bind:checked={sleepAdjustmentEnabled} />
      <div class="toggle-text">
        <span class="toggle-title">Enable sleep adjustment</span>
        <span class="toggle-desc">Hold the gauge steady during your nightly sleep window</span>
      </div>
    </label>

    <div class="field-row" class:disabled={!sleepAdjustmentEnabled}>
      <div class="field-group">
        <label class="field-label" for="sleep-start-hour">Sleep start</label>
        <div class="time-row">
          <select id="sleep-start-hour" bind:value={sleepStartHour} disabled={!sleepAdjustmentEnabled}>
            {#each hourOptions() as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
          <select bind:value={sleepStartMinute} disabled={!sleepAdjustmentEnabled} aria-label="Sleep start minute">
            {#each minuteOptions() as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>
      <div class="field-group">
        <label class="field-label" for="sleep-end-hour">Sleep end</label>
        <div class="time-row">
          <select id="sleep-end-hour" bind:value={sleepEndHour} disabled={!sleepAdjustmentEnabled}>
            {#each hourOptions() as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
          <select bind:value={sleepEndMinute} disabled={!sleepAdjustmentEnabled} aria-label="Sleep end minute">
            {#each minuteOptions() as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>

    <span class="sleep-hint" class:disabled={!sleepAdjustmentEnabled}>
      {#if sleepAdjustmentEnabled}
        {dailySleepHours().toFixed(1)}h sleep/day · {((24 - dailySleepHours()) * 7).toFixed(1)} active hrs/week
      {:else}
        168 active hrs/week
      {/if}
    </span>
  </div>

  <div class="divider"></div>

  <!-- Save footer: schedule / tz / sleep / startup -->
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
    <p class="footer-note">
      Display options above save automatically. This button saves reset schedule,
      timezone, startup, and sleep-adjustment.
    </p>
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

  .save-error.inline {
    margin-top: 4px;
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

  .footer-note {
    font-size: 11px;
    color: var(--claude-text-muted);
    line-height: 1.5;
    text-align: center;
  }

  /* ── Sleep adjustment ─────────────────────────────────────── */
  .time-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .sleep-hint {
    font-size: 12px;
    color: var(--claude-text-muted);
    font-weight: 500;
  }

  .sleep-hint.disabled {
    opacity: 0.45;
  }

  .field-row.disabled .field-label {
    opacity: 0.45;
  }

  /* ── Theme toggle ──────────────────────────────────────────── */
  .theme-toggle {
    display: flex;
    gap: 0;
    background: var(--claude-track);
    border-radius: var(--radius-sm);
    padding: 3px;
    width: fit-content;
    flex-wrap: wrap;
  }

  .theme-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
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

  .theme-btn .sublabel {
    font-weight: 400;
    opacity: 0.7;
    font-size: 11px;
  }

  /* ── Accent picker ─────────────────────────────────────────── */
  .accent-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .color-picker {
    width: 44px;
    height: 34px;
    padding: 0;
    border: 1.5px solid var(--claude-border);
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }

  .accent-hex {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    color: var(--claude-text);
    font-weight: 600;
    letter-spacing: 0.5px;
  }

  .reset-chip {
    margin-left: auto;
    font-size: 12px;
    font-weight: 500;
    color: var(--claude-text-muted);
    background: transparent;
    border: 1.5px solid var(--claude-border);
    border-radius: 16px;
    padding: 4px 12px;
    transition: border-color 0.15s, color 0.15s;
  }

  .reset-chip:hover {
    border-color: var(--claude-orange);
    color: var(--claude-orange);
  }

  /* ── Tray mode radio group ─────────────────────────────────── */
  .radio-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .radio-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 10px 12px;
    background: var(--claude-surface);
    border: 1.5px solid var(--claude-border);
    border-radius: var(--radius-sm);
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }

  .radio-card:hover {
    border-color: var(--claude-orange);
  }

  .radio-card.active {
    border-color: var(--claude-orange);
    background: var(--claude-orange-glow);
  }

  .radio-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--claude-text);
  }

  .radio-desc {
    font-size: 11px;
    color: var(--claude-text-muted);
  }
</style>
