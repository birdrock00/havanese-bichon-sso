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
import { useFormContext } from "react-hook-form";
import { Account } from "./action-dialog";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Button } from "@/components/ui/button";
import { format } from "date-fns";
import { CalendarIcon } from "lucide-react";
import { Calendar } from "@/components/ui/calendar";
import { cn, dateFnsLocaleMap } from "@/lib/utils";
import { useState } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { useTranslation } from "react-i18next";
import { enUS } from "date-fns/locale";
import i18n from "@/i18n";


type SyncMode = 'all' | 'since_fixed' | 'since_relative' | 'before_relative';

export default function Step3() {
    const { t } = useTranslation();
    const { control, getValues, setValue } = useFormContext<Account>();
    const current = getValues();

    const [syncMode, setSyncMode] = useState<SyncMode>(() => {
        if (current.date_before) return 'before_relative';
        if (current.date_since?.fixed) return 'since_fixed';
        if (current.date_since?.relative) return 'since_relative';
        return 'all';
    });


    const handleModeChange = (mode: SyncMode) => {
        setSyncMode(mode);

        setValue("date_since", undefined);
        setValue("date_before", undefined);

        if (mode === 'since_fixed') {
            setValue("date_since.fixed", undefined);
        } else if (mode === 'since_relative') {
            setValue("date_since.relative", { value: 1, unit: 'Months' });
        } else if (mode === 'before_relative') {
            setValue("date_before", { value: 1, unit: 'Years' });
        }
    };

    return (
        <div className="space-y-8">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <FormField
                    control={control}
                    name="sync_interval_min"
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel>{t('accounts.incrementalSync')}</FormLabel>
                            <FormControl>
                                <Input type="number" {...field} onChange={(e) => field.onChange(parseInt(e.target.value, 10))} />
                            </FormControl>
                            <FormMessage />
                            <FormDescription>
                                {t('accounts.incrementalSyncDescription')}
                            </FormDescription>
                        </FormItem>
                    )}
                />
                <FormField
                    control={control}
                    name="sync_batch_size"
                    render={({ field }) => (
                        <FormItem>
                            <FormLabel>{t('accounts.syncBatchSize')}</FormLabel>
                            <FormControl>
                                <Input type="number" {...field} onChange={(e) => field.onChange(parseInt(e.target.value, 10))} />
                            </FormControl>
                            <FormMessage />
                            <FormDescription>
                                {t('accounts.syncBatchSizeDescription')}
                            </FormDescription>
                        </FormItem>
                    )}
                />
            </div>

            <FormField
                control={control}
                name="enabled"
                render={({ field }) => (
                    <FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4 shadow-sm">
                        <FormControl>
                            <Checkbox checked={field.value} onCheckedChange={field.onChange} />
                        </FormControl>
                        <div className="space-y-1 leading-none">
                            <FormLabel>{t('accounts.enabled')}</FormLabel>
                            <FormDescription>{t('accounts.enabledDescription')}</FormDescription>
                        </div>
                    </FormItem>
                )}
            />

            <hr className="my-4" />
            <div className="space-y-4">
                <FormItem>
                    <FormLabel className="text-base font-semibold">{t('accounts.syncScope', 'Sync Strategy')}</FormLabel>
                    <FormDescription>
                        {t('accounts.syncScopeDescription', 'Choose which emails should be indexed and archived.')}
                    </FormDescription>
                    <Select value={syncMode} onValueChange={(v) => handleModeChange(v as SyncMode)}>
                        <SelectTrigger className="w-full">
                            <SelectValue placeholder={t('accounts.selectMode')} />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="all">{t('accounts.syncAll', 'Sync All Emails')}</SelectItem>
                            <SelectItem value="since_fixed">{t('accounts.sinceFixed', 'Since Specific Date')}</SelectItem>
                            <SelectItem value="since_relative">{t('accounts.sinceRelative', 'Keep Recent Emails')}</SelectItem>
                            <SelectItem value="before_relative">{t('accounts.beforeRelative', 'Archive Old Emails Only')}</SelectItem>
                        </SelectContent>
                    </Select>
                </FormItem>
                <div className="pl-2 border-l-2 border-primary/20 space-y-4 pt-2">
                    {syncMode === 'since_fixed' && (
                        <FormField
                            control={control}
                            name="date_since.fixed"
                            render={({ field }) => {
                                const currentLang = i18n.language.toLowerCase().replace('_', '-');
                                const dateLocale = dateFnsLocaleMap[currentLang] || enUS;
                                return <FormItem className="flex flex-col">
                                    <FormLabel>{t('accounts.selectDate')}</FormLabel>
                                    <Popover>
                                        <PopoverTrigger asChild>
                                            <FormControl>
                                                <Button
                                                    variant="outline"
                                                    className={cn("w-[440px] pl-3 text-left font-normal", !field.value && "text-muted-foreground")}
                                                >
                                                    {field.value ? format(new Date(field.value), "PPP", { locale: dateLocale }) : <span>{t('accounts.selectDate')}</span>}
                                                    <CalendarIcon className="ml-auto h-4 w-4 opacity-50" />
                                                </Button>
                                            </FormControl>
                                        </PopoverTrigger>
                                        <PopoverContent className="w-auto p-0" align="start">
                                            <Calendar
                                                mode="single"
                                                selected={field.value ? new Date(field.value) : undefined}
                                                onSelect={(date) => field.onChange(date?.toLocaleDateString('en-CA'))}
                                                disabled={(date) => date > new Date() || date < new Date("1900-01-01")}
                                                locale={dateLocale}
                                            />
                                        </PopoverContent>
                                    </Popover>
                                    <FormMessage />
                                </FormItem>;

                            }}
                        />
                    )}

                    {(syncMode === 'since_relative' || syncMode === 'before_relative') && (
                        <div className="flex flex-row items-end gap-4 animate-in fade-in slide-in-from-left-2">
                            <FormField
                                control={control}
                                name={syncMode === 'since_relative' ? "date_since.relative.value" : "date_before.value"}
                                render={({ field }) => (
                                    <FormItem className="flex-1 max-w-[150px]">
                                        <FormLabel>{t('accounts.duration', 'Duration')}</FormLabel>
                                        <FormControl>
                                            <Input type="number" {...field} onChange={(e) => field.onChange(parseInt(e.target.value, 10))} />
                                        </FormControl>
                                        <FormMessage />
                                    </FormItem>
                                )}
                            />
                            <FormField
                                control={control}
                                name={syncMode === 'since_relative' ? "date_since.relative.unit" : "date_before.unit"}
                                render={({ field }) => (
                                    <FormItem className="w-[180px]">
                                        <FormLabel>{t('accounts.unit', 'Unit')}</FormLabel>
                                        <Select onValueChange={field.onChange} value={field.value}>
                                            <FormControl>
                                                <SelectTrigger>
                                                    <SelectValue placeholder={t('accounts.selectUnit')} />
                                                </SelectTrigger>
                                            </FormControl>
                                            <SelectContent>
                                                <SelectItem value="Days">{t('accounts.days')}</SelectItem>
                                                <SelectItem value="Months">{t('accounts.months')}</SelectItem>
                                                <SelectItem value="Years">{t('accounts.years')}</SelectItem>
                                            </SelectContent>
                                        </Select>
                                        <FormMessage />
                                    </FormItem>
                                )}
                            />
                        </div>
                    )}
                </div>
            </div>

            <hr className="my-4" />

            <FormField
                control={control}
                name="folder_limit"
                render={({ field }) => (
                    <FormItem>
                        <FormLabel>{t('accounts.folderLimit')}</FormLabel>
                        <FormDescription>{t('accounts.folderLimitDescription')}</FormDescription>
                        <FormControl>
                            <Input
                                type="number"
                                placeholder={t('accounts.folderLimitPlaceholder')}
                                value={field.value ?? ''}
                                onChange={(e) => {
                                    const value = e.target.value;
                                    field.onChange(value === '' ? null : Number(value));
                                }}
                            />
                        </FormControl>
                        <FormMessage />
                    </FormItem>
                )}
            />
        </div>
    );
}