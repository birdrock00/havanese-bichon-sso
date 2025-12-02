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


import { z } from 'zod';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useToast } from '@/hooks/use-toast';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ToastAction } from '@/components/ui/toast';
import { AxiosError } from 'axios';
import React from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { create_account, update_account } from '@/api/account/api';
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
import { Loader2 } from 'lucide-react';
import { AccountModel } from '../data/schema';
import { useTranslation } from 'react-i18next';


const accountSchema = (t: (key: string) => string) =>
  z.object({
    name: z.string().optional(),
    email: z.string({ required_error: t('validation.emailRequired') }).email({ message: t('validation.invalidEmail') }),
    enabled: z.boolean()
  });


export type NoSyncAccount = {
  name?: string;
  email: string;
  enabled: boolean;
};



interface Props {
  currentRow?: AccountModel;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}


const defaultValues: NoSyncAccount = {
  name: '',
  email: '',
  enabled: true
};


const mapCurrentRowToFormValues = (currentRow: AccountModel): NoSyncAccount => {
  let account = {
    name: currentRow.name === null ? '' : currentRow.name,
    email: currentRow.email,
    enabled: currentRow.enabled
  };
  return account;
};


export function NoSyncAccountDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const isEdit = !!currentRow;
  const { toast } = useToast();

  const form = useForm<NoSyncAccount>({
    mode: "all",
    defaultValues: isEdit ? mapCurrentRowToFormValues(currentRow) : defaultValues,
    resolver: zodResolver(accountSchema(t)),
  });

  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: create_account,
    onSuccess: handleSuccess,
    onError: handleError,
  });

  const updateMutation = useMutation({
    mutationFn: (data: Record<string, any>) => update_account(currentRow?.id!, data),
    onSuccess: handleSuccess,
    onError: handleError,
  });

  function handleSuccess() {
    toast({
      title: isEdit ? t('accounts.accountUpdated') : t('accounts.accountCreated'),
      description: isEdit ? t('accounts.accountUpdatedDesc') : t('accounts.accountCreatedDesc'),
      action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
    });

    queryClient.invalidateQueries({ queryKey: ['account-list'] });
    form.reset();
    onOpenChange(false);
  }

  function handleError(error: AxiosError) {
    const errorMessage =
      (error.response?.data as { message?: string })?.message ||
      error.message ||
      (isEdit ? t('accounts.updateFailed') : t('accounts.creationFailed'));

    toast({
      variant: "destructive",
      title: isEdit ? t('accounts.accountUpdateFailed') : t('accounts.accountCreationFailed'),
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }

  const onSubmit = React.useCallback(
    (data: NoSyncAccount) => {
      const commonData = {
        email: data.email,
        name: data.name,
        enabled: data.enabled,
        use_dangerous: false
      };
      if (isEdit) {
        updateMutation.mutate(commonData);
      } else {
        const payload = {
          ...commonData,
          account_type: "NoSync"
        };
        createMutation.mutate(payload);
      }
    },
    [isEdit, updateMutation, createMutation]
  );
  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        form.reset();
        onOpenChange(state);
      }}
    >
      <DialogContent className='max-w-2xl'>
        <DialogHeader className='text-left mb-4'>
          <DialogTitle>{isEdit ? t('accounts.updateAccount') : t('accounts.addAccount')}</DialogTitle>
          <DialogDescription>
            {isEdit ? t('accounts.updateTheEmailAccountHere') : t('accounts.addNewEmailAccountHere')}
            {t('accounts.clickSaveWhenDone')}
          </DialogDescription>
        </DialogHeader>
        <ScrollArea className='h-[23rem] w-full pr-4 -mr-4 py-1'>
          <Form {...form}>
            <form
              id='nosync-account-form'
              onSubmit={form.handleSubmit(onSubmit)}
              className='space-y-4 p-0.5'
            >
              <FormField
                control={form.control}
                name="email"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="flex items-center justify-between">
                      {t('accounts.emailAddress')}:
                    </FormLabel>
                    <FormControl>
                      <Input placeholder={t('accounts.emailPlaceholder')} {...field} />
                    </FormControl>
                    <FormMessage />
                    <FormDescription>
                      {t('accounts.thisAccountIsUsedForIdentificationPurposesOnly')}
                    </FormDescription>
                  </FormItem>
                )}
              />
              {/* <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="flex items-center justify-between">
                      {t('accounts.name')}:
                    </FormLabel>
                    <FormControl>
                      <Input placeholder={t('accounts.namePlaceholder')} {...field} />
                    </FormControl>
                    <FormDescription>{t('accounts.optional')}</FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              /> */}
              <FormField
                control={form.control}
                name='enabled'
                render={({ field }) => (
                  <FormItem className='flex flex-col items-start gap-y-1'>
                    <FormLabel>{t('accounts.enabled')}:</FormLabel>
                    <FormControl>
                      <Checkbox
                        checked={field.value}
                        onCheckedChange={field.onChange}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('accounts.determinesWhetherThisAccountIsActiveNoSync')}
                    </FormDescription>
                  </FormItem>
                )}
              />
            </form>
          </Form>
        </ScrollArea>
        <DialogFooter>
          <Button
            type='submit'
            form='nosync-account-form'
            disabled={isEdit ? updateMutation.isPending : createMutation.isPending}
          >
            {isEdit ? (
              updateMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('oauth2.saving')}
                </>
              ) : (
                t('accounts.saveChanges')
              )
            ) : (
              createMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('oauth2.creating')}
                </>
              ) : (
                t('common.create')
              )
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}