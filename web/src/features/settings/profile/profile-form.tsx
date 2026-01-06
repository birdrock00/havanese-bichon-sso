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
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Loader2 } from 'lucide-react'
import { AxiosError } from 'axios'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { toast } from '@/hooks/use-toast'
import { PasswordInput } from '@/components/password-input'
import { update_user, User } from '@/api/users/api'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'

import { FileWithPreview } from '@/hooks/use-file-upload'
import AvatarUpload from './avatar-upload'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { PermissionsDialog } from './permissions-dialog'

const profileSchema = (t: (key: string) => string) => z.object({
  username: z
    .string({
      required_error: t('settings.profile.validation.username.required'),
    })
    .min(5, {
      message: t('settings.profile.validation.username.min'),
    })
    .max(32, {
      message: t('settings.profile.validation.username.max'),
    }),

  email: z
    .string({
      required_error: t('settings.profile.validation.email.required'),
    })
    .email({
      message: t('settings.profile.validation.email.invalid'),
    }),

  password: z
    .string()
    .min(8, {
      message: t('settings.profile.validation.password.min'),
    })
    .max(256, {
      message: t('settings.profile.validation.password.max'),
    })
    .or(z.literal(''))
    .optional()
    .transform((v) => (v ? v : undefined)),
})

export type ProfileFormValues = z.infer<ReturnType<typeof profileSchema>>

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const result = reader.result as string
      resolve(result.split(',')[1])
    }
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

interface UserProfileFormProps {
  user: User
}

export function UserProfileForm({ user }: UserProfileFormProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [avatarFile, setAvatarFile] = useState<FileWithPreview | null>(null)

  const [permissionsOpen, setPermissionsOpen] = useState(false)
  const [permissionsMode, setPermissionsMode] =
    useState<'global' | 'account'>('global')
  const [permissionsAccountId, setPermissionsAccountId] =
    useState<number | undefined>(undefined)

  const form = useForm<ProfileFormValues>({
    resolver: zodResolver(profileSchema(t)),
    mode: 'onChange',
    defaultValues: {
      username: user.username,
      email: user.email,
      password: '',
    },
  })

  const avatarSrc = user.avatar
    ? `data:image/png;base64,${user.avatar}`
    : undefined

  const mutation = useMutation({
    mutationFn: async (values: ProfileFormValues) => {
      let avatar_base64: string | undefined

      if (avatarFile?.file instanceof File) {
        avatar_base64 = await fileToBase64(avatarFile.file)
      }

      return update_user(user.id, {
        ...values,
        avatar_base64,
      })
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['current-user'] })
      toast({ title: t('settings.profile.toast.updated') })
    },
    onError: (err: AxiosError) => {
      toast({
        variant: 'destructive',
        title: t('settings.profile.toast.update_failed'),
        description: (err.response?.data as any)?.message || err.message,
      })
    },
  })

  const { getEmailById } = useMinimalAccountList()

  const accessibleAccountIds = user.account_access_map instanceof Map
    ? Array.from(user.account_access_map.keys())
    : Object.keys(user.account_access_map || {}).map(Number)

  const hasAccess = accessibleAccountIds.length > 0
  const roleNames = user.global_roles_names
  const roleSummary = user.account_roles_summary || {}

  return (
    <>
      <Form {...form}>
        <form
          onSubmit={form.handleSubmit((values) => mutation.mutate(values))}
          className="space-y-6 w-full max-w-screen-xl mx-auto px-4 md:px-6"
        >
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_auto_1fr] gap-6">
            <div className="space-y-6">
              <div className="flex justify-center">
                <AvatarUpload
                  onFileChange={setAvatarFile}
                  defaultAvatar={avatarSrc}
                  disabled={mutation.isPending}
                />
              </div>

              {roleNames && roleNames.length > 0 && (
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="text-sm font-medium text-muted-foreground">
                      {t('settings.profile.section.roles')}
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="text-xs"
                      onClick={() => {
                        setPermissionsMode('global')
                        setPermissionsAccountId(undefined)
                        setPermissionsOpen(true)
                      }}
                    >
                      {t('settings.profile.button.view_global_permissions')}
                    </Button>
                  </div>

                  <div className="flex flex-wrap gap-2">
                    {roleNames.map((role, index) => (
                      <Badge key={index} variant="secondary">
                        {role}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              <FormField
                control={form.control}
                name="username"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>
                      {t('settings.profile.field.username')}
                    </FormLabel>
                    <FormControl>
                      <Input {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="email"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>
                      {t('settings.profile.field.email')}
                    </FormLabel>
                    <FormControl>
                      <Input {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="password"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>
                      {t('settings.profile.field.password')}
                    </FormLabel>
                    <FormControl>
                      <PasswordInput
                        placeholder={t(
                          'settings.profile.placeholder.password_keep',
                        )}
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            {hasAccess && (
              <Separator orientation="vertical" className="hidden lg:block" />
            )}

            {hasAccess && (
              <div className="space-y-4">
                <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
                  {t('settings.profile.section.accounts', {
                    count: accessibleAccountIds.length,
                  })}
                </h2>

                <ScrollArea className="h-[calc(100vh-16rem)] pr-4">
                  <div className="grid grid-cols-1 gap-3">
                    {accessibleAccountIds.map((accountId) => {
                      const email = getEmailById(accountId)
                      const roleName = roleSummary[accountId]
                      if (!email) return null

                      return (
                        <div
                          key={accountId}
                          className="group flex items-center justify-between p-3 rounded-xl border bg-card hover:bg-accent/40 transition-all shadow-sm"
                        >
                          <div className="flex items-center min-w-0">
                            <div className="flex items-center justify-center w-10 h-10 rounded-full bg-primary/10 text-primary text-sm font-bold mr-4 shrink-0">
                              {email.charAt(0).toUpperCase()}
                            </div>
                            <div className="flex flex-col min-w-0">
                              <span className="text-sm font-semibold truncate">
                                {email}
                              </span>
                              <span className="text-[10px] text-muted-foreground font-mono">
                                {t('settings.profile.account.id', {
                                  id: accountId,
                                })}
                              </span>
                            </div>
                          </div>

                          <div className="flex items-center gap-2">
                            {roleName && (
                              <Badge
                                variant="outline"
                                className="text-[11px]"
                              >
                                {roleName}
                              </Badge>
                            )}

                            <Button
                              type="button"
                              variant="ghost"
                              className="text-[10px]"
                              onClick={() => {
                                setPermissionsMode('account')
                                setPermissionsAccountId(accountId)
                                setPermissionsOpen(true)
                              }}
                            >
                              {t('settings.profile.button.permissions')}
                            </Button>
                          </div>
                        </div>
                      )
                    })}
                  </div>
                </ScrollArea>
              </div>
            )}
          </div>

          <div className="flex justify-start pt-4">
            <Button
              type="submit"
              disabled={form.formState.isSubmitting || mutation.isPending}
            >
              {(form.formState.isSubmitting || mutation.isPending) && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('settings.profile.button.update_profile')}
            </Button>
          </div>
        </form>
      </Form>

      <PermissionsDialog
        currentRow={user}
        open={permissionsOpen}
        onOpenChange={setPermissionsOpen}
        mode={permissionsMode}
        accountId={permissionsAccountId}
      />
    </>
  )
}
