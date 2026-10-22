import { render, screen, within } from '@/test/test-utils'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { MoreFiltersPopover } from '../more-filters-popover'
import { SearchHarness } from './search-context-harness'

vi.mock('@/hooks/use-attachment-metadata', () => ({
    useAttachmentMetadata: () => ({ data: undefined, isLoading: false }),
}))

const ENVELOPE_ID = '550e8400-e29b-41d4-a716-446655440000'

/** The filter as the popover left it. */
const currentFilter = () =>
    JSON.parse(screen.getByTestId('filter').textContent || '{}')

/** The trigger is the only button until the popover opens. */
async function openPopover(
    user: ReturnType<typeof userEvent.setup>,
    initial: Record<string, unknown> = {}
) {
    render(
        <SearchHarness initial={initial}>
            <MoreFiltersPopover />
        </SearchHarness>
    )
    await user.click(screen.getByRole('button'))
    return screen.getByRole('dialog')
}

describe('MoreFiltersPopover envelope id', () => {
    it('searches on the id it was given', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user)

        await user.type(
            within(dialog).getByPlaceholderText(ENVELOPE_ID),
            ENVELOPE_ID
        )
        await user.click(within(dialog).getByRole('button', { name: 'Apply filters' }))

        expect(currentFilter()).toEqual({ id: ENVELOPE_ID })
    })

    // An id copied out of the detail view or a log line usually carries
    // whitespace; the backend matches the value verbatim, so the popover has to
    // be the layer that trims it.
    it('trims the id before searching on it', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user)

        await user.type(
            within(dialog).getByPlaceholderText(ENVELOPE_ID),
            `  ${ENVELOPE_ID}  `
        )
        await user.click(within(dialog).getByRole('button', { name: 'Apply filters' }))

        expect(currentFilter()).toEqual({ id: ENVELOPE_ID })
    })

    it('drops the id when the field is emptied', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user, { id: ENVELOPE_ID })

        await user.clear(within(dialog).getByPlaceholderText(ENVELOPE_ID))
        await user.click(within(dialog).getByRole('button', { name: 'Apply filters' }))

        expect(currentFilter()).toEqual({})
    })

    it('shows the id as an active filter and clears it on reset', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user, { id: ENVELOPE_ID, message_id: '<a@b>' })

        // The trigger badge counts the id alongside the fields that were
        // already there, and the popover offers a reset because of it.
        const trigger = screen.getByRole('button', { name: /Advanced/ })
        expect(within(trigger).getByText('2')).toBeInTheDocument()

        await user.click(within(dialog).getByRole('button', { name: 'Reset' }))

        expect(currentFilter()).toEqual({})
    })
})
