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


import { z } from 'zod'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { toast } from '@/hooks/use-toast'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { Proxy } from '../data/schema'
import { AxiosError } from 'axios'
import { ToastAction } from '@/components/ui/toast'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Loader2 } from 'lucide-react'
import { add_proxy, update_proxy } from '@/api/system/api'
import { useTranslation } from 'react-i18next'


const proxyFormSchema = z.object({
  url: z.string()
    .min(1, "Proxy address cannot be empty")
    .refine(
      (value) => {
        try {
          const url = new URL(value);
          return url.protocol === 'socks5:' || url.protocol === 'http:';
        } catch {
          return false;
        }
      },
      {
        message: "URL must start with http:// or socks5://",
      }
    )
    .refine(
      (value) => {
        const url = new URL(value);
        return /^[a-zA-Z0-9\-\.]+$/.test(url.hostname);
      },
      {
        message: "Hostname contains invalid characters",
      }
    )
    .refine(
      (value) => {
        const url = new URL(value);
        const port = parseInt(url.port || '1080');
        return port > 0 && port <= 65535;
      },
      {
        message: "Port must be between 1-65535",
      }
    )
    .refine(
      (value) => {
        const url = new URL(value);
        if (url.username && !url.password) return false;
        return true;
      },
      {
        message: "Password cannot be empty when username is provided",
      }
    )
    .refine(
      (value) => {
        const url = new URL(value);
        if (url.password) return url.password.length >= 8;
        return true;
      },
      {
        message: "Password must be at least 8 characters",
      }
    )
});

export type ProxyForm = z.infer<typeof proxyFormSchema>;


interface Props {
  currentRow?: Proxy
  open: boolean
  onOpenChange: (open: boolean) => void
}

const defaultValues = {
  url: ""
};


const mapCurrentRowToFormValues = (currentRow: Proxy) => {
  let data = {
    url: currentRow.url
  };
  return data;
};

export function ProxyActionDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const isEdit = !!currentRow
  const queryClient = useQueryClient();
  const form = useForm<ProxyForm>({
    resolver: zodResolver(proxyFormSchema),
    defaultValues: isEdit
      ? mapCurrentRowToFormValues(currentRow)
      : defaultValues,
  });


  const createMutation = useMutation({
    mutationFn: add_proxy,
    onSuccess: handleSuccess,
    onError: handleError
  });

  const updateMutation = useMutation({
    mutationFn: (url: string) => update_proxy(currentRow?.id!, url),
    onSuccess: handleSuccess,
    onError: handleError
  })

  function handleSuccess() {
    toast({
      title: `${t('settings.proxy')} ${isEdit ? t('common.update') : t('common.add')}`,
      description: t('settings.yourProxyHasBeenSuccessfully', { action: isEdit ? t('common.update').toLowerCase() : t('common.add').toLowerCase() }),
      action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
    });

    queryClient.invalidateQueries({ queryKey: ['proxy-list'] });
    form.reset();
    onOpenChange(false);
  }

  function handleError(error: AxiosError) {
    const errorMessage = (error.response?.data as { message?: string })?.message ||
      error.message ||
      t('settings.proxyUpdateOrAddFailed', { action: isEdit ? t('common.update') : t('common.add') });

    toast({
      variant: "destructive",
      title: `${t('settings.proxy')} ${isEdit ? t('common.update') : t('common.add')} ${t('common.error')}`,
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }


  const onSubmit = (values: ProxyForm) => {
    const url = values.url;
    if (isEdit) {
      updateMutation.mutate(url);
    } else {
      createMutation.mutate(url);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        form.reset()
        onOpenChange(state)
      }}
    >
      <DialogContent className='max-w-xl'>
        <DialogHeader className='text-left mb-4'>
          <DialogTitle>{isEdit ? t('settings.edit') + ' ' + t('settings.proxy') : t('common.add') + ' ' + t('settings.proxy')}</DialogTitle>
          <DialogDescription>
            {isEdit ? t('settings.updateTheProxyHere') : t('settings.addNewProxyHere')}
            {t('accounts.clickSaveWhenDone')}
          </DialogDescription>
        </DialogHeader>
        <Form {...form}>
          <form
            id='proxy-form'
            onSubmit={form.handleSubmit(onSubmit)}
            className='space-y-4 p-0.5'
          >
            <FormField
              control={form.control}
              name="url"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t('settings.proxy')} URL</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="socks5://127.0.0.1:22308"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                  <FormDescription>
                    {t('settings.pleaseUseAnIpAddressForBetterReliability')}
                  </FormDescription>
                </FormItem>
              )}
            />
          </form>
        </Form>
        <DialogFooter>
          <Button
            type="submit"
            form="proxy-form"
            disabled={isEdit ? updateMutation.isPending : createMutation.isPending}
            className="min-w-[120px] relative"
          >
            <span className="inline-flex items-center justify-center">
              {(isEdit ? updateMutation.isPending : createMutation.isPending) && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              <span>
                {isEdit
                  ? updateMutation.isPending
                    ? t('oauth2.saving')
                    : t('accounts.saveChanges')
                  : createMutation.isPending
                    ? t('settings.adding')
                    : t('common.add')}
              </span>
            </span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
