//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

export const THEME_MODES = ['light', 'dark'] as const

export type ThemeMode = (typeof THEME_MODES)[number]

/** Preview colors for a theme picker; approximations of the theme's tokens in theme.css. */
export type ThemeSwatch = {
  background: string
  color: string
  muted: string
}

type ThemeFamilyDef = {
  id: string
  labelKey: string
  light: ThemeVariantDef
  dark: ThemeVariantDef
}

type ThemeVariantDef = ThemeSwatch & {
  /** The class applied to <html>; theme.css styles it (`light` is `:root`). */
  value: string
  /** Browser UI color (`<meta name="theme-color">`) while the theme is applied. */
  metaColor: string
}

/**
 * Every theme, grouped by color family. The single source for theme names,
 * labels and picker colors — adding a theme means one entry here plus its
 * CSS block in theme.css.
 */
export const THEME_FAMILIES = [
  {
    id: 'default',
    labelKey: 'theme.default',
    light: { value: 'light', background: '#ffffff', color: '#0f172a', muted: '#cbd5e1', metaColor: '#ffffff' },
    dark: { value: 'dark', background: '#020817', color: '#94a3b8', muted: '#334155', metaColor: '#020817' },
  },
  {
    id: 'rose',
    labelKey: 'theme.rose',
    light: { value: 'rose-light', background: '#fff1f2', color: '#e11d48', muted: '#fda4af', metaColor: '#fb7185' },
    dark: { value: 'rose-dark', background: '#1f1115', color: '#e11d48', muted: '#881337', metaColor: '#881337' },
  },
  {
    id: 'orange',
    labelKey: 'theme.orange',
    light: { value: 'orange-light', background: '#fff7ed', color: '#ea580c', muted: '#fdba74', metaColor: '#fb923c' },
    dark: { value: 'orange-dark', background: '#1c120d', color: '#f97316', muted: '#9a3412', metaColor: '#9a3412' },
  },
  {
    id: 'green',
    labelKey: 'theme.green',
    light: { value: 'green-light', background: '#f0fdf4', color: '#16a34a', muted: '#86efac', metaColor: '#4ade80' },
    dark: { value: 'green-dark', background: '#0d1b12', color: '#22c55e', muted: '#166534', metaColor: '#166534' },
  },
  {
    id: 'yellow',
    labelKey: 'theme.yellow',
    light: { value: 'yellow-light', background: '#fefce8', color: '#ca8a04', muted: '#fde047', metaColor: '#facc15' },
    dark: { value: 'yellow-dark', background: '#1c1608', color: '#facc15', muted: '#a16207', metaColor: '#a16207' },
  },
  {
    id: 'blue',
    labelKey: 'theme.blue',
    light: { value: 'blue-light', background: '#eff6ff', color: '#2563eb', muted: '#bfdbfe', metaColor: '#60a5fa' },
    dark: { value: 'blue-dark', background: '#172554', color: '#60a5fa', muted: '#1e3a8a', metaColor: '#1e40af' },
  },
] as const satisfies readonly ThemeFamilyDef[]

type ThemeFamily = (typeof THEME_FAMILIES)[number]

type ThemeFamilyId = ThemeFamily['id']

type ThemeVariant = ThemeFamily[ThemeMode]

export type Theme = ThemeVariant['value']

export const ALL_THEMES: readonly Theme[] = THEME_FAMILIES.flatMap((family) =>
  THEME_MODES.map((mode) => family[mode].value)
)

const THEME_INDEX = new Map<string, { family: ThemeFamily; mode: ThemeMode }>(
  THEME_FAMILIES.flatMap((family) =>
    THEME_MODES.map((mode) => [family[mode].value, { family, mode }] as const)
  )
)

/**
 * Where a theme sits in THEME_FAMILIES. Values that only claim to be a Theme
 * (stale localStorage, an unchecked server value) fall back to the default
 * family, keeping the mode their name suggests.
 */
function locate(theme: Theme): { family: ThemeFamily; mode: ThemeMode } {
  const found = THEME_INDEX.get(theme)
  if (found) return found
  return { family: THEME_FAMILIES[0], mode: String(theme).endsWith('dark') ? 'dark' : 'light' }
}

export function isTheme(value: unknown): value is Theme {
  return typeof value === 'string' && THEME_INDEX.has(value)
}

/** Color family of a theme: 'light'/'dark' → 'default', 'rose-light' → 'rose'. */
export function themeFamily(theme: Theme): ThemeFamilyId {
  return locate(theme).family.id
}

export function themeMode(theme: Theme): ThemeMode {
  return locate(theme).mode
}

/** The table entry for a theme: its picker colors and meta color. */
export function themeVariant(theme: Theme): ThemeVariant {
  const { family, mode } = locate(theme)
  return family[mode]
}

/** The light or dark variant of the same color family. */
export function themeWithMode(theme: Theme, mode: ThemeMode): Theme {
  return locate(theme).family[mode].value
}

/**
 * The theme to apply: the user's choice as-is, or — when following the
 * system — its family in the light/dark variant the client prefers.
 */
export function resolveTheme(theme: Theme, followSystem: boolean, prefersDark: boolean): Theme {
  if (!followSystem) return theme
  return themeWithMode(theme, prefersDark ? 'dark' : 'light')
}
