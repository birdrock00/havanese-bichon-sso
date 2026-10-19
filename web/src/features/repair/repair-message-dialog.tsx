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
import { useMutation } from '@tanstack/react-query'
import { AxiosError } from 'axios'
import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { repairMessage, type BlobRepairReport } from '@/api/repair/api'
import { Button } from '@/components/ui/button'
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog'
import { ToastAction } from '@/components/ui/toast'
import { toast } from '@/hooks/use-toast'
import { RepairReportView } from './repair-report-view'

interface RepairMessageDialogProps {
    open: boolean
    onOpenChange: (open: boolean) => void
    accountId?: number
    envelopeId?: string
    /** Displayed before the repair runs. The report carries its own subject afterwards. */
    subject?: string
}

/**
 * Confirms and runs a repair for one message, then shows the report in place.
 *
 * Deliberately not a `ConfirmDialog`: that wraps Radix `AlertDialog`, which
 * renders `desc` through `AlertDialogDescription asChild` (so the report would
 * inherit descriptive styling), traps focus under `role="alertdialog"`, and has
 * no scroll container for the hash lists.
 *
 * The dialog stays open on success — the report *is* the result, so closing
 * would throw away the answer.
 */
export function RepairMessageDialog({
    open,
    onOpenChange,
    accountId,
    envelopeId,
    subject,
}: RepairMessageDialogProps) {
    const { t } = useTranslation()
    const [report, setReport] = useState<BlobRepairReport | null>(null)

    // Reset whenever the target changes, so reopening never shows a stale report.
    useEffect(() => {
        if (open) setReport(null)
    }, [open, envelopeId])

    const canRepair = accountId != null && !!envelopeId

    const mutation = useMutation({
        mutationFn: () => repairMessage(accountId as number, envelopeId as string),
        onSuccess: setReport,
        onError: (error: AxiosError) => {
            // A `status: "failed"` verdict is an HTTP 200 and lands in onSuccess,
            // so anything here is a genuine transport / rejection error.
            const errorMessage =
                (error.response?.data as { message?: string })?.message ||
                error.message ||
                t('repair_message.failedHint', 'The repair could not be completed. See the error below.')

            toast({
                variant: 'destructive',
                title: t('repair_message.statusFailed', 'Failed'),
                description: errorMessage,
                action: (
                    <ToastAction altText={t('common.tryAgain')}>
                        {t('common.tryAgain')}
                    </ToastAction>
                ),
            })
        },
    })

    const phase: 'confirm' | 'running' | 'result' = report
        ? 'result'
        : mutation.isPending
            ? 'running'
            : 'confirm'

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent
                className='sm:max-w-lg'
                // Radix always points `aria-describedby` at the description node
                // and warns when no such node is mounted. Only the confirm phase
                // renders a `DialogDescription`, so suppress the attribute in the
                // other two rather than warning on every open. The report body is
                // still read normally by assistive tech.
                {...(phase === 'confirm' ? {} : { 'aria-describedby': undefined })}
            >
                <DialogHeader>
                    <DialogTitle>
                        {phase === 'result'
                            ? t('repair_message.reportTitle', 'Repair report')
                            : t('repair_message.title', 'Repair message')}
                    </DialogTitle>
                    {phase === 'confirm' && (
                        <DialogDescription asChild>
                            <div className='space-y-3 pt-1 text-sm text-muted-foreground'>
                                <p>
                                    {t(
                                        'repair_message.desc',
                                        'Re-fetches this message from the IMAP server and re-attaches any missing content. Only missing content can be healed.'
                                    )}
                                </p>

                                <div className='rounded-md border bg-muted/20 p-3'>
                                    <div className='text-xs font-medium text-muted-foreground'>
                                        {t('mail.subject', 'Subject')}
                                    </div>
                                    <div className='mt-0.5 break-words font-medium text-foreground'>
                                        {subject || '—'}
                                    </div>
                                </div>

                                <p className='text-xs'>
                                    {t(
                                        'repair_message.wholeMessageNote',
                                        'This repairs the whole message — its body and every attachment.'
                                    )}
                                </p>
                            </div>
                        </DialogDescription>
                    )}
                </DialogHeader>

                {phase === 'running' && (
                    <div className='flex items-start gap-3 py-2'>
                        <Loader2 className='mt-0.5 h-4 w-4 shrink-0 animate-spin' />
                        <div className='space-y-1'>
                            <div className='text-sm font-medium'>
                                {t('repair_message.repairing', 'Repairing…')}
                            </div>
                            <p className='text-xs text-muted-foreground'>
                                {t(
                                    'repair_message.repairingHint',
                                    'Fetching the message from the IMAP server. This can take a while.'
                                )}
                            </p>
                        </div>
                    </div>
                )}

                {phase === 'result' && report && <RepairReportView report={report} />}

                <DialogFooter>
                    {phase === 'confirm' && (
                        <>
                            <Button
                                variant='outline'
                                onClick={() => onOpenChange(false)}
                            >
                                {t('common.cancel', 'Cancel')}
                            </Button>
                            <Button
                                onClick={() => mutation.mutate()}
                                disabled={!canRepair}
                            >
                                {t('repair_message.confirm', 'Repair')}
                            </Button>
                        </>
                    )}
                    {phase === 'running' && (
                        <Button variant='outline' disabled>
                            {t('common.cancel', 'Cancel')}
                        </Button>
                    )}
                    {phase === 'result' && (
                        <Button onClick={() => onOpenChange(false)}>
                            {t('common.close', 'Close')}
                        </Button>
                    )}
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}
