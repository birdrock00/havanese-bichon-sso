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
import { useState } from 'react'
import { useFormContext, useWatch, type Path } from 'react-hook-form'
import { HelpCircle, Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { PatternEntry } from '@/lib/pattern-utils'
import { newPatternId, simplePatternToRegex } from '@/lib/pattern-utils'
import { Button } from '@/components/ui/button'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { PatternInput } from './pattern-input'
import type { AccountFormValues } from './schema'

function toPatternEntries(patterns: string[]): PatternEntry[] {
  return patterns.map((p) => ({
    id: newPatternId(),
    matchType: 'regex' as const,
    value: p,
  }))
}

function getIn(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, key) => {
    if (acc == null || typeof acc !== 'object') return undefined
    return (acc as Record<string, unknown>)[key]
  }, obj)
}

function patternsToRegexList(entries: PatternEntry[]): string[] {
  // Empty entries are written as '' so the form schema can flag them as
  // required instead of silently dropping them from the payload.
  return entries.map((e) =>
    e.value.trim() === '' ? '' : simplePatternToRegex(e.matchType, e.value)
  )
}

interface FilterRuleEditorProps {
  /** Form path, e.g. "extraction_rules.senders". */
  path: string
  title: string
  help?: string
}

export function FilterRuleEditor({ path, title, help }: FilterRuleEditorProps) {
  const { t } = useTranslation()
  const { control, setValue, formState } = useFormContext<AccountFormValues>()
  const rule = useWatch({ control, name: path as Path<AccountFormValues> }) as
    | { include?: string[]; exclude?: string[] }
    | undefined

  const include = rule?.include ?? []
  const exclude = rule?.exclude ?? []

  const [includeEntries, setIncludeEntries] = useState<PatternEntry[]>(() =>
    toPatternEntries(include)
  )
  const [excludeEntries, setExcludeEntries] = useState<PatternEntry[]>(() =>
    toPatternEntries(exclude)
  )

  // Resync local state when form values change externally (e.g. after form.reset)
  const [lastSyncKey, setLastSyncKey] = useState<string>('')
  const syncKey = JSON.stringify({ include, exclude })
  if (syncKey !== lastSyncKey) {
    setLastSyncKey(syncKey)
    setIncludeEntries(toPatternEntries(include))
    setExcludeEntries(toPatternEntries(exclude))
  }

  const syncToForm = (
    nextInclude: PatternEntry[],
    nextExclude: PatternEntry[]
  ) => {
    setValue(
      `${path}.include` as Path<AccountFormValues>,
      patternsToRegexList(nextInclude),
      { shouldValidate: true, shouldDirty: true }
    )
    setValue(
      `${path}.exclude` as Path<AccountFormValues>,
      patternsToRegexList(nextExclude),
      { shouldValidate: true, shouldDirty: true }
    )
  }

  const includeErrors = (getIn(formState.errors, `${path}.include`) ??
    []) as Array<{ message?: string } | undefined>
  const excludeErrors = (getIn(formState.errors, `${path}.exclude`) ??
    []) as Array<{ message?: string } | undefined>

  const addEntry = (side: 'include' | 'exclude') => {
    const newEntry: PatternEntry = {
      id: newPatternId(),
      matchType: 'contains',
      value: '',
    }
    const nextInclude =
      side === 'include' ? [...includeEntries, newEntry] : includeEntries
    const nextExclude =
      side === 'exclude' ? [...excludeEntries, newEntry] : excludeEntries
    setIncludeEntries(nextInclude)
    setExcludeEntries(nextExclude)
    syncToForm(nextInclude, nextExclude)
  }

  const updateEntry = (
    id: string,
    partial: Partial<PatternEntry>,
    side: 'include' | 'exclude'
  ) => {
    if (side === 'include') {
      const updated = includeEntries.map((e) =>
        e.id === id ? { ...e, ...partial } : e
      )
      setIncludeEntries(updated)
      syncToForm(updated, excludeEntries)
    } else {
      const updated = excludeEntries.map((e) =>
        e.id === id ? { ...e, ...partial } : e
      )
      setExcludeEntries(updated)
      syncToForm(includeEntries, updated)
    }
  }

  const removeEntry = (id: string, side: 'include' | 'exclude') => {
    if (side === 'include') {
      const filtered = includeEntries.filter((e) => e.id !== id)
      setIncludeEntries(filtered)
      syncToForm(filtered, excludeEntries)
    } else {
      const filtered = excludeEntries.filter((e) => e.id !== id)
      setExcludeEntries(filtered)
      syncToForm(includeEntries, filtered)
    }
  }

  const renderRow = (
    entry: PatternEntry,
    side: 'include' | 'exclude',
    index: number
  ) => {
    const error =
      side === 'include'
        ? includeErrors[index]?.message
        : excludeErrors[index]?.message
    return (
      <PatternInput
        key={entry.id}
        entry={entry}
        error={error}
        onChange={(id, partial) => updateEntry(id, partial, side)}
        onRemove={(id) => removeEntry(id, side)}
      />
    )
  }

  const renderBlock = (
    side: 'include' | 'exclude',
    entries: PatternEntry[]
  ) => (
    <div>
      <p className='text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider'>
        {side === 'include'
          ? t('accounts.filters.include')
          : t('accounts.filters.exclude')}
      </p>
      {entries.length === 0 ? (
        <p className='text-xs text-muted-foreground italic'>
          {side === 'include'
            ? t('accounts.filters.noIncludePatterns')
            : t('accounts.filters.noExcludePatterns')}
        </p>
      ) : (
        <div className='space-y-2'>
          {entries.map((e, index) => renderRow(e, side, index))}
        </div>
      )}
      <Button
        variant='ghost'
        type='button'
        size='sm'
        className='mt-2 h-8 text-xs'
        onClick={() => addEntry(side)}
      >
        <Plus className='h-3 w-3 mr-1' />
        {t('accounts.filters.addPattern')}
      </Button>
    </div>
  )

  return (
    <div className='space-y-4 rounded-md border p-5'>
      <div className='flex items-center gap-2'>
        <h4 className='text-sm font-semibold'>{title}</h4>
        {help && (
          <Tooltip>
            <TooltipTrigger asChild>
              <HelpCircle className='h-3.5 w-3.5 text-muted-foreground' />
            </TooltipTrigger>
            <TooltipContent>{help}</TooltipContent>
          </Tooltip>
        )}
      </div>
      <div className='space-y-3'>
        {renderBlock('include', includeEntries)}
        {renderBlock('exclude', excludeEntries)}
      </div>
    </div>
  )
}
