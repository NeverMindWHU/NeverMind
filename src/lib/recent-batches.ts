/**
 * 本地「最近生成批次」记忆：
 * - 解决了宝库页需要手动输入 UUID 才能检索到刚生成的卡片的问题
 * - 完全在 localStorage 里，不依赖后端新增接口
 * - 同一个 batchId 重复写入时仅更新时间戳并提升到队列头部
 * - 最多保留 `MAX_ENTRIES` 条，按时间从新到旧
 *
 * 注意：这是 UX 层的便利；真正权威的数据源依然是后端的 `list_generated_cards(batchId)`。
 */

const STORAGE_KEY = "nevermind:recent-batches:v1";
const MAX_ENTRIES = 20;

export interface RecentBatchEntry {
  batchId: string;
  /** 首张卡片的 keyword，用作人类可读的标题（可选）。 */
  title?: string;
  /** 本批次最终保留的卡片数（accepted + pending）。 */
  cardCount: number;
  /** 后端 generation_batches.source_type。 */
  sourceType?: string;
  createdAt: string;
}

function read(): RecentBatchEntry[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (x): x is RecentBatchEntry =>
        typeof x === "object" &&
        x !== null &&
        typeof (x as RecentBatchEntry).batchId === "string" &&
        typeof (x as RecentBatchEntry).createdAt === "string"
    );
  } catch {
    return [];
  }
}

function write(entries: RecentBatchEntry[]) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify(entries.slice(0, MAX_ENTRIES))
    );
  } catch {
    // 超出配额时静默忽略；下次写入自然会清退。
  }
}

export function listRecentBatches(): RecentBatchEntry[] {
  return read();
}

export function rememberBatch(
  entry: Omit<RecentBatchEntry, "createdAt"> & { createdAt?: string }
): void {
  const now = entry.createdAt ?? new Date().toISOString();
  const dedup = read().filter((x) => x.batchId !== entry.batchId);
  const next: RecentBatchEntry[] = [
    { ...entry, createdAt: now },
    ...dedup,
  ];
  write(next);
}

export function forgetBatch(batchId: string): void {
  write(read().filter((x) => x.batchId !== batchId));
}

export function clearRecentBatches(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(STORAGE_KEY);
}
