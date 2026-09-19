import { useFormContext, useWatch, type Path } from "react-hook-form";
import { Checkbox } from "@/components/ui/checkbox";
import { HelpCircle } from "lucide-react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { AccountFormValues } from "./schema";

interface ExtensionPickerProps {
  /** Form path of the include array, e.g. "extraction_rules.extensions.include". */
  path: string;
  title: string;
  help?: string;
  /** Fixed set of supported extensions, e.g. ["pdf", "docx"]. */
  options: readonly string[];
}

/**
 * Fixed-option checkbox group for attachment extraction extensions.
 * Checked values are stored verbatim in the `include` array (exact match).
 */
export function ExtensionPicker({ path, title, help, options }: ExtensionPickerProps) {
  const { control, setValue } = useFormContext<AccountFormValues>();
  const selected = (useWatch({ control, name: path as Path<AccountFormValues> }) as
    | string[]
    | undefined) ?? [];

  const toggle = (ext: string, checked: boolean) => {
    setValue(
      path as Path<AccountFormValues>,
      checked ? [...selected, ext] : selected.filter((e) => e !== ext)
    );
  };

  return (
    <div className="space-y-4 rounded-md border p-5">
      <div className="flex items-center gap-2">
        <h4 className="text-sm font-semibold">{title}</h4>
        {help && (
          <Tooltip>
            <TooltipTrigger asChild>
              <HelpCircle className="h-3.5 w-3.5 text-muted-foreground" />
            </TooltipTrigger>
            <TooltipContent>{help}</TooltipContent>
          </Tooltip>
        )}
      </div>
      <div className="flex flex-wrap gap-x-6 gap-y-2">
        {options.map((ext) => (
          <label key={ext} className="flex items-center gap-2 cursor-pointer">
            <Checkbox
              checked={selected.includes(ext)}
              onCheckedChange={(checked) => toggle(ext, !!checked)}
            />
            <span className="text-sm font-mono">{ext}</span>
          </label>
        ))}
      </div>
    </div>
  );
}