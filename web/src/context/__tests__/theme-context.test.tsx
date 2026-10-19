import { act, render } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { ThemeProvider, useTheme } from '../theme-context'

const THEME_KEY = 'vite-ui-theme'
const FOLLOW_KEY = 'vite-ui-theme-follow-system'

type SchemeListener = (e: MediaQueryListEvent) => void

/** Installs a matchMedia stand-in whose dark-scheme answer the test controls. */
function fakeColorScheme(dark: boolean) {
  const listeners = new Set<SchemeListener>()
  const media = {
    get matches() {
      return dark
    },
    addEventListener: (_: string, listener: SchemeListener) =>
      listeners.add(listener),
    removeEventListener: (_: string, listener: SchemeListener) =>
      listeners.delete(listener),
  }
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: () => media,
  })
  return {
    listeners,
    setDark(next: boolean) {
      dark = next
      act(() => {
        for (const listener of [...listeners])
          listener({ matches: next } as MediaQueryListEvent)
      })
    },
  }
}

function renderProvider() {
  let context!: ReturnType<typeof useTheme>
  function Probe() {
    context = useTheme()
    return null
  }
  const view = render(
    <ThemeProvider>
      <Probe />
    </ThemeProvider>
  )
  return { ...view, context: () => context }
}

const htmlClasses = () => [...document.documentElement.classList]

afterEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
  delete (window as { matchMedia?: unknown }).matchMedia
})

describe('ThemeProvider stored theme', () => {
  it('restores a known theme', () => {
    localStorage.setItem(THEME_KEY, 'rose-dark')
    const { context } = renderProvider()

    expect(context().theme).toBe('rose-dark')
    expect(htmlClasses()).toEqual(['rose-dark'])
  })

  it('ignores an unknown theme instead of applying it as a class', () => {
    localStorage.setItem(THEME_KEY, 'purple-dark')
    const { context } = renderProvider()

    expect(context().theme).toBe('light')
    expect(htmlClasses()).toEqual(['light'])
    expect(localStorage.getItem(THEME_KEY)).toBe('light')
  })
})

describe('ThemeProvider turning "use system theme" off', () => {
  it('keeps the variant the system picked', () => {
    localStorage.setItem(THEME_KEY, 'light')
    localStorage.setItem(FOLLOW_KEY, 'true')
    fakeColorScheme(true)
    const { context } = renderProvider()
    expect(htmlClasses()).toEqual(['dark'])

    act(() => context().setFollowSystem(false))

    expect(context().followSystem).toBe(false)
    expect(context().theme).toBe('dark')
    expect(htmlClasses()).toEqual(['dark'])
    expect(localStorage.getItem(THEME_KEY)).toBe('dark')
    expect(localStorage.getItem(FOLLOW_KEY)).toBe('false')
  })

  it('keeps the color family', () => {
    localStorage.setItem(THEME_KEY, 'rose-light')
    localStorage.setItem(FOLLOW_KEY, 'true')
    fakeColorScheme(true)
    const { context } = renderProvider()

    act(() => context().setFollowSystem(false))

    expect(context().theme).toBe('rose-dark')
    expect(htmlClasses()).toEqual(['rose-dark'])
  })
})

describe('ThemeProvider following the system color scheme', () => {
  it('switches variant when the OS setting changes', () => {
    localStorage.setItem(THEME_KEY, 'rose-light')
    localStorage.setItem(FOLLOW_KEY, 'true')
    const scheme = fakeColorScheme(false)
    const { context } = renderProvider()
    expect(htmlClasses()).toEqual(['rose-light'])

    scheme.setDark(true)

    expect(htmlClasses()).toEqual(['rose-dark'])
    expect(context().resolvedTheme).toBe('rose-dark')
    // Following only changes what's applied, not the saved choice.
    expect(context().theme).toBe('rose-light')
  })

  it('ignores the OS setting while the toggle is off', () => {
    localStorage.setItem(THEME_KEY, 'blue-light')
    const scheme = fakeColorScheme(true)
    renderProvider()

    expect(htmlClasses()).toEqual(['blue-light'])
    expect(scheme.listeners.size).toBe(0)
  })

  it('stops listening when the toggle is turned off', () => {
    localStorage.setItem(FOLLOW_KEY, 'true')
    const scheme = fakeColorScheme(false)
    const { context } = renderProvider()
    expect(scheme.listeners.size).toBe(1)

    act(() => context().setFollowSystem(false))

    expect(scheme.listeners.size).toBe(0)
  })

  it('stops listening on unmount', () => {
    localStorage.setItem(FOLLOW_KEY, 'true')
    const scheme = fakeColorScheme(false)
    const { unmount } = renderProvider()
    expect(scheme.listeners.size).toBe(1)

    unmount()

    expect(scheme.listeners.size).toBe(0)
  })
})
