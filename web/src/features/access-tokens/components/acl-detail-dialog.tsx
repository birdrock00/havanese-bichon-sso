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


import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { AccessToken } from '../data/schema'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow: AccessToken
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AclDetailDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        onOpenChange(state)
      }}
    >
      <DialogContent className='sm:max-w-xl'>
        <DialogHeader className='text-left'>
          <DialogTitle>{t('settings.acl')}</DialogTitle>
          <DialogDescription>
            {t('accessTokens.aclRulesForAccessTokens')}
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className='h-[33rem] w-full pr-4 -mr-4 py-1'>
          <div className="space-y-4">
            {/* IP Whitelist */}
            <div className="grid w-full items-center">
              <Label className="mb-2">IP Whitelist</Label>
              <Textarea
                className="col-span-5 max-h-[240px] min-h-[300px]"
                value={currentRow.acl?.ip_whitelist?.join('\n')}
              />
            </div>

            {/* Quota */}
            <div className="grid w-full items-center">
              <Label className="mb-2">Quota</Label>
              <Input
                type="number"
                value={currentRow.acl?.rate_limit?.quota}
                className="col-span-5"
              />
            </div>

            {/* Interval (seconds) */}
            <div className="grid w-full items-center">
              <Label className="mb-2">Interval (seconds)</Label>
              <Input
                type="number"
                className="col-span-5"
                value={currentRow.acl?.rate_limit?.interval}
              />
            </div>
          </div>
        </ScrollArea>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant='outline' className="px-2 py-1 text-sm h-auto">Close</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
