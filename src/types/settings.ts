/**
 * 对齐 `docs/architecture/contracts/settings.md`
 * 和 Rust `commands::settings`。
 */

export type ThemeMode = "light" | "dark" | "system";
export type Language = "zh-CN" | "en-US";
export type ModelProvider = "openai-compatible" | "qwen" | "custom";

export interface StorageSettingsData {
  exportDirectory: string | null;
}

export interface AppSettingsData {
  theme: ThemeMode;
  language: Language;
  notificationEnabled: boolean;
  reviewReminderEnabled: boolean;
  /** `HH:mm` */
  reviewReminderTime: string;
  defaultModelProfileId: string | null;
  storage: StorageSettingsData;
}

export interface UpdateSettingsInput {
  theme: ThemeMode;
  language: Language;
  notificationEnabled: boolean;
  reviewReminderEnabled: boolean;
  reviewReminderTime: string;
  storage: StorageSettingsData;
}

export interface UpdateSettingsData {
  updatedAt: string;
}

export interface ModelProfileItem {
  profileId: string;
  name: string;
  provider: string;
  endpoint: string;
  isDefault: boolean;
  isAvailable: boolean;
}

export interface ListModelProfilesData {
  items: ModelProfileItem[];
}

export interface SaveModelProfileInput {
  profileId?: string | null;
  name: string;
  provider: ModelProvider;
  endpoint: string;
  apiKey: string;
  model?: string | null;
  timeoutMs: number;
}

export interface SaveModelProfileData {
  profileId: string;
  updatedAt: string;
}

export interface TestModelProfileInput {
  profileId?: string | null;
  provider: ModelProvider;
  endpoint: string;
  apiKey: string;
  model?: string | null;
  timeoutMs: number;
}

export interface TestModelProfileData {
  reachable: boolean;
  latencyMs: number;
}

/** 一键清库返回：各表实际删除的行数。 */
export interface ClearLibraryData {
  deletedCards: number;
  deletedBatches: number;
  deletedReviewSchedules: number;
  deletedReviewLogs: number;
}
