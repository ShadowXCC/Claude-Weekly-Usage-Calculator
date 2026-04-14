<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import MainView from './lib/MainView.svelte'
  import SettingsView from './lib/SettingsView.svelte'

  const appWindow = getCurrentWindow()

  type Tab = 'main' | 'settings'
  let activeTab: Tab = $state('main')

  // ── Theme ───────────────────────────────────────────────────
  // Apply synchronously before first render to avoid flash
  const _storedTheme = (localStorage.getItem('claude-usage-theme') as 'light' | 'dark') ?? 'light'
  document.documentElement.setAttribute('data-theme', _storedTheme)

  let theme = $state<'light' | 'dark'>(_storedTheme)

  $effect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('claude-usage-theme', theme)
  })

  function setTheme(t: 'light' | 'dark') {
    theme = t
  }

  // ── Auto-refresh every 60s ──────────────────────────────────
  let refreshInterval: ReturnType<typeof setInterval> | undefined

  onMount(() => {
    refreshInterval = setInterval(() => {
      refreshTick += 1
    }, 60_000)
  })

  onDestroy(() => {
    if (refreshInterval !== undefined) clearInterval(refreshInterval)
  })

  let refreshTick: number = $state(0)

  async function openUsagePage() {
    await openUrl('https://claude.ai/settings/usage')
  }
</script>

<div class="shell">
  <!-- Custom title bar — drag region + window controls -->
  <div class="titlebar" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <div class="logo-mark" data-tauri-drag-region>U</div>
      <span class="app-title" data-tauri-drag-region>Claude Usage Tracker</span>
    </div>
    <div class="titlebar-controls">
      <button
        class="usage-link-btn"
        onclick={openUsagePage}
        title="Open Claude usage page"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
          <polyline points="15 3 21 3 21 9"/>
          <line x1="10" y1="14" x2="21" y2="3"/>
        </svg>
        Usage Page
      </button>
      <div class="wm-buttons">
        <button
          class="wm-btn wm-minimize"
          onclick={() => appWindow.hide()}
          title="Minimize to tray"
          aria-label="Minimize to tray"
        >
          <svg width="10" height="2" viewBox="0 0 10 2" fill="currentColor">
            <rect width="10" height="2" rx="1"/>
          </svg>
        </button>
        <button
          class="wm-btn wm-close"
          onclick={() => appWindow.hide()}
          title="Close to tray"
          aria-label="Close to tray"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round">
            <line x1="1" y1="1" x2="9" y2="9"/>
            <line x1="9" y1="1" x2="1" y2="9"/>
          </svg>
        </button>
      </div>
    </div>
  </div>

  <!-- Tab bar -->
  <nav class="tabs">
    <button
      class="tab-btn"
      class:active={activeTab === 'main'}
      onclick={() => activeTab = 'main'}
    >
      Weekly
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === 'settings'}
      onclick={() => activeTab = 'settings'}
    >
      Settings
    </button>
  </nav>

  <!-- Page content -->
  <main class="content">
    {#if activeTab === 'main'}
      <MainView {refreshTick} />
    {:else}
      <SettingsView onSaved={() => { activeTab = 'main'; refreshTick += 1 }} {theme} onThemeChange={setTheme} />
    {/if}
  </main>
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--claude-cream);
  }

  /* ── Custom title bar ────────────────────────────────── */
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 0 16px;
    height: 46px;
    background: var(--claude-surface);
    border-bottom: 1.5px solid var(--claude-border);
    flex-shrink: 0;
    cursor: default;
  }

  .titlebar-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-mark {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2.5px solid var(--claude-orange);
    color: var(--claude-orange);
    font-weight: 700;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    pointer-events: none;
  }

  .app-title {
    font-weight: 600;
    font-size: 14px;
    color: var(--claude-text);
    letter-spacing: -0.2px;
    pointer-events: none;
  }

  .titlebar-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .usage-link-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--claude-orange);
    font-size: 12px;
    font-weight: 500;
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    border: 1.5px solid var(--claude-border);
    transition: background 0.15s, border-color 0.15s;
  }

  .usage-link-btn:hover {
    background: var(--claude-orange-glow);
    border-color: var(--claude-orange);
  }

  /* Window control buttons (minimize / close) */
  .wm-buttons {
    display: flex;
    gap: 4px;
    margin-left: 4px;
  }

  .wm-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    color: var(--claude-text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s, color 0.12s;
    flex-shrink: 0;
  }

  .wm-btn:hover {
    background: var(--claude-cream-dark);
    color: var(--claude-text);
  }

  .wm-close:hover {
    background: var(--claude-close-hover-bg);
    color: var(--claude-close-hover-text);
  }

  /* ── Tabs ─────────────────────────────────────────────── */
  .tabs {
    display: flex;
    background: var(--claude-surface);
    border-bottom: 1.5px solid var(--claude-border);
    padding: 0 20px;
    flex-shrink: 0;
  }

  .tab-btn {
    background: transparent;
    color: var(--claude-text-muted);
    font-size: 13px;
    font-weight: 500;
    padding: 10px 16px;
    border-bottom: 2.5px solid transparent;
    margin-bottom: -1.5px;
    transition: color 0.15s, border-color 0.15s;
    border-radius: 0;
  }

  .tab-btn:hover {
    color: var(--claude-text);
  }

  .tab-btn.active {
    color: var(--claude-orange);
    border-bottom-color: var(--claude-orange);
  }

  /* ── Content ──────────────────────────────────────────── */
  .content {
    flex: 1;
    overflow-y: scroll;
    min-height: 0;
    scrollbar-width: thin;
    scrollbar-color: var(--claude-border) transparent;
  }
</style>
