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
import { useTranslation } from 'react-i18next'
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
import { ScrollArea } from '@/components/ui/scroll-area'
import { Input } from '@/components/ui/input'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { Loader2, Clock } from 'lucide-react'
import { AccessToken, create_access_token, update_access_token } from '@/api/users/api'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'


const getAccessTokenSchema = (t: any) => z.object({
  name: z
    .string()
    .max(32, t('apiTokens.form.errorMax'))
    .optional()
    .or(z.literal('')),
  expire_in: z
    .number({
      invalid_type_error: t('apiTokens.form.errorNumber'),
    })
    .int(t('apiTokens.form.errorInt'))
    .positive(t('apiTokens.form.errorPositive'))
    .optional(),
})

export type AccessTokenForm = z.infer<ReturnType<typeof getAccessTokenSchema>>

interface Props {
  currentRow?: AccessToken
  open: boolean
  userId: number
  onOpenChange: (open: boolean) => void
}

const defaultValues: AccessTokenForm = {
  name: '',
  expire_in: undefined,
}

export function TokensActionDialog({
  currentRow,
  open,
  onOpenChange,
  userId,
}: Props) {
  const { t } = useTranslation()
  const isEdit = !!currentRow
  const queryClient = useQueryClient()

  const form = useForm<AccessTokenForm>({
    resolver: zodResolver(getAccessTokenSchema(t)),
    defaultValues: isEdit
      ? {
        name: currentRow.name ?? '',
        expire_in: undefined,
      }
      : defaultValues,
  })

  const createMutation = useMutation({
    mutationFn: create_access_token,
    onSuccess: handleSuccess,
    onError: handleError,
  })

  const updateMutation = useMutation({
    mutationFn: (data: Record<string, any>) =>
      update_access_token(currentRow?.token ?? '', data),
    onSuccess: handleSuccess,
    onError: handleError,
  })

  function handleSuccess() {
    toast({
      title: isEdit
        ? t('apiTokens.notifications.updateSuccess')
        : t('apiTokens.notifications.createSuccess'),
      description: isEdit
        ? t('apiTokens.notifications.updateSuccessDesc')
        : t('apiTokens.notifications.createSuccessDesc'),
      action: (
        <ToastAction altText={t('apiTokens.notifications.close')}>
          {t('apiTokens.notifications.close')}
        </ToastAction>
      ),
    })

    queryClient.invalidateQueries({ queryKey: ['access-tokens'] })
    queryClient.invalidateQueries({ queryKey: ['user-tokens', userId] })
    form.reset(defaultValues)
    onOpenChange(false)
  }

  function handleError(error: AxiosError) {
    const errorMessage =
      (error.response?.data as { message?: string })?.message ||
      error.message ||
      t('apiTokens.notifications.genericError')

    toast({
      variant: 'destructive',
      title: isEdit
        ? t('apiTokens.notifications.updateFailed')
        : t('apiTokens.notifications.createFailed'),
      description: errorMessage,
      action: (
        <ToastAction altText={t('apiTokens.notifications.tryAgain')}>
          {t('apiTokens.notifications.tryAgain')}
        </ToastAction>
      ),
    })

    console.error(error)
  }

  const onSubmit = (values: AccessTokenForm) => {
    const payload = {
      user_id: userId,
      name: values.name?.trim() || undefined,
      expire_in: values.expire_in || undefined,
    }

    isEdit
      ? updateMutation.mutate(payload)
      : createMutation.mutate(payload)
  }

  const isPending = isEdit
    ? updateMutation.isPending
    : createMutation.isPending

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        form.reset(defaultValues)
        onOpenChange(state)
      }}
    >
      <DialogContent className="max-w-xl">
        <DialogHeader className="text-left mb-4">
          <DialogTitle>
            {isEdit ? t('apiTokens.dialog.editTitle') : t('apiTokens.dialog.createTitle')}
          </DialogTitle>
          <DialogDescription>
            {t('apiTokens.dialog.description')}
          </DialogDescription>
        </DialogHeader>

        <ScrollArea className="h-[14rem] w-full pr-4 -mr-4">
          <Form {...form}>
            <form
              id="token-form"
              onSubmit={form.handleSubmit(onSubmit)}
              className="space-y-6"
            >
              {/* name */}
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t('apiTokens.form.nameLabel')}</FormLabel>
                    <FormControl>
                      <Input
                        {...field}
                        maxLength={32}
                        placeholder={t('apiTokens.form.namePlaceholder')}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('apiTokens.form.nameDescription')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              {/* expire_in */}
              <FormField
                control={form.control}
                name="expire_in"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="flex items-center gap-2">
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      {t('apiTokens.form.expirationLabel')}
                    </FormLabel>

                    <Select
                      value={field.value?.toString() ?? 'never'}
                      onValueChange={(value) => {
                        field.onChange(
                          value === 'never' ? undefined : Number(value)
                        )
                      }}
                    >
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder={t('apiTokens.form.expirationPlaceholder')} />
                        </SelectTrigger>
                      </FormControl>

                      <SelectContent>
                        <SelectItem value="never">{t('apiTokens.form.never')}</SelectItem>
                        <SelectItem value="24">{t('apiTokens.form.1day')}</SelectItem>
                        <SelectItem value="168">{t('apiTokens.form.7days')}</SelectItem>
                        <SelectItem value="720">{t('apiTokens.form.30days')}</SelectItem>
                      </SelectContent>
                    </Select>

                    <FormDescription>
                      {t('apiTokens.form.expirationDescription')}
                    </FormDescription>
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
            disabled={isPending}
            className="min-w-[120px]"
          >
            {isPending && (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            )}
            {isEdit ? t('apiTokens.dialog.saveChanges') : t('apiTokens.dialog.save')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}