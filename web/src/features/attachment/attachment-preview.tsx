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

import { useEffect, useMemo, useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Download, FileIcon, ZoomIn, ZoomOut, RotateCcw } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { toast } from '@/hooks/use-toast';
import { useTranslation } from 'react-i18next';

import { preview_attachment, download_attachment } from '@/api/mailbox/envelope/api';
import { getFileConfig } from './mail-message-view';

const PREVIEWABLE_IMAGE = /^image\/(png|jpeg|gif|webp|svg\+xml)$/;
const PREVIEWABLE_TEXT = /^(text\/(plain|csv|html|xml|css|javascript|markdown)|application\/(json|xml|javascript|x-httpd-php|x-sh|x-perl|x-python|x-ruby))$/;

interface AttachmentPreviewProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  accountId: number;
  envelopeId: string;
  contentHash: string;
  contentType: string;
  fileName: string;
}

function isImagePreview(contentType: string) {
  return PREVIEWABLE_IMAGE.test(contentType);
}

function isPdfPreview(contentType: string) {
  return contentType === 'application/pdf';
}

function isTextPreview(contentType: string) {
  return PREVIEWABLE_TEXT.test(contentType);
}

export default function AttachmentPreview({
  open,
  onOpenChange,
  accountId,
  envelopeId,
  contentHash,
  contentType,
  fileName,
}: AttachmentPreviewProps) {
  const { t } = useTranslation();
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [textContent, setTextContent] = useState<string | null>(null);
  const [imageZoom, setImageZoom] = useState(1);

  const previewMutation = useMutation({
    mutationFn: () => preview_attachment(accountId, envelopeId, contentHash),
    onSuccess: (blob) => {
      if (isTextPreview(contentType)) {
        blob.text().then(setTextContent);
      } else {
        // Re-wrap with the actual MIME type so browsers render PDFs/images inline
        // instead of triggering a download (the HTTP response uses application/octet-stream).
        const typedBlob = new Blob([blob], { type: contentType });
        setBlobUrl(URL.createObjectURL(typedBlob));
      }
    },
    onError: (error: any) => {
      toast({
        title: t('attachment_preview.failedToLoad'),
        description: error.message,
        variant: 'destructive',
      });
    },
  });

  useEffect(() => {
    if (open) {
      setBlobUrl(null);
      setTextContent(null);
      setImageZoom(1);
      previewMutation.mutate();
    }
  }, [open]);

  useEffect(() => {
    return () => {
      if (blobUrl) URL.revokeObjectURL(blobUrl);
    };
  }, [blobUrl]);

  const handleDownload = () => {
    download_attachment(accountId, envelopeId, contentHash, fileName);
  };

  const { icon, color } = useMemo(() => getFileConfig(contentType), [contentType]);

  const isImage = isImagePreview(contentType);
  const isPdf = isPdfPreview(contentType);
  const isText = isTextPreview(contentType);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="w-[calc(100vw-2rem)] max-w-4xl h-[85vh] flex flex-col p-0 gap-0"
        onInteractOutside={(e) => {
          // Don't close when interacting with the PDF viewer toolbar
          if (isPdf) e.preventDefault();
        }}
      >
        {/* Toolbar */}
        <div className="flex items-center justify-between px-4 py-2 border-b shrink-0">
          <div className="flex items-center gap-2 min-w-0">
            <div className={color}>{icon}</div>
            <span className="text-sm font-medium truncate max-w-[400px]">
              {fileName}
            </span>
          </div>
          <div className="flex items-center gap-1 pr-16">
            {isImage && blobUrl && (
              <>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => setImageZoom((z) => Math.min(z + 0.25, 3))}
                    >
                      <ZoomIn className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('attachment_preview.zoomIn')}</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => setImageZoom((z) => Math.max(z - 0.25, 0.25))}
                    >
                      <ZoomOut className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('attachment_preview.zoomOut')}</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => setImageZoom(1)}
                    >
                      <RotateCcw className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('attachment_preview.resetZoom')}</TooltipContent>
                </Tooltip>
                <Separator orientation="vertical" className="h-5 mx-1" />
              </>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8"
                  onClick={handleDownload}
                >
                  <Download className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('attachment.download')}</TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* Preview body */}
        <div className="flex-1 min-h-0 bg-muted/30">
          {previewMutation.isPending ? (
            <div className="flex items-center justify-center h-full">
              <div className="flex flex-col items-center gap-3">
                <Skeleton className="w-64 h-4" />
                <Skeleton className="w-48 h-4" />
                <Skeleton className="w-56 h-4" />
              </div>
            </div>
          ) : isImage && blobUrl ? (
            <div className="w-full h-full overflow-auto flex items-center justify-center">
              <img
                src={blobUrl}
                alt={fileName}
                className="max-w-full"
                style={{
                  transform: `scale(${imageZoom})`,
                  transformOrigin: 'center center',
                }}
              />
            </div>
          ) : isPdf && blobUrl ? (
            <iframe
              src={blobUrl}
              className="w-full h-full border-0"
              title={fileName}
            />
          ) : isText && textContent !== null ? (
            <pre className="w-full h-full overflow-auto whitespace-pre-wrap text-sm font-mono p-6">
              {textContent}
            </pre>
          ) : !previewMutation.isPending ? (
            <div className="flex items-center justify-center h-full">
              <div className="flex flex-col items-center gap-4 text-muted-foreground">
                <FileIcon className="h-16 w-16 opacity-30" />
                <p className="text-sm">{t('attachment_preview.notAvailable')}</p>
                <p className="text-xs text-center max-w-md">
                  {t('attachment_preview.notAvailableDesc', {
                    type: contentType || 'unknown',
                  })}
                </p>
                <Button variant="outline" size="sm" onClick={handleDownload}>
                  <Download className="h-4 w-4 mr-2" />
                  {t('attachment.download')}
                </Button>
              </div>
            </div>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}
