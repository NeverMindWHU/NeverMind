import { useCallback, useEffect, useState } from "react";
import { useToast } from "@/lib/toast";
import { humanizeError } from "@/lib/format";
import type { CommandError } from "@/types/common";
import type {
  AppSettingsData,
  ModelProfileItem,
  SaveModelProfileInput,
  TestModelProfileInput,
  UpdateSettingsInput,
} from "@/types/settings";
import {
  getSettings,
  listModelProfiles,
  saveModelProfile as apiSave,
  testModelProfile as apiTest,
  updateSettings,
} from "../services/api";

interface State {
  loading: boolean;
  saving: boolean;
  error: CommandError | null;
  settings: AppSettingsData | null;
  profiles: ModelProfileItem[];
}

export function useSettings() {
  const toast = useToast();
  const [state, setState] = useState<State>({
    loading: true,
    saving: false,
    error: null,
    settings: null,
    profiles: [],
  });

  const load = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const [settings, profiles] = await Promise.all([
        getSettings(),
        listModelProfiles(),
      ]);
      setState((s) => ({
        ...s,
        loading: false,
        settings,
        profiles: profiles.items,
      }));
    } catch (err) {
      const ce = err as CommandError;
      setState((s) => ({ ...s, loading: false, error: ce }));
      toast.error("设置加载失败", humanizeError(ce));
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load]);

  const saveSettings = useCallback(
    async (input: UpdateSettingsInput) => {
      setState((s) => ({ ...s, saving: true }));
      try {
        await updateSettings(input);
        setState((s) => ({
          ...s,
          saving: false,
          settings: s.settings
            ? {
                ...s.settings,
                ...input,
                storage: { ...input.storage },
              }
            : s.settings,
        }));
        toast.success("设置已保存");
      } catch (err) {
        setState((s) => ({ ...s, saving: false }));
        toast.error("保存失败", humanizeError(err as CommandError));
        throw err;
      }
    },
    [toast]
  );

  const saveProfile = useCallback(
    async (input: SaveModelProfileInput) => {
      try {
        const data = await apiSave(input);
        toast.success("模型配置已保存");
        await load();
        return data;
      } catch (err) {
        toast.error("保存失败", humanizeError(err as CommandError));
        throw err;
      }
    },
    [load, toast]
  );

  const testProfile = useCallback(
    async (input: TestModelProfileInput) => {
      try {
        const data = await apiTest(input);
        toast.success(
          data.reachable ? "连接成功" : "未达成功响应",
          `延迟 ${data.latencyMs} ms`
        );
        return data;
      } catch (err) {
        toast.error("连通性测试失败", humanizeError(err as CommandError));
        throw err;
      }
    },
    [toast]
  );

  return {
    ...state,
    reload: load,
    saveSettings,
    saveProfile,
    testProfile,
  };
}
