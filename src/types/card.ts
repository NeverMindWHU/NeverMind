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
  /** 主关键词（一般等于 `keywords[0]`），与老版 UI 兼容。 */
  keyword: string;
  /** 完整问题文本（疑问句）。v2 新增。 */
  question: string;
  /** 3 个关键词（已解析回字符串数组）。v2 新增。 */
  keywords: string[];
  definition: string;
  explanation: string;
  relatedTerms: string[];
  scenarios: string[];
  sourceExcerpt: string | null;
  status: CardStatus;
  createdAt: string;
  reviewHistory: string[];
  nextReviewAt: string | null;
  /** 所属批次，便于从宝库跳回上下文。v2 新增。 */
  batchId: string | null;
}

/**
 * 宝库"按关键词桶"视图的一项。
 * 对齐 `src-tauri/src/models/card.rs::KeywordBucket`。
 */
export interface KeywordBucket {
  keyword: string;
  questionCount: number;
  sampleQuestions: KeywordBucketQuestion[];
  lastUpdatedAt: string;
}

export interface KeywordBucketQuestion {
  cardId: string;
  question: string;
  status: CardStatus;
  createdAt: string;
}

export interface KeywordBucketsResult {
  buckets: KeywordBucket[];
}

export interface SearchCardsResult {
  keyword: string | null;
  query: string | null;
  cards: GeneratedCard[];
}

export interface SearchByKeywordInput {
  keyword: string;
  onlyAccepted?: boolean;
}

export interface SearchByQuestionInput {
  query: string;
  onlyAccepted?: boolean;
  limit?: number | null;
}

export interface ListKeywordBucketsInput {
  onlyAccepted?: boolean;
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
