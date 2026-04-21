export const DEFAULT_ACCENT = '#DA7756'

function hexToRgb(hex: string): [number, number, number] | null {
  const s = hex.replace('#', '').trim()
  if (s.length !== 6 || !/^[0-9a-fA-F]+$/.test(s)) return null
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)]
}

function rgbToHex(r: number, g: number, b: number): string {
  const c = (n: number) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, '0')
  return `#${c(r)}${c(g)}${c(b)}`
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255; g /= 255; b /= 255
  const max = Math.max(r, g, b), min = Math.min(r, g, b)
  let h = 0
  const l = (max + min) / 2
  const s = max === min ? 0 : l > 0.5 ? (max - min) / (2 - max - min) : (max - min) / (max + min)
  if (max !== min) {
    if (max === r) h = ((g - b) / (max - min)) + (g < b ? 6 : 0)
    else if (max === g) h = ((b - r) / (max - min)) + 2
    else h = ((r - g) / (max - min)) + 4
    h /= 6
  }
  return [h * 360, s * 100, l * 100]
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  h /= 360; s /= 100; l /= 100
  if (s === 0) return [l * 255, l * 255, l * 255]
  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1
    if (t > 1) t -= 1
    if (t < 1 / 6) return p + (q - p) * 6 * t
    if (t < 1 / 2) return q
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6
    return p
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const p = 2 * l - q
  return [hue2rgb(p, q, h + 1 / 3) * 255, hue2rgb(p, q, h) * 255, hue2rgb(p, q, h - 1 / 3) * 255]
}

export function deriveAccentShades(hex: string): { base: string; deep: string; glow: string } {
  const rgb = hexToRgb(hex) ?? hexToRgb(DEFAULT_ACCENT)!
  const [h, s, l] = rgbToHsl(rgb[0], rgb[1], rgb[2])
  const [dr, dg, db] = hslToRgb(h, s, Math.max(0, l - 12))
  const deep = rgbToHex(dr, dg, db)
  const [r, g, b] = rgb
  const glow = `rgba(${r}, ${g}, ${b}, 0.20)`
  return { base: rgbToHex(r, g, b), deep, glow }
}

export function applyAccent(hex: string) {
  const { base, deep, glow } = deriveAccentShades(hex)
  const root = document.documentElement
  root.style.setProperty('--claude-orange', base)
  root.style.setProperty('--claude-orange-deep', deep)
  root.style.setProperty('--claude-orange-glow', glow)
}
