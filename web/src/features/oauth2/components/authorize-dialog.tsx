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


import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { OAuth2Entity } from '../data/schema'
import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { VirtualizedSelect } from '@/components/virtualized-select'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { useMutation } from '@tanstack/react-query'
import { get_authorize_url } from '@/api/oauth2/api'
import { toast } from '@/hooks/use-toast'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow: OAuth2Entity
  open: boolean
  onOpenChange: (open: boolean) => void
}


export function AuthorizeDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [accountId, setAccountId] = useState<number | null>(null)
  const { accountsOptions, minimalList, isLoading } = useMinimalAccountList();


  const authorizeMutation = useMutation({
    mutationFn: () => get_authorize_url({ account_id: accountId, oauth2_id: currentRow.id }),
    onSuccess: handleSuccess,
    onError: handleError
  });


  function handleSuccess(url: string) {
    window.open(url, '_blank');
    onOpenChange(false);
  }
  function handleError(error: AxiosError) {
    const errorMessage = (error.response?.data as { message?: string })?.message ||
      error.message ||
      `get authorize url failed, please try again later`;

    toast({
      variant: "destructive",
      title: t('oauth2.getAuthorizeUrlFailed'),
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }


  function doAuthorize() {
    authorizeMutation.mutate();
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        onOpenChange(state);
      }}
    >
      <DialogContent className='sm:max-w-lg' autoFocus>
        <DialogHeader className='text-left'>
          <DialogTitle>{t('oauth2.authorizeEmailAccount')}</DialogTitle>
          <DialogDescription>
            {t('oauth2.authorizeAnEmailAccountToStartTheOAuth2AuthorizationProcess')}
          </DialogDescription>
        </DialogHeader>
        <div className='flex flex-col space-y-4 h-24'>
          {isLoading && <div className="flex justify-center items-center h-full">
            <div>{t('oauth2.loadingAccounts')}</div>
          </div>}
          {!isLoading && minimalList && minimalList.length > 0 && (
            <div className="flex justify-start items-start h-full ml-4">
              <VirtualizedSelect
                isLoading={isLoading}
                className='w-full'
                options={accountsOptions}
                onSelectOption={(values) => setAccountId(parseInt(values[0], 10))}
                placeholder={t('oauth2.selectAnAccount')}
              />
            </div>
          )}
          {!isLoading && minimalList?.length === 0 && (
            <div className="flex flex-col items-center justify-center h-full">
              <p className="mb-4">{t('oauth2.noEmailAccountsRegistered')}</p>
              <Button onClick={() => {
                navigate({ to: '/accounts' })
              }}>
                {t('oauth2.createAccount')}
              </Button>
            </div>
          )}
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button variant='outline' className="px-2 py-1 text-sm h-auto">{t('common.close')}</Button>
          </DialogClose>
          {!isLoading && minimalList && minimalList.length > 0 && <Button disabled={!accountId} onClick={doAuthorize}>{t('oauth2.authorize')}</Button>}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
