import { Check, Laptop, Moon, Sun, type LucideIcon } from "lucide-react";
import { useTheme } from "next-themes";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages.js";

type ThemeValue = "system" | "light" | "dark";

const THEME_OPTIONS: readonly {
  value: ThemeValue;
  label: () => string;
  icon: LucideIcon;
}[] = [
  { value: "system", label: () => m.theme_system(), icon: Laptop },
  { value: "light", label: () => m.theme_light(), icon: Sun },
  { value: "dark", label: () => m.theme_dark(), icon: Moon },
] as const;

export function ThemeSwitcher() {
  const { theme = "system", setTheme } = useTheme();
  const TriggerIcon = theme === "dark" ? Moon : theme === "light" ? Sun : Laptop;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          data-slot="theme-switcher-trigger"
          type="button"
          variant="outline"
          size="icon"
          className="h-9 w-9"
          aria-label={m.theme_label()}
        >
          <TriggerIcon className="size-4" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" data-slot="theme-switcher-content">
        {THEME_OPTIONS.map((option) => {
          const isActive = option.value === theme;
          const OptionIcon = option.icon;

          return (
            <DropdownMenuItem
              key={option.value}
              onSelect={() => {
                if (option.value !== theme) {
                  setTheme(option.value);
                }
              }}
              className={cn("flex items-center justify-between", isActive && "bg-accent")}
            >
              <span className="flex items-center gap-2">
                <OptionIcon className="size-4" aria-hidden="true" />
                {option.label()}
              </span>
              {isActive ? <Check className="size-4" aria-hidden="true" /> : null}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
