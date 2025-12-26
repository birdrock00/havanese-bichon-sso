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

import * as React from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Skeleton } from "@/components/ui/skeleton"
import { ShieldCheck, Server, Database, Globe, Lock, Activity, InfoIcon } from "lucide-react"
import { get_system_configurations } from "@/api/system/api"
import { useQuery } from "@tanstack/react-query"
import { useTranslation } from "react-i18next"

function BooleanBadge({ value }: { value: boolean }) {
  const { t } = useTranslation()
  return value ? (
    <Badge variant="secondary">{t("systemConfig.status.enabled")}</Badge>
  ) : (
    <Badge>{t("systemConfig.status.disabled")}</Badge>
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
    <div className="py-1">
      <div className="grid grid-cols-[1fr_auto] items-center gap-3">
        <div className="text-sm font-medium leading-tight">{label}</div>
        <div className="text-sm text-right break-all leading-tight">{value}</div>
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
    <Card className="h-full">
      <CardHeader className="flex flex-row items-center gap-2 py-3">
        <Icon className="h-5 w-5 text-muted-foreground" />
        <div>
          <CardTitle className="font-normal">{title}</CardTitle>
          {description && <CardDescription>{description}</CardDescription>}
        </div>
      </CardHeader>
      <CardContent className="space-y-1">{children}</CardContent>
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
            <Skeleton className="h-3 w-4/6" />
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

  if (isLoading) {
    return (
      <ScrollArea className="h-full">
        <PageSkeleton />
      </ScrollArea>
    )
  }

  if (isError || !data) {
    return (
      <div className="p-4 text-sm text-destructive">{t("systemConfig.fields.loadError")}</div>
    )
  }

  return (
    <div className="w-full max-w-5xl ml-0 px-4">
      <ScrollArea className="h-full w-full">
        <div className="px-4 pt-6 pb-2">
          <div className="flex items-start gap-3 p-4 rounded-xl border bg-secondary/30">
            <InfoIcon className="h-5 w-5 mt-0.5 text-muted-foreground" />
            <div>
              <h4 className="text-sm font-semibold italic text-foreground/80">
                {t("systemConfig.pageTitle")}
              </h4>
              <p className="text-xs text-muted-foreground mt-1 leading-relaxed">
                {t("systemConfig.pageDescription")}
              </p>
            </div>
          </div>
        </div>

        <div className="p-4 grid grid-cols-1 md:grid-cols-2 gap-4">
          <SettingsCard
            icon={Server}
            title={t("systemConfig.sections.network.title")}
            description={t("systemConfig.sections.network.desc")}
          >
            <SettingRow label="bichon_bind_ip" value={data.bichon_bind_ip ?? "—"} />
            <SettingRow label="bichon_http_port" value={data.bichon_http_port} />
            <SettingRow label="bichon_public_url" value={data.bichon_public_url} />
            <SettingRow
              label="bichon_enable_rest_https"
              value={<BooleanBadge value={data.bichon_enable_rest_https} />}
            />
          </SettingsCard>

          <SettingsCard
            icon={Globe}
            title={t("systemConfig.sections.cors.title")}
            description={t("systemConfig.sections.cors.desc")}
          >
            <SettingRow
              label="bichon_cors_origins"
              value={data.bichon_cors_origins?.join(", ") ?? t("systemConfig.status.notSet")}
            />
            <SettingRow label="bichon_cors_max_age" value={data.bichon_cors_max_age} />
          </SettingsCard>

          <SettingsCard
            icon={Activity}
            title={t("systemConfig.sections.logging.title")}
            description={t("systemConfig.sections.logging.desc")}
          >
            <SettingRow label="bichon_log_level" value={data.bichon_log_level} />
            <SettingRow label="bichon_ansi_logs" value={<BooleanBadge value={data.bichon_ansi_logs} />} />
            <SettingRow label="bichon_json_logs" value={<BooleanBadge value={data.bichon_json_logs} />} />
            <SettingRow label="bichon_log_to_file" value={<BooleanBadge value={data.bichon_log_to_file} />} />
            <SettingRow label="bichon_max_server_log_files" value={data.bichon_max_server_log_files} />
          </SettingsCard>

          <SettingsCard
            icon={Database}
            title={t("systemConfig.sections.storage.title")}
            description={t("systemConfig.sections.storage.desc")}
          >
            <SettingRow label="bichon_root_dir" value={data.bichon_root_dir} />
            <SettingRow label="bichon_metadata_cache_size" value={data.bichon_metadata_cache_size ?? "—"} />
            <SettingRow label="bichon_envelope_cache_size" value={data.bichon_envelope_cache_size ?? "—"} />
          </SettingsCard>

          <SettingsCard
            icon={ShieldCheck}
            title={t("systemConfig.sections.security.title")}
            description={t("systemConfig.sections.security.desc")}
          >
            <SettingRow
              label="bichon_encrypt_password_set"
              value={
                data.bichon_encrypt_password_set ? (
                  <Badge variant="secondary">{t("systemConfig.status.configured")}</Badge>
                ) : (
                  <Badge variant="destructive">{t("systemConfig.status.missing")}</Badge>
                )
              }
              description={t("systemConfig.fields.encryptPasswordDesc")}
            />
            <SettingRow
              label="bichon_webui_token_expiration_hours"
              value={data.bichon_webui_token_expiration_hours}
            />
          </SettingsCard>

          <SettingsCard
            icon={Lock}
            title={t("systemConfig.sections.performance.title")}
            description={t("systemConfig.sections.performance.desc")}
          >
            <SettingRow
              label="bichon_http_compression_enabled"
              value={<BooleanBadge value={data.bichon_http_compression_enabled} />}
            />
            <SettingRow
              label="bichon_sync_concurrency"
              value={data.bichon_sync_concurrency ?? t("systemConfig.status.auto")}
            />
          </SettingsCard>
        </div>
      </ScrollArea>
    </div>
  )
}