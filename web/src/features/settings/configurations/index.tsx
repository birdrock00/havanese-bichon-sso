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

import * as React from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Skeleton } from "@/components/ui/skeleton"
import {
  ShieldCheck,
  Server,
  Database,
  Activity,
  InfoIcon,
  Mail,
  Zap
} from "lucide-react"
import { get_system_configurations } from "@/api/system/api"
import { useQuery } from "@tanstack/react-query"
import { useTranslation } from "react-i18next"

const formatMB = (bytes?: number) => {
  if (!bytes) return "—";
  return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
};

function BooleanBadge({ value }: { value: boolean }) {
  const { t } = useTranslation()
  return value ? (
    <Badge variant="secondary">{t("systemConfig.status.enabled")}</Badge>
  ) : (
    <Badge variant="outline" className="opacity-70">{t("systemConfig.status.disabled")}</Badge>
  )
}

function SettingRow({
  label,
  value,
  description,
}: {
  label: string
  value: React.ReactNode
  description?: string
}) {
  return (
    <div className="py-1.5 border-b border-border/40 last:border-0">
      <div className="grid grid-cols-[1fr_auto] items-center gap-3">
        <div className="text-sm font-medium text-foreground/80 leading-tight font-mono">{label}</div>
        <div className="text-sm text-right break-all leading-tight font-medium">{value}</div>
      </div>
      {description && (
        <div className="mt-0.5 text-[11px] leading-tight text-muted-foreground">{description}</div>
      )}
    </div>
  )
}

function SettingsCard({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: React.ElementType
  title: string
  description?: string
  children: React.ReactNode
}) {
  return (
    <Card className="h-full shadow-sm">
      <CardHeader className="flex flex-row items-center gap-2 py-3 bg-muted/20">
        <Icon className="h-4 w-4 text-primary/70" />
        <div className="space-y-0.5">
          <CardTitle className="text-sm font-semibold">{title}</CardTitle>
          {description && <CardDescription className="text-[11px] leading-none">{description}</CardDescription>}
        </div>
      </CardHeader>
      <CardContent className="pt-2 pb-1 px-4">{children}</CardContent>
    </Card>
  )
}

function PageSkeleton() {
  return (
    <div className="p-4 grid grid-cols-1 md:grid-cols-2 gap-4">
      {Array.from({ length: 6 }).map((_, i) => (
        <Card key={i}>
          <CardHeader className="py-3">
            <Skeleton className="h-4 w-40" />
            <Skeleton className="h-3 w-56" />
          </CardHeader>
          <CardContent className="space-y-2">
            <Skeleton className="h-3 w-full" />
            <Skeleton className="h-3 w-5/6" />
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

export default function ServerConfigurationsPage() {
  const { t } = useTranslation()
  const { data, isLoading, isError } = useQuery({
    queryKey: ["system-configurations"],
    queryFn: get_system_configurations,
  })

  if (isLoading) return <ScrollArea className="h-full"><PageSkeleton /></ScrollArea>
  if (isError || !data) {
    return <div className="p-8 text-center text-sm text-destructive">{t("systemConfig.fields.loadError")}</div>
  }

  return (
    <div className="w-full max-w-6xl mx-auto">
      <ScrollArea className="h-full w-full">
        <div className="px-6 pt-6 pb-2">
          <div className="flex items-start gap-4 p-5 rounded-xl border bg-gradient-to-br from-secondary/50 to-background shadow-inner">
            <div className="p-2 bg-primary/10 rounded-lg">
              <InfoIcon className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h4 className="text-base font-bold text-foreground">
                {t("systemConfig.pageTitle")}
              </h4>
              <p className="text-xs text-muted-foreground mt-1 max-w-2xl leading-relaxed">
                {t("systemConfig.pageDescription")}
              </p>
            </div>
          </div>
        </div>
        <div className="p-6 grid grid-cols-1 lg:grid-cols-2 gap-6">
          <SettingsCard
            icon={Server}
            title={t("systemConfig.sections.network.title")}
            description={t("systemConfig.sections.network.desc")}
          >
            <SettingRow label="BICHON_BIND_IP" value={data.bichon_bind_ip ?? "0.0.0.0"} />
            <SettingRow label="BICHON_HTTP_PORT" value={data.bichon_http_port} />
            <SettingRow label="BICHON_BASE_URL" value={data.bichon_base_url} />
            <SettingRow label="BICHON_PUBLIC_URL" value={data.bichon_public_url} />
            <SettingRow
              label="BICHON_ENABLE_REST_HTTPS"
              value={<BooleanBadge value={data.bichon_enable_rest_https} />}
            />
          </SettingsCard>
          <SettingsCard
            icon={Mail}
            title={t("systemConfig.sections.smtp.title", "SMTP Server")}
            description={t("systemConfig.sections.smtp.desc", "Incoming mail reception settings")}
          >
            <SettingRow label="BICHON_ENABLE_SMTP" value={<BooleanBadge value={data.bichon_enable_smtp} />} />
            <SettingRow label="BICHON_SMTP_PORT" value={data.bichon_smtp_port} />
            <SettingRow label="BICHON_SMTP_ENCRYPTION" value={data.bichon_smtp_encryption} />
            <SettingRow label="BICHON_SMTP_AUTH_REQUIRED" value={<BooleanBadge value={data.bichon_smtp_auth_required} />} />
          </SettingsCard>
          <SettingsCard
            icon={Zap}
            title={t("systemConfig.sections.performance.title")}
            description={t("systemConfig.sections.performance.desc")}
          >
            <SettingRow label="BICHON_SYNC_CONCURRENCY" value={data.bichon_sync_concurrency ?? t("systemConfig.status.auto")} />
            <SettingRow
              label="BICHON_HTTP_COMPRESSION_ENABLED"
              value={<BooleanBadge value={data.bichon_http_compression_enabled} />}
            />
          </SettingsCard>
          <SettingsCard
            icon={Database}
            title={t("systemConfig.sections.storage.title")}
            description={t("systemConfig.sections.storage.desc")}
          >
            <SettingRow label="BICHON_ROOT_DIR" value={<span className="font-mono">{data.bichon_root_dir}</span>} />
            <SettingRow label="BICHON_DATA_DIR" value={data.bichon_data_dir ? <span className="font-mono">{data.bichon_data_dir}</span> : "—"} />
            <SettingRow label="BICHON_INDEX_DIR" value={data.bichon_index_dir ? <span className="font-mono">{data.bichon_index_dir}</span> : "—"} />
            <SettingRow label="BICHON_METADATA_CACHE_SIZE" value={formatMB(data.bichon_metadata_cache_size)} />
            <SettingRow label="BICHON_ENVELOPE_CACHE_SIZE" value={formatMB(data.bichon_envelope_cache_size)} />
          </SettingsCard>
          <SettingsCard
            icon={ShieldCheck}
            title={t("systemConfig.sections.security.title")}
            description={t("systemConfig.sections.security.desc")}
          >
            <SettingRow
              label="BICHON_ENCRYPT_PASSWORD_SET"
              value={
                data.bichon_encrypt_password_set ? (
                  <Badge variant="secondary" className="bg-emerald-500/10 text-emerald-600 border-emerald-500/20">
                    {t("systemConfig.status.configured")}
                  </Badge>
                ) : (
                  <Badge variant="destructive">{t("systemConfig.status.missing")}</Badge>
                )
              }
            />
            <SettingRow
              label="BICHON_WEBUI_TOKEN_EXPIRATION_HOURS"
              value={`${data.bichon_webui_token_expiration_hours}h`}
            />
          </SettingsCard>
          <SettingsCard
            icon={Activity}
            title={t("systemConfig.sections.logging.title")}
            description={t("systemConfig.sections.logging.desc")}
          >
            <SettingRow label="BICHON_LOG_LEVEL" value={<Badge variant="outline" className="uppercase">{data.bichon_log_level}</Badge>} />
            <SettingRow label="BICHON_ANSI_LOGS" value={<BooleanBadge value={data.bichon_ansi_logs} />} />
            <SettingRow label="BICHON_JSON_LOGS" value={<BooleanBadge value={data.bichon_json_logs} />} />
            <SettingRow label="BICHON_LOG_TO_FILE" value={<BooleanBadge value={data.bichon_log_to_file} />} />
            <SettingRow label="BICHON_MAX_SERVER_LOG_FILES" value={data.bichon_max_server_log_files} />
          </SettingsCard>

        </div>
      </ScrollArea>
    </div>
  )
}