import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, it, expect } from 'vitest'
import {
  ALL_THEMES,
  THEME_FAMILIES,
  THEME_MODES,
  type Theme,
  isTheme,
  resolveTheme,
  themeFamily,
  themeMode,
  themeVariant,
  themeWithMode,
} from '../theme'

describe('THEME_FAMILIES', () => {
  it('keeps the theme names users already have stored', () => {
    // Saved in user profiles and localStorage, and used as CSS classes:
    // renaming one breaks existing settings.
    expect([...ALL_THEMES].sort()).toEqual([
      'blue-dark', 'blue-light',
      'dark',
      'green-dark', 'green-light',
      'light',
      'orange-dark', 'orange-light',
      'rose-dark', 'rose-light',
      'yellow-dark', 'yellow-light',
    ])
  })

  it('has unique family ids', () => {
    const ids = THEME_FAMILIES.map((family) => family.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('puts each variant under the mode its name says', () => {
    for (const family of THEME_FAMILIES) {
      for (const mode of THEME_MODES) {
        const expected = family.id === 'default' ? mode : `${family.id}-${mode}`
        expect(family[mode].value).toBe(expected)
      }
    }
  })
})

describe('theme.css', () => {
  const css = readFileSync(resolve(__dirname, '../../theme.css'), 'utf8')
    .replace(/\/\*[\s\S]*?\*\//g, '')
  // Every class named in a selector (the text before each `{`), so selector
  // lists and selectors split across lines count too.
  const styledClasses = [...css.matchAll(/([^{}]*)\{/g)].flatMap(([, selector]) =>
    [...selector.matchAll(/\.([a-z][\w-]*)/g)].map((m) => m[1])
  )

  it('styles every theme', () => {
    // `light` is the :root default and has no class of its own.
    expect(css).toMatch(/^\s*:root\s*\{/m)
    const missing = ALL_THEMES.filter((theme) => theme !== 'light' && !styledClasses.includes(theme))
    expect(missing).toEqual([])
  })

  it('only styles known themes', () => {
    // Other classes may live in theme.css; only theme-shaped names must be known.
    const themeShaped = styledClasses.filter((name) => /^([a-z]+-)?(light|dark)$/.test(name))
    expect(themeShaped.filter((name) => !isTheme(name))).toEqual([])
  })
})

describe('isTheme', () => {
  it('accepts every known theme', () => {
    for (const theme of ALL_THEMES) expect(isTheme(theme)).toBe(true)
  })

  it('rejects anything else', () => {
    expect(isTheme('system')).toBe(false)
    expect(isTheme('purple-dark')).toBe(false)
    expect(isTheme('')).toBe(false)
    expect(isTheme(undefined)).toBe(false)
    expect(isTheme(42)).toBe(false)
  })
})

describe('themeVariant', () => {
  it('returns the table entry for a theme', () => {
    expect(themeVariant('rose-light')).toBe(THEME_FAMILIES.find((f) => f.id === 'rose')!.light)
    expect(themeVariant('dark').metaColor).toBe('#020817')
  })
})

describe('unknown theme values', () => {
  // Stale localStorage or an unchecked server value can reach the helpers.
  const unknown = 'purple-dark' as Theme

  it('fall back to the default family, keeping the mode', () => {
    expect(themeFamily(unknown)).toBe('default')
    expect(themeMode(unknown)).toBe('dark')
    expect(themeWithMode(unknown, 'light')).toBe('light')
    expect(themeVariant(unknown).value).toBe('dark')
  })
})

describe('themeFamily', () => {
  it('maps the default pair to "default"', () => {
    expect(themeFamily('light')).toBe('default')
    expect(themeFamily('dark')).toBe('default')
  })

  it('strips the mode suffix from colored themes', () => {
    expect(themeFamily('rose-light')).toBe('rose')
    expect(themeFamily('blue-dark')).toBe('blue')
  })
})

describe('themeMode', () => {
  it('detects light and dark variants', () => {
    expect(themeMode('light')).toBe('light')
    expect(themeMode('dark')).toBe('dark')
    expect(themeMode('green-light')).toBe('light')
    expect(themeMode('green-dark')).toBe('dark')
  })
})

describe('themeWithMode', () => {
  it('swaps the default pair', () => {
    expect(themeWithMode('light', 'dark')).toBe('dark')
    expect(themeWithMode('dark', 'light')).toBe('light')
  })

  it('keeps the color family', () => {
    expect(themeWithMode('rose-light', 'dark')).toBe('rose-dark')
    expect(themeWithMode('orange-dark', 'light')).toBe('orange-light')
    expect(themeWithMode('yellow-dark', 'dark')).toBe('yellow-dark')
  })

  it('always yields a known theme', () => {
    for (const theme of ALL_THEMES) {
      expect(ALL_THEMES).toContain(themeWithMode(theme, 'light'))
      expect(ALL_THEMES).toContain(themeWithMode(theme, 'dark'))
    }
  })
})

describe('resolveTheme', () => {
  it('returns the chosen theme unchanged when not following the system', () => {
    expect(resolveTheme('rose-light', false, true)).toBe('rose-light')
    expect(resolveTheme('dark', false, false)).toBe('dark')
  })

  it('follows a dark system preference', () => {
    expect(resolveTheme('rose-light', true, true)).toBe('rose-dark')
    expect(resolveTheme('blue-dark', true, true)).toBe('blue-dark')
    expect(resolveTheme('light', true, true)).toBe('dark')
  })

  it('follows a light system preference', () => {
    expect(resolveTheme('rose-dark', true, false)).toBe('rose-light')
    expect(resolveTheme('green-light', true, false)).toBe('green-light')
    expect(resolveTheme('dark', true, false)).toBe('light')
  })
})
