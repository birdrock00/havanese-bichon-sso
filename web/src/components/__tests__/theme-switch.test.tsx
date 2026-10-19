import { render, screen, within } from '@/test/test-utils'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import { ThemeProvider, useTheme } from '@/context/theme-context'
import { ThemeSwitch } from '../theme-switch'

const THEME_KEY = 'vite-ui-theme'
const FOLLOW_KEY = 'vite-ui-theme-follow-system'

/** Installs a matchMedia stand-in whose dark-scheme answer the test controls. */
function fakeColorScheme(dark: boolean) {
  const media = {
    matches: dark,
    addEventListener: () => {},
    removeEventListener: () => {},
  }
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: () => media,
  })
}

/** Exposes the context so a click in the popover can be read back. */
function Probe() {
  const { theme, followSystem, resolvedTheme } = useTheme()
  return (
    <output data-testid='probe'>{`${theme}|${followSystem}|${resolvedTheme}`}</output>
  )
}

function renderSwitch() {
  render(
    <ThemeProvider>
      <ThemeSwitch />
      <Probe />
    </ThemeProvider>
  )
}

const probe = () => screen.getByTestId('probe').textContent

/** The trigger is the only button until the popover opens. */
async function openPopover(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button'))
  return screen.getByRole('dialog')
}

afterEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
  delete (window as { matchMedia?: unknown }).matchMedia
})

describe('ThemeSwitch with "use system theme" off', () => {
  it('offers a light column and a dark column', async () => {
    const user = userEvent.setup()
    renderSwitch()

    const dialog = await openPopover(user)

    expect(within(dialog).getByText('Light')).toBeInTheDocument()
    expect(within(dialog).getByText('Dark')).toBeInTheDocument()
    // Each family appears once per column.
    expect(within(dialog).getAllByText('Rose')).toHaveLength(2)
    expect(within(dialog).getByRole('switch')).not.toBeChecked()
  })

  it('picking from the dark column applies the dark variant', async () => {
    const user = userEvent.setup()
    renderSwitch()
    const dialog = await openPopover(user)

    await user.click(within(dialog).getAllByText('Rose')[1])

    expect(probe()).toBe('rose-dark|false|rose-dark')
    expect([...document.documentElement.classList]).toEqual(['rose-dark'])
  })
})

describe('ThemeSwitch with "use system theme" on', () => {
  it('offers one entry per family', async () => {
    localStorage.setItem(FOLLOW_KEY, 'true')
    fakeColorScheme(false)
    const user = userEvent.setup()
    renderSwitch()

    const dialog = await openPopover(user)

    expect(within(dialog).getByRole('switch')).toBeChecked()
    expect(within(dialog).queryByText('Light')).not.toBeInTheDocument()
    expect(within(dialog).queryByText('Dark')).not.toBeInTheDocument()
    expect(within(dialog).getAllByText('Rose')).toHaveLength(1)
  })

  it('picking a family keeps the variant the system is showing', async () => {
    localStorage.setItem(THEME_KEY, 'light')
    localStorage.setItem(FOLLOW_KEY, 'true')
    fakeColorScheme(true)
    const user = userEvent.setup()
    renderSwitch()
    expect(probe()).toBe('light|true|dark')

    const dialog = await openPopover(user)
    await user.click(within(dialog).getByText('Rose'))

    expect(probe()).toBe('rose-dark|true|rose-dark')
    expect([...document.documentElement.classList]).toEqual(['rose-dark'])
  })

  it('turning the toggle off keeps the variant on screen', async () => {
    localStorage.setItem(THEME_KEY, 'rose-light')
    localStorage.setItem(FOLLOW_KEY, 'true')
    fakeColorScheme(true)
    const user = userEvent.setup()
    renderSwitch()
    expect(probe()).toBe('rose-light|true|rose-dark')

    const dialog = await openPopover(user)
    await user.click(within(dialog).getByRole('switch'))

    expect(probe()).toBe('rose-dark|false|rose-dark')
    expect(within(dialog).getByText('Light')).toBeInTheDocument()
    expect(localStorage.getItem(FOLLOW_KEY)).toBe('false')
  })
})
