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

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Skeleton } from '@/components/ui/skeleton';
import { XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, PieChart, Pie, Cell, BarChart, Bar } from 'recharts';
import { Mail, Users, Inbox, Zap } from 'lucide-react';
import { formatBytes, formatNumber } from '@/lib/utils';
import { useQuery } from '@tanstack/react-query';
import { get_dashboard_stats, INITIAL_DASHBOARD_STATS, TimeBucket } from '@/api/system/api';
import { Main } from '@/components/layout/main';
import { FixedHeader } from '@/components/layout/fixed-header';
import { useTranslation } from 'react-i18next';
import { getToken } from '@/stores/authStore';
import { useNavigate } from '@tanstack/react-router';
import useMinimalAccountList from '@/hooks/use-minimal-account-list';
import { Badge } from '@/components/ui/badge';

interface DailyActivity {
  date: string;
  count: number;
  timestamp_ms: number;
}

const GithubIcon = ({ className }: { className?: string }) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
  >
    <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
    <path d="M9 18c-4.51 2-5-2-7-2" />
  </svg>
);

function convertRecentActivity(timeBuckets: TimeBucket[], locale: string): DailyActivity[] {
  const dateFormatter = new Intl.DateTimeFormat(locale, {
    month: 'short',
    day: 'numeric',
  });

  return timeBuckets.map(bucket => {
    const date = new Date(bucket.timestamp_ms);
    return {
      date: dateFormatter.format(date),
      count: bucket.count,
      timestamp_ms: bucket.timestamp_ms,
    };
  });
}

const formatTooltipDate = (timestamp_ms: number, locale: string): string => {
  const date = new Date(timestamp_ms);
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }).format(date);
};

const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444'];

const MetricCardSkeleton = () => (
  <Card>
    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
      <Skeleton className="h-4 w-32" />
      <Skeleton className="h-4 w-4" />
    </CardHeader>
    <CardContent>
      <Skeleton className="h-8 w-24 mb-1" />
      <Skeleton className="h-3 w-20" />
    </CardContent>
  </Card>
);

const EmptyChart = ({ title }: { title: string }) => (
  <div className="h-80 flex flex-col items-center justify-center text-muted-foreground">
    <Inbox className="h-12 w-12 mb-3 opacity-40" />
    <p className="text-sm font-medium">{title}</p>
  </div>
);

const EmptyTable = ({ title }: { title: string }) => (
  <div className="py-10 text-center text-muted-foreground">
    <Inbox className="h-10 w-10 mx-auto mb-3 opacity-40" />
    <p className="text-sm font-medium">{title}</p>
  </div>
);

export default function MailArchiveDashboard() {
  const token = getToken();
  const { data: stats, isLoading } = useQuery({
    queryKey: ['dashboard-stats'],
    enabled: !!token,
    queryFn: get_dashboard_stats,
    placeholderData: INITIAL_DASHBOARD_STATS,
  });

  const navigate = useNavigate();
  const { t, i18n } = useTranslation();
  const currentLocale = i18n.resolvedLanguage || i18n.language || navigator.language;

  const logicalSize = stats?.total_size_bytes ?? 0;
  const blobSize = stats?.storage_usage_bytes ?? 0;
  const indexSize = stats?.index_usage_bytes ?? 0;
  const physicalTotal = blobSize + indexSize;

  const savingsPercent = logicalSize > 0
    ? Math.max(0, ((1 - physicalTotal / logicalSize) * 100)).toFixed(1)
    : "0.0";

  const blobWidth = logicalSize > 0 ? (blobSize / logicalSize) * 100 : 0;
  const indexWidth = logicalSize > 0 ? (indexSize / logicalSize) * 100 : 0;

  const totalAttachments = (stats?.with_attachment_count ?? 0) + (stats?.without_attachment_count ?? 0);
  const attachmentRatio = totalAttachments > 0 ? (stats?.with_attachment_count ?? 0) / totalAttachments : 0;

  const hasRecentActivity = stats?.recent_activity && stats.recent_activity.length > 0;
  const hasTopSenders = stats?.top_senders && stats.top_senders.length > 0;
  const hasTopEmails = stats?.top_largest_emails && stats.top_largest_emails.length > 0;
  const hasTopAccounts = stats?.top_accounts && stats.top_accounts.length > 0;

  const { minimalList } = useMinimalAccountList();
  const getAccountIdByEmail = (email: string): number | null => {
    if (!minimalList) return null;
    const account = minimalList.find(a => a.email === email);
    return account ? account.id : null;
  };

  const handleQuickSearch = (filter: Record<string, any>) => {
    navigate({
      to: '/search',
      search: (prev: any) => ({
        page: 1,
        pageSize: prev.pageSize ?? 50,
        sortBy: prev.sortBy ?? "DATE",
        sortOrder: prev.sortOrder ?? "desc",
        q: JSON.stringify(filter),
      }),
    });
  };

  const attachmentData = totalAttachments > 0
    ? [
      { name: 'With Attachments', value: attachmentRatio, fill: COLORS[1] },
      { name: 'No Attachments', value: 1 - attachmentRatio, fill: '#e5e7eb' },
    ]
    : [
      { name: 'No Data', value: 1, fill: '#e5e7eb' },
    ];

  if (isLoading) {
    return (
      <div className="flex-1 space-y-6 p-6 md:p-8">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {[...Array(4)].map((_, i) => <MetricCardSkeleton key={i} />)}
        </div>
        <Skeleton className="h-80 w-full" />
      </div>
    );
  }

  return (
    <>
      <FixedHeader />
      <Main higher>
        <div className="flex-1 space-y-6 p-6 md:p-8">
          <div className="grid gap-4 grid-cols-1 md:grid-cols-12">
            <Card className="md:col-span-2 lg:col-span-2">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">{t('dashboard.mailAccounts')}</CardTitle>
                <Users className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-4xl font-bold">{formatNumber(stats!.account_count)}</div>
                <p className="text-xs text-muted-foreground">{t('dashboard.connected')}</p>
              </CardContent>
            </Card>

            <Card className="md:col-span-3 lg:col-span-3">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">{t('dashboard.totalEmails')}</CardTitle>
                <Mail className="h-4 w-4 text-muted-foreground" />
              </CardHeader>
              <CardContent>
                <div className="text-4xl font-bold">{formatNumber(stats!.email_count)}</div>
                <p className="text-xs text-muted-foreground">{t('dashboard.syncedLocally')}</p>
              </CardContent>
            </Card>

            <Card className="border-primary/20 bg-primary/[0.01] md:col-span-4 lg:col-span-4">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-bold text-primary flex items-center gap-1 uppercase">
                  <Zap className="h-3.5 w-3.5 fill-primary" />
                  {t('dashboard.efficiency', 'Efficiency')}
                </CardTitle>
                <div className="flex flex-col items-end leading-none">
                  <span className="text-xl font-black text-primary">{savingsPercent}%</span>
                  <span className="text-[9px] font-bold text-primary uppercase">{t('dashboard.saved', 'Saved')}</span>
                </div>
              </CardHeader>
              <CardContent className="py-2.5 space-y-2">
                <div className="h-2 w-full bg-muted rounded-full overflow-hidden flex">
                  <div className="h-full bg-primary transition-all" style={{ width: `${blobWidth}%` }} />
                  <div className="h-full bg-orange-400 transition-all" style={{ width: `${indexWidth}%` }} />
                </div>

                <div className="flex flex-col gap-1">
                  <div className="flex justify-between items-center text-[11px]">
                    <div className="flex items-center gap-1.5 overflow-hidden">
                      <span className="text-muted-foreground uppercase text-[9px] shrink-0">{t('dashboard.logicalVolume')}:</span>
                      <span className="font-bold tracking-tight truncate">{formatBytes(logicalSize)}</span>
                    </div>
                    <div className="flex items-center gap-1.5 overflow-hidden ml-2">
                      <span className="text-muted-foreground uppercase text-[9px] shrink-0">{t('dashboard.actualDiskUsage')}:</span>
                      <span className="font-bold tracking-tight truncate">{formatBytes(physicalTotal)}</span>
                    </div>
                  </div>
                  <div className="flex justify-between items-center text-[10px]">
                    <div className="flex items-center gap-1.5 overflow-hidden">
                      <div className="h-1.5 w-1.5 shrink-0 rounded-full bg-primary" />
                      <span className="text-muted-foreground truncate">
                        {t('dashboard.dataStorage')}: <span className="text-foreground font-medium">{formatBytes(blobSize)}</span>
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 overflow-hidden ml-2">
                      <div className="h-1.5 w-1.5 shrink-0 rounded-full bg-orange-400" />
                      <span className="text-muted-foreground truncate text-right">
                        {t('dashboard.indexSize')}: <span className="text-foreground font-medium">{formatBytes(indexSize)}</span>
                      </span>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card className="md:col-span-3 lg:col-span-3">
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">{t('dashboard.systemVersion')}</CardTitle>
                <Badge variant="secondary" className="text-[9px] h-4.5 font-bold px-1.5 uppercase tracking-wider">Community</Badge>
              </CardHeader>
              <CardContent>
                <div className='text-4xl font-bold truncate text-primary tracking-tighter'>
                  {stats!.system_version ? (
                    <a href={`https://github.com/rustmailer/bichon/releases/tag/${stats!.system_version}`} target="_blank" rel="noopener noreferrer" className="hover:underline">
                      {stats!.system_version}
                    </a>
                  ) : 'N/A'}
                </div>
                <div className="flex items-center gap-1.5 mt-1">
                  <GithubIcon className="h-5 w-5 text-muted-foreground" />
                  <p className="text-[10px] text-muted-foreground font-mono truncate">{stats!.commit_hash?.substring(0, 7) ?? 'N/A'}</p>
                </div>
              </CardContent>
            </Card>
          </div>

          <Tabs defaultValue="trend" className="space-y-4">
            <TabsList className="grid w-full grid-cols-3 lg:w-auto">
              <TabsTrigger value="trend">{t('dashboard.dayTrend')}</TabsTrigger>
              <TabsTrigger value="attachment">{t('dashboard.attachments')}</TabsTrigger>
              <TabsTrigger value="top">{t('dashboard.topLists')}</TabsTrigger>
            </TabsList>

            <TabsContent value="trend" className="space-y-4">
              <Card>
                <CardHeader>
                  <CardTitle>{t('dashboard.newEmails')}</CardTitle>
                  <CardDescription>{t('dashboard.messageDistribution')}</CardDescription>
                </CardHeader>
                <CardContent className="h-80">
                  {hasRecentActivity ? (
                    <ResponsiveContainer width="100%" height="100%">
                      <BarChart data={convertRecentActivity(stats!.recent_activity, currentLocale)} margin={{ top: 20, right: 30, left: 20, bottom: 5 }}>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} opacity={0.3} />
                        <XAxis dataKey="date" tick={{ fontSize: 12 }} interval="preserveStart" tickCount={10} />
                        <YAxis tick={{ fontSize: 12 }} />
                        <Tooltip
                          formatter={(v) => formatNumber(v as number)}
                          content={({ payload }) => {
                            if (payload && payload.length) {
                              const dataPoint = payload[0].payload;
                              const fullDate = formatTooltipDate(dataPoint.timestamp_ms, currentLocale);
                              return (
                                <div className="p-2 border rounded-lg shadow-md bg-white dark:bg-gray-800">
                                  <p className="font-semibold text-sm mb-1">{fullDate}</p>
                                  <p className="text-xs">{t('dashboard.emails')}: {formatNumber(dataPoint.count)}</p>
                                </div>
                              );
                            }
                            return null;
                          }}
                        />
                        <Bar dataKey="count" fill="currentColor" className="text-primary" radius={[4, 4, 0, 0]} barSize={28} />
                      </BarChart>
                    </ResponsiveContainer>
                  ) : (
                    <EmptyChart title={t('dashboard.noRecentActivity')} />
                  )}
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="attachment" className="space-y-4">
              <Card>
                <CardHeader>
                  <CardTitle>{t('dashboard.attachmentRatio')}</CardTitle>
                  <CardDescription>
                    {totalAttachments > 0
                      ? t('dashboard.attachmentRatioDesc', { percent: (attachmentRatio * 100).toFixed(1) })
                      : t('dashboard.noEmailsSynced')}
                  </CardDescription>
                </CardHeader>
                <CardContent className="flex items-center justify-center h-80">
                  <ResponsiveContainer width={300} height={300}>
                    <PieChart>
                      <Pie
                        data={attachmentData}
                        cx="50%"
                        cy="50%"
                        innerRadius={60}
                        outerRadius={100}
                        paddingAngle={5}
                        dataKey="value"
                        stroke="none"
                      >
                        {attachmentData.map((entry, i) => (
                          <Cell key={`cell-${i}`} fill={entry.fill} />
                        ))}
                      </Pie>
                      <Tooltip formatter={(v) => totalAttachments > 0 ? `${((v as number) * 100).toFixed(1)}%` : '0%'} />
                    </PieChart>
                  </ResponsiveContainer>
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="top" className="space-y-6">
              <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm font-bold uppercase">{t('dashboard.top10Senders')}</CardTitle>
                  </CardHeader>
                  <CardContent className="p-0">
                    {hasTopSenders ? (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="text-xs">{t('dashboard.sender')}</TableHead>
                            <TableHead className="text-right text-xs">{t('dashboard.count')}</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {stats!.top_senders.map((s) => (
                            <TableRow key={s.key} className="cursor-pointer hover:bg-accent/50 group" onClick={() => handleQuickSearch({ from: s.key })}>
                              <TableCell className="max-w-[200px] truncate text-sm group-hover:text-primary transition-colors">{s.key}</TableCell>
                              <TableCell className="text-right font-mono text-sm">{formatNumber(s.count)}</TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <EmptyTable title={t('dashboard.noSendersData')} />
                    )}
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm font-bold uppercase">{t('dashboard.top10LargestEmails')}</CardTitle>
                  </CardHeader>
                  <CardContent className="p-0">
                    {hasTopEmails ? (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="text-xs">{t('dashboard.subject')}</TableHead>
                            <TableHead className="text-right text-xs">{t('dashboard.size')}</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {stats!.top_largest_emails.map((m, i) => (
                            <TableRow key={i} className="cursor-pointer hover:bg-accent/50 group" onClick={() => handleQuickSearch({ subject: m.subject })}>
                              <TableCell className="max-w-[200px] truncate text-sm group-hover:text-primary transition-colors">{m.subject || t('dashboard.noSubject')}</TableCell>
                              <TableCell className="text-right font-mono text-sm text-orange-600">{formatBytes(m.size_bytes)}</TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <EmptyTable title={t('dashboard.noLargeEmails')} />
                    )}
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm font-bold uppercase">{t('dashboard.top10Accounts')}</CardTitle>
                  </CardHeader>
                  <CardContent className="p-0">
                    {hasTopAccounts ? (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="text-xs">{t('dashboard.account')}</TableHead>
                            <TableHead className="text-right text-xs">{t('dashboard.emails')}</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {stats!.top_accounts.map((acc) => (
                            <TableRow key={acc.key} className="cursor-pointer hover:bg-accent/50 group" onClick={() => handleQuickSearch({ account_ids: [getAccountIdByEmail(acc.key) || 0] })}>
                              <TableCell className="max-w-[200px] truncate text-sm group-hover:text-primary transition-colors">{acc.key}</TableCell>
                              <TableCell className="text-right font-mono text-sm">{formatNumber(acc.count)}</TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <EmptyTable title={t('dashboard.noAccountData')} />
                    )}
                  </CardContent>
                </Card>
              </div>
            </TabsContent>
          </Tabs>
        </div>
      </Main>
      <div className="mt-auto p-6 text-center text-xs text-muted-foreground border-t">
        © 2025-2026 <a href="https://rustmailer.com" target="_blank" rel="noopener noreferrer" className="hover:underline">rustmailer.com</a> - Bichon Email Archiving Project
      </div>
    </>
  );
}