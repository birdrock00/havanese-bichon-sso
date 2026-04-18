import {
    FileText,
    FileImage,
    FileVideo,
    FileAudio,
    FileArchive,
    FileCode,
    FilePlus,
    Presentation,
    FileSpreadsheet,
    FileLock
} from "lucide-react";
import { cn } from "@/lib/utils";

interface AttachmentIconProps {
    contentType: string;
    className?: string;
}

export function AttachmentIcon({ contentType, className }: AttachmentIconProps) {
    const type = contentType.toLowerCase();

    const getIconConfig = () => {
        if (type.includes("pdf")) {
            return { Icon: FileText, color: "text-red-600" };
        }
        if (type.includes("word") || type.includes("officedocument.word") || type === "application/msword") {
            return { Icon: FileText, color: "text-blue-600" };
        }
        if (type.includes("presentation") || type.includes("powerpoint")) {
            return { Icon: Presentation, color: "text-orange-600" };
        }
        if (type.includes("spreadsheet") || type.includes("excel") || type.includes("csv")) {
            return { Icon: FileSpreadsheet, color: "text-green-600" };
        }

        if (type.startsWith("image/")) {
            return { Icon: FileImage, color: "text-purple-600" };
        }

        if (type.startsWith("video/")) {
            return { Icon: FileVideo, color: "text-pink-600" };
        }

        if (type.startsWith("audio/")) {
            return { Icon: FileAudio, color: "text-amber-600" };
        }

        if (type.includes("zip") || type.includes("tar") || type.includes("rar") || type.includes("7z")) {
            return { Icon: FileArchive, color: "text-gray-600" };
        }

        if (type.startsWith("text/") || type.includes("json") || type.includes("javascript") || type.includes("xml")) {
            return { Icon: FileCode, color: "text-sky-600" };
        }

        if (type.includes("encrypted") || type.includes("pkcs")) {
            return { Icon: FileLock, color: "text-yellow-700" };
        }

        return { Icon: FilePlus, color: "text-muted-foreground" };
    };

    const { Icon, color } = getIconConfig();

    return (
        <Icon
            className={cn("h-4 w-4 shrink-0 opacity-90", color, className)}
            strokeWidth={2}
        />
    );
}