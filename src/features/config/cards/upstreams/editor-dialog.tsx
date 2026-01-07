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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { getUpstreamLabel } from "@/features/config/cards/upstreams/constants";
import type { UpstreamEditorState } from "@/features/config/cards/upstreams/types";
import type { UpstreamForm } from "@/features/config/types";

type EditorFieldProps = {
  label: string;
  htmlFor?: string;
  action?: ReactNode;
  children: ReactNode;
};

function EditorField({ label, htmlFor, action, children }: EditorFieldProps) {
  return (
    <div className="grid gap-2">
      <div className="flex items-center justify-between gap-2">
        {htmlFor ? <Label htmlFor={htmlFor}>{label}</Label> : <Label>{label}</Label>}
        {action ?? null}
      </div>
      {children}
    </div>
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
    <div className="grid gap-4">
      <EditorField label="Id" htmlFor="upstream-editor-id">
        <Input id="upstream-editor-id" value={draft.id} onChange={(e) => onChangeDraft({ id: e.target.value })} placeholder="openai-default" />
      </EditorField>
      <EditorField label="Provider">
        <Select value={draft.provider.trim() ? draft.provider : undefined} onValueChange={(value) => onChangeDraft({ provider: value })}>
          <SelectTrigger><SelectValue placeholder="openai" /></SelectTrigger>
          <SelectContent>
            {providerOptions.map((option) => (<SelectItem key={option} value={option}>{option}</SelectItem>))}
          </SelectContent>
        </Select>
      </EditorField>
      <EditorField label="Base URL" htmlFor="upstream-editor-baseUrl">
        <Input id="upstream-editor-baseUrl" value={draft.baseUrl} onChange={(e) => onChangeDraft({ baseUrl: e.target.value })} placeholder="https://api.openai.com" />
      </EditorField>
      <EditorField label="API Key" htmlFor="upstream-editor-apiKey">
        <PasswordInput
          id="upstream-editor-apiKey"
          visible={showApiKeys}
          onVisibilityChange={onToggleApiKeys}
          value={draft.apiKey}
          onChange={(e) => onChangeDraft({ apiKey: e.target.value })}
          placeholder="Optional"
        />
      </EditorField>
      <div className="grid gap-4 sm:grid-cols-2">
        <EditorField label="Priority" htmlFor="upstream-editor-priority">
          <Input id="upstream-editor-priority" value={draft.priority} onChange={(e) => onChangeDraft({ priority: e.target.value })} placeholder="0" inputMode="numeric" />
        </EditorField>
        <EditorField label="Index" htmlFor="upstream-editor-index">
          <Input id="upstream-editor-index" value={draft.index} onChange={(e) => onChangeDraft({ index: e.target.value })} placeholder="Optional" inputMode="numeric" />
        </EditorField>
      </div>
      <EditorField label="Status">
        <Select value={draft.enabled ? "enabled" : "disabled"} onValueChange={(value) => onChangeDraft({ enabled: value === "enabled" })}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="enabled">Enabled</SelectItem>
            <SelectItem value="disabled">Disabled</SelectItem>
          </SelectContent>
        </Select>
      </EditorField>
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
  const title = editor.open ? (editor.mode === "create" ? "Add Upstream" : "Edit Upstream") : "Upstream";
  const description =
    editor.open && editor.mode === "edit"
      ? `${getUpstreamLabel(editor.index)} settings.`
      : "Create a new upstream entry.";

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
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction onClick={onSave}>Save</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
