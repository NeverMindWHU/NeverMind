import { invoke } from "@/lib/tauri";
import type {
  KeywordBucketsResult,
  ListKeywordBucketsInput,
  SearchByKeywordInput,
  SearchByQuestionInput,
  SearchCardsResult,
} from "@/types/card";

/**
 * 宝库：按关键词精确查询（大小写不敏感，跨批次）。
 */
export async function searchByKeyword(
  input: SearchByKeywordInput
): Promise<SearchCardsResult> {
  return invoke<SearchCardsResult>("library_search_by_keyword", {
    input: {
      keyword: input.keyword,
      onlyAccepted: input.onlyAccepted ?? false,
    },
  });
}

/**
 * 宝库：按问题文本模糊匹配（跨 question / definition / explanation / keyword）。
 */
export async function searchByQuestion(
  input: SearchByQuestionInput
): Promise<SearchCardsResult> {
  return invoke<SearchCardsResult>("library_search_by_question", {
    input: {
      query: input.query,
      onlyAccepted: input.onlyAccepted ?? false,
      limit: input.limit ?? 50,
    },
  });
}

/**
 * 宝库首屏：按关键词聚合（跨批次）。
 *
 * 默认排除已拒绝（rejected）的卡，让宝库只展示"活着"的知识；
 * 传 `onlyAccepted: true` 可进一步只看已入库的。
 */
export async function listKeywordBuckets(
  input: ListKeywordBucketsInput = {}
): Promise<KeywordBucketsResult> {
  return invoke<KeywordBucketsResult>("library_list_keyword_buckets", {
    input: {
      onlyAccepted: input.onlyAccepted ?? false,
    },
  });
}
