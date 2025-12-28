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

import React from 'react'
import { z } from 'zod'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Loader2, ShieldCheck, Users, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'

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
    FormField,
    FormItem,
    FormLabel,
    FormMessage,
} from '@/components/ui/form'
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select'
import { Checkbox } from '@/components/ui/checkbox'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Input } from '@/components/ui/input'
import { useToast } from '@/hooks/use-toast'
import { useRoles } from '@/hooks/use-roles'
import { useMinimalUsers } from '@/hooks/use-minimal-users'
import { access_assign, AccountModel } from '@/api/account/api'

interface Props {
    currentRow: AccountModel
    open: boolean
    onOpenChange: (open: boolean) => void
}

export function AccountAccessAssignmentDialog({
    currentRow,
    open,
    onOpenChange,
}: Props) {
    const { t } = useTranslation()
    const { toast } = useToast()
    const queryClient = useQueryClient()

    const { accountRoles, isLoading: isLoadingRoles } = useRoles()
    const { users, isLoading: isLoadingUsers } = useMinimalUsers()

    const [keyword, setKeyword] = React.useState('')

    // 1. 定义校验 Schema (集成国际化错误提示)
    const assignmentSchema = z.object({
        account_ids: z.array(z.number()),
        user_ids: z.array(z.number()).min(1, {
            message: t('accounts.access_control.validation.user_required'),
        }),
        role_id: z.number({
            required_error: t('accounts.access_control.validation.role_required'),
        }),
    })

    type AssignmentFormValues = z.infer<typeof assignmentSchema>

    const form = useForm<AssignmentFormValues>({
        resolver: zodResolver(assignmentSchema),
        defaultValues: {
            account_ids: [currentRow.id],
            user_ids: [],
            role_id: undefined as any,
        },
    })

    const filteredUsers = React.useMemo(() => {
        if (!keyword.trim()) return users
        const lowerKeyword = keyword.toLowerCase()
        return users.filter(
            (user) =>
                user.username.toLowerCase().includes(lowerKeyword) ||
                user.email.toLowerCase().includes(lowerKeyword)
        )
    }, [users, keyword])

    const { mutate, isPending } = useMutation({
        mutationFn: access_assign,
        onSuccess: () => {
            toast({
                title: t('accounts.access_control.toast.success_title'),
                description: t('accounts.access_control.toast.success_desc', { email: currentRow.email }),
            })
            queryClient.invalidateQueries({ queryKey: ['account-access-list'] })
            onOpenChange(false)
        },
        onError: (error: any) => {
            toast({
                variant: 'destructive',
                title: t('accounts.access_control.toast.failed_title'),
                description: error.response?.data?.message || error.message,
            })
        },
    })

    const onSubmit = (data: AssignmentFormValues) => {
        mutate(data)
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="max-w-md gap-0 p-0 overflow-hidden">
                <DialogHeader className="px-6 pt-6 pb-4">
                    <DialogTitle className="flex items-center gap-2">
                        <ShieldCheck className="w-5 h-5 text-blue-600" />
                        {t('accounts.access_control.title')}
                    </DialogTitle>
                    <DialogDescription>
                        {t('accounts.access_control.description', { email: currentRow.email })}
                    </DialogDescription>
                </DialogHeader>

                <Form {...form}>
                    <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                        <div className="px-6 space-y-6">
                            <FormField
                                control={form.control}
                                name="role_id"
                                render={({ field }) => (
                                    <FormItem>
                                        <FormLabel>{t('accounts.access_control.role_label')}</FormLabel>
                                        <Select
                                            disabled={isLoadingRoles}
                                            onValueChange={(value) => field.onChange(Number(value))}
                                            value={field.value?.toString()}
                                        >
                                            <FormControl>
                                                <SelectTrigger>
                                                    <SelectValue
                                                        placeholder={
                                                            isLoadingRoles
                                                                ? t('accounts.access_control.role_loading')
                                                                : t('accounts.access_control.role_placeholder')
                                                        }
                                                    />
                                                </SelectTrigger>
                                            </FormControl>
                                            <SelectContent>
                                                {accountRoles.map((role) => (
                                                    <SelectItem key={role.id} value={role.id.toString()}>
                                                        {role.name}
                                                    </SelectItem>
                                                ))}
                                            </SelectContent>
                                        </Select>
                                        <FormMessage />
                                    </FormItem>
                                )}
                            />

                            <div className="space-y-3">
                                <FormLabel className="flex items-center gap-2">
                                    <Users className="w-4 h-4" />
                                    {t('accounts.access_control.user_label')}
                                </FormLabel>

                                <div className="relative">
                                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                                    <Input
                                        placeholder={t('accounts.access_control.user_search_placeholder')}
                                        value={keyword}
                                        onChange={(e) => setKeyword(e.target.value)}
                                        className="pl-10"
                                    />
                                </div>

                                <div className="border rounded-md">
                                    <ScrollArea className="h-64">
                                        {isLoadingUsers ? (
                                            <div className="flex justify-center py-8">
                                                <Loader2 className="h-6 w-6 animate-spin" />
                                            </div>
                                        ) : (
                                            <div className="p-3 space-y-1">
                                                {filteredUsers.length === 0 ? (
                                                    <div className="text-center py-8 text-sm text-muted-foreground">
                                                        {t('accounts.access_control.user_empty')}
                                                    </div>
                                                ) : (
                                                    filteredUsers.map((user) => (
                                                        <FormField
                                                            key={user.id}
                                                            control={form.control}
                                                            name="user_ids"
                                                            render={({ field }) => (
                                                                <FormItem className="flex flex-row items-center space-x-3 space-y-0 rounded-md hover:bg-accent/50 px-2 py-2 transition-colors">
                                                                    <FormControl>
                                                                        <Checkbox
                                                                            checked={field.value?.includes(user.id) ?? false}
                                                                            onCheckedChange={(checked) => {
                                                                                if (checked) {
                                                                                    field.onChange([...(field.value ?? []), user.id])
                                                                                } else {
                                                                                    field.onChange(
                                                                                        field.value?.filter((id: number) => id !== user.id) ?? []
                                                                                    )
                                                                                }
                                                                            }}
                                                                        />
                                                                    </FormControl>
                                                                    <label className="flex-1 cursor-pointer select-none space-y-1">
                                                                        <div className="font-medium text-sm">{user.username}</div>
                                                                        <div className="text-xs text-muted-foreground">
                                                                            {user.email}
                                                                        </div>
                                                                    </label>
                                                                </FormItem>
                                                            )}
                                                        />
                                                    ))
                                                )}
                                            </div>
                                        )}
                                    </ScrollArea>
                                </div>

                                <FormMessage>{form.formState.errors.user_ids?.message}</FormMessage>

                                {form.watch('user_ids')?.length > 0 && (
                                    <div className="text-sm text-muted-foreground">
                                        {t('accounts.access_control.user_selected_count', { count: form.watch('user_ids').length })}
                                    </div>
                                )}
                            </div>
                        </div>

                        <DialogFooter className="bg-muted/50 px-6 py-4">
                            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                                {t('accounts.access_control.buttons.cancel')}
                            </Button>
                            <Button type="submit" disabled={isPending}>
                                {isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                                {t('accounts.access_control.buttons.save')}
                            </Button>
                        </DialogFooter>
                    </form>
                </Form>
            </DialogContent>
        </Dialog>
    )
}