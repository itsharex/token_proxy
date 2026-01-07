import { useCallback, type Dispatch, type SetStateAction } from "react";

import { createEmptyUpstream } from "@/features/config/form";
import type { ConfigForm } from "@/features/config/types";

function updateListItem<T extends Record<string, string>>(
  list: T[],
  index: number,
  patch: Partial<T>
) {
  return list.map((item, current) => (current === index ? { ...item, ...patch } : item));
}

function removeListItem<T>(list: T[], index: number) {
  return list.filter((_, current) => current !== index);
}

export function useConfigListActions(
  setForm: Dispatch<SetStateAction<ConfigForm>>
) {
  const updateUpstream = useCallback(
    (index: number, patch: Partial<ConfigForm["upstreams"][number]>) => {
      setForm((prev) => ({
        ...prev,
        upstreams: updateListItem(prev.upstreams, index, patch),
      }));
    },
    [setForm]
  );

  const addUpstream = useCallback(() => {
    setForm((prev) => ({
      ...prev,
      upstreams: [...prev.upstreams, createEmptyUpstream()],
    }));
  }, [setForm]);

  const removeUpstream = useCallback(
    (index: number) => {
      setForm((prev) => ({
        ...prev,
        upstreams: removeListItem(prev.upstreams, index),
      }));
    },
    [setForm]
  );

  return {
    updateUpstream,
    addUpstream,
    removeUpstream,
  };
}
