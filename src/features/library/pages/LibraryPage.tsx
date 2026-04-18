import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  Check,
  CheckCheck,
  ChevronLeft,
  LibraryBig,
  Search,
  Sparkles,
  Tag as TagIcon,
  Trash2,
  X,
} from "lucide-react";
import { Link } from "react-router-dom";
import clsx from "clsx";
import { Button } from "@/components/Button";
import { FieldLabel, Input, Panel, Tag } from "@/components/Card";
import { EmptyState } from "@/components/EmptyState";
import { Spinner } from "@/components/Spinner";
import { useToast } from "@/lib/toast";
import { humanizeError, formatDateTime, formatRelative } from "@/lib/format";
import {
  forgetBatch,
  listRecentBatches,
  rememberBatch,
  type RecentBatchEntry,
} from "@/lib/recent-batches";
import {
  listGeneratedCards,
  reviewGeneratedCards,
} from "@/features/card-generation/services/api";
import {
  listKeywordBuckets,
  searchByKeyword,
  searchByQuestion,
} from "@/features/library/services/api";
import type {
  GeneratedCard,
  GeneratedCardBatchResult,
  KeywordBucket,
} from "@/types/card";
import type { CommandError } from "@/types/common";

/**
 * 知识宝库 v2：
 *
 * 入口视图（首屏）：
 *   - 顶部：两个搜索框
 *     1. 按关键词：精确匹配（会跳到"该关键词下的所有问题"）
 *     2. 按问题文本：模糊匹配（对 question/definition/explanation/keyword 一起 LIKE）
 *   - 中间：关键词桶列表（跨批次聚合）
 *   - 底部：最近批次（保留，作为辅助入口，支持查看单个批次 + pending 卡入库）
 *
 * 详情视图：
 *   - 「关键词详情」视图：展示该关键词下的所有问题
 *   - 「批次详情」视图：展示某个批次的全部卡片（含 pending 接受/拒绝操作，与 v1 相同）
 *   - 「问题搜索结果」视图：展示问题模糊匹配命中的卡片
 */
type Detail =
  | { kind: "keyword"; keyword: string; cards: GeneratedCard[] }
  | { kind: "search"; query: string; cards: GeneratedCard[] }
  | { kind: "batch"; batch: GeneratedCardBatchResult };

export function LibraryPage() {
  const toast = useToast();
  const [detail, setDetail] = useState<Detail | null>(null);
  const [loading, setLoading] = useState(false);

  // 首屏数据
  const [buckets, setBuckets] = useState<KeywordBucket[]>([]);
  const [recent, setRecent] = useState<RecentBatchEntry[]>([]);

  // 搜索输入
  const [keywordQuery, setKeywordQuery] = useState("");
  const [textQuery, setTextQuery] = useState("");

  const refreshRecent = useCallback(() => {
    setRecent(listRecentBatches());
  }, []);

  const refreshBuckets = useCallback(async () => {
    try {
      const data = await listKeywordBuckets({ onlyAccepted: false });
      setBuckets(data.buckets);
    } catch (err) {
      toast.error("加载关键词失败", humanizeError(err as CommandError));
    }
  }, [toast]);

  useEffect(() => {
    refreshRecent();
    void refreshBuckets();
  }, [refreshRecent, refreshBuckets]);

  // ---- 交互：关键词搜索/桶点击 ----
  const openKeyword = useCallback(
    async (keyword: string) => {
      const kw = keyword.trim();
      if (!kw) return;
      setLoading(true);
      try {
        const data = await searchByKeyword({ keyword: kw, onlyAccepted: false });
        setDetail({ kind: "keyword", keyword: kw, cards: data.cards });
      } catch (err) {
        toast.error("关键词搜索失败", humanizeError(err as CommandError));
      } finally {
        setLoading(false);
      }
    },
    [toast]
  );

  const runTextSearch = useCallback(async () => {
    const q = textQuery.trim();
    if (!q) return;
    setLoading(true);
    try {
      const data = await searchByQuestion({ query: q, onlyAccepted: false, limit: 100 });
      setDetail({ kind: "search", query: q, cards: data.cards });
    } catch (err) {
      toast.error("搜索失败", humanizeError(err as CommandError));
    } finally {
      setLoading(false);
    }
  }, [textQuery, toast]);

  // ---- 批次详情：接受/拒绝 pending 卡片 ----
  const loadBatch = useCallback(
    async (batchId: string) => {
      if (!batchId.trim()) return;
      setLoading(true);
      try {
        const data = await listGeneratedCards(batchId.trim());
        setDetail({ kind: "batch", batch: data });
        rememberBatch({
          batchId: data.batchId,
          title: data.cards[0]?.question ?? data.cards[0]?.keyword,
          cardCount: data.cards.length,
        });
        refreshRecent();
      } catch (err) {
        toast.error("查询失败", humanizeError(err as CommandError));
      } finally {
        setLoading(false);
      }
    },
    [refreshRecent, toast]
  );

  const [actioning, setActioning] = useState(false);

  const runReview = useCallback(
    async (
      batch: GeneratedCardBatchResult,
      targets: GeneratedCard[],
      decision: "accept" | "reject"
    ) => {
      if (targets.length === 0) return;
      setActioning(true);
      try {
        const ids = targets.map((c) => c.cardId);
        const resp = await reviewGeneratedCards({
          batchId: batch.batchId,
          acceptCardIds: decision === "accept" ? ids : [],
          rejectCardIds: decision === "reject" ? ids : [],
        });
        toast.success(
          decision === "accept" ? "已入库" : "已拒绝",
          decision === "accept"
            ? `${resp.acceptedCount} 张卡片已进入复习队列`
            : `${resp.rejectedCount} 张卡片已忽略`
        );
        const next = await listGeneratedCards(batch.batchId);
        setDetail({ kind: "batch", batch: next });
        // 接受/拒绝会让关键词桶的计数变化，顺带刷新一下首屏
        await refreshBuckets();
      } catch (err) {
        toast.error("操作失败", humanizeError(err as CommandError));
      } finally {
        setActioning(false);
      }
    },
    [refreshBuckets, toast]
  );

  // ---- 派生数据 ----
  const detailStats = useMemo(() => {
    if (!detail || detail.kind !== "batch") return null;
    const counts = { accepted: 0, rejected: 0, pending: 0 };
    for (const c of detail.batch.cards) {
      if (c.status === "accepted" || c.status === "active") counts.accepted++;
      else if (c.status === "rejected") counts.rejected++;
      else counts.pending++;
    }
    return counts;
  }, [detail]);

  const pendingCards = useMemo(() => {
    if (!detail || detail.kind !== "batch") return [];
    return detail.batch.cards.filter((c) => c.status === "pending");
  }, [detail]);

  const batchKeywords = useMemo(() => {
    if (!detail || detail.kind !== "batch") return [];
    const seen = new Set<string>();
    const out: Array<{ keyword: string; status: string }> = [];
    for (const c of detail.batch.cards) {
      const key = c.keyword.trim();
      if (!key || seen.has(key)) continue;
      seen.add(key);
      out.push({ keyword: key, status: c.status });
    }
    return out;
  }, [detail]);

  // ===========================================================================
  // 详情视图
  // ===========================================================================
  if (detail) {
    return (
      <div className="space-y-6">
        <header className="flex flex-wrap items-center justify-between gap-3">
          <div>
            {detail.kind === "keyword" && (
              <>
                <h1 className="text-2xl font-semibold text-ink-900">
                  关键词「{detail.keyword}」
                </h1>
                <p className="mt-1 text-sm text-ink-500">
                  该关键词下共 <strong>{detail.cards.length}</strong> 个问题（跨批次聚合）。
                </p>
              </>
            )}
            {detail.kind === "search" && (
              <>
                <h1 className="text-2xl font-semibold text-ink-900">搜索结果</h1>
                <p className="mt-1 text-sm text-ink-500">
                  关键句「{detail.query}」命中 <strong>{detail.cards.length}</strong> 张卡片。
                </p>
              </>
            )}
            {detail.kind === "batch" && (
              <>
                <h1 className="text-2xl font-semibold text-ink-900">批次详情</h1>
                <p className="mt-1 text-sm text-ink-500">
                  Batch <code className="rounded bg-ink-100 px-1 py-0.5">{detail.batch.batchId}</code>
                </p>
              </>
            )}
          </div>
          <Button
            variant="secondary"
            leftIcon={<ChevronLeft className="h-4 w-4" />}
            onClick={() => setDetail(null)}
          >
            返回宝库
          </Button>
        </header>

        {/* 批次场景：关键词一览 + pending 操作 */}
        {detail.kind === "batch" && batchKeywords.length > 0 && (
          <Panel
            title={
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-brand-600" />
                <span>本批关键词</span>
                <Tag tone="brand">{batchKeywords.length} 个</Tag>
              </div>
            }
            description="每个关键词对应本批次中的一张卡片（问题）。点击可跳转到该关键词在宝库中的所有问题。"
          >
            <div className="flex flex-wrap gap-2">
              {batchKeywords.map((k) => (
                <button
                  key={k.keyword}
                  onClick={() => void openKeyword(k.keyword)}
                  className="inline-flex cursor-pointer"
                >
                  <Tag
                    tone={
                      k.status === "accepted" || k.status === "active"
                        ? "success"
                        : k.status === "rejected"
                          ? "warn"
                          : "default"
                    }
                  >
                    {k.keyword}
                  </Tag>
                </button>
              ))}
            </div>
          </Panel>
        )}

        {detail.kind === "batch" && pendingCards.length > 0 && (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
            <div>
              本批次还有 <strong>{pendingCards.length}</strong> 张卡片处于「待定」状态，
              尚未进入复习队列。
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="ghost"
                leftIcon={<X className="h-3.5 w-3.5" />}
                loading={actioning}
                onClick={() => void runReview(detail.batch, pendingCards, "reject")}
              >
                全部拒绝
              </Button>
              <Button
                size="sm"
                variant="success"
                leftIcon={<CheckCheck className="h-3.5 w-3.5" />}
                loading={actioning}
                onClick={() => void runReview(detail.batch, pendingCards, "accept")}
              >
                全部接受入库
              </Button>
            </div>
          </div>
        )}

        {/* 问题列表 */}
        <Panel
          title={
            <div className="flex items-center gap-2">
              <LibraryBig className="h-4 w-4 text-brand-600" />
              <span>
                共 {detail.kind === "batch" ? detail.batch.cards.length : detail.cards.length} 张
              </span>
              {detail.kind === "batch" && detailStats && (
                <>
                  <Tag tone="success">接受 {detailStats.accepted}</Tag>
                  <Tag tone="warn">未决 {detailStats.pending}</Tag>
                  <Tag>拒绝 {detailStats.rejected}</Tag>
                </>
              )}
            </div>
          }
        >
          {(detail.kind === "batch" ? detail.batch.cards : detail.cards).length === 0 ? (
            <EmptyState
              icon={<LibraryBig className="h-8 w-8" />}
              title="没有命中任何问题"
              description="换一个关键词或搜索词试试。"
            />
          ) : (
            <ul className="divide-y divide-ink-100">
              {(detail.kind === "batch" ? detail.batch.cards : detail.cards).map((c) => (
                <QuestionRow
                  key={c.cardId}
                  card={c}
                  showActions={detail.kind === "batch" && c.status === "pending"}
                  actioning={actioning}
                  onAccept={
                    detail.kind === "batch"
                      ? () => void runReview(detail.batch, [c], "accept")
                      : undefined
                  }
                  onReject={
                    detail.kind === "batch"
                      ? () => void runReview(detail.batch, [c], "reject")
                      : undefined
                  }
                  onOpenKeyword={openKeyword}
                />
              ))}
            </ul>
          )}
        </Panel>
      </div>
    );
  }

  // ===========================================================================
  // 入口视图
  // ===========================================================================
  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold text-ink-900">知识宝库</h1>
        <p className="mt-1 text-sm text-ink-500">
          以关键词为索引组织你的问题：每个关键词下可能有多张卡。
          也可以按问题文本做模糊搜索，快速找回之前记过的概念。
        </p>
      </header>

      {/* 两种搜索 */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Panel
          title={
            <div className="flex items-center gap-2">
              <TagIcon className="h-4 w-4 text-brand-600" />
              <span>按关键词搜索</span>
            </div>
          }
          description="精确匹配关键词，返回跨批次所有命中问题。"
        >
          <div className="flex items-end gap-2">
            <div className="flex-1">
              <FieldLabel>关键词</FieldLabel>
              <Input
                type="text"
                placeholder="例如：闭包、动态规划、Transformer"
                value={keywordQuery}
                onChange={(e) => setKeywordQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void openKeyword(keywordQuery);
                }}
              />
            </div>
            <Button
              leftIcon={<Search className="h-4 w-4" />}
              loading={loading}
              onClick={() => void openKeyword(keywordQuery)}
              disabled={!keywordQuery.trim()}
            >
              搜索
            </Button>
          </div>
        </Panel>

        <Panel
          title={
            <div className="flex items-center gap-2">
              <Search className="h-4 w-4 text-brand-600" />
              <span>按问题搜索</span>
            </div>
          }
          description="在问题/定义/解释中做模糊匹配，找回记过的问题。"
        >
          <div className="flex items-end gap-2">
            <div className="flex-1">
              <FieldLabel>关键句 / 问题片段</FieldLabel>
              <Input
                type="text"
                placeholder="例如：梯度消失、为什么、是什么区别"
                value={textQuery}
                onChange={(e) => setTextQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void runTextSearch();
                }}
              />
            </div>
            <Button
              leftIcon={<Search className="h-4 w-4" />}
              loading={loading}
              onClick={() => void runTextSearch()}
              disabled={!textQuery.trim()}
            >
              搜索
            </Button>
          </div>
        </Panel>
      </div>

      {/* 关键词桶列表（首屏主内容） */}
      <Panel
        title={
          <div className="flex items-center gap-2">
            <TagIcon className="h-4 w-4 text-brand-600" />
            <span>关键词一览</span>
            <Tag tone="brand">{buckets.length}</Tag>
          </div>
        }
        description="跨批次聚合。点击任一关键词查看它名下的所有问题。"
        actions={
          <Link to="/generate">
            <Button size="sm" variant="secondary" leftIcon={<Sparkles className="h-3.5 w-3.5" />}>
              去生成
            </Button>
          </Link>
        }
      >
        {buckets.length === 0 ? (
          <EmptyState
            icon={<TagIcon className="h-8 w-8" />}
            title="还没有关键词"
            description="从「生成卡片」开始吧：每张卡会附 3 个关键词，宝库就会以它们为索引组织起来。"
            action={
              <Link to="/generate">
                <Button variant="primary" leftIcon={<Sparkles className="h-4 w-4" />}>
                  去生成第一张卡
                </Button>
              </Link>
            }
          />
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {buckets.map((b) => (
              <button
                key={b.keyword}
                onClick={() => void openKeyword(b.keyword)}
                className={clsx(
                  "group rounded-xl border border-ink-200 bg-white p-4 text-left transition",
                  "hover:-translate-y-0.5 hover:border-brand-300 hover:shadow-card"
                )}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="truncate text-base font-semibold text-ink-900 group-hover:text-brand-700">
                    {b.keyword}
                  </span>
                  <Tag tone="brand">{b.questionCount} 个问题</Tag>
                </div>
                <div className="mt-1 text-[11px] text-ink-500">
                  最近更新 {formatRelative(b.lastUpdatedAt)}
                </div>
                <ul className="mt-2 space-y-1">
                  {b.sampleQuestions.slice(0, 3).map((q) => (
                    <li
                      key={q.cardId}
                      className="line-clamp-1 text-sm leading-5 text-ink-700"
                    >
                      · {q.question}
                    </li>
                  ))}
                  {b.sampleQuestions.length === 0 && (
                    <li className="text-xs italic text-ink-400">（暂无问题）</li>
                  )}
                </ul>
              </button>
            ))}
          </div>
        )}
      </Panel>

      {/* 辅助入口：最近批次 */}
      <Panel
        title="最近批次"
        description="保留批次维度，方便接受/拒绝最近生成的 pending 卡片。"
      >
        {recent.length === 0 && (
          <EmptyState
            icon={<LibraryBig className="h-8 w-8" />}
            title="还没有批次"
            description="新生成的批次会自动出现在这里。"
          />
        )}
        {recent.length > 0 && (
          <ul className="divide-y divide-ink-100">
            {recent.map((b) => (
              <li
                key={b.batchId}
                className="flex items-center gap-3 py-3 transition hover:bg-ink-50/50"
              >
                <button
                  type="button"
                  onClick={() => void loadBatch(b.batchId)}
                  className="flex min-w-0 flex-1 items-start gap-3 text-left"
                >
                  <div className="mt-0.5 rounded-md bg-brand-50 p-1.5 text-brand-600">
                    <Archive className="h-4 w-4" />
                  </div>
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="truncate font-medium text-ink-900">
                        {b.title ?? "未命名批次"}
                      </span>
                      <Tag tone="brand">{b.cardCount} 张</Tag>
                      {b.sourceType && <Tag>{b.sourceType}</Tag>}
                    </div>
                    <div className="mt-0.5 flex items-center gap-2 text-[11px] text-ink-500">
                      <code className="truncate rounded bg-ink-100 px-1 py-0.5">
                        {b.batchId}
                      </code>
                      <span>·</span>
                      <span>{formatRelative(b.createdAt)}</span>
                    </div>
                  </div>
                </button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    forgetBatch(b.batchId);
                    refreshRecent();
                  }}
                  aria-label="从最近记录里移除"
                  title="从最近记录里移除（不会删除数据库里的数据）"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      {loading && (
        <div className="flex justify-center">
          <Spinner label="加载中…" />
        </div>
      )}
    </div>
  );
}

/**
 * 列表中的单张问题行。
 * - 正面：`question`（问题）+ 3 个关键词 tag（可点击跳到关键词详情）
 * - 次行：`definition` 预览、状态、创建时间
 */
function QuestionRow({
  card,
  showActions,
  actioning,
  onAccept,
  onReject,
  onOpenKeyword,
}: {
  card: GeneratedCard;
  showActions: boolean;
  actioning: boolean;
  onAccept?: () => void;
  onReject?: () => void;
  onOpenKeyword: (kw: string) => void;
}) {
  const keywords = card.keywords && card.keywords.length > 0 ? card.keywords : [card.keyword];
  return (
    <li className="flex items-start gap-3 py-3">
      <div className="mt-0.5 rounded-md bg-brand-50 p-1.5 text-brand-600">
        <Archive className="h-4 w-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-start gap-2">
          <span className="flex-1 font-medium text-ink-900">{card.question}</span>
          <Tag
            tone={
              card.status === "accepted" || card.status === "active"
                ? "success"
                : card.status === "rejected"
                  ? "warn"
                  : "default"
            }
          >
            {statusLabel(card.status)}
          </Tag>
          <span className="text-[11px] text-ink-400">
            {formatDateTime(card.createdAt)}
          </span>
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-1.5">
          {keywords.map((k) => (
            <button
              key={k}
              onClick={() => onOpenKeyword(k)}
              className="inline-flex cursor-pointer"
              title={`跳转到关键词「${k}」`}
            >
              <Tag tone="brand">{k}</Tag>
            </button>
          ))}
        </div>
        <p className="mt-1 line-clamp-2 text-sm leading-6 text-ink-700">
          {card.definition}
        </p>
      </div>
      {showActions && (
        <div className="flex flex-none items-center gap-2">
          <Button
            size="sm"
            variant="ghost"
            leftIcon={<X className="h-3.5 w-3.5" />}
            disabled={actioning}
            onClick={onReject}
          >
            拒绝
          </Button>
          <Button
            size="sm"
            variant="success"
            leftIcon={<Check className="h-3.5 w-3.5" />}
            disabled={actioning}
            onClick={onAccept}
          >
            入库
          </Button>
        </div>
      )}
    </li>
  );
}

function statusLabel(status: string): string {
  switch (status) {
    case "accepted":
    case "active":
      return "已接受";
    case "rejected":
      return "已拒绝";
    case "pending":
      return "待定";
    case "archived":
      return "归档";
    default:
      return status;
  }
}
