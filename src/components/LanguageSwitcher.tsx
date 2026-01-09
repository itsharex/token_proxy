import { Check, Globe } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useI18n } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages.js";
import { isLocale, type Locale } from "@/paraglide/runtime.js";

const LANGUAGE_OPTIONS: readonly { value: Locale; label: string }[] = [
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
] as const;

type LanguageSwitcherProps = {
  triggerClassName?: string;
};

export function LanguageSwitcher({ triggerClassName }: LanguageSwitcherProps) {
  const { locale, setLocale } = useI18n();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="icon"
          className={cn("h-9 w-9", triggerClassName)}
          aria-label={m.language_label()}
        >
          <Globe className="size-4" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" data-slot="language-switcher-content">
        {LANGUAGE_OPTIONS.map((option) => {
          const isActive = option.value === locale;
          return (
            <DropdownMenuItem
              key={option.value}
              onSelect={() => {
                if (isLocale(option.value) && option.value !== locale) {
                  setLocale(option.value);
                }
              }}
              className={cn("flex items-center justify-between", isActive && "bg-accent")}
            >
              <span>{option.label}</span>
              {isActive ? <Check className="size-4" aria-hidden="true" /> : null}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
