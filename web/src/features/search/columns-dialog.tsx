//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

import { useTranslation } from 'react-i18next'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { SquarePen } from 'lucide-react'
import { Button } from '@/components/button'
import { Checkbox } from '@/components/ui/checkbox'
import { useState } from 'react'
import { useSearchContext } from './context'
import { Label } from '@/components/ui/label'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

const defaultColumns = (t: (key: string) => string) => [
    {
        label: t('search.account'),
        value: "account_email"
    },
    {
        label: t('search.mailbox'),
        value: "mailbox_name"
    },
    {
        label: t('search.from'),
        value: "from"
    },
    {
        label: t('search.to'),
        value: "to"
    },
    {
        label: t('search.subject'),
        value: "subject"
    },
    {
        label: t('mail.attachments'),
        value: "attachments"
    },
    {
        label: t('search.size'),
        value: "size"
    },
    {
        label: t('search.date'),
        value: "date"
    },
]

export function ColumnsDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const { setColumnVisibility } = useSearchContext()
  const columns = defaultColumns(t)

  const [selected, setSelected] = useState(() => {
    const _columns = localStorage.getItem("searchTableColumns")
      ? JSON.parse(localStorage.getItem("searchTableColumns") as string) as Record<string, boolean>
      : undefined

      if (_columns) return new Map(Object.entries(_columns).map(([key, value]) => [key, value]))
      return new Map(columns.map((col) => [col.value, true]))
  })

  const handleSave = () => {
    const _selected = Object.fromEntries(selected)
    setColumnVisibility(_selected)
    localStorage.setItem("searchTableColumns", JSON.stringify(_selected))
    onOpenChange(false)
  }

  const toggleSelected = (column: string) => {
    setSelected(prev => {
      const value = new Map(prev)

      if (value.get(column)) {
        value.set(column, false)
      } else {
        value.set(column, true)
      }

      return value
    })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <SquarePen className="h-5 w-5" />
              {t('common.columns')}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-5 py-4">
          <div className="flex flex-wrap flex-col gap-2">
            {columns.map(col => (
              <div key={col.value} className="flex flex-row gap-2 items-center">
                <Checkbox
                  id={col.value}
                  checked={selected.get(col.value)}
                  onCheckedChange={() => toggleSelected(col.value)}
                />
                <Label htmlFor={col.value} className="cursor-pointer text-sm font-normal">
                  {col.label}
                </Label>
              </div>
            ))}
          </div>
        </div>

        <div className="flex justify-end items-center">
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              {t('search.addTags.cancel')}
            </Button>
            <Button onClick={handleSave}>
              {t('search.addTags.save')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
