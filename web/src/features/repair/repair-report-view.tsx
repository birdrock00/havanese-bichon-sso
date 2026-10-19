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


import { AlertTriangle, CheckCircle2, Info } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { BlobRepairReport, BlobRepairStatus } from '@/api/repair/api'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'
import { repairStatusLabel, repairStatusVariant } from './status'

/**
 * Renders a repair outcome. Branches on `report.status` only — the report is the
 * verdict, and the `missing_before` / `missing_after` hash lists are best-effort
 * naming that can legitimately be empty on a broken record.
 */
export function RepairReportView({
    report,
    className,
}: {
    report: BlobRepairReport
    className?: string
}) {
    const { t } = useTranslation()

    const tone: Record<BlobRepairStatus, string> = {
        complete: 'border-border bg-muted/30',
        repaired: 'border-green-600/40 bg-green-600/10',
        partial: 'border-amber-600/40 bg-amber-600/10',
        failed: 'border-destructive/40 bg-destructive/10',
    }

    const badgeVariant = repairStatusVariant(report.status)

    const StatusIcon =
        report.status === 'complete' || report.status === 'repaired'
            ? CheckCircle2
            : AlertTriangle

    // A backup window leaves the source data recovered but unwritten, so this is
    // reported as `partial` with its own wording — showing `partialHint`
    // ("corrupt content cannot be repaired") here would be actively misleading.
    const hint = report.skipped_due_to_backup
        ? { text: t('repair_message.skippedDueToBackup', 'Skipped: a backup was running. Retry once it finishes.'), icon: Info }
        : report.status === 'partial'
            ? { text: t('repair_message.partialHint', 'Content that is present but corrupt cannot be repaired by this action.'), icon: Info }
            : report.status === 'failed'
                ? { text: t('repair_message.failedHint', 'The repair could not be completed. See the error below.'), icon: AlertTriangle }
                : null

    return (
        <div className={cn('space-y-3 text-sm', className)}>
            <div className={cn('rounded-md border p-3', tone[report.status])}>
                <div className='flex items-center gap-2'>
                    <StatusIcon
                        className={cn(
                            'h-4 w-4 shrink-0',
                            report.status === 'repaired' && 'text-green-600',
                            report.status === 'complete' && 'text-muted-foreground',
                            report.status === 'partial' && 'text-amber-600',
                            report.status === 'failed' && 'text-destructive'
                        )}
                    />
                    <span className='font-medium'>
                        {t('repair_message.status', 'Result')}:
                    </span>
                    <Badge variant={badgeVariant}>
                        {repairStatusLabel(t, report.status)}
                    </Badge>
                </div>

                {(report.subject || report.message_id) && (
                    <div className='mt-2 space-y-0.5 pl-6 text-xs text-muted-foreground'>
                        {report.subject && (
                            <div className='break-words'>{report.subject}</div>
                        )}
                        {report.message_id && (
                            <div className='break-all font-mono'>
                                {report.message_id}
                            </div>
                        )}
                    </div>
                )}

                {hint && (
                    <div className='mt-2 flex items-start gap-2 pl-6 text-xs text-muted-foreground'>
                        <hint.icon className='mt-0.5 h-3.5 w-3.5 shrink-0' />
                        <span>{hint.text}</span>
                    </div>
                )}
            </div>

            {report.email_missing_after ? (
                <ReportLine
                    icon={AlertTriangle}
                    tone='text-amber-600'
                    text={t('repair_message.emailBlobMissing', 'Email content missing')}
                />
            ) : (
                report.email_missing_before && (
                    <ReportLine
                        icon={CheckCircle2}
                        tone='text-green-600'
                        text={t('repair_message.emailBlobRestored', 'Email content restored')}
                    />
                )
            )}

            <HashList
                label={t('repair_message.missingBefore', 'Suspected missing before')}
                hashes={report.missing_before}
            />
            <HashList
                label={t('repair_message.missingAfter', 'Still missing')}
                hashes={report.missing_after}
            />

            {report.error && (
                <div className='rounded-md border border-destructive/40 bg-destructive/10 p-3'>
                    <div className='text-xs font-medium text-destructive'>
                        {t('repair_message.error', 'Error')}
                    </div>
                    <div className='mt-1 break-all font-mono text-xs text-destructive/90'>
                        {report.error}
                    </div>
                </div>
            )}
        </div>
    )
}

function ReportLine({
    icon: Icon,
    tone,
    text,
}: {
    icon: typeof AlertTriangle
    tone: string
    text: string
}) {
    return (
        <div className='flex items-center gap-2'>
            <Icon className={cn('h-4 w-4 shrink-0', tone)} />
            <span>{text}</span>
        </div>
    )
}

function HashList({ label, hashes }: { label: string; hashes: string[] }) {
    if (!hashes.length) return null
    return (
        <div>
            <div className='text-xs font-medium text-muted-foreground'>{label}</div>
            <ScrollArea className='mt-1 max-h-32 rounded border bg-muted/20'>
                <div className='space-y-0.5 p-2'>
                    {hashes.map((hash) => (
                        <div key={hash} className='break-all font-mono text-xs'>
                            {hash}
                        </div>
                    ))}
                </div>
            </ScrollArea>
        </div>
    )
}
