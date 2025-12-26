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
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { AxiosError } from 'axios'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Loader2, LockIcon, ShieldCheck, UserCog } from 'lucide-react'
import { create_role, getPermissions, update_role, UserRole } from '@/api/users/api'
import { Textarea } from '@/components/ui/textarea'
import { Checkbox } from '@/components/ui/checkbox'
import {
  RadioGroup,
  RadioGroupItem,
} from '@/components/ui/radio-group'
import { cn } from '@/lib/utils'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow?: UserRole
  open: boolean
  onOpenChange: (open: boolean) => void
}

const CATEGORY_MAP: Record<'Global' | 'Account', { titleKey: string; keys: string[] }[]> = {
  Global: [
    {
      titleKey: 'roles.categories.identity',
      keys: ['system:access', 'system:root', 'user:manage', 'user:view', 'token:manage', 'account:create'],
    },
    {
      titleKey: 'roles.categories.global_data',
      keys: [
        'account:manage:all',
        'data:read:all',
        'data:manage:all',
        'data:raw:download:all',
        'data:delete:all',
        'data:export:batch:all',
      ],
    },
  ],
  Account: [
    {
      titleKey: 'roles.categories.account_resource',
      keys: [
        'account:manage',
        'account:read_details',
        'data:read',
        'data:manage',
        'data:raw:download',
        'data:delete',
        'data:export:batch',
        'data:import:batch',
      ],
    },
  ],
}

export function RoleActionDialog({ currentRow, open, onOpenChange }: Props) {
  const isEdit = !!currentRow
  const queryClient = useQueryClient()
  const { t } = useTranslation()

  const roleFormSchema = z.object({
    name: z.string().min(1, t('roles.validation.name_required')),
    role_type: z.enum(['Global', 'Account']),
    permissions: z.array(z.string()).min(1, t('roles.validation.perm_required')),
    description: z.string().optional(),
  })

  type RoleForm = z.infer<typeof roleFormSchema>

  const form = useForm<RoleForm>({
    resolver: zodResolver(roleFormSchema),
    defaultValues: {
      name: isEdit ? currentRow.name : '',
      role_type: isEdit ? currentRow.role_type : 'Account',
      permissions: isEdit ? Array.from(currentRow.permissions) : [],
      description: isEdit ? currentRow.description ?? undefined : '',
    },
  })

  const mutation = useMutation({
    mutationFn: (values: RoleForm) =>
      isEdit ? update_role(currentRow!.id, values) : create_role(values),
    onSuccess: () => {
      toast({ title: t(isEdit ? 'roles.actions.success_update' : 'roles.actions.success_create') })
      queryClient.invalidateQueries({ queryKey: ['role-list'] })
      onOpenChange(false)
    },
    onError: (error: AxiosError) => {
      toast({
        variant: 'destructive',
        title: t('roles.actions.failed'),
        description: error.message,
      })
    },
  })

  const selectedType = form.watch('role_type')

  const handleOpenChange = (v: boolean) => {
    if (!v) {
      form.reset()
    }
    onOpenChange(v)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-7xl w-[95vw] max-h-[90vh] flex flex-col p-0 overflow-hidden">
        <div className="p-6 border-b bg-white">
          <DialogHeader>
            <DialogTitle>
              {isEdit ? t('roles.title.edit', { name: currentRow?.name }) : t('roles.title.create')}
            </DialogTitle>
            <DialogDescription>
              {t('roles.description_hint')}
            </DialogDescription>
          </DialogHeader>
        </div>

        <Form {...form}>
          <form
            id="role-form"
            onSubmit={form.handleSubmit((v) => mutation.mutate(v))}
            className="flex-1 overflow-y-auto p-6 bg-slate-50/50"
          >
            <div className="grid grid-cols-1 lg:grid-cols-4 gap-8">
              <div className="lg:col-span-1 space-y-6">
                <FormField
                  control={form.control}
                  name="name"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[11px] font-bold text-slate-500 uppercase">
                        {t('roles.form.name_label')}
                      </FormLabel>
                      <FormControl>
                        <Input {...field} className="bg-white" />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="role_type"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[11px] font-bold text-slate-500 uppercase">
                        {t('roles.form.type_label')}
                      </FormLabel>
                      <FormControl>
                        <RadioGroup
                          value={field.value}
                          onValueChange={(v) => {
                            if (isEdit) return
                            field.onChange(v)
                            form.setValue('permissions', [])
                          }}
                          className="space-y-2"
                        >
                          {(['Global', 'Account'] as const).map((type) => {
                            const isSelected = field.value === type
                            const disabled = isEdit && !isSelected

                            return (
                              <label
                                key={type}
                                className={cn(
                                  'flex items-center justify-between p-3 rounded-md border-2 transition-all',
                                  isSelected
                                    ? 'border-primary bg-primary/5 shadow-sm'
                                    : 'border-slate-200 bg-white',
                                  disabled
                                    ? 'opacity-40 cursor-not-allowed'
                                    : 'cursor-pointer hover:shadow-sm'
                                )}
                              >
                                <div className="flex items-center gap-2 text-sm font-bold">
                                  {type === 'Global'
                                    ? <ShieldCheck className="w-4 h-4 text-primary" />
                                    : <UserCog className="w-4 h-4 text-primary" />}
                                  {t(`roles.types.${type}`)}
                                </div>
                                <RadioGroupItem value={type} disabled={disabled} />
                              </label>
                            )
                          })}
                        </RadioGroup>
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="description"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[11px] font-bold text-slate-500 uppercase">
                        {t('roles.form.desc_label')}
                      </FormLabel>
                      <FormControl>
                        <Textarea {...field} className="bg-white min-h-[120px]" />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>

              <div className="lg:col-span-3">
                <FormField
                  control={form.control}
                  name="permissions"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="mb-4 block border-b pb-2">
                        <div className="flex items-center justify-between">
                          <span className="text-[11px] font-bold text-slate-500 uppercase">
                            {t('roles.form.matrix_label', { type: t(`roles.types.${selectedType}`) })}
                          </span>

                          {currentRow && currentRow.is_builtin && (
                            <LockIcon className="h-3 w-3 text-muted-foreground shrink-0" />
                          )}
                        </div>
                      </FormLabel>
                      {currentRow?.is_builtin && (
                        <div className="col-span-full px-4 pt-1 pb-2">
                          <div className="flex items-start gap-3 p-4 rounded-xl border bg-secondary/30 w-full">
                            <LockIcon className="h-5 w-5 mt-0.5 text-muted-foreground shrink-0" />
                            <div>
                              <h4 className="text-sm font-semibold italic text-foreground/80">
                                {t('roles.form.builtin_badge')}
                              </h4>
                              <p className="text-xs text-muted-foreground mt-1 leading-relaxed">
                                {t('roles.form.builtin_desc')}
                              </p>
                            </div>
                          </div>
                        </div>
                      )}
                      <div
                        className={cn(
                          'grid gap-8',
                          selectedType === 'Global'
                            ? 'grid-cols-1 md:grid-cols-2'
                            : 'grid-cols-1'
                        )}
                      >
                        {CATEGORY_MAP[selectedType].map((cat) => (
                          <div key={cat.titleKey} className="space-y-4">
                            <h3 className="text-[11px] font-black text-slate-400 uppercase tracking-widest">
                              {t(cat.titleKey)}
                            </h3>

                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                              {cat.keys.map((key) => {
                                const item = getPermissions(t).find(p => p.value === key)
                                if (!item) return null

                                const checked = field.value.includes(item.value)

                                return (
                                  <label
                                    key={item.value}
                                    className={cn(
                                      'flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition',
                                      checked
                                        ? 'bg-white border-primary/30 shadow-sm'
                                        : 'bg-slate-50/50 border-slate-100 opacity-70'
                                    )}
                                  >
                                    <Checkbox
                                      checked={checked}
                                      onCheckedChange={(v) => {
                                        field.onChange(
                                          v
                                            ? [...field.value, item.value]
                                            : field.value.filter(x => x !== item.value)
                                        )
                                      }}
                                    />
                                    <div>
                                      <div className="text-xs font-bold">{item.label}</div>
                                      <code className="text-[10px] text-slate-400">
                                        {item.value}
                                      </code>
                                    </div>
                                  </label>
                                )
                              })}
                            </div>
                          </div>
                        ))}
                      </div>

                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
            </div>
          </form>
        </Form>

        <div className="p-4 border-t bg-white flex justify-end gap-3">
          <Button variant="outline" size="sm" onClick={() => handleOpenChange(false)}>
            {t('roles.actions.cancel')}
          </Button>
          <Button
            type="submit"
            form="role-form"
            size="sm"
            disabled={mutation.isPending}
            className="px-8 font-bold"
          >
            {mutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {isEdit ? t('roles.actions.submit_update') : t('roles.actions.submit_create')}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}