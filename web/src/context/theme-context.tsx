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


import { createContext, useContext, useEffect, useState } from 'react'
import { ALL_THEMES, isTheme, resolveTheme, Theme } from '@/lib/theme'

export type { Theme } from '@/lib/theme'

type ThemeProviderProps = {
  children: React.ReactNode
  defaultTheme?: Theme
  storageKey?: string
}

type ThemeProviderState = {
  /** The theme the user picked (what gets saved to the server). */
  theme: Theme
  setTheme: (theme: Theme) => void
  /** Pick the light/dark variant of `theme` from the client's color scheme. */
  followSystem: boolean
  setFollowSystem: (followSystem: boolean) => void
  /** The theme actually applied to the document. */
  resolvedTheme: Theme
}

const initialState: ThemeProviderState = {
  theme: 'dark',
  setTheme: () => null,
  followSystem: false,
  setFollowSystem: () => null,
  resolvedTheme: 'dark',
}

const ThemeProviderContext = createContext<ThemeProviderState>(initialState)

const DARK_SCHEME_QUERY = '(prefers-color-scheme: dark)'

function systemPrefersDark() {
  return typeof window.matchMedia === 'function' && window.matchMedia(DARK_SCHEME_QUERY).matches
}

export function ThemeProvider({
  children,
  defaultTheme = 'light',
  storageKey = 'vite-ui-theme',
  ...props
}: ThemeProviderProps) {
  const followSystemKey = `${storageKey}-follow-system`
  const [theme, setTheme] = useState<Theme>(() => {
    // Ignore stale or hand-edited values; they'd end up as a class on <html>.
    const stored = localStorage.getItem(storageKey)
    return isTheme(stored) ? stored : defaultTheme
  })
  const [followSystem, setFollowSystemState] = useState<boolean>(
    () => localStorage.getItem(followSystemKey) === 'true'
  )
  const [prefersDark, setPrefersDark] = useState<boolean>(systemPrefersDark)

  useEffect(() => {
    if (!followSystem || typeof window.matchMedia !== 'function') return
    const media = window.matchMedia(DARK_SCHEME_QUERY)
    const onChange = (e: MediaQueryListEvent) => setPrefersDark(e.matches)
    setPrefersDark(media.matches)
    media.addEventListener('change', onChange)
    return () => media.removeEventListener('change', onChange)
  }, [followSystem])

  const resolvedTheme = resolveTheme(theme, followSystem, prefersDark)

  const setFollowSystem = (follow: boolean) => {
    if (follow) {
      // Read the scheme now. The listener that keeps `prefersDark` fresh only
      // runs while following, so a change made before the toggle was turned on
      // would otherwise show the wrong variant for a frame.
      setPrefersDark(systemPrefersDark())
    } else {
      // Turning it off keeps what's on screen: the shown variant becomes the choice.
      setTheme(resolvedTheme)
    }
    setFollowSystemState(follow)
  }

  useEffect(() => {
    const root = window.document.documentElement
    root.classList.remove(...ALL_THEMES)
    root.classList.add(resolvedTheme)
  }, [resolvedTheme])

  useEffect(() => {
    localStorage.setItem(storageKey, theme)
  }, [theme, storageKey])

  useEffect(() => {
    localStorage.setItem(followSystemKey, String(followSystem))
  }, [followSystem, followSystemKey])

  const value = {
    theme,
    setTheme,
    followSystem,
    setFollowSystem,
    resolvedTheme,
  }

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  )
}

export const useTheme = () => {
  const context = useContext(ThemeProviderContext)

  if (context === undefined)
    throw new Error('useTheme must be used within a ThemeProvider')

  return context
}