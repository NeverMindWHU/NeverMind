/**
 * 对齐 `docs/architecture/contracts/card-generation.md`
 * 和 Rust `models::card` / `commands::generate`。
 */

export type SourceType = "manual" | "selection" | "import" | "image";
export type CardStatus = "pending" | "accepted" | "rejected" | "active" | "archived";

export interface GenerateCardsInput {
  sourceText: string;
  selectedKeyword?: string | null;
  contextTitle?: string | null;
  sourceType: SourceType;
  modelProfileId?: string | null;
  /**
   * 每项是 `http(s)://` URL 或 `data:image/<mime>;base64,...`。
   * 与 `sourceText` 至少有一项非空；单次最多 8 张。
   */
  imageUrls?: string[];
}

export interface GeneratedCard {
  cardId: string;
  keyword: string;
  definition: string;
  explanation: string;
  relatedTerms: string[];
  scenarios: string[];
  sourceExcerpt: string | null;
  status: CardStatus;
  createdAt: string;
  reviewHistory: string[];
  nextReviewAt: string | null;
}

export interface GeneratedCardBatchResult {
  batchId: string;
  cards: GeneratedCard[];
}

export interface ReviewGeneratedCardsInput {
  batchId: string;
  acceptCardIds: string[];
  rejectCardIds: string[];
}

export interface ReviewedGeneratedCardsResult {
  batchId: string;
  acceptedCount: number;
  rejectedCount: number;
  pendingCount: number;
}
