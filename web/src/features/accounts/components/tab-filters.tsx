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

import { useFormContext, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
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
import { Button } from "@/components/ui/button";
import { Plus, HelpCircle } from "lucide-react";
import { PatternInput } from "./pattern-input";
import type { PatternEntry } from "@/lib/pattern-utils";
import { newPatternId, simplePatternToRegex } from "@/lib/pattern-utils";
import { useState } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { AccountFormValues } from "./schema";

const SUGGESTED_SPAM_HEADERS = [
  'X-Spam-Flag',
  'X-Spam',
  'X-Spam-Status',
  'X-Barracuda-Spam-Status',
  'X-Barracuda-Spam-Flag',
  'X-MS-Exchange-Organization-SCL',
];

function toPatternEntries(patterns: string[]): PatternEntry[] {
  return patterns.map((p) => ({
    id: newPatternId(),
    matchType: 'regex' as const,
    value: p,
  }));
}

function patternsToRegexList(entries: PatternEntry[]): string[] {
  return entries
    .filter((e) => e.value.trim() !== '')
    .map((e) => simplePatternToRegex(e.matchType, e.value));
}

export function TabFilters() {
  const { t } = useTranslation();
  const { control, setValue } = useFormContext<AccountFormValues>();
  const archiveRules = useWatch({ control, name: 'archive_rules' });

  const enabled = archiveRules?.enabled ?? false;
  const sendersInclude = archiveRules?.senders?.include ?? [];
  const sendersExclude = archiveRules?.senders?.exclude ?? [];
  const subjectsInclude = archiveRules?.subjects?.include ?? [];
  const subjectsExclude = archiveRules?.subjects?.exclude ?? [];
  const spamHeaders = archiveRules?.spam_headers ?? [];

  const [senderIncludeEntries, setSenderIncludeEntries] = useState<PatternEntry[]>(
    () => toPatternEntries(sendersInclude)
  );
  const [senderExcludeEntries, setSenderExcludeEntries] = useState<PatternEntry[]>(
    () => toPatternEntries(sendersExclude)
  );
  const [subjectIncludeEntries, setSubjectIncludeEntries] = useState<PatternEntry[]>(
    () => toPatternEntries(subjectsInclude)
  );
  const [subjectExcludeEntries, setSubjectExcludeEntries] = useState<PatternEntry[]>(
    () => toPatternEntries(subjectsExclude)
  );

  // Resync local state when form values change externally (e.g. after form.reset)
  const [lastSyncKey, setLastSyncKey] = useState<string>('');
  const syncKey = JSON.stringify({ sendersInclude, sendersExclude, subjectsInclude, subjectsExclude });
  if (syncKey !== lastSyncKey) {
    setLastSyncKey(syncKey);
    setSenderIncludeEntries(toPatternEntries(sendersInclude));
    setSenderExcludeEntries(toPatternEntries(sendersExclude));
    setSubjectIncludeEntries(toPatternEntries(subjectsInclude));
    setSubjectExcludeEntries(toPatternEntries(subjectsExclude));
  }

  const syncToForm = (
    includeEntries: PatternEntry[],
    excludeEntries: PatternEntry[],
    fieldPrefix: string
  ) => {
    const include = patternsToRegexList(includeEntries);
    const exclude = patternsToRegexList(excludeEntries);
    setValue(`${fieldPrefix}.include` as any, include);
    setValue(`${fieldPrefix}.exclude` as any, exclude);
  };

  const addEntry = (
    side: 'include' | 'exclude',
    includeEntries: PatternEntry[],
    excludeEntries: PatternEntry[],
    setIncludeEntries: React.Dispatch<React.SetStateAction<PatternEntry[]>>,
    setExcludeEntries: React.Dispatch<React.SetStateAction<PatternEntry[]>>,
    fieldPrefix: string
  ) => {
    const newEntry: PatternEntry = { id: newPatternId(), matchType: 'contains', value: '' };
    const newInclude = side === 'include' ? [...includeEntries, newEntry] : includeEntries;
    const newExclude = side === 'exclude' ? [...excludeEntries, newEntry] : excludeEntries;
    setIncludeEntries(newInclude);
    setExcludeEntries(newExclude);
    syncToForm(newInclude, newExclude, fieldPrefix);
  };

  const updateEntry = (
    id: string,
    partial: Partial<PatternEntry>,
    side: 'include' | 'exclude',
    includeEntries: PatternEntry[],
    excludeEntries: PatternEntry[],
    setIncludeEntries: React.Dispatch<React.SetStateAction<PatternEntry[]>>,
    setExcludeEntries: React.Dispatch<React.SetStateAction<PatternEntry[]>>,
    fieldPrefix: string
  ) => {
    if (side === 'include') {
      const updated = includeEntries.map((e) => (e.id === id ? { ...e, ...partial } : e));
      setIncludeEntries(updated);
      syncToForm(updated, excludeEntries, fieldPrefix);
    } else {
      const updated = excludeEntries.map((e) => (e.id === id ? { ...e, ...partial } : e));
      setExcludeEntries(updated);
      syncToForm(includeEntries, updated, fieldPrefix);
    }
  };

  const removeEntry = (
    id: string,
    side: 'include' | 'exclude',
    includeEntries: PatternEntry[],
    excludeEntries: PatternEntry[],
    setIncludeEntries: React.Dispatch<React.SetStateAction<PatternEntry[]>>,
    setExcludeEntries: React.Dispatch<React.SetStateAction<PatternEntry[]>>,
    fieldPrefix: string
  ) => {
    if (side === 'include') {
      const filtered = includeEntries.filter((e) => e.id !== id);
      setIncludeEntries(filtered);
      syncToForm(filtered, excludeEntries, fieldPrefix);
    } else {
      const filtered = excludeEntries.filter((e) => e.id !== id);
      setExcludeEntries(filtered);
      syncToForm(includeEntries, filtered, fieldPrefix);
    }
  };

  const handleEnableChange = (checked: boolean) => {
    if (checked) {
      setValue('archive_rules', {
        enabled: true,
        senders: { include: [], exclude: [] },
        subjects: { include: [], exclude: [] },
        skip_larger_than: undefined,
        spam_headers: [],
      });
    } else {
      setValue('archive_rules', undefined);
    }
  };

  const addSpamHeader = (header: string) => {
    if (!spamHeaders.includes(header)) {
      setValue('archive_rules.spam_headers', [...spamHeaders, header]);
    }
  };

  const removeSpamHeader = (header: string) => {
    setValue('archive_rules.spam_headers', spamHeaders.filter((h) => h !== header));
  };

  const [newSpamHeader, setNewSpamHeader] = useState('');

  const handleAddCustomSpamHeader = () => {
    const trimmed = newSpamHeader.trim();
    if (trimmed && !spamHeaders.includes(trimmed)) {
      setValue('archive_rules.spam_headers', [...spamHeaders, trimmed]);
      setNewSpamHeader('');
    }
  };

  const BYTES_PER_MB = 1024 * 1024;

  return (
    <div className="space-y-8">
      {/* Master Switch */}
      <div className="rounded-md border p-5 space-y-2 bg-muted/30">
        <FormItem className="flex flex-row items-start space-x-3 space-y-0">
          <FormControl>
            <Checkbox checked={enabled} onCheckedChange={handleEnableChange} />
          </FormControl>
          <div className="space-y-1 leading-none">
            <FormLabel>{t('accounts.filters.enableFiltering')}</FormLabel>
            <FormDescription>
              {t('accounts.filters.enableFilteringDesc')}
            </FormDescription>
          </div>
        </FormItem>
      </div>

      {enabled && (
        <>
          {/* Sender Filters */}
          <div className="space-y-4 rounded-md border p-5">
            <div className="flex items-center gap-2">
              <h4 className="text-sm font-semibold">{t('accounts.filters.senderFilter')}</h4>
              <Tooltip>
                <TooltipTrigger asChild>
                  <HelpCircle className="h-3.5 w-3.5 text-muted-foreground" />
                </TooltipTrigger>
                <TooltipContent>
                  {t('accounts.filters.senderFilterHelp')}
                </TooltipContent>
              </Tooltip>
            </div>

            <div className="space-y-3">
              <div>
                <p className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">
                  {t('accounts.filters.include')}
                </p>
                {senderIncludeEntries.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">
                    {t('accounts.filters.noIncludePatterns')}
                  </p>
                ) : (
                  <div className="space-y-2">
                    {senderIncludeEntries.map((entry) => (
                      <PatternInput
                        key={entry.id}
                        entry={entry}
                        onChange={(id, partial) =>
                          updateEntry(id, partial, 'include', senderIncludeEntries, senderExcludeEntries, setSenderIncludeEntries, setSenderExcludeEntries, 'archive_rules.senders')
                        }
                        onRemove={(id) =>
                          removeEntry(id, 'include', senderIncludeEntries, senderExcludeEntries, setSenderIncludeEntries, setSenderExcludeEntries, 'archive_rules.senders')
                        }
                      />
                    ))}
                  </div>
                )}
                <Button
                  variant="ghost"
                  type="button"
                  size="sm"
                  className="mt-2 h-8 text-xs"
                  onClick={() => addEntry('include', senderIncludeEntries, senderExcludeEntries, setSenderIncludeEntries, setSenderExcludeEntries, 'archive_rules.senders')}
                >
                  <Plus className="h-3 w-3 mr-1" />
                  {t('accounts.filters.addPattern')}
                </Button>
              </div>

              <div>
                <p className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">
                  {t('accounts.filters.exclude')}
                </p>
                {senderExcludeEntries.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">
                    {t('accounts.filters.noExcludePatterns')}
                  </p>
                ) : (
                  <div className="space-y-2">
                    {senderExcludeEntries.map((entry) => (
                      <PatternInput
                        key={entry.id}
                        entry={entry}
                        onChange={(id, partial) =>
                          updateEntry(id, partial, 'exclude', senderIncludeEntries, senderExcludeEntries, setSenderIncludeEntries, setSenderExcludeEntries, 'archive_rules.senders')
                        }
                        onRemove={(id) =>
                          removeEntry(id, 'exclude', senderIncludeEntries, senderExcludeEntries, setSenderIncludeEntries, setSenderExcludeEntries, 'archive_rules.senders')
                        }
                      />
                    ))}
                  </div>
                )}
                <Button
                  variant="ghost"
                  type="button"
                  size="sm"
                  className="mt-2 h-8 text-xs"
                  onClick={() => addEntry('exclude', senderIncludeEntries, senderExcludeEntries, setSenderIncludeEntries, setSenderExcludeEntries, 'archive_rules.senders')}
                >
                  <Plus className="h-3 w-3 mr-1" />
                  {t('accounts.filters.addPattern')}
                </Button>
              </div>
            </div>
          </div>

          {/* Subject Filters */}
          <div className="space-y-4 rounded-md border p-5">
            <div className="flex items-center gap-2">
              <h4 className="text-sm font-semibold">{t('accounts.filters.subjectFilter')}</h4>
              <Tooltip>
                <TooltipTrigger asChild>
                  <HelpCircle className="h-3.5 w-3.5 text-muted-foreground" />
                </TooltipTrigger>
                <TooltipContent>
                  {t('accounts.filters.subjectFilterHelp')}
                </TooltipContent>
              </Tooltip>
            </div>
            <div className="space-y-3">
              <div>
                <p className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">
                  {t('accounts.filters.include')}
                </p>
                {subjectIncludeEntries.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">
                    {t('accounts.filters.noIncludePatterns')}
                  </p>
                ) : (
                  <div className="space-y-2">
                    {subjectIncludeEntries.map((entry) => (
                      <PatternInput
                        key={entry.id}
                        entry={entry}
                        onChange={(id, partial) =>
                          updateEntry(id, partial, 'include', subjectIncludeEntries, subjectExcludeEntries, setSubjectIncludeEntries, setSubjectExcludeEntries, 'archive_rules.subjects')
                        }
                        onRemove={(id) =>
                          removeEntry(id, 'include', subjectIncludeEntries, subjectExcludeEntries, setSubjectIncludeEntries, setSubjectExcludeEntries, 'archive_rules.subjects')
                        }
                      />
                    ))}
                  </div>
                )}
                <Button
                  variant="ghost"
                  type="button"
                  size="sm"
                  className="mt-2 h-8 text-xs"
                  onClick={() => addEntry('include', subjectIncludeEntries, subjectExcludeEntries, setSubjectIncludeEntries, setSubjectExcludeEntries, 'archive_rules.subjects')}
                >
                  <Plus className="h-3 w-3 mr-1" />
                  {t('accounts.filters.addPattern')}
                </Button>
              </div>

              <div>
                <p className="text-xs font-medium text-muted-foreground mb-2 uppercase tracking-wider">
                  {t('accounts.filters.exclude')}
                </p>
                {subjectExcludeEntries.length === 0 ? (
                  <p className="text-xs text-muted-foreground italic">
                    {t('accounts.filters.noExcludePatterns')}
                  </p>
                ) : (
                  <div className="space-y-2">
                    {subjectExcludeEntries.map((entry) => (
                      <PatternInput
                        key={entry.id}
                        entry={entry}
                        onChange={(id, partial) =>
                          updateEntry(id, partial, 'exclude', subjectIncludeEntries, subjectExcludeEntries, setSubjectIncludeEntries, setSubjectExcludeEntries, 'archive_rules.subjects')
                        }
                        onRemove={(id) =>
                          removeEntry(id, 'exclude', subjectIncludeEntries, subjectExcludeEntries, setSubjectIncludeEntries, setSubjectExcludeEntries, 'archive_rules.subjects')
                        }
                      />
                    ))}
                  </div>
                )}
                <Button
                  variant="ghost"
                  type="button"
                  size="sm"
                  className="mt-2 h-8 text-xs"
                  onClick={() => addEntry('exclude', subjectIncludeEntries, subjectExcludeEntries, setSubjectIncludeEntries, setSubjectExcludeEntries, 'archive_rules.subjects')}
                >
                  <Plus className="h-3 w-3 mr-1" />
                  {t('accounts.filters.addPattern')}
                </Button>
              </div>
            </div>
          </div>

          {/* Size Limit */}
          <div className="space-y-4 rounded-md border p-5">
            <div className="flex items-center gap-2">
              <h4 className="text-sm font-semibold">{t('accounts.filters.sizeLimit')}</h4>
            </div>
            <FormField
              control={control}
              name="archive_rules.skip_larger_than"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t('accounts.filters.skipLargerThan')}</FormLabel>
                  <FormControl>
                    <div className="flex items-center gap-2">
                      <Input
                        type="number"
                        placeholder={t('accounts.filters.noLimit')}
                        className="max-w-[160px]"
                        value={field.value ? field.value / BYTES_PER_MB : ''}
                        onChange={(e) => {
                          const parsed = parseInt(e.target.value, 10);
                          field.onChange(isNaN(parsed) ? undefined : parsed * BYTES_PER_MB);
                        }}
                      />
                      <span className="text-sm text-muted-foreground">MB</span>
                    </div>
                  </FormControl>
                  <FormDescription>{t('accounts.filters.sizeLimitDesc')}</FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />
          </div>

          {/* Spam Headers */}
          <div className="space-y-4 rounded-md border p-5">
            <div className="flex items-center gap-2">
              <h4 className="text-sm font-semibold">{t('accounts.filters.spamHeaders')}</h4>
              <Tooltip>
                <TooltipTrigger asChild>
                  <HelpCircle className="h-3.5 w-3.5 text-muted-foreground" />
                </TooltipTrigger>
                <TooltipContent>
                  {t('accounts.filters.spamHeadersHelp')}
                </TooltipContent>
              </Tooltip>
            </div>
            <div className="space-y-2">
              {spamHeaders.length > 0 ? (
                spamHeaders.map((header) => (
                  <div key={header} className="flex items-center gap-2">
                    <div className="flex-1 rounded-md border bg-muted/50 px-3 py-1.5 text-sm font-mono">
                      {header}
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 shrink-0"
                      onClick={() => removeSpamHeader(header)}
                    >
                      <span className="text-muted-foreground">&#x2715;</span>
                    </Button>
                  </div>
                ))
              ) : (
                <p className="text-xs text-muted-foreground italic">
                  {t('accounts.filters.noSpamHeaders')}
                </p>
              )}
            </div>

            <div className="flex items-center gap-2">
              <Input
                className="h-8 text-sm max-w-[220px]"
                placeholder="X-Spam-Flag"
                value={newSpamHeader}
                onChange={(e) => setNewSpamHeader(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    handleAddCustomSpamHeader();
                  }
                }}
              />
              <Button
                variant="outline"
                type="button"
                size="sm"
                className="h-8 text-xs"
                onClick={handleAddCustomSpamHeader}
              >
                <Plus className="h-3 w-3 mr-1" />
                {t('accounts.filters.addHeader')}
              </Button>
            </div>

            <div>
              <p className="text-xs text-muted-foreground mb-2">{t('accounts.filters.suggestions')}</p>
              <div className="flex flex-wrap gap-1.5">
                {SUGGESTED_SPAM_HEADERS.filter((h) => !spamHeaders.includes(h)).map((header) => (
                  <button
                    key={header}
                    type="button"
                    className="inline-flex items-center rounded-full border bg-background px-2.5 py-0.5 text-xs font-mono text-muted-foreground hover:text-foreground hover:border-primary/50 transition-colors"
                    onClick={() => addSpamHeader(header)}
                  >
                    + {header}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
