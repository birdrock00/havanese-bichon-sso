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

import { zodResolver } from '@hookform/resolvers/zod';
import * as React from 'react';
import { useForm } from 'react-hook-form';
import { Button } from '@/components/ui/button';
import { Form } from '@/components/ui/form';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useToast } from '@/hooks/use-toast';
import Step1 from './step1';
import Step2 from './step2';
import Step3 from './step3';
import Step4 from './step4';
import { create_account, autoconfig, update_account, AccountModel, ImapConfig } from '@/api/account/api';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ToastAction } from '@/components/ui/toast';
import { AxiosError } from 'axios';
import { useTranslation } from 'react-i18next';
import { cn } from "@/lib/utils";
import { getAccountSchema, type AccountFormValues } from './schema';

export type Account = AccountFormValues;

type Step = {
  id: `step-${number}`;
  name: string;
  fields: (keyof Account)[];
};

export type Steps = [...Step[]];

const getSteps = (t: (key: string) => string): Steps => [
  { id: "step-1", name: t('accounts.steps.emailAddress'), fields: ["email", "account_name"] },
  { id: "step-2", name: t('accounts.steps.imap'), fields: ["imap", "use_dangerous", "login_name"] },
  { id: "step-3", name: t('accounts.steps.syncPreferences'), fields: ["enabled", "date_since", "date_before", "download_interval_min", "download_batch_size", "auto_download_new_mailboxes", "download_schedule"] },
  { id: "step-4", name: t('accounts.steps.summary'), fields: [] },
];

const LAST_STEP = 4;

interface Props {
  currentRow?: AccountModel;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const defaultValues: Account = {
  login_name: undefined,
  account_name: undefined,
  email: '',
  imap: {
    host: "",
    port: 993,
    encryption: 'Ssl',
    auth: {
      auth_type: 'Password',
      password: undefined,
    },
    use_proxy: undefined
  },
  enabled: true,
  use_dangerous: false,
  date_since: undefined,
  date_before: undefined,
  download_interval_min: 60,
  download_batch_size: 30,
  auto_download_new_mailboxes: true,
  download_schedule: undefined,
};

const emptyImap: ImapConfig = {
  host: "",
  port: 0,
  encryption: "None",
  auth: { auth_type: "Password", password: undefined },
  use_proxy: undefined,
};

const mapCurrentRowToFormValues = (currentRow: AccountModel): Account => {
  const imap = { ...(currentRow.imap ?? emptyImap) };
  imap.auth = { ...imap.auth, password: undefined };
  if (imap.use_proxy === null) {
    imap.use_proxy = undefined;
  }

  return {
    account_name: currentRow.account_name ?? undefined,
    login_name: currentRow.login_name ?? undefined,
    email: currentRow.email,
    imap,
    enabled: currentRow.enabled,
    use_dangerous: currentRow.use_dangerous,
    date_since: currentRow.date_since ?? undefined,
    date_before: currentRow.date_before ?? undefined,
    download_interval_min: currentRow.download_interval_min ?? 60,
    download_batch_size: currentRow.download_batch_size ?? 30,
    auto_download_new_mailboxes: currentRow.auto_download_new_mailboxes ?? true,
    download_schedule: currentRow.download_schedule ?? undefined,
  };
};

export function AccountActionDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation();
  const steps = getSteps(t);
  const isEdit = !!currentRow;
  const [currentStep, setCurrentStep] = React.useState(1);
  const { toast } = useToast();
  const [autoConfigLoading, setAutoConfigLoading] = React.useState(false);

  const accountSchema = getAccountSchema(isEdit, t);
  const form = useForm<Account>({
    mode: "onChange",
    defaultValues: isEdit ? mapCurrentRowToFormValues(currentRow) : defaultValues,
    resolver: zodResolver(accountSchema),
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
    (data: Account) => {
      const commonData = {
        email: data.email,
        account_name: data.account_name,
        login_name: data.login_name,
        imap: {
          ...data.imap,
          auth: {
            ...data.imap.auth,
            password: data.imap.auth.auth_type === 'OAuth2'
              ? undefined
              : (isEdit && !data.imap.auth.password ? undefined : data.imap.auth.password),
          },
        },
        enabled: data.enabled,
        use_dangerous: data.use_dangerous,
        date_since: data.date_since,
        date_before: data.date_before,
        download_interval_min: data.download_interval_min,
        download_batch_size: data.download_batch_size,
        auto_download_new_mailboxes: data.auto_download_new_mailboxes,
        download_schedule: data.download_schedule || null,
      };
      if (isEdit) {
        const isAllMode = !data.date_since && !data.date_before;
        const clear_download_schedule = !data.download_schedule && currentRow?.download_schedule;
        updateMutation.mutate({
          ...commonData,
          ...(isAllMode ? { clear_date_range: true } : {}),
          ...(clear_download_schedule ? { clear_download_schedule: true } : {})
        });
      } else {
        createMutation.mutate({ ...commonData, account_type: "IMAP" });
      }
    },
    [isEdit, updateMutation, createMutation]
  );

  const handleNav = async (index: number) => {
    let isValid = true;
    let failedStep = currentStep;
    for (let i = currentStep - 1; i < index - 1 && isValid; i++) {
      isValid = await form.trigger(steps[i].fields);
      if (!isValid) failedStep = i;
    }
    if (isValid) setCurrentStep(index);
    else setCurrentStep(failedStep);
  };

  async function handleContinue() {
    const isValid = await form.trigger(steps[currentStep - 1].fields);
    if (!isValid) return;

    if (currentStep === 1) {
      let allValues = form.getValues();
      if (allValues.imap.host.trim() !== "" && allValues.imap.port > 0) {
        handleNav(currentStep + 1);
        return;
      }
      setAutoConfigLoading(true);
      const email = form.getValues('email');
      form.setValue('login_name', email);
      try {
        const result = await autoconfig(email);
        if (result) {
          form.setValue('imap.host', result.imap.host);
          form.setValue('imap.port', result.imap.port);
          form.setValue('imap.encryption', result.imap.encryption);
          if (result.oauth2) form.setValue('imap.auth.auth_type', 'OAuth2');
        }
      } catch (error) {
        console.error('Auto-configuration failed:', error);
      }
      setAutoConfigLoading(false);
      handleNav(currentStep + 1);
    } else {
      handleNav(currentStep + 1);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        if (!state) {
          form.reset();
          setCurrentStep(1);
        }
        onOpenChange(state);
      }}
    >
      <DialogContent className="max-w-[95vw] md:max-w-5xl w-full p-0 overflow-hidden flex flex-col h-[50rem]">
        <div className="p-6 pb-2 flex-shrink-0">
          <DialogHeader className="text-left">
            <DialogTitle>{isEdit ? t('accounts.updateAccount') : t('accounts.addAccount')}</DialogTitle>
            <DialogDescription>
              {isEdit ? t('accounts.updateAccount') : t('accounts.addAccount')}
              {t('accounts.clickSaveWhenDone')}
            </DialogDescription>
          </DialogHeader>
        </div>

        <div className="flex flex-col md:flex-row flex-1 min-h-0 overflow-hidden border-y">
          <div className="md:hidden flex px-6 py-2 space-x-2 overflow-x-auto border-b flex-shrink-0 bg-background/50">
            {steps.map((step, index) => (
              <div key={step.id} className="flex flex-col items-center flex-shrink-0 min-w-[70px]">
                <Button
                  variant={currentStep === index + 1 ? "default" : "secondary"}
                  className="size-8 rounded-full font-bold p-0"
                  disabled={currentStep === index + 1}
                  onClick={() => setCurrentStep(index + 1)}
                >
                  {index + 1}
                </Button>
                <span className="text-[10px] mt-1 text-muted-foreground line-clamp-1">{step.name}</span>
              </div>
            ))}
          </div>

          <div className="hidden md:block w-[240px] flex-shrink-0 px-8 py-4 border-r overflow-y-auto">
            {steps.map((step, index) => (
              <div className="mb-8 flex items-center" key={step.id}>
                <Button
                  variant={currentStep === index + 1 ? "default" : "secondary"}
                  className="size-9 rounded-full text-sm font-bold"
                  disabled={currentStep === index + 1}
                  onClick={() => setCurrentStep(index + 1)}
                >
                  {index + 1}
                </Button>
                <div className="flex flex-col items-baseline uppercase ml-4">
                  <span className="text-[10px] text-muted-foreground">{t('accounts.step', { index: index + 1 })}</span>
                  <span className={cn("font-bold text-sm tracking-wider", currentStep === index + 1 ? "text-foreground" : "text-muted-foreground")}>
                    {step.name}
                  </span>
                </div>
              </div>
            ))}
          </div>

          <div className="flex-1 min-h-0 relative">
            <ScrollArea className="h-full w-full">
              <div className="p-6 md:p-6 lg:p-8">
                <Form {...form}>
                  <form id="account-register-form" onSubmit={form.handleSubmit(onSubmit)}>
                    {currentStep === 1 && <Step1 isEdit={isEdit} />}
                    {currentStep === 2 && <Step2 isEdit={isEdit} />}
                    {currentStep === 3 && <Step3 />}
                    {currentStep === 4 && <Step4 />}
                  </form>
                </Form>
              </div>
            </ScrollArea>
          </div>
        </div>

        <DialogFooter className="p-4 md:p-6 bg-background flex flex-row sm:justify-end gap-2 flex-shrink-0">
          {currentStep > 1 && (
            <Button
              type="button"
              variant="outline"
              className="flex-1 sm:flex-none"
              onClick={() => setCurrentStep(currentStep - 1)}
            >
              {t('accounts.goBack')}
            </Button>
          )}
          {currentStep < LAST_STEP && (
            <Button
              type="button"
              className="flex-1 sm:flex-none px-8"
              onClick={handleContinue}
            >
              {autoConfigLoading ? t('accounts.autoConfiguring') : t('accounts.continue')}
            </Button>
          )}
          {currentStep === LAST_STEP && (
            <Button
              type="submit"
              form="account-register-form"
              className="flex-1 sm:flex-none px-10"
            >
              {isEdit ? t('accounts.saveChanges') : t('accounts.submit')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
