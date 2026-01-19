import * as React from 'react'
import { AtSign, ChevronDown, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'

import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { cn } from '@/lib/utils'
import { useSearchContext } from './context'

export function AccountPopover() {
  const { t } = useTranslation()
  const { filter, setFilter } = useSearchContext()
  const [search, setSearch] = React.useState('')
  const { minimalList = [] } = useMinimalAccountList()

  const selectedIds: number[] = filter.account_ids ?? []

  const toggleAccount = (id: number) => {
    setFilter(prev => {
      const next = { ...prev }
      const set = new Set<number>(next.account_ids ?? [])

      if (set.has(id)) {
        set.delete(id)
      } else {
        set.add(id)
      }

      if (set.size === 0) {
        delete next.account_ids
        delete next.mailbox_ids
      } else {
        next.account_ids = Array.from(set).sort()
        delete next.mailbox_ids
      }

      return next
    })
  }

  const clearAccounts = () => {
    setFilter(prev => {
      const next = { ...prev }
      delete next.account_ids
      delete next.mailbox_ids
      return next
    })
  }

  const filtered = React.useMemo(() => {
    const q = search.toLowerCase()

    return minimalList
      .filter(a =>
        !q ||
        a.email.toLowerCase().includes(q) ||
        String(a.id).includes(q)
      )
      .sort((a, b) => {
        const aSel = selectedIds.includes(a.id)
        const bSel = selectedIds.includes(b.id)

        if (aSel && !bSel) return -1
        if (!aSel && bSel) return 1
        return a.id - b.id
      })
  }, [minimalList, search, selectedIds])

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          size="sm"
          variant="outline"
          className={cn(
            'h-8 gap-1.5 px-3 rounded-none',
            selectedIds.length > 0 &&
            'bg-primary/10 border-primary text-primary'
          )}
        >
          <AtSign className="h-4 w-4" />
          {t('search_accounts.label')}
          {selectedIds.length > 0 && (
            <Badge
              variant="secondary"
              className="ml-1 h-5 px-1.5 text-xs"
            >
              {selectedIds.length}
            </Badge>
          )}
          <ChevronDown className="h-3.5 w-3.5 opacity-60" />
        </Button>
      </PopoverTrigger>

      <PopoverContent align="start" className="w-96 p-1">
        <div className="p-1 pb-2">
          <Input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('search_accounts.search_placeholder')}
            className="h-8 text-sm"
          />
        </div>
        {!search && selectedIds.length > 0 && (
          <div className="p-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={clearAccounts}
              className="flex h-8 w-full items-center justify-start gap-2 px-2 text-xs font-medium text-destructive hover:bg-destructive/10 hover:text-destructive transition-colors"
            >
              <div className="flex h-4 w-4 items-center justify-center">
                <X className="h-3.5 w-3.5" />
              </div>
              <span className="flex-1 text-left">
                {t('search_accounts.clear_accounts')}
              </span>
              <span className="text-[10px] opacity-60 font-mono">
                ({selectedIds.length})
              </span>
            </Button>
            <div className="my-1 h-px bg-border/60" />
          </div>
        )}
        <ScrollArea className="h-96 p-1">
          {filtered.length === 0 ? (
            <p className="px-3 py-2 text-xs text-muted-foreground">
              {t('search_accounts.no_accounts_found')}
            </p>
          ) : (
            filtered.map(account => {
              const checked = selectedIds.includes(account.id)
              const id = `account-${account.id}`

              return (
                <div
                  key={account.id}
                  onClick={() => toggleAccount(account.id)}
                  className={cn(
                    'flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer',
                    'hover:bg-accent transition-colors'
                  )}
                >
                  <Checkbox
                    id={id}
                    checked={checked}
                    onCheckedChange={() =>
                      toggleAccount(account.id)
                    }
                    onClick={e => e.stopPropagation()}
                  />

                  <Label
                    htmlFor={id}
                    className="flex-1 truncate text-xs cursor-pointer"
                  >
                    <div className="flex items-center gap-2">
                      <span className="truncate">
                        {account.email}
                      </span>
                      <span className="text-[10px] text-muted-foreground">
                        #{account.id}
                      </span>
                    </div>
                  </Label>
                </div>
              )
            })
          )}
        </ScrollArea>
      </PopoverContent>
    </Popover>
  )
}