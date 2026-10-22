import { render, screen, within } from '@/test/test-utils'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { TagFilterPopover } from '../tag-filter-popover'
import { SearchHarness } from './search-context-harness'

vi.mock('@/hooks/use-available-tags', () => ({
    useAvailableTags: () => ({
        tags: ['work'],
        tagsCount: [{ tag: 'work', count: 3 }],
        isLoading: false,
        isError: false,
        error: null,
        refetch: () => { },
    }),
}))

/** The filter as the popover left it. */
const currentFilter = () =>
    JSON.parse(screen.getByTestId('filter').textContent || '{}')

/** The trigger is the only button until the popover opens. */
async function openPopover(user: ReturnType<typeof userEvent.setup>) {
    render(
        <SearchHarness>
            <TagFilterPopover />
        </SearchHarness>
    )
    await user.click(screen.getByRole('button'))
    return screen.getByRole('dialog')
}

describe('TagFilterPopover rows', () => {
    // A label click reaches the row twice — once by bubbling, once because the
    // browser forwards it to the checkbox — so each row has to swallow one of
    // them. Clicking the text must select, not toggle twice and cancel out.
    it('selecting "no tags" by clicking its text toggles it on and off', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user)

        await user.click(within(dialog).getByText('No tags'))
        expect(currentFilter()).toEqual({ has_tags: false })

        await user.click(within(dialog).getByText('No tags'))
        expect(currentFilter()).toEqual({})
    })

    it('selecting a tag by clicking its text toggles it on and off', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user)

        await user.click(within(dialog).getByText('work'))
        expect(currentFilter()).toEqual({ tags: ['work'] })

        await user.click(within(dialog).getByText('work'))
        expect(currentFilter()).toEqual({})
    })

    it('selecting a tag by clicking its checkbox toggles it once', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user)

        await user.click(within(dialog).getByRole('checkbox', { name: 'work' }))
        expect(currentFilter()).toEqual({ tags: ['work'] })
    })

    it('makes "no tags" and a tag selection replace each other', async () => {
        const user = userEvent.setup()
        const dialog = await openPopover(user)

        await user.click(within(dialog).getByText('work'))
        await user.click(within(dialog).getByText('No tags'))
        expect(currentFilter()).toEqual({ has_tags: false })

        await user.click(within(dialog).getByText('work'))
        expect(currentFilter()).toEqual({ tags: ['work'] })
    })
})
