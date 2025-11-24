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

import { ProgressMap } from "@/api/account/api";
import { Progress } from "@/components/ui/progress";
import { useTranslation } from "react-i18next";

interface Props {
  progressMap: ProgressMap | undefined | null;
}

export function FolderSyncProgress({ progressMap }: Props) {
  const { t } = useTranslation();

  if (!progressMap || Object.keys(progressMap).length === 0) {
    return <div className="text-muted-foreground text-sm">{t('accounts.folderSync.noData')}</div>;
  }

  const folderNames = Object.keys(progressMap);

  if (folderNames.length === 0) {
    return <div className="text-muted-foreground text-sm">{t('accounts.folderSync.noFolders')}</div>;
  }

  return (
    <div className="space-y-4">
      {folderNames.map((folder) => {
        const progress = progressMap[folder];
        const percentage =
          progress.total_batches > 0
            ? Math.min((progress.current_batch / progress.total_batches) * 100, 100)
            : 0;
        const isComplete = progress.current_batch >= progress.total_batches;
        const textColor = isComplete ? "text-green-800" : "text-blue-800";
        const progressColor = isComplete
          ? "bg-gray-200 [&>div]:bg-green-800 [&>div]:rounded-full h-1.5"
          : "bg-gray-200 [&>div]:bg-blue-800 [&>div]:rounded-full h-1.5";

        return (
          <div key={folder} className="space-y-1">
            <div className={`flex justify-between text-sm ${textColor}`}>
              <span className="truncate max-w-[70%]">{folder}</span>
              <span className="text-sm font-medium">
                {progress.current_batch}/{progress.total_batches} {t('accounts.folderSync.batches')}
              </span>
            </div>
            <Progress value={percentage} className={progressColor} />
          </div>
        );
      })}
    </div>
  );
}
