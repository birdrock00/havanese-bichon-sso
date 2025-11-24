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


import { useEffect, useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Loader, Download, Trash2, MessageSquareMore } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { toast } from '@/hooks/use-toast';
import { formatBytes } from '@/lib/utils';
import EmailIframe from '@/components/mail-iframe';
import {
  AttachmentInfo,
  download_attachment,
  download_message,
  getContent,
  load_message,
} from '@/api/mailbox/envelope/api';
import { AxiosError } from 'axios';
import { useSearchContext } from './context';
import { MailThreadDialog } from './thread-dialog';
import useMinimalAccountList from '@/hooks/use-minimal-account-list';
import { useTranslation } from 'react-i18next';


interface MailMessageViewProps {
  envelope: {
    id: number;
    account_id: number,
    from?: string;
    to?: string[];
    cc?: string[];
    bcc?: string[];
    subject?: string;
    internal_date?: number;
  };
  showActions?: boolean;
  showHeader?: boolean;
  showAttachments?: boolean;
}

const Multilines: React.FC<{ title: string; lines: string[] }> = ({ title, lines }) => {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="text-xs">
      <div className="flex items-start space-x-2">
        <span className="font-medium text-gray-400 whitespace-nowrap">{title}:</span>
        <div className="flex-1">
          <ul className="list-disc list-inside">
            {lines.slice(0, expanded ? lines.length : 3).map((ref, i) => (
              <li key={i} className="line-clamp-1">{ref}</li>
            ))}
          </ul>
          {lines.length > 3 && (
            <button
              className="text-blue-500 hover:underline text-xs"
              onClick={() => setExpanded(!expanded)}
            >
              {expanded ? t('common.showLess') : t('common.showMore')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export function MailMessageView({
  envelope,
  showActions = true,
  showAttachments = true,
  showHeader = true
}: MailMessageViewProps) {
  const { t } = useTranslation()
  const { setToDelete, setOpen } = useSearchContext();

  const [content, setContent] = useState<string | null>(null);
  const [contentType, setContentType] = useState<'Plain' | 'Html' | null>(null);
  const [attachments, setAttachments] = useState<AttachmentInfo[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [downloadingAttachmentFileName, setDownloadingAttachmentFileName] = useState<string | null>(null);

  const { getEmailById } = useMinimalAccountList();
  const [threadOpen, setThreadOpen] = useState(false);

  const downloadAttachmentMutation = useMutation({
    mutationFn: ({ fileName }: { fileName: string }) =>
      download_attachment(envelope.account_id, envelope.id, fileName),
    onSuccess: () => setDownloadingAttachmentFileName(null),
    onError: (error: any) => {
      setDownloadingAttachmentFileName(null);
      toast({
        title: t('mail.failedToDownloadFile'),
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  const loadMessageMutation = useMutation({
    mutationFn: () => load_message(envelope.account_id, envelope.id),
    onSuccess: (data) => {
      setLoading(false);
      setContent(getContent(data));
      if (data.attachments) setAttachments(data.attachments);
      setContentType(data.html ? 'Html' : 'Plain');
    },
    onError: (error: any) => {
      setLoading(false);
      toast({
        title: t('mail.failedToLoadEmail'),
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  useEffect(() => {
    setLoading(true);
    loadMessageMutation.mutate();
  }, [envelope.id]);


  const toggleToDelete = (accountId: number, mailId: number) => {
    setToDelete(prev => {
      const next = new Map(prev);
      const set = new Set(next.get(accountId) || []);

      if (set.has(mailId)) {
        set.delete(mailId);
        if (set.size === 0) next.delete(accountId);
        else next.set(accountId, set);
      } else {
        set.add(mailId);
        next.set(accountId, set);
      }

      return next;
    });
  };

  const handleDelete = () => {
    if (envelope) {
      toggleToDelete(envelope.account_id, envelope.id)
      setOpen("delete")
    }
  }

  const downloadEmlFile = async () => {
    try {
      toast({ title: t('mail.downloadStarted'), description: t('mail.isBeingDownloaded', { id: envelope.id }) });
      await download_message(envelope.account_id, envelope.id);
      toast({ title: t('mail.downloadComplete'), description: t('mail.downloaded', { id: envelope.id }) });
    } catch (error) {
      let msg = t('mail.downloadFailed');
      if (error instanceof AxiosError) {
        msg = error.response?.data?.message || error.response?.data?.error || error.message;
        if (error.response?.status) msg = `${error.response.status}: ${msg}`;
      } else if (error instanceof Error) {
        msg = error.message;
      }
      toast({ title: t('mail.downloadFailed'), description: msg, variant: 'destructive' });
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header Info */}
      {showHeader && <div className="grid gap-1 text-xs">
        <div className="flex space-x-2">
          <span className="font-medium text-gray-400">{t('mail.account')}:</span>
          <span>{getEmailById(envelope.account_id)}</span>
        </div>
        <div className="flex space-x-2">
          <span className="font-medium text-gray-400">{t('mail.id')}:</span>
          <span>{envelope.id}</span>
        </div>
        {envelope.from && (
          <div className="flex space-x-2">
            <span className="font-medium text-gray-400">{t('mail.from')}:</span>
            <span>{envelope.from}</span>
          </div>
        )}
        {envelope.to && envelope.to.length > 0 && <Multilines title={t('mail.to')} lines={envelope.to} />}
        {envelope.cc && envelope.cc.length > 0 && <Multilines title={t('mail.cc')} lines={envelope.cc} />}
        {envelope.bcc && envelope.bcc.length > 0 && <Multilines title={t('mail.bcc')} lines={envelope.bcc} />}
        {envelope.subject && (
          <div className="flex space-x-2">
            <span className="font-medium text-gray-400">{t('mail.subject')}:</span>
            <span>{envelope.subject}</span>
          </div>
        )}
        {envelope.internal_date && (
          <div className="flex space-x-2">
            <span className="font-medium text-gray-400">{t('mail.date')}:</span>
            <span>{formatTimestamp(envelope.internal_date)}</span>
          </div>
        )}
      </div>}
      {/* Action Bar */}
      {showActions && (
        <>
          <div className="flex items-center mt-2 space-x-2">
            <Separator orientation="horizontal" className="flex-1 bg-border" />
          </div>
          <div className="flex items-center justify-start gap-3 text-xs text-gray-500">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon" onClick={handleDelete} className="hover:text-destructive">
                  <Trash2 className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('mail.delete')}</TooltipContent>
            </Tooltip>
            <Separator orientation="vertical" className="h-5" />
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon" onClick={downloadEmlFile}>
                  <Download className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('mail.download')}</TooltipContent>
            </Tooltip>
            <Separator orientation="vertical" className="h-5" />
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setThreadOpen(true)}
                >
                  <MessageSquareMore className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('mail.viewThread')}</TooltipContent>
            </Tooltip>
          </div>
        </>
      )}
      {showAttachments && <Separator className="my-2" />}
      {/* Attachments */}
      {showAttachments && (
        <div className="mb-2">
          {loading ? (
            <span className="text-gray-500 text-xs" />
          ) : attachments && attachments.length > 0 ? (
            (() => {
              const nonInline = attachments.filter((a) => !a.inline);
              return nonInline.length > 0 ? (
                <div className="space-y-2">
                  {nonInline.map((attachment, i) => (
                    <div key={i} className="flex items-center">
                      <div className="flex items-center space-x-8">
                        <span className="truncate text-xs">{attachment.filename}</span>
                        <span className="text-xs px-2 py-1 rounded">[{attachment.file_type}]</span>
                      </div>
                      <div className="flex items-center space-x-4 ml-auto">
                        <span className="text-gray-500 text-xs shrink-0">
                          {formatBytes(attachment.size)}
                        </span>
                        {downloadingAttachmentFileName === attachment.filename ? (
                          <Loader className="w-4 h-4 animate-spin" />
                        ) : (
                          <Download
                            className="w-4 h-4 cursor-pointer"
                            onClick={() => {
                              setDownloadingAttachmentFileName(attachment.filename);
                              downloadAttachmentMutation.mutate({ fileName: attachment.filename });
                            }}
                          />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <span className="text-gray-500 text-xs italic">
                  {t('mail.onlyNonInlineAttachments')}
                </span>
              );
            })()
          ) : (
            <span className="text-gray-500 text-xs">{t('mail.noAttachments')}</span>
          )}
        </div>
      )}
      {showAttachments && <Separator className="mb-2" />}
      {/* Content */}
      <div className="flex-1 overflow-auto">
        {loading ? (
          <div className="flex justify-center items-center py-8">
            <Loader className="w-6 h-6 animate-spin" />
            <span className="ml-2 text-sm text-muted-foreground">loading...</span>
          </div>
        ) : content ? (
          <div className="bg-gray-100 rounded-lg border border-gray-300 p-4">
            {contentType === 'Html' ? (
              <EmailIframe emailHtml={content} />
            ) : (
              <pre className="whitespace-pre-wrap text-gray-800 text-sm font-sans">{content}</pre>
            )}
          </div>
        ) : (
          <div className="text-center text-muted-foreground text-sm">No content available</div>
        )}
      </div>

      <MailThreadDialog open={threadOpen} onOpenChange={setThreadOpen} />
    </div>
  );
}

function formatTimestamp(milliseconds: number): string {
  const date = new Date(milliseconds);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');
  const timezoneOffset = date.getTimezoneOffset();
  const offsetSign = timezoneOffset > 0 ? '-' : '+';
  const offsetHours = String(Math.floor(Math.abs(timezoneOffset) / 60)).padStart(2, '0');
  const offsetMinutes = String(Math.abs(timezoneOffset) % 60).padStart(2, '0');
  return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}${offsetSign}${offsetHours}:${offsetMinutes}`;
}