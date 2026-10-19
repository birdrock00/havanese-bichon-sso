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
// The GDPR Art. 17 erasure request form.
//
// This dialog is not a delete confirmation with extra fields. What it collects
// *is* the record: who asked, how their mail was identified, and why the
// operator believes no statutory retention obligation covers it. That
// assessment is what a supervisory authority reads when it asks why a data
// subject's mail is gone, and it is what the approver signs — so all three
// fields are required before the button enables, and the server refuses a
// request without them as well.
import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { AxiosError } from 'axios'
import { toast } from '@/hooks/use-toast'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { IconAlertCircle } from '@tabler/icons-react'
import { create_erasure_request, OP_GDPR_ERASURE } from '@/api/approvals/api'
import { useGatedOp } from '@/features/approvals/use-gated-op'
import { useSearchContext } from './context'
import { mapToRecordOfArrays } from '@/lib/utils'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
}

const errorMessage = (error: unknown): string =>
  (error as AxiosError<{ message?: string }>)?.response?.data?.message ??
  (error as Error)?.message ??
  'Unknown error'

export function GdprErasureDialog({ open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { toDelete, setToDelete, setSelected } = useSearchContext()
  const { gated, announceIfQueued } = useGatedOp(OP_GDPR_ERASURE)

  const [subject, setSubject] = useState('')
  const [criteria, setCriteria] = useState('')
  const [assessment, setAssessment] = useState('')

  const selectedCount = Array.from(toDelete.values()).reduce(
    (sum, set) => sum + set.size,
    0
  )

  const reset = () => {
    setSubject('')
    setCriteria('')
    setAssessment('')
    setToDelete(new Map())
    setSelected(new Map())
  }

  const submit = useMutation({
    mutationFn: () =>
      create_erasure_request({
        subject: subject.trim(),
        criteria: criteria.trim(),
        assessment: assessment.trim(),
        messages: mapToRecordOfArrays(toDelete),
      }),
    retry: false,
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['approvals'] })
      onOpenChange(false)
      reset()

      // Queued: nothing has been erased, and saying otherwise would be the
      // one lie this whole workflow exists to prevent.
      if (announceIfQueued(data)) return

      // The policy does not cover erasure here, so it ran directly — and it
      // may have failed. `ok` is the outcome, not "the request was accepted":
      // the request is on the record either way, so a failure must be
      // reported as one rather than dressed up as a completed erasure.
      if (!data.ok) {
        const firstError = (data.result ?? []).find((item) => !item.ok)?.error
        toast({
          variant: 'destructive',
          title: t('approvals.erasureFailed', 'The erasure request was not recorded'),
          description: firstError ?? undefined,
        })
        return
      }

      toast({
        title: t(
          'approvals.erasureExecuted',
          'Erasure executed — the messages are gone and the request is on the record'
        ),
      })
    },
    onError: (error) => {
      toast({
        variant: 'destructive',
        title: t('approvals.erasureFailed', 'The erasure request was not recorded'),
        description: errorMessage(error),
      })
    },
  })

  // Whitespace is not a basis — the server trims and refuses an empty one, and
  // the button must agree rather than letting the operator submit and be told.
  const canSubmit =
    subject.trim().length > 0 &&
    criteria.trim().length > 0 &&
    assessment.trim().length > 0 &&
    selectedCount > 0 &&
    !submit.isPending

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-lg'>
        <DialogHeader>
          <DialogTitle>
            {t('approvals.erasureTitle', 'GDPR erasure request (Art. 17)')}
          </DialogTitle>
          <DialogDescription>
            {t(
              'approvals.erasureDesc',
              'Record that a data subject asked for their data to be erased, and why the messages selected below are not covered by a statutory retention obligation.'
            )}
          </DialogDescription>
        </DialogHeader>

        <div className='space-y-3'>
          <div className='space-y-1'>
            <Label htmlFor='erasure-subject'>
              {t('approvals.erasureSubject', 'Data subject')}
            </Label>
            <Input
              id='erasure-subject'
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder='name@example.com'
            />
          </div>

          <div className='space-y-1'>
            <Label htmlFor='erasure-criteria'>
              {t('approvals.erasureCriteria', 'How the messages were identified')}
            </Label>
            <Input
              id='erasure-criteria'
              value={criteria}
              onChange={(e) => setCriteria(e.target.value)}
              placeholder={t(
                'approvals.erasureCriteriaPlaceholder',
                'e.g. sender or recipient name@example.com, 01.01.2024–31.12.2025'
              )}
            />
          </div>

          <div className='space-y-1'>
            <Label htmlFor='erasure-assessment'>
              {t('approvals.erasureAssessment', 'Assessment')}
            </Label>
            <Textarea
              id='erasure-assessment'
              value={assessment}
              onChange={(e) => setAssessment(e.target.value)}
              rows={3}
              placeholder={t(
                'approvals.erasureAssessmentPlaceholder',
                'Why no statutory retention obligation applies to these messages'
              )}
            />
            <p className='text-xs text-muted-foreground'>
              {t(
                'approvals.erasureAssessmentHint',
                'This is the basis the approver signs, and the sentence a supervisory authority will read. It cannot be left empty.'
              )}
            </p>
          </div>

          <p className='text-sm text-muted-foreground'>
            {t(
              'approvals.erasureCount',
              '{{count}} selected message(s) will be erased if this request is approved.',
              { count: selectedCount }
            )}
          </p>

          {gated ? (
            <Alert>
              <IconAlertCircle className='h-4 w-4' />
              <AlertTitle>
                {t(
                  'approvals.gatedErasureTitle',
                  'Dual control is on for this operation'
                )}
              </AlertTitle>
              <AlertDescription>
                {t(
                  'approvals.gatedErasureDesc',
                  'This will NOT erase anything now. It records the request and submits it for a second person, who must state the basis on which the erasure is lawful.'
                )}
              </AlertDescription>
            </Alert>
          ) : (
            <Alert variant='destructive'>
              <IconAlertCircle className='h-4 w-4' />
              <AlertTitle>
                {t('approvals.erasureDirectTitle', 'This erases immediately')}
              </AlertTitle>
              <AlertDescription>
                {t(
                  'approvals.erasureDirectDesc',
                  'Dual control does not cover erasure requests on this installation, so this runs on your signature alone. Deletion in the archive is final — there is no recycle bin.'
                )}
              </AlertDescription>
            </Alert>
          )}
        </div>

        <DialogFooter>
          <Button
            variant='outline'
            onClick={() => onOpenChange(false)}
            disabled={submit.isPending}
          >
            {t('common.cancel')}
          </Button>
          <Button onClick={() => submit.mutate()} disabled={!canSubmit}>
            {gated
              ? t('approvals.submitForApproval', 'Submit for approval')
              : t('approvals.erasureSubmit', 'Record and erase')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
