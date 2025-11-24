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


import { HTMLAttributes, useState } from 'react'
import { z } from 'zod'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { cn } from '@/lib/utils'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { PasswordInput } from '@/components/password-input'
import { useMutation } from '@tanstack/react-query'
import { login } from '@/api/access-tokens/api'
import { setAccessToken } from '@/stores/authStore'
import { toast } from '@/hooks/use-toast'
import { AxiosError } from 'axios'
import { ToastAction } from '@/components/ui/toast'
import { useLocation, useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/button'
import { useTranslation } from 'react-i18next'
import i18n from '@/i18n'

type UserAuthFormProps = HTMLAttributes<HTMLDivElement>

const getFormSchema = (t: (key: string, options?: Record<string, any>) => string) => z.object({
  username: z
    .string(),
  password: z
    .string()
    .min(1, { message: t('validation.pleaseEnterPassword') })
    .min(4, { message: t('validation.passwordMinLength', { min: 4 }) }),
});

export function UserAuthForm({ className, ...props }: UserAuthFormProps) {
  const [isLoading, setIsLoading] = useState(false)
  const navigate = useNavigate()
  const { t } = useTranslation()

  const { search } = useLocation();
  const redirect = new URLSearchParams(search).get('redirect') || '/';

  const formSchema = getFormSchema(t)
  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      username: 'root',
      password: '',
    },
  })

  const mutation = useMutation({
    mutationFn: (password: string) => login(password),
    retry: 0,
  });

  async function onSubmit(data: z.infer<typeof formSchema>) {
    setIsLoading(true)

    mutation.mutate(data.password, {
      onSuccess: (rootToken) => {
        setAccessToken(rootToken);
        setIsLoading(false);
        navigate({ to: redirect });
      },
      onError: (error) => {
        const { t } = i18n
        if (error instanceof AxiosError && error.response && error.response.status === 401) {
          toast({
            variant: "destructive",
            title: t('auth.loginFailed'),
            description: t('auth.invalidPassword'),
            action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
          })
        } else {
          toast({
            variant: "destructive",
            title: t('auth.somethingWentWrong'),
            description: (error as Error).message,
            action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
          })
        }
        setIsLoading(false)
      }
    });
  }

  return (
    <div className={cn('grid gap-6', className)} {...props}>
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)}>
          <div className='grid gap-2'>
            <FormField
              control={form.control}
              name='username'
              render={({ field }) => (
                <FormItem className='space-y-1'>
                  <FormLabel>{t('auth.username')}</FormLabel>
                  <FormControl>
                    <Input disabled {...field} value={"root"} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name='password'
              render={({ field }) => (
                <FormItem className='space-y-1'>
                  <div className='flex items-center justify-between'>
                    <FormLabel>{t('auth.password')}</FormLabel>
                  </div>
                  <FormControl>
                    <PasswordInput placeholder='********' {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <Button className='mt-2' loading={isLoading}>
              {t('auth.login')}
            </Button>
          </div>
        </form>
      </Form>
    </div>
  )
}