import type { ReactNode } from "react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { getUpstreamLabel } from "@/features/config/cards/upstreams/constants";
import type { UpstreamEditorState } from "@/features/config/cards/upstreams/types";
import type { UpstreamForm } from "@/features/config/types";
import { m } from "@/paraglide/messages.js";

type EditorFieldProps = {
  label: string;
  htmlFor?: string;
  children: ReactNode;
};

/** 单个字段：label 左侧，input 右侧 */
function EditorField({ label, htmlFor, children }: EditorFieldProps) {
  return (
    <>
      {htmlFor ? (
        <Label htmlFor={htmlFor}>{label}</Label>
      ) : (
        <Label>{label}</Label>
      )}
      {children}
    </>
  );
}

type UpstreamEditorFieldsProps = {
  draft: UpstreamForm;
  providerOptions: readonly string[];
  showApiKeys: boolean;
  onToggleApiKeys: () => void;
  onChangeDraft: (patch: Partial<UpstreamForm>) => void;
};

function UpstreamEditorFields({
  draft,
  providerOptions,
  showApiKeys,
  onToggleApiKeys,
  onChangeDraft,
}: UpstreamEditorFieldsProps) {
  return (
    <div className="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-4">
      <EditorField label={m.field_id()} htmlFor="upstream-editor-id">
        <Input
          id="upstream-editor-id"
          value={draft.id}
          onChange={(e) => onChangeDraft({ id: e.target.value })}
          placeholder="openai-default"
        />
      </EditorField>

      {/* Provider 和 Status 并排：四列布局 */}
      <Label>{m.field_provider()}</Label>
      <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
        <Select
          value={draft.provider.trim() ? draft.provider : undefined}
          onValueChange={(value) => onChangeDraft({ provider: value })}
        >
          <SelectTrigger>
            <SelectValue placeholder="openai" />
          </SelectTrigger>
          <SelectContent>
            {providerOptions.map((option) => (
              <SelectItem key={option} value={option}>
                {option}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Label>{m.field_status()}</Label>
        <Select
          value={draft.enabled ? "enabled" : "disabled"}
          onValueChange={(value) =>
            onChangeDraft({ enabled: value === "enabled" })
          }
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="enabled">{m.common_enabled()}</SelectItem>
            <SelectItem value="disabled">{m.common_disabled()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <EditorField label={m.field_base_url()} htmlFor="upstream-editor-baseUrl">
        <Input
          id="upstream-editor-baseUrl"
          value={draft.baseUrl}
          onChange={(e) => onChangeDraft({ baseUrl: e.target.value })}
          placeholder="https://api.openai.com"
        />
      </EditorField>

      <EditorField label={m.field_api_key()} htmlFor="upstream-editor-apiKey">
        <PasswordInput
          id="upstream-editor-apiKey"
          visible={showApiKeys}
          onVisibilityChange={onToggleApiKeys}
          value={draft.apiKey}
          onChange={(e) => onChangeDraft({ apiKey: e.target.value })}
          placeholder={m.common_optional()}
        />
      </EditorField>

      {/* Priority 和 Index 并排：四列布局 */}
      <Label htmlFor="upstream-editor-priority">{m.field_priority()}</Label>
      <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
        <Input
          id="upstream-editor-priority"
          value={draft.priority}
          onChange={(e) => onChangeDraft({ priority: e.target.value })}
          placeholder="0"
          inputMode="numeric"
        />
        <Label htmlFor="upstream-editor-index">{m.field_index()}</Label>
        <Input
          id="upstream-editor-index"
          value={draft.index}
          onChange={(e) => onChangeDraft({ index: e.target.value })}
          placeholder={m.common_optional()}
          inputMode="numeric"
        />
      </div>
    </div>
  );
}

type UpstreamEditorDialogProps = {
  editor: UpstreamEditorState;
  providerOptions: readonly string[];
  showApiKeys: boolean;
  onToggleApiKeys: () => void;
  onOpenChange: (open: boolean) => void;
  onChangeDraft: (patch: Partial<UpstreamForm>) => void;
  onSave: () => void;
};

export function UpstreamEditorDialog({
  editor,
  providerOptions,
  showApiKeys,
  onToggleApiKeys,
  onOpenChange,
  onChangeDraft,
  onSave,
}: UpstreamEditorDialogProps) {
  const title = editor.open
    ? editor.mode === "create"
      ? m.upstreams_editor_title_add()
      : m.upstreams_editor_title_edit()
    : m.upstreams_editor_title_default();
  const description =
    editor.open && editor.mode === "edit"
      ? m.upstreams_editor_description_edit({ rowLabel: getUpstreamLabel(editor.index) })
      : m.upstreams_editor_description_create();

  return (
    <AlertDialog open={editor.open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="max-w-xl">
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        {editor.open ? (
          <UpstreamEditorFields
            draft={editor.draft}
            providerOptions={providerOptions}
            showApiKeys={showApiKeys}
            onToggleApiKeys={onToggleApiKeys}
            onChangeDraft={onChangeDraft}
          />
        ) : null}
        <AlertDialogFooter>
          <AlertDialogCancel>{m.common_cancel()}</AlertDialogCancel>
          <AlertDialogAction onClick={onSave}>{m.common_save()}</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
