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


import { useEffect, useState } from 'react'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { useTranslation } from 'react-i18next'
import { toast } from '@/hooks/use-toast'
import { start_account_download, AccountModel } from '@/api/account/api'

interface Props {
  row: AccountModel
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function StartDownloadDialog({ row, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const [runGapFill, setRunGapFill] = useState(false)
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    if (open) {
      setRunGapFill(false)
      setSubmitting(false)
    }
  }, [open])

  const handleConfirm = async () => {
    setSubmitting(true)
    try {
      await start_account_download(row.id, runGapFill)
      toast({ title: t('accounts.downloadStarted') })
      onOpenChange(false)
    } catch (error: any) {
      toast({
        variant: 'destructive',
        title: t('accounts.downloadFailed'),
        description: error.response?.data?.message || error.message,
      })
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('accounts.startDownload')}</DialogTitle>
          <DialogDescription>
            {t('accounts.startDownloadConfirmDesc')}
          </DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-2 py-2">
          <Checkbox id="run-gap-fill" checked={runGapFill} onCheckedChange={(v) => setRunGapFill(!!v)} />
          <Label htmlFor="run-gap-fill">{t('accounts.runGapFill')}</Label>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={submitting}>
            {t('common.cancel')}
          </Button>
          <Button onClick={handleConfirm} disabled={submitting}>
            {t('accounts.startDownload')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
