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

import { useState } from "react";
import { useFormContext } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { format } from "date-fns";
import { CalendarIcon } from "lucide-react";
import { enUS } from "date-fns/locale";
import {
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
  FormControl,
  FormDescription,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Calendar } from "@/components/ui/calendar";
import { cn, dateFnsLocaleMap } from "@/lib/utils";
import i18n from "@/i18n";
import { AccountFormValues } from "./schema";

type SyncMode = 'all' | 'since_fixed' | 'since_relative' | 'before_relative';
type ScheduleMode = 'interval' | 'cron';
type CronMode = 'simple' | 'advanced';
type CronFrequency = 'daily' | 'weekly' | 'monthly';

interface CronSimpleState {
  frequency: CronFrequency;
  hour: number;
  minute: number;
  dayOfWeek: number;
  dayOfMonth: number;
}

const DEFAULT_CRON_SIMPLE: CronSimpleState = {
  frequency: 'daily',
  hour: 0,
  minute: 0,
  dayOfWeek: 1,
  dayOfMonth: 1,
};

function buildCronFromSimple(s: CronSimpleState): string {
  switch (s.frequency) {
    case 'daily':
      return `0 ${s.minute} ${s.hour} * * *`;
    case 'weekly':
      return `0 ${s.minute} ${s.hour} * * ${s.dayOfWeek}`;
    case 'monthly':
      return `0 ${s.minute} ${s.hour} ${s.dayOfMonth} * *`;
  }
}

function tryParseCronToSimple(cron: string): CronSimpleState | null {
  const fields = cron.trim().split(/\s+/);
  if (fields.length < 6) return null;
  const sec = fields[0];
  const min = fields[1];
  const hour = fields[2];
  const dom = fields[3];
  const month = fields[4];
  const dow = fields[5];
  if (sec !== '0') return null;
  if (month !== '*') return null;
  const minuteVal = parseInt(min, 10);
  const hourVal = parseInt(hour, 10);
  if (isNaN(minuteVal) || isNaN(hourVal)) return null;
  if (dom === '*' && dow === '*') {
    return { frequency: 'daily', hour: hourVal, minute: minuteVal, dayOfWeek: 1, dayOfMonth: 1 };
  }
  if (dom === '*') {
    const dowVal = parseInt(dow, 10);
    if (!isNaN(dowVal)) {
      return { frequency: 'weekly', hour: hourVal, minute: minuteVal, dayOfWeek: dowVal, dayOfMonth: 1 };
    }
  }
  if (dow === '*') {
    const domVal = parseInt(dom, 10);
    if (!isNaN(domVal)) {
      return { frequency: 'monthly', hour: hourVal, minute: minuteVal, dayOfWeek: 1, dayOfMonth: domVal };
    }
  }
  return null;
}

export function TabDownload() {
  const { t } = useTranslation();
  const { control, getValues, setValue } = useFormContext<AccountFormValues>();
  const current = getValues();

  const [syncMode, setSyncMode] = useState<SyncMode>(() => {
    if (current.date_before) return 'before_relative';
    if (current.date_since?.fixed) return 'since_fixed';
    if (current.date_since?.relative) return 'since_relative';
    return 'all';
  });

  const [scheduleMode, setScheduleMode] = useState<ScheduleMode>(() => {
    if (current.download_schedule) return 'cron';
    return 'interval';
  });

  const [cronMode, setCronMode] = useState<CronMode>(() => {
    if (current.download_schedule && tryParseCronToSimple(current.download_schedule)) {
      return 'simple';
    }
    if (current.download_schedule) return 'advanced';
    return 'simple';
  });

  const [cronSimple, setCronSimple] = useState<CronSimpleState>(() => {
    if (current.download_schedule) {
      return tryParseCronToSimple(current.download_schedule) ?? DEFAULT_CRON_SIMPLE;
    }
    return DEFAULT_CRON_SIMPLE;
  });

  const updateCronFromSimple = (partial: Partial<CronSimpleState>) => {
    const next = { ...cronSimple, ...partial };
    setCronSimple(next);
    setValue('download_schedule', buildCronFromSimple(next));
  };

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

  const handleScheduleModeChange = (mode: ScheduleMode) => {
    setScheduleMode(mode);
    if (mode === 'interval') {
      setValue("download_schedule", undefined);
    } else {
      setValue("download_interval_min", 60);
      if (cronMode === 'simple') {
        updateCronFromSimple(cronSimple);
      }
    }
  };

  const BYTES_PER_MB = 1024 * 1024;

  return (
    <div className="space-y-8">
      {/* Schedule */}
      <div className="space-y-4">
        <h4 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          {t('accounts.settings.schedule')}
        </h4>

        <FormItem>
          <FormLabel>{t('accounts.scheduleMode')}</FormLabel>
          <FormDescription>{t('accounts.scheduleModeDescription')}</FormDescription>
          <Select value={scheduleMode} onValueChange={(v) => handleScheduleModeChange(v as ScheduleMode)}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="interval">{t('accounts.scheduleModeInterval')}</SelectItem>
              <SelectItem value="cron">{t('accounts.scheduleModeCron')}</SelectItem>
            </SelectContent>
          </Select>
        </FormItem>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {scheduleMode === 'interval' && (
            <FormField
              control={control}
              name="download_interval_min"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t('accounts.downloadInterval')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
                  <FormControl>
                    <Input type="number" {...field} onChange={(e) => field.onChange(parseInt(e.target.value, 10))} />
                  </FormControl>
                  <FormDescription>{t('accounts.downloadIntervalPlaceholder')}</FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />
          )}
        </div>

        {scheduleMode === 'cron' && (
          <div className="space-y-3">
              <div className="flex items-center gap-2">
                <div className="flex items-center rounded-md border text-xs">
                  <button
                    type="button"
                    className={`px-2 py-1 rounded-l-md ${cronMode === 'simple' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
                    onClick={() => setCronMode('simple')}
                  >
                    {t('accounts.cronSimple')}
                  </button>
                  <button
                    type="button"
                    className={`px-2 py-1 rounded-r-md ${cronMode === 'advanced' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}
                    onClick={() => setCronMode('advanced')}
                  >
                    {t('accounts.cronAdvanced')}
                  </button>
                </div>
              </div>

              {cronMode === 'simple' ? (
                <>
                  <div className="flex items-end gap-2 overflow-x-auto pb-1">
                    <FormItem className="w-[120px] shrink-0">
                      <FormLabel className="text-sm">{t('accounts.cronFrequency')}</FormLabel>
                      <Select
                        value={cronSimple.frequency}
                        onValueChange={(v) => updateCronFromSimple({ frequency: v as CronFrequency })}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="daily">{t('accounts.cronDaily')}</SelectItem>
                          <SelectItem value="weekly">{t('accounts.cronWeekly')}</SelectItem>
                          <SelectItem value="monthly">{t('accounts.cronMonthly')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </FormItem>

                    <FormItem className="w-[72px] shrink-0">
                      <FormLabel className="text-sm">{t('accounts.cronHour')}</FormLabel>
                      <Select
                        value={String(cronSimple.hour)}
                        onValueChange={(v) => updateCronFromSimple({ hour: parseInt(v, 10) })}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {Array.from({ length: 24 }, (_, i) => (
                            <SelectItem key={i} value={String(i)}>
                              {String(i).padStart(2, '0')}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </FormItem>

                    <span className="text-muted-foreground font-medium pb-2 shrink-0">:</span>

                    <FormItem className="w-[72px] shrink-0">
                      <FormLabel className="text-sm">{t('accounts.cronMinute')}</FormLabel>
                      <Select
                        value={String(cronSimple.minute)}
                        onValueChange={(v) => updateCronFromSimple({ minute: parseInt(v, 10) })}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {[0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55].map((m) => (
                            <SelectItem key={m} value={String(m)}>
                              {String(m).padStart(2, '0')}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </FormItem>

                    {cronSimple.frequency === 'weekly' && (
                      <FormItem className="w-[120px] shrink-0">
                        <FormLabel className="text-sm">{t('accounts.cronDayOfWeek')}</FormLabel>
                        <Select
                          value={String(cronSimple.dayOfWeek)}
                          onValueChange={(v) => updateCronFromSimple({ dayOfWeek: parseInt(v, 10) })}
                        >
                          <SelectTrigger>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="1">{t('accounts.cronMonday')}</SelectItem>
                            <SelectItem value="2">{t('accounts.cronTuesday')}</SelectItem>
                            <SelectItem value="3">{t('accounts.cronWednesday')}</SelectItem>
                            <SelectItem value="4">{t('accounts.cronThursday')}</SelectItem>
                            <SelectItem value="5">{t('accounts.cronFriday')}</SelectItem>
                            <SelectItem value="6">{t('accounts.cronSaturday')}</SelectItem>
                            <SelectItem value="0">{t('accounts.cronSunday')}</SelectItem>
                          </SelectContent>
                        </Select>
                      </FormItem>
                    )}

                    {cronSimple.frequency === 'monthly' && (
                      <FormItem className="w-[80px] shrink-0">
                        <FormLabel className="text-sm">{t('accounts.cronDayOfMonth')}</FormLabel>
                        <Select
                          value={String(cronSimple.dayOfMonth)}
                          onValueChange={(v) => updateCronFromSimple({ dayOfMonth: parseInt(v, 10) })}
                        >
                          <SelectTrigger>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent className="max-h-[200px]">
                            {Array.from({ length: 31 }, (_, i) => i + 1).map((d) => (
                              <SelectItem key={d} value={String(d)}>{d}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </FormItem>
                    )}
                  </div>

                  <p className="text-xs text-muted-foreground">
                    {cronSimple.frequency === 'daily' && t('accounts.cronSummaryDaily', { hour: String(cronSimple.hour).padStart(2, '0'), minute: String(cronSimple.minute).padStart(2, '0') })}
                    {cronSimple.frequency === 'weekly' && (() => {
                      const dayNames = ['cronSunday', 'cronMonday', 'cronTuesday', 'cronWednesday', 'cronThursday', 'cronFriday', 'cronSaturday'];
                      return t('accounts.cronSummaryWeekly', { hour: String(cronSimple.hour).padStart(2, '0'), minute: String(cronSimple.minute).padStart(2, '0'), day: t(`accounts.${dayNames[cronSimple.dayOfWeek]}`) });
                    })()}
                    {cronSimple.frequency === 'monthly' && t('accounts.cronSummaryMonthly', { hour: String(cronSimple.hour).padStart(2, '0'), minute: String(cronSimple.minute).padStart(2, '0'), day: cronSimple.dayOfMonth })}
                    {cronSimple.frequency === 'monthly' && cronSimple.dayOfMonth > 28 && (
                      <span className="block text-xs text-yellow-600 mt-0.5">{t('accounts.cronMonthAlignNote')}</span>
                    )}
                  </p>
                </>
              ) : (
                <FormField
                  control={control}
                  name="download_schedule"
                  render={({ field }) => (
                    <FormItem>
                      <FormControl>
                        <Input
                          {...field}
                          value={field.value ?? ''}
                          placeholder={t('accounts.downloadSchedulePlaceholder')}
                        />
                      </FormControl>
                      <FormDescription>{t('accounts.downloadScheduleDescription')}</FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              )}

              <div className="text-xs text-muted-foreground mt-1 flex items-center gap-1">
                <span>{t('accounts.cronTimezoneNote')}</span>
              </div>
            </div>
        )}
      </div>

      <hr />

      {/* Batch & Size */}
      <div className="space-y-4">
        <h4 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          {t('accounts.settings.performance')}
        </h4>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <FormField
            control={control}
            name="download_batch_size"
            render={({ field }) => (
              <FormItem>
                <FormLabel>{t('accounts.downloadBatchSize')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
                <FormControl>
                  <Input type="number" {...field} onChange={(e) => field.onChange(parseInt(e.target.value, 10))} />
                </FormControl>
                <FormDescription>{t('accounts.downloadBatchSizeDescription')}</FormDescription>
                <FormMessage />
              </FormItem>
            )}
          />
          <FormField
            control={control}
            name="max_email_size_bytes"
            render={({ field }) => (
              <FormItem>
                <FormLabel>{t('accounts.maxEmailSizeBytes')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
                <FormControl>
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      placeholder={t('accounts.maxEmailSizeBytesPlaceholder')}
                      className="flex-1"
                      value={field.value ? field.value / BYTES_PER_MB : ''}
                      onChange={(e) => {
                        const parsed = parseInt(e.target.value, 10);
                        field.onChange(isNaN(parsed) ? parsed : parsed * BYTES_PER_MB);
                      }}
                    />
                    <span className="text-sm text-muted-foreground whitespace-nowrap">MB</span>
                  </div>
                </FormControl>
                <FormDescription>{t('accounts.maxEmailSizeBytesDescription')}</FormDescription>
                <FormMessage />
              </FormItem>
            )}
          />
        </div>
      </div>

      <hr />

      {/* Download Scope */}
      <div className="space-y-4">
        <h4 className="text-sm font-semibold uppercase tracking-wider text-muted-foreground">
          {t('accounts.settings.scope')}
        </h4>

        <FormItem>
          <FormLabel>{t('accounts.downloadScope')}</FormLabel>
          <FormDescription>{t('accounts.downloadScopeDescription')}</FormDescription>
          <Select value={syncMode} onValueChange={(v) => handleModeChange(v as SyncMode)}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{t('accounts.downloadAll')}</SelectItem>
              <SelectItem value="since_fixed">{t('accounts.sinceFixed')}</SelectItem>
              <SelectItem value="since_relative">{t('accounts.sinceRelative')}</SelectItem>
              <SelectItem value="before_relative">{t('accounts.beforeRelative')}</SelectItem>
            </SelectContent>
          </Select>
          <FormDescription className="mt-2">
            {syncMode === 'all' && t('accounts.downloadAllDesc')}
            {syncMode === 'since_fixed' && t('accounts.sinceFixedDesc')}
            {syncMode === 'since_relative' && t('accounts.sinceRelativeDesc')}
            {syncMode === 'before_relative' && t('accounts.beforeRelativeDesc')}
          </FormDescription>
        </FormItem>

        <div className="pl-2 border-l-2 border-primary/20 space-y-4 pt-2">
          {syncMode === 'since_fixed' && (
            <FormField
              control={control}
              name="date_since.fixed"
              render={({ field }) => {
                const currentLang = i18n.language.toLowerCase().replace('_', '-');
                const dateLocale = dateFnsLocaleMap[currentLang] || enUS;
                return (
                  <FormItem className="flex flex-col">
                    <FormLabel>{t('accounts.selectDate')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
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
                  </FormItem>
                );
              }}
            />
          )}

          {(syncMode === 'since_relative' || syncMode === 'before_relative') && (
            <div className="flex flex-row items-end gap-4">
              <FormField
                control={control}
                name={syncMode === 'since_relative' ? "date_since.relative.value" : "date_before.value"}
                render={({ field }) => (
                  <FormItem className="flex-1 max-w-[150px]">
                    <FormLabel>{t('accounts.duration')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
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
                    <FormLabel>{t('accounts.unit', 'Unit')}<span className="text-red-500 align-super text-xs">*</span></FormLabel>
                    <Select onValueChange={field.onChange} value={field.value}>
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue />
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

      <hr />

      <FormField
        control={control}
        name="auto_download_new_mailboxes"
        render={({ field }) => (
          <FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4">
            <FormControl>
              <Checkbox checked={field.value} onCheckedChange={field.onChange} />
            </FormControl>
            <div className="space-y-1 leading-none">
              <FormLabel>{t('accounts.autoDownloadNewMailboxes')}</FormLabel>
              <FormDescription>{t('accounts.autoDownloadNewMailboxesDescription')}</FormDescription>
            </div>
          </FormItem>
        )}
      />
    </div>
  );
}
