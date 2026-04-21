export function arcPath(pct: number, r: number, cx: number, cy: number): string {
  const frac = Math.min(Math.max(pct / 100, 0), 1)
  if (frac >= 0.9999) {
    return `M ${cx} ${cy - r} A ${r} ${r} 0 1 1 ${cx - 0.001} ${cy - r} Z`
  }
  const angle = frac * 2 * Math.PI - Math.PI / 2
  const startAngle = -Math.PI / 2
  const x1 = cx + r * Math.cos(startAngle)
  const y1 = cy + r * Math.sin(startAngle)
  const x2 = cx + r * Math.cos(angle)
  const y2 = cy + r * Math.sin(angle)
  const largeArc = frac > 0.5 ? 1 : 0
  return `M ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2}`
}

export const WEEKDAY_NAMES = [
  'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday',
]

/** Short label for a day offset (0..6) relative to the reset weekday. */
export function weekdayAtOffset(resetWeekday: number, offset: number): string {
  const idx = ((resetWeekday + offset) % 7 + 7) % 7
  return WEEKDAY_NAMES[idx] ?? ''
}

export function formatPct(pct: number, digits = 1): string {
  return pct.toFixed(digits)
}
