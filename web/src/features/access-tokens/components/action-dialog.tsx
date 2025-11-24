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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


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
import { ScrollArea } from '@/components/ui/scroll-area'
// import { MultiSelect } from '@/components/multi-select'
import { Textarea } from '@/components/ui/textarea'
import { AccessToken } from '../data/schema'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { create_access_token, update_access_token } from '@/api/access-tokens/api'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { VirtualizedSelect } from '@/components/virtualized-select'
import { Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

const isValidIP = (ip: string) => {
  const ipv4Regex = /^(?:(?:\d{1,3}\.){3}\d{1,3})$/;
  const ipv6Regex = /^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$/;
  return ipv4Regex.test(ip) || ipv6Regex.test(ip);
};

const RateLimitBaseSchema = z.object({
  quota: z.optional(z.number()),
  interval: z.optional(z.number()),
});

const AccessControlBaseSchema = z.object({
  ip_whitelist: z.string().optional(),
  rate_limit: z.optional(RateLimitBaseSchema),
});

const AccessTokenBaseSchema = z.object({
  accounts: z.array(z.number()),
  description: z.optional(z.string()),
  acl: z.optional(AccessControlBaseSchema),
});

export type AccessTokenForm = z.infer<typeof AccessTokenBaseSchema>;

const getRateLimitSchema = (t: (key: string) => string) => z.object({
  quota: z.optional(z.number().int().positive({ message: t('accessTokens.quotaMustBeAPositiveInteger') })),
  interval: z.optional(z.number().int().positive({ message: t('accessTokens.intervalMustBeAPositiveInteger') })),
});

const getAccessControlSchema = (t: (key: string) => string) => AccessControlBaseSchema.extend({
  rate_limit: getRateLimitSchema(t).optional(),
}).transform((data) => {
  if (data.ip_whitelist) {
    const ips = data.ip_whitelist
      .split('\n')
      .map((ip) => ip.trim())
      .filter((ip) => ip !== '');

    return {
      ...data,
      ip_whitelist: ips.join('\n'),
    };
  }
  return data;
}).refine(
  (data) => {
    if (data.ip_whitelist) {
      const ips = data.ip_whitelist.split('\n');
      const invalidIPs = ips.filter((ip) => !isValidIP(ip));
      return invalidIPs.length === 0;
    }
    return true;
  },
  {
    message: t('accessTokens.invalidIpAddressesFound'),
    path: ['ip_whitelist'],
  }
).transform((data) => {
  if (data.rate_limit && !data.rate_limit.interval && !data.rate_limit.quota) {
    return {
      ...data,
      rate_limit: undefined,
    };
  }
  return data;
})
  .transform((data) => {
    if (!data.ip_whitelist && !data.rate_limit) {
      return undefined;
    }
    return data;
  });

const getAccessTokenFormSchema = (t: (key: string) => string) => AccessTokenBaseSchema.extend({
  accounts: z
    .array(z.number())
    .min(1, { message: t('accessTokens.atLeastOneAccountIsRequired') }),
  description: z
    .optional(z.string().max(255, { message: t('accessTokens.descriptionMustNotExceed255Characters') })),
  acl: z.optional(getAccessControlSchema(t)),
});

interface Props {
  currentRow?: AccessToken
  open: boolean
  onOpenChange: (open: boolean) => void
}

const defaultValues = {
  accounts: [],
  description: undefined,
  access_scopes: [],
  acl: undefined,
};


export function TokensActionDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const isEdit = !!currentRow
  const queryClient = useQueryClient();
  const form = useForm<AccessTokenForm>({
    resolver: zodResolver(getAccessTokenFormSchema(t)),
    defaultValues: isEdit
      ? {
        accounts: currentRow.accounts.map(value => value.id),
        description: currentRow.description ?? undefined,
        acl: currentRow.acl
          ? {
            ip_whitelist: currentRow.acl.ip_whitelist
              ? currentRow.acl.ip_whitelist.join('\n')
              : undefined,
            rate_limit: currentRow.acl.rate_limit ? currentRow.acl.rate_limit : undefined
          }
          : undefined,
      }
      : defaultValues,
  });

  const createMutation = useMutation({
    mutationFn: create_access_token,
    onSuccess: handleSuccess,
    onError: handleError
  });

  const updateMutation = useMutation({
    mutationFn: (data: Record<string, any>) => update_access_token(currentRow?.token ?? '', data),
    onSuccess: handleSuccess,
    onError: handleError
  })

  function handleSuccess() {
    toast({
      title: `${t('accessTokens.title')} ${isEdit ? t('accessTokens.updated') : t('accessTokens.created')}`,
      description: t('accessTokens.yourAccessTokenHasBeenSuccessfully', { action: isEdit ? t('accessTokens.updated').toLowerCase() : t('accessTokens.created').toLowerCase() }),
      action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
    });

    queryClient.invalidateQueries({ queryKey: ['access-tokens'] });
    form.reset();
    onOpenChange(false);
  }

  function handleError(error: AxiosError) {
    const errorMessage = (error.response?.data as { message?: string })?.message ||
      error.message ||
      t('accessTokens.updateOrCreationFailed', { action: isEdit ? t('accessTokens.updateFailed') : t('accessTokens.creationFailed') });

    toast({
      variant: "destructive",
      title: `${t('accessTokens.title')} ${isEdit ? t('accessTokens.updateFailed') : t('accessTokens.creationFailed')}`,
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }

  const { accountsOptions, isLoading } = useMinimalAccountList();

  const onSubmit = (values: AccessTokenForm) => {
    const payload = {
      accounts: values.accounts,
      description: values.description,
      acl: values.acl
        ? {
          ...values.acl,
          ip_whitelist: values.acl.ip_whitelist
            ? (() => {
              const ipSet = new Set(
                values.acl.ip_whitelist
                  .split('\n')
                  .map(ip => ip.trim())
                  .filter(ip => ip !== ''),
              );
              return ipSet.size > 0 ? Array.from(ipSet) : undefined;
            })()
            : undefined,
        }
        : undefined,
    };

    if (isEdit) {
      updateMutation.mutate(payload);
    } else {
      createMutation.mutate(payload);
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
      <DialogContent className='max-w-4xl'>
        <DialogHeader className='text-left mb-4'>
          <DialogTitle>{isEdit ? t('accessTokens.editToken') : t('accessTokens.addNew')}</DialogTitle>
          <DialogDescription>
            {isEdit ? t('accessTokens.updateTheAccessTokenHere') : t('accessTokens.createNewAccessTokenHere')}
            {t('accounts.clickSaveWhenDone')}
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className='h-[28rem] w-full pr-4 -mr-4 py-1'>
          <Form {...form}>
            <form
              id='token-form'
              onSubmit={form.handleSubmit(onSubmit)}
              className='space-y-4 p-0.5'
            >
              <FormField
                control={form.control}
                name='accounts'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('accessTokens.accounts')}:</FormLabel>
                    <FormControl>
                      <VirtualizedSelect
                        multiple
                        options={accountsOptions}
                        className='w-full'
                        isLoading={isLoading}
                        onSelectOption={(options) => {
                          const numberArray = options.map((v) => parseInt(v, 10));
                          return field.onChange(numberArray);
                        }}
                        value={field.value.map(String)}
                        placeholder={t('accessTokens.selectAccounts')}
                      />
                    </FormControl>
                    <FormMessage />
                    <FormDescription>
                      {t('accessTokens.selectMultipleAccountsForTheAccessToken')}
                    </FormDescription>
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="acl.ip_whitelist"
                render={({ field }) => (
                  <FormItem className="flex flex-col gap-y-1 space-y-0">
                    <FormLabel className='mb-1'>{t('settings.acl')}:</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder={t('accessTokens.enterOneIpAddressPerLine')}
                        {...field}
                        className="max-h-[500px] min-h-[180px]"
                      />
                    </FormControl>
                    <FormDescription>
                      {t('accessTokens.aListOfIpAddressesAllowed')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="flex gap-4">
                <FormField
                  control={form.control}
                  name="acl.rate_limit.quota"
                  render={({ field }) => (
                    <FormItem className="flex flex-col gap-y-1 space-y-0 w-1/2">
                      <FormLabel className='mb-1'>{t('accessTokens.quota')}:</FormLabel>
                      <FormControl>
                        <Input
                          type="number"
                          placeholder={t('accessTokens.enterQuota')}
                          {...field}
                          onChange={(e) => field.onChange(parseInt(e.target.value, 10))}
                        />
                      </FormControl>
                      <FormDescription>
                        {t('accessTokens.theMaximumNumberOfRequestsAllowed')}
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="acl.rate_limit.interval"
                  render={({ field }) => (
                    <FormItem className="flex flex-col gap-y-1 space-y-0 w-1/2">
                      <FormLabel className='mb-1'>{t('accessTokens.interval')}:</FormLabel>
                      <FormControl>
                        <Input
                          type="number"
                          placeholder={t('accessTokens.enterIntervalInSeconds')}
                          {...field}
                          onChange={(e) => field.onChange(parseInt(e.target.value, 10))}
                        />
                      </FormControl>
                      <FormDescription>
                        {t('accessTokens.theTimeWindowForTheRateLimit')}
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
              <FormField
                control={form.control}
                name='description'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('settings.description')}:</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder={t('accessTokens.describeThePurposeOfTheAccessToken')}
                        {...field}
                        className="max-h-[240px] min-h-[80px]"
                      />
                    </FormControl>
                    <FormDescription>{t('oauth2.optional')}</FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </form>
          </Form>
        </ScrollArea>
        <DialogFooter>
          <Button
            type="submit"
            form="token-form"
            disabled={isEdit ? updateMutation.isPending : createMutation.isPending}
            className="min-w-[100px] relative transition-all"
          >
            <span className="inline-flex items-center justify-center gap-2">
              {(isEdit ? updateMutation.isPending : createMutation.isPending) && (
                <Loader2 className="h-4 w-4 animate-spin" />
              )}
              <span>
                {isEdit
                  ? updateMutation.isPending
                    ? t('accessTokens.updating')
                    : t('accessTokens.saveChanges')
                  : createMutation.isPending
                    ? t('accessTokens.creating')
                    : t('accessTokens.save')}
              </span>
            </span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}