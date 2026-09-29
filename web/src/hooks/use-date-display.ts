import React from 'react'

export type DateDisplayMode = 'relative' | 'absolute'

const STORAGE_KEY = 'bichon.dateDisplay'

/**
 * Shared date-column display preference for the search and attachment list
 * views. Persisted in localStorage so the choice survives reloads and is
 * applied consistently across both views.
 */
export function useDateDisplay() {
  const [dateDisplay, setDateDisplay] = React.useState<DateDisplayMode>(() => {
    if (typeof window !== 'undefined') {
      const saved = window.localStorage.getItem(STORAGE_KEY)
      if (saved === 'relative' || saved === 'absolute') return saved
    }
    return 'relative'
  })

  const handleSetDateDisplay = React.useCallback((mode: DateDisplayMode) => {
    setDateDisplay(mode)
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(STORAGE_KEY, mode)
    }
  }, [])

  return { dateDisplay, setDateDisplay: handleSetDateDisplay }
}
