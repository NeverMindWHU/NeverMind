import { invokeData } from "@/lib/tauri";
import type {
  AppSettingsData,
  ClearLibraryData,
  ListModelProfilesData,
  SaveModelProfileData,
  SaveModelProfileInput,
  TestModelProfileData,
  TestModelProfileInput,
  UpdateSettingsData,
  UpdateSettingsInput,
} from "@/types/settings";

export function getSettings() {
  return invokeData<AppSettingsData>("get_settings");
}

export function updateSettings(input: UpdateSettingsInput) {
  return invokeData<UpdateSettingsData>("update_settings", { input });
}

export function listModelProfiles() {
  return invokeData<ListModelProfilesData>("list_model_profiles");
}

export function saveModelProfile(input: SaveModelProfileInput) {
  return invokeData<SaveModelProfileData>("save_model_profile", { input });
}

export function testModelProfile(input: TestModelProfileInput) {
  return invokeData<TestModelProfileData>("test_model_profile", { input });
}

/**
 * 一键清库：清除所有卡片、批次、复习排程与复习日志。
 * 设置与模型配置不会被清掉。该操作不可撤销。
 */
export function clearLibrary() {
  return invokeData<ClearLibraryData>("clear_library");
}
