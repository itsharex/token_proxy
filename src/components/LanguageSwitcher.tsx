import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useI18n } from "@/lib/i18n";
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
    <Select
      value={locale}
      onValueChange={(value) => {
        if (isLocale(value) && value !== locale) {
          setLocale(value);
        }
      }}
    >
      <SelectTrigger
        data-slot="language-switcher-trigger"
        className={triggerClassName}
        aria-label={m.language_label()}
      >
        <SelectValue placeholder={m.language_label()} />
      </SelectTrigger>
      <SelectContent data-slot="language-switcher-content">
        {LANGUAGE_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

