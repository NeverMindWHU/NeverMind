import type { GenerateCardsInput } from "@/types/card";

export const GENERATION_FROM_SCREENSHOT_EVENT = "nevermind/generation-from-screenshot";

export type GenerationFromScreenshotPayload = Pick<
  GenerateCardsInput,
  "sourceText" | "sourceType" | "imageUrls" | "selectedKeyword" | "contextTitle" | "modelProfileId"
>;
