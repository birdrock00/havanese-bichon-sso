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
import { Loader2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  getComplianceConfig,
  updateComplianceConfig,
  type ComplianceConfigView,
} from '@/api/compliance/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useEdition } from '@/hooks/use-edition'
import { useCurrentUser } from '@/hooks/use-current-user'
import { useToast } from '@/hooks/use-toast'

/** Pro/Enterprise compliance configuration. Each field is an override on top
 *  of the env/CLI default: leave it empty to keep the default, type a value
 *  to override, and clear it (empty again) to return to the default. */
export function ComplianceSettings() {
  const { t } = useTranslation()
  const { toast } = useToast()
  const { isPro, isEnterprise } = useEdition()
  const { require_any_permission } = useCurrentUser()

  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [current, setCurrent] = useState<ComplianceConfigView | null>(null)

  const [auditRetentionDays, setAuditRetentionDays] = useState('')
  const [integrityScheduleHours, setIntegrityScheduleHours] = useState('')
  const [integrityRetentionDays, setIntegrityRetentionDays] = useState('')
  const [tspIntervalHours, setTspIntervalHours] = useState('')

  const canManage =
    (isPro || isEnterprise) && require_any_permission(['system:root'])

  useEffect(() => {
    let mounted = true
    getComplianceConfig()
      .then((cfg) => {
        if (!mounted) return
        setCurrent(cfg)
        setAuditRetentionDays(cfg.audit_retention_days?.toString() ?? '')
        setIntegrityScheduleHours(
          cfg.integrity_schedule_hours?.toString() ?? ''
        )
        setIntegrityRetentionDays(
          cfg.integrity_retention_days?.toString() ?? ''
        )
        setTspIntervalHours(cfg.tsp_interval_hours?.toString() ?? '')
      })
      .catch(() => {
        if (mounted) {
          toast({
            variant: 'destructive',
            title: t(
              'settings.compliance.loadFailed',
              'Failed to load compliance configuration',
            ),
          })
        }
      })
      .finally(() => {
        if (mounted) setLoading(false)
      })
    return () => {
      mounted = false
    }
  }, [t, toast])

  if (!canManage) {
    return (
      <div className='w-full p-6 text-muted-foreground'>
        {t(
          'settings.compliance.forbidden',
          'Compliance configuration requires the Pro or Enterprise edition and the system:root permission.',
        )}
      </div>
    )
  }

  if (loading) {
    return (
      <div className='flex h-64 items-center justify-center'>
        <Loader2 className='h-6 w-6 animate-spin' />
      </div>
    )
  }

  const numOrEmpty = (value: string) => (value.trim() ? value.trim() : '')

  const handleSave = async () => {
    setSaving(true)
    try {
      // Always send all four fields: a filled input sets the override, an
      // empty one clears it back to the env/CLI default.
      const payload = {
        audit_retention_days: numOrEmpty(auditRetentionDays),
        integrity_schedule_hours: numOrEmpty(integrityScheduleHours),
        integrity_retention_days: numOrEmpty(integrityRetentionDays),
        tsp_interval_hours: numOrEmpty(tspIntervalHours),
      }
      const saved = await updateComplianceConfig(payload)
      setCurrent(saved)
      toast({
        title: t('settings.compliance.saved', 'Compliance configuration saved'),
      })
    } catch (err: unknown) {
      const message =
        (err as { response?: { data?: { message?: string } } })?.response?.data
          ?.message || (err instanceof Error ? err.message : String(err))
      toast({
        variant: 'destructive',
        title: t(
          'settings.compliance.saveFailed',
          'Failed to save compliance configuration',
        ),
        description: message,
      })
    } finally {
      setSaving(false)
    }
  }

  const numField = (
    id: string,
    value: string,
    onChange: (v: string) => void,
    key: string,
    fallback: string,
    hintKey: string
  ) => (
    <div className='space-y-2'>
      <Label htmlFor={id}>{t(`settings.compliance.${key}`, key)}</Label>
      <Input
        id={id}
        type='number'
        min={0}
        value={value}
        placeholder={fallback}
        onChange={(e) => onChange(e.target.value)}
      />
      <p className='text-xs text-muted-foreground'>
        {t(`settings.compliance.${hintKey}`, '')}
      </p>
    </div>
  )

  return (
    <div className='w-full max-w-7xl space-y-6 px-4'>
      <div className='space-y-2'>
        <h2 className='text-xl font-bold'>
          {t('settings.compliance.title', 'Compliance configuration')}
        </h2>
        <p className='text-sm text-muted-foreground'>
          {t(
            'settings.compliance.description',
            'Retention windows and schedule intervals for the audit trail, integrity checks and timestamp anchoring. Empty fields keep the server defaults; type a value to override.',
          )}
        </p>
      </div>

      <div className='space-y-6 rounded-lg border p-6'>
        <div className='grid gap-4 sm:grid-cols-2'>
          {numField(
            'compliance-audit-retention',
            auditRetentionDays,
            setAuditRetentionDays,
            'auditRetentionDays',
            current?.audit_retention_days?.toString() ?? '',
            'auditRetentionDaysHint',
          )}
          {numField(
            'compliance-integrity-schedule',
            integrityScheduleHours,
            setIntegrityScheduleHours,
            'integrityScheduleHours',
            current?.integrity_schedule_hours?.toString() ?? '',
            'integrityScheduleHoursHint',
          )}
          {numField(
            'compliance-integrity-retention',
            integrityRetentionDays,
            setIntegrityRetentionDays,
            'integrityRetentionDays',
            current?.integrity_retention_days?.toString() ?? '',
            'integrityRetentionDaysHint',
          )}
          {numField(
            'compliance-tsp-interval',
            tspIntervalHours,
            setTspIntervalHours,
            'tspIntervalHours',
            current?.tsp_interval_hours?.toString() ?? '',
            'tspIntervalHoursHint',
          )}
        </div>

        <div className='flex items-center justify-end'>
          <Button type='button' onClick={handleSave} disabled={saving}>
            {saving && <Loader2 className='mr-2 h-4 w-4 animate-spin' />}
            {t('settings.compliance.save', 'Save configuration')}
          </Button>
        </div>
      </div>
    </div>
  )
}
