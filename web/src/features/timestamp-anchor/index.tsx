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
// Timestamp anchoring console (Enterprise). Lists the anchored Merkle-tree
// roots (one per cycle, usually daily), triggers a manual anchor, and proves a
// single email's presence in an anchored tree. A proof bundle can be exported
// (leaf hash + Merkle path + root + TSA token) so a third party can verify it
// independently with `openssl ts -verify -data root.bin -in token.tsr`.
import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  CheckCircle2,
  ChevronDown,
  Clock,
  Copy,
  Download,
  Fingerprint,
  Loader2,
  ScrollText,
  ShieldCheck,
  XCircle,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import {
  anchor_now,
  list_timestamps,
  verify_email,
  type EmailProof,
} from '@/api/timestamp/api'
import { useCurrentUser } from '@/hooks/use-current-user'
import { useEdition } from '@/hooks/use-edition'
import { toast } from '@/hooks/use-toast'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { FixedHeader } from '@/components/layout/fixed-header'
import { Main } from '@/components/layout/main'
import { TableSkeleton } from '@/components/table-skeleton'

function formatTime(ts: number): string {
  const d = new Date(ts)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

function shortHash(h: string): string {
  if (h.length <= 16) return h
  return `${h.slice(0, 8)}…${h.slice(-8)}`
}

function statusBadge(status: string): React.ReactNode {
  if (status === 'verified') {
    return (
      <Badge
        variant='outline'
        className='border-emerald-500/40 bg-emerald-500/10 text-emerald-600'
      >
        <ShieldCheck className='mr-1 h-3 w-3' />
        Verified
      </Badge>
    )
  }
  if (status === 'local') {
    return (
      <Badge variant='outline' className='text-muted-foreground'>
        <Clock className='mr-1 h-3 w-3' />
        Local
      </Badge>
    )
  }
  return (
    <Badge
      variant='outline'
      className='border-sky-500/40 bg-sky-500/10 text-sky-600'
    >
      <Fingerprint className='mr-1 h-3 w-3' />
      Stored
    </Badge>
  )
}

/** Hex (or base64) → raw bytes, then trigger a browser download. */
function downloadBytes(filename: string, bytes: Uint8Array): void {
  const blob = new Blob([bytes], {
    type: 'application/octet-stream',
  })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

function decodeB64(b64: string): Uint8Array {
  const bin = atob(b64)
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}

function hexToBytes(hex: string): Uint8Array {
  const clean = hex.replace(/\s+/g, '')
  const out = new Uint8Array(clean.length / 2)
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16)
  }
  return out
}

function ProofPanel({ proof }: { proof: EmailProof }) {
  const { t } = useTranslation()
  const [pathOpen, setPathOpen] = useState(false)

  const canExport = proof.anchored && proof.root_hash && proof.tsa_token_b64

  const copyToken = async () => {
    if (!proof.tsa_token_b64) return
    await navigator.clipboard.writeText(proof.tsa_token_b64)
    toast({ title: t('timestampAnchor.tokenCopied', 'Token copied to clipboard') })
  }

  const downloadToken = () => {
    if (!proof.tsa_token_b64) return
    downloadBytes(`token-${proof.anchor_id}.tsr`, decodeB64(proof.tsa_token_b64))
  }

  const downloadRoot = () => {
    if (!proof.root_hash) return
    downloadBytes(`root-${proof.anchor_id}.bin`, hexToBytes(proof.root_hash))
  }

  if (!proof.found) {
    return (
      <Card>
        <CardContent className='flex items-center gap-2 p-6 text-muted-foreground'>
          <XCircle className='h-4 w-4 text-red-500' />
          {t(
            'timestampAnchor.notFound',
            'No archived envelope with that ID exists, so no counter-proof can be produced.'
          )}
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className='flex items-center gap-2'>
          <CheckCircle2 className='h-4 w-4 text-primary' />
          {t('timestampAnchor.proofTitle', 'Counter-proof')}
        </CardTitle>
        <CardDescription>
          {proof.anchored
            ? t(
                'timestampAnchor.proofHint',
                'This email is covered by an anchored Merkle tree. Export the token and root so a third party can verify the proof independently.'
              )
            : t(
                'timestampAnchor.proofUnanchored',
                'This envelope exists but no anchor covers it yet — the next anchoring cycle will include it.'
              )}
        </CardDescription>
      </CardHeader>
      <CardContent className='space-y-3'>
        <div className='grid gap-3 text-xs sm:grid-cols-2'>
          <div className='space-y-1'>
            <Label className='text-muted-foreground'>
              {t('timestampAnchor.leafHash', 'Leaf hash')}
            </Label>
            <div className='break-all rounded border bg-muted/40 p-2 font-mono'>
              {proof.leaf_hash}
            </div>
          </div>
          <div className='space-y-1'>
            <Label className='text-muted-foreground'>
              {t('timestampAnchor.ingestAt', 'Ingested at')}
            </Label>
            <div className='rounded border bg-muted/40 p-2'>
              {formatTime(proof.ingest_at)}
              {proof.deleted_at
                ? ` · ${t('timestampAnchor.deletedAt', 'deleted')} ${formatTime(proof.deleted_at)}`
                : ` · ${t('timestampAnchor.live', 'live')}`}
            </div>
          </div>
        </div>

        <Collapsible open={pathOpen} onOpenChange={setPathOpen}>
          <CollapsibleTrigger asChild>
            <Button variant='outline' size='sm'>
              <ChevronDown
                className={`mr-1 h-3.5 w-3.5 transition-transform ${pathOpen ? 'rotate-180' : ''}`}
              />
              {t('timestampAnchor.merklePath', 'Merkle path')} (
              {proof.merkle_path.length})
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className='mt-2 space-y-1'>
            {proof.merkle_path.length === 0 ? (
              <div className='rounded border bg-muted/40 p-2 font-mono text-muted-foreground'>
                {t('timestampAnchor.singleLeaf', 'Single-leaf tree — no siblings.')}
              </div>
            ) : (
              proof.merkle_path.map((step, i) => (
                <div
                  key={i}
                  className='flex items-start gap-2 rounded border bg-muted/40 p-2 font-mono'
                >
                  <Badge variant='secondary' className='mt-0.5 shrink-0'>
                    {step.side === 'left' ? '←' : '→'} {step.side}
                  </Badge>
                  <span className='break-all'>{step.hash}</span>
                </div>
              ))
            )}
          </CollapsibleContent>
        </Collapsible>

        <div className='flex flex-wrap items-center gap-2 pt-1'>
          {proof.anchored && (
            <>
              <span className='rounded border bg-muted/40 px-2 py-1 font-mono text-muted-foreground'>
                {t('timestampAnchor.anchorRoot', 'root')}{' '}
                {shortHash(proof.root_hash ?? '')}
              </span>
              <span className='text-muted-foreground'>
                {t('timestampAnchor.anchorTime', 'anchored')}{' '}
                {proof.gen_time ? formatTime(proof.gen_time) : '—'}
              </span>
            </>
          )}
        </div>

        {canExport && (
          <div className='flex flex-wrap gap-2'>
            <Button variant='outline' size='sm' onClick={copyToken}>
              <Copy className='mr-1 h-3.5 w-3.5' />
              {t('timestampAnchor.copyToken', 'Copy token')}
            </Button>
            <Button variant='outline' size='sm' onClick={downloadToken}>
              <Download className='mr-1 h-3.5 w-3.5' />
              {t('timestampAnchor.downloadToken', 'Download .tsr')}
            </Button>
            <Button variant='outline' size='sm' onClick={downloadRoot}>
              <Download className='mr-1 h-3.5 w-3.5' />
              {t('timestampAnchor.downloadRoot', 'Download root')}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

export default function TimestampAnchorPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { isEnterprise } = useEdition()
  const { require_any_permission } = useCurrentUser()

  // Read access includes compliance officers (`compliance:audit`,
  // separation of duties); anchoring/verifying stays manage-only.
  const canView =
    isEnterprise &&
    require_any_permission([
      'timestamp:manage',
      'system:root',
      'compliance:audit',
    ])
  const canManage =
    isEnterprise &&
    require_any_permission(['timestamp:manage', 'system:root'])

  const { data: anchors, isLoading } = useQuery({
    queryKey: ['timestamps'],
    queryFn: list_timestamps,
    enabled: canView,
  })

  const anchorMutation = useMutation({
    mutationFn: anchor_now,
    onSuccess: (a) => {
      toast({
        title: t('timestampAnchor.anchored', 'Timestamp anchored'),
        description: t(
          'timestampAnchor.anchoredDesc',
          'Root {root} anchored with {leaves} leaves.',
          {
            root: shortHash(a.root_hash),
            leaves: String(a.leaf_count),
          }
        ),
      })
      queryClient.invalidateQueries({ queryKey: ['timestamps'] })
    },
    onError: (e) => {
      toast({
        title: t('timestampAnchor.anchorFailed', 'Anchoring failed'),
        description: (e as Error).message,
        variant: 'destructive',
      })
    },
  })

  const [envelopeId, setEnvelopeId] = useState('')
  const [proof, setProof] = useState<EmailProof | null>(null)
  const verifyMutation = useMutation({
    mutationFn: verify_email,
    onSuccess: setProof,
    onError: (e) => {
      toast({
        title: t('timestampAnchor.verifyFailed', 'Verification failed'),
        description: (e as Error).message,
        variant: 'destructive',
      })
    },
  })

  const latest = useMemo(() => anchors?.[0] ?? null, [anchors])

  if (!canView) {
    return (
      <>
        <FixedHeader />
        <Main>
          <div className='mx-auto w-full max-w-7xl px-4 py-16 text-center text-muted-foreground'>
            {t(
              'timestampAnchor.forbidden',
              'Timestamp anchoring is available in the Enterprise edition with the timestamp:manage permission.'
            )}
          </div>
        </Main>
      </>
    )
  }

  return (
    <>
      <FixedHeader />
      <Main>
        <div className='mx-auto w-full max-w-7xl space-y-6 text-xs'>
          <div className='flex flex-wrap items-start justify-between gap-4'>
            <div>
              <h1 className='flex items-center gap-2 text-lg font-semibold'>
                <Fingerprint className='h-5 w-5 text-primary' />
                {t('timestampAnchor.title', 'Timestamp anchoring')}
              </h1>
              <p className='max-w-3xl text-muted-foreground'>
                {t(
                  'timestampAnchor.subtitle',
                  'Each cycle, a Merkle tree over the archived content hashes is rooted and the root is sent to an external RFC 3161 TSA, so the archive can later be proven to have existed unmodified at a certified time. One token per cycle, regardless of message volume.'
                )}
              </p>
            </div>
            {canManage && (
              <Button
                onClick={() => anchorMutation.mutate()}
                disabled={anchorMutation.isPending}
              >
                {anchorMutation.isPending ? (
                  <Loader2 className='mr-1 h-4 w-4 animate-spin' />
                ) : (
                  <Fingerprint className='mr-1 h-4 w-4' />
                )}
                {anchorMutation.isPending
                  ? t('timestampAnchor.anchoring', 'Anchoring…')
                  : t('timestampAnchor.anchorNow', 'Anchor now')}
              </Button>
            )}
          </div>

          <Card>
            <CardHeader>
              <CardTitle className='flex items-center gap-2'>
                <ScrollText className='h-4 w-4 text-primary' />
                {t('timestampAnchor.anchors', 'Anchors')}
              </CardTitle>
              <CardDescription>
                {t(
                  'timestampAnchor.anchorsHint',
                  'Every anchored tree root with its TSA token. The newest anchor is the current one.'
                )}
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoading && !anchors ? (
                <TableSkeleton rows={5} />
              ) : (anchors ?? []).length === 0 ? (
                <div className='rounded border p-6 text-center text-muted-foreground'>
                  {t(
                    'timestampAnchor.noAnchors',
                    'No anchors yet. Run “Anchor now” to create the first one.'
                  )}
                </div>
              ) : (
                <Table className='text-xs'>
                  <TableHeader>
                    <TableRow>
                      <TableHead>
                        {t('timestampAnchor.anchoredAt', 'Anchored at')}
                      </TableHead>
                      <TableHead>{t('timestampAnchor.root', 'Tree root')}</TableHead>
                      <TableHead>{t('timestampAnchor.leaves', 'Leaves')}</TableHead>
                      <TableHead>{t('timestampAnchor.status', 'Status')}</TableHead>
                      <TableHead>{t('timestampAnchor.tsa', 'TSA')}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {(anchors ?? []).map((a) => (
                      <TableRow
                        key={a.id}
                        className={a.id === latest?.id ? 'bg-primary/5' : undefined}
                      >
                        <TableCell className='whitespace-nowrap'>
                          {formatTime(a.gen_time)}
                        </TableCell>
                        <TableCell className='font-mono'>
                          {shortHash(a.root_hash)}
                        </TableCell>
                        <TableCell>{a.leaf_count.toLocaleString()}</TableCell>
                        <TableCell>{statusBadge(a.status)}</TableCell>
                        <TableCell className='max-w-[220px] truncate text-muted-foreground'>
                          {a.tsa_url ?? '—'}
                          {a.serial ? ` (${a.serial.slice(0, 8)}…)` : ''}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          {canManage && (
          <Card>
            <CardHeader>
              <CardTitle className='flex items-center gap-2'>
                <ShieldCheck className='h-4 w-4 text-primary' />
                {t('timestampAnchor.verifyTitle', 'Verify an email')}
              </CardTitle>
              <CardDescription>
                {t(
                  'timestampAnchor.verifyHint',
                  'Enter an envelope ID to build its counter-proof: leaf hash, Merkle path, anchored root, and the TSA token.'
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className='space-y-3'>
              <div className='flex flex-wrap items-end gap-2'>
                <div className='min-w-[280px] flex-1 space-y-1'>
                  <Label>{t('timestampAnchor.envelopeId', 'Envelope ID')}</Label>
                  <Input
                    value={envelopeId}
                    onChange={(e) => setEnvelopeId(e.target.value)}
                    placeholder='e.g. 01J...'
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' && envelopeId.trim()) {
                        verifyMutation.mutate(envelopeId.trim())
                      }
                    }}
                  />
                </div>
                <Button
                  disabled={!envelopeId.trim() || verifyMutation.isPending}
                  onClick={() => verifyMutation.mutate(envelopeId.trim())}
                >
                  {verifyMutation.isPending ? (
                    <Loader2 className='mr-1 h-4 w-4 animate-spin' />
                  ) : (
                    <ShieldCheck className='mr-1 h-4 w-4' />
                  )}
                  {t('timestampAnchor.verify', 'Verify')}
                </Button>
              </div>
              {proof && <ProofPanel proof={proof} />}
            </CardContent>
          </Card>
          )}
        </div>
      </Main>
    </>
  )
}
