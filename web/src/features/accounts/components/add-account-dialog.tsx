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
//

import * as React from 'react'
import { Mail, Database } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { useAccountContext } from '../context'

export type AddAccountType = 'IMAP' | 'NoSync'

interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

export function AddAccountDialog({
    open,
    onOpenChange
}: Props) {
    const { t } = useTranslation()
    const { setOpen } = useAccountContext()
    const [value, setValue] = React.useState<AddAccountType>('IMAP')

    function handleContinue() {
        if (value === 'IMAP') {
            setOpen('add-imap')
        } else {
            setOpen('add-nosync')
        }
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-xl">
                <DialogHeader className="text-left">
                    <DialogTitle>
                        {t('accounts.add')}
                    </DialogTitle>

                    <DialogDescription>
                        {t('accounts.selectAccountType')}
                    </DialogDescription>
                </DialogHeader>

                <RadioGroup
                    value={value}
                    onValueChange={(v) => setValue(v as AddAccountType)}
                    className="space-y-4 py-2"
                >
                    <Label
                        htmlFor="imap-account"
                        className={cn(
                            'flex cursor-pointer items-start gap-4 rounded-2xl border p-5 transition-all',
                            value === 'IMAP'
                                ? 'border-primary bg-muted/50'
                                : 'hover:bg-muted/30'
                        )}
                    >
                        <RadioGroupItem
                            value="IMAP"
                            id="imap-account"
                            className="mt-1"
                        />

                        <div className="flex flex-1 gap-4">
                            <div className="rounded-xl border p-2">
                                <Mail className="h-5 w-5" />
                            </div>

                            <div className="space-y-1">
                                <div className="font-medium">
                                    {t('accounts.imapAccount')}
                                </div>

                                <div className="text-sm text-muted-foreground">
                                    {t('accounts.imapAccountDescription')}
                                </div>
                            </div>
                        </div>
                    </Label>

                    <Label
                        htmlFor="nosync-account"
                        className={cn(
                            'flex cursor-pointer items-start gap-4 rounded-2xl border p-5 transition-all',
                            value === 'NoSync'
                                ? 'border-primary bg-muted/50'
                                : 'hover:bg-muted/30'
                        )}
                    >
                        <RadioGroupItem
                            value="NoSync"
                            id="nosync-account"
                            className="mt-1"
                        />

                        <div className="flex flex-1 gap-4">
                            <div className="rounded-xl border p-2">
                                <Database className="h-5 w-5" />
                            </div>

                            <div className="space-y-1">
                                <div className="font-medium">
                                    {t('accounts.noSyncAccount')}
                                </div>

                                <div className="text-sm text-muted-foreground">
                                    {t('accounts.noSyncAccountDescription')}
                                </div>
                            </div>
                        </div>
                    </Label>
                </RadioGroup>

                <DialogFooter>
                    <Button
                        variant="outline"
                        onClick={() => onOpenChange(false)}
                    >
                        {t('common.cancel')}
                    </Button>

                    <Button onClick={handleContinue}>
                        {t('accounts.continue')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}