import { render, screen, waitFor, within } from '@/test/test-utils'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { themeVariant } from '@/lib/theme'
import { ThemeProvider } from '@/context/theme-context'
import { ThemeSwitch } from '@/components/theme-switch'
import { AppearanceForm } from '../appearance-form'

const { updateUser } = vi.hoisted(() => ({ updateUser: vi.fn() }))

vi.mock('@/hooks/use-current-user', () => ({
  useCurrentUser: () => ({ data: { id: 7 }, isLoading: false }),
}))

vi.mock('@/api/users/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api/users/api')>()
  return { ...actual, update_user: updateUser }
})

/** #rrggbb as jsdom reports it back from `style.backgroundColor`. */
function rgb(hex: string) {
  const n = Number.parseInt(hex.slice(1), 16)
  return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`
}

function renderSettings() {
  render(
    <ThemeProvider>
      <ThemeSwitch />
      <AppearanceForm />
    </ThemeProvider>
  )
}

/** The theme cards are the children of the page's only radiogroup. */
function themeCards() {
  return [...screen.getByRole('radiogroup').children] as HTMLElement[]
}

/** The card showing the tick, i.e. the one the settings page calls selected. */
function selectedCard() {
  const ticked = themeCards().filter((card) => card.querySelector('svg'))
  expect(ticked).toHaveLength(1)
  return ticked[0]
}

/** The color the card's miniature window is painted in. */
function previewBackground(card: HTMLElement) {
  return card.querySelector<HTMLElement>('div[style]')?.style.backgroundColor
}

/** The trigger is the only round icon button in the header. */
async function openHeaderPopover(user: ReturnType<typeof userEvent.setup>) {
  const trigger = document.querySelector<HTMLElement>('button.rounded-full')
  if (!trigger) throw new Error('theme switch trigger not found')
  await user.click(trigger)
  return screen.getByRole('dialog')
}

beforeEach(() => {
  updateUser.mockReset()
  updateUser.mockResolvedValue({})
})

afterEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
  delete (window as { matchMedia?: unknown }).matchMedia
})

describe('AppearanceForm theme field', () => {
  it('saves the theme picked in the header', async () => {
    const user = userEvent.setup()
    renderSettings()

    const popover = await openHeaderPopover(user)
    await user.click(within(popover).getAllByText('Rose')[0])

    // The card for the picked theme is the one marked selected.
    expect(previewBackground(selectedCard())).toBe(
      rgb(themeVariant('rose-light').background)
    )

    await user.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(updateUser).toHaveBeenCalled())
    expect(updateUser).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ theme: 'rose-light' })
    )
  })

  it('keeps the header pick when only the language is saved', async () => {
    const user = userEvent.setup()
    renderSettings()

    const popover = await openHeaderPopover(user)
    await user.click(within(popover).getAllByText('Blue')[1])

    await user.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(updateUser).toHaveBeenCalled())
    expect(updateUser).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ theme: 'blue-dark' })
    )
  })
})

describe('AppearanceForm "use system theme" toggle', () => {
  it('shows one card per family, matching the header', async () => {
    const user = userEvent.setup()
    renderSettings()
    expect(themeCards()).toHaveLength(12)

    const popover = await openHeaderPopover(user)
    await user.click(within(popover).getByRole('switch'))

    expect(themeCards()).toHaveLength(6)
    expect(document.getElementById('appearance-follow-system')).toBeChecked()
    // The header drops its light/dark columns at the same time.
    expect(within(popover).queryByText('Light')).not.toBeInTheDocument()
    expect(within(popover).getAllByText('Rose')).toHaveLength(1)
  })

  it('picking a family saves the variant the system is showing', async () => {
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      writable: true,
      value: () => ({
        matches: true,
        addEventListener: () => {},
        removeEventListener: () => {},
      }),
    })
    const user = userEvent.setup()
    renderSettings()

    await user.click(document.getElementById('appearance-follow-system')!)
    await user.click(screen.getByText('Rose'))

    await user.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(updateUser).toHaveBeenCalled())
    expect(updateUser).toHaveBeenCalledWith(
      7,
      expect.objectContaining({ theme: 'rose-dark' })
    )
  })
})
