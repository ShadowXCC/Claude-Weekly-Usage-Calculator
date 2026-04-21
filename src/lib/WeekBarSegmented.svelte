<script lang="ts">
  import { weekdayAtOffset } from './gauge'

  let {
    fillPct,
    resetWeekday,
  }: {
    fillPct: number           // 0..100 — how much of the week is "filled"
    resetWeekday: number      // 0 = Monday … 6 = Sunday
  } = $props()

  const SEGMENTS = 7
  const SEG_PCT = 100 / SEGMENTS

  // Fractional segment position across the 7 days.
  let position = $derived(Math.max(0, Math.min(SEGMENTS, (fillPct / 100) * SEGMENTS)))
  let currentIndex = $derived(Math.min(SEGMENTS - 1, Math.floor(position)))
  let currentFraction = $derived(Math.max(0, Math.min(1, position - currentIndex)))
</script>

<div class="seg-bar" role="group" aria-label="Week progress, 7 day segments">
  {#each Array(SEGMENTS) as _, i}
    {@const state = i < currentIndex ? 'full' : i === currentIndex ? 'partial' : 'future'}
    {@const fillWidth = state === 'full' ? 100 : state === 'partial' ? currentFraction * 100 : 0}
    <div class="seg" class:current={state === 'partial'}>
      <div class="seg-track">
        <div class="seg-fill" style="width: {fillWidth}%"></div>
      </div>
      <div class="seg-label">{weekdayAtOffset(resetWeekday, i).slice(0, 3)}</div>
    </div>
  {/each}
</div>

<style>
  .seg-bar {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
  }

  .seg {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
  }

  .seg-track {
    position: relative;
    height: 18px;
    background: var(--claude-track);
    border-radius: 4px;
    overflow: hidden;
    transition: box-shadow 0.2s;
  }

  .seg.current .seg-track {
    box-shadow: 0 0 0 2px var(--claude-orange-deep);
  }

  .seg-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--claude-orange-deep), var(--claude-orange));
    transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .seg-label {
    font-size: 10px;
    text-align: center;
    color: var(--claude-text-muted);
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
  }

  .seg.current .seg-label {
    color: var(--claude-orange-deep);
  }
</style>
