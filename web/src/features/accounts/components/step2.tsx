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


import {
    FormField,
    FormItem,
    FormLabel,
    FormMessage,
    FormControl,
    FormDescription,
} from "@/components/ui/form";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { useFormContext, useWatch } from "react-hook-form";
import { Account } from "./action-dialog";
import { PasswordInput } from "@/components/password-input";
import useProxyList from "@/hooks/use-proxy";
import { useTranslation } from "react-i18next";

interface StepProps {
    isEdit: boolean;
}

export default function Step2({ isEdit }: StepProps) {
    const { t } = useTranslation()
    const { control } = useFormContext<Account>();
    const { proxyOptions } = useProxyList();
    const imapAuthMethod = useWatch({
        control,
        name: "imap.auth.auth_type",
    });

    return (
        <>
            <div className="space-y-8">
                <FormField
                    control={control}
                    name="imap.host"
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel className="flex items-center justify-between">
                                {t('accounts.imapHost')}:
                            </FormLabel>
                            <FormControl>
                                <Input placeholder={t('accounts.imapHostPlaceholder')} {...field} />
                            </FormControl>
                            <FormMessage />
                        </FormItem>
                    )}
                />
                <FormField
                    control={control}
                    name="imap.port"
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel className="flex items-center justify-between">
                                {t('accounts.imapPort')}:
                            </FormLabel>
                            <FormControl>
                                <Input type="number" placeholder={t('accounts.imapPortPlaceholder')} {...field} onChange={(e) => field.onChange(parseInt(e.target.value, 10))} />
                            </FormControl>
                            <FormMessage />
                        </FormItem>
                    )}
                />
                <FormField
                    control={control}
                    name="imap.encryption"
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel>{t('accounts.imapEncryption')}:</FormLabel>
                            <Select onValueChange={field.onChange} defaultValue={field.value}>
                                <FormControl>
                                    <SelectTrigger>
                                        <SelectValue placeholder={t('accounts.selectEncryptionMethod')} />
                                    </SelectTrigger>
                                </FormControl>
                                <SelectContent>
                                    <SelectItem value="Ssl">Ssl</SelectItem>
                                    <SelectItem value="StartTls">StartTls</SelectItem>
                                    <SelectItem value="None">None</SelectItem>
                                </SelectContent>
                            </Select>
                            <FormDescription>
                                {t('accounts.chooseEncryptionMethod')}
                            </FormDescription>
                            <FormMessage />
                        </FormItem>
                    )}
                />
                <FormField
                    control={control}
                    name="imap.auth.auth_type"
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel>{t('accounts.imapAuthMethod')}:</FormLabel>
                            <Select onValueChange={field.onChange} defaultValue={field.value}>
                                <FormControl>
                                    <SelectTrigger>
                                        <SelectValue placeholder={t('accounts.selectAuthMethod')} />
                                    </SelectTrigger>
                                </FormControl>
                                <SelectContent>
                                    <SelectItem value="OAuth2">OAuth2</SelectItem>
                                    <SelectItem value="Password">Password</SelectItem>
                                </SelectContent>
                            </Select>
                            <FormDescription>
                                {t('accounts.chooseAuthMethod')}
                            </FormDescription>
                            <FormMessage />
                        </FormItem>
                    )}
                />
                {imapAuthMethod === "Password" && (
                    <FormField
                        control={control}
                        name="imap.auth.password"
                        render={({ field }) => (
                            <FormItem>
                                <FormLabel className="flex items-center justify-between">
                                    {t('accounts.imapPassword')}:
                                </FormLabel>
                                <FormControl>
                                    <PasswordInput placeholder={isEdit ? t('accounts.leaveEmptyToKeepPassword') : t('accounts.enterPassword')} {...field} />
                                </FormControl>
                                <FormMessage />
                                {isEdit && (
                                    <FormDescription>
                                        {t('accounts.leaveEmptyToKeepExisting')}
                                    </FormDescription>
                                )}
                            </FormItem>
                        )}
                    />
                )}
                <FormField
                    control={control}
                    name='imap.use_proxy'
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel className="flex items-center justify-between">{t('accounts.useProxy')} ({t('accounts.optional')}):</FormLabel>
                            <FormControl>
                                <Select
                                    onValueChange={(val) => field.onChange(Number(val))}
                                    defaultValue={field.value?.toString()}
                                >
                                    <FormControl>
                                        <SelectTrigger>
                                            <SelectValue placeholder={t('accounts.selectProxy')} />
                                        </SelectTrigger>
                                    </FormControl>
                                    <SelectContent>
                                        {proxyOptions && proxyOptions.length > 0 ? (
                                            proxyOptions.map((option) => (
                                                <SelectItem key={option.value} value={option.value.toString()}>
                                                    {option.label}
                                                </SelectItem>
                                            ))
                                        ) : (
                                            <SelectItem disabled value="__none__">{t('settings.noProxies')}</SelectItem>
                                        )}
                                    </SelectContent>
                                </Select>
                            </FormControl>
                            <FormDescription className='flex-1'>
                                {t('accounts.imapProxy')}
                            </FormDescription>
                            <FormMessage />
                        </FormItem>
                    )}
                />
            </div>
        </>
    );
}