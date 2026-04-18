import { invoke } from "@/lib/tauri";
import type {
  GenerateCardsInput,
  GeneratedCardBatchResult,
  ReviewGeneratedCardsInput,
  ReviewedGeneratedCardsResult,
} from "@/types/card";

export async function generateCards(
  input: GenerateCardsInput
): Promise<GeneratedCardBatchResult> {
  return invoke<GeneratedCardBatchResult>("generate_cards", { input });
}

export async function listGeneratedCards(
  batchId: string
): Promise<GeneratedCardBatchResult> {
  return invoke<GeneratedCardBatchResult>("list_generated_cards", { batchId });
}

export async function reviewGeneratedCards(
  input: ReviewGeneratedCardsInput
): Promise<ReviewedGeneratedCardsResult> {
  return invoke<ReviewedGeneratedCardsResult>("review_generated_cards", { input });
}
