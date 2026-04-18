import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  Check,
  CheckCheck,
  ChevronLeft,
  FileSearch,
  LibraryBig,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { Link } from "react-router-dom";
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
import type { GeneratedCard, GeneratedCardBatchResult } from "@/types/card";
import type { CommandError } from "@/types/common";

/**
 * 宝库 MVP：
 * - 主入口：从 localStorage 读「最近批次」列表，一键查看
 * - 辅助入口：手动输入 Batch ID（UUID）查询任意批次
 * - 查询结果使用后端 `list_generated_cards(batchId)` 命令
 */
export function LibraryPage() {
  const toast = useToast();
  const [recent, setRecent] = useState<RecentBatchEntry[]>([]);
  const [batchId, setBatchId] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<GeneratedCardBatchResult | null>(null);

  const refreshRecent = useCallback(() => {
    setRecent(listRecentBatches());
  }, []);

  useEffect(() => {
    refreshRecent();
  }, [refreshRecent]);

  const load = useCallback(
    async (id: string) => {
      if (!id.trim()) return;
      setLoading(true);
      try {
        const data = await listGeneratedCards(id.trim());
        setResult(data);
        // 手动查询后也登记到最近列表，方便二次查看。
        const title =
          recent.find((r) => r.batchId === id.trim())?.title ??
          data.cards[0]?.keyword;
        rememberBatch({
          batchId: data.batchId,
          title,
          cardCount: data.cards.length,
        });
        refreshRecent();
      } catch (err) {
        toast.error("查询失败", humanizeError(err as CommandError));
        setResult(null);
      } finally {
        setLoading(false);
      }
    },
    [recent, refreshRecent, toast]
  );

  const stats = useMemo(() => {
    if (!result) return null;
    const counts = { accepted: 0, rejected: 0, pending: 0 };
    for (const c of result.cards) {
      if (c.status === "accepted" || c.status === "active") counts.accepted++;
      else if (c.status === "rejected") counts.rejected++;
      else counts.pending++;
    }
    return counts;
  }, [result]);

  const pendingCards = useMemo(
    () => (result?.cards ?? []).filter((c) => c.status === "pending"),
    [result]
  );

  // 本批次覆盖到的所有关键词（等价于每张卡 card.keyword）。
  // 去重后保持原顺序，便于用户一眼看到"3 个问题 = 3 个关键词"的对应。
  const batchKeywords = useMemo(() => {
    if (!result) return [] as Array<{ keyword: string; status: string }>;
    const seen = new Set<string>();
    const out: Array<{ keyword: string; status: string }> = [];
    for (const c of result.cards) {
      const key = c.keyword.trim();
      if (!key || seen.has(key)) continue;
      seen.add(key);
      out.push({ keyword: key, status: c.status });
    }
    return out;
  }, [result]);

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
        // 重新拉一次这个批次的最新状态
        const next = await listGeneratedCards(batch.batchId);
        setResult(next);
        rememberBatch({
          batchId: next.batchId,
          title: next.cards[0]?.keyword,
          cardCount: next.cards.length,
        });
        refreshRecent();
      } catch (err) {
        toast.error("操作失败", humanizeError(err as CommandError));
      } finally {
        setActioning(false);
      }
    },
    [refreshRecent, toast]
  );

  // ---- 查看结果视图 ----
  if (result) {
    return (
      <div className="space-y-6">
        <header className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold text-ink-900">批次详情</h1>
            <p className="mt-1 text-sm text-ink-500">
              Batch <code className="rounded bg-ink-100 px-1 py-0.5">{result.batchId}</code>
            </p>
          </div>
          <Button
            variant="secondary"
            leftIcon={<ChevronLeft className="h-4 w-4" />}
            onClick={() => {
              setResult(null);
              setBatchId("");
            }}
          >
            返回宝库
          </Button>
        </header>

        {batchKeywords.length > 0 && (
          <Panel
            title={
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-brand-600" />
                <span>本批关键词</span>
                <Tag tone="brand">{batchKeywords.length} 个</Tag>
              </div>
            }
            description="每个关键词对应本批次中的一张卡片（问题）。"
          >
            <div className="flex flex-wrap gap-2">
              {batchKeywords.map((k) => (
                <Tag
                  key={k.keyword}
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
              ))}
            </div>
          </Panel>
        )}

        {pendingCards.length > 0 && (
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
                onClick={() => void runReview(result, pendingCards, "reject")}
              >
                全部拒绝
              </Button>
              <Button
                size="sm"
                variant="success"
                leftIcon={<CheckCheck className="h-3.5 w-3.5" />}
                loading={actioning}
                onClick={() => void runReview(result, pendingCards, "accept")}
              >
                全部接受入库
              </Button>
            </div>
          </div>
        )}

        <Panel
          title={
            <div className="flex items-center gap-2">
              <LibraryBig className="h-4 w-4 text-brand-600" />
              <span>共 {result.cards.length} 张</span>
              {stats && (
                <>
                  <Tag tone="success">接受 {stats.accepted}</Tag>
                  <Tag tone="warn">未决 {stats.pending}</Tag>
                  <Tag>拒绝 {stats.rejected}</Tag>
                </>
              )}
            </div>
          }
        >
          <ul className="divide-y divide-ink-100">
            {result.cards.map((c) => (
              <li key={c.cardId} className="flex items-start gap-3 py-3">
                <div className="mt-0.5 rounded-md bg-brand-50 p-1.5 text-brand-600">
                  <Archive className="h-4 w-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="truncate font-medium text-ink-900">
                      {c.keyword}
                    </span>
                    <Tag
                      tone={
                        c.status === "accepted" || c.status === "active"
                          ? "success"
                          : c.status === "rejected"
                            ? "warn"
                            : "default"
                      }
                    >
                      {statusLabel(c.status)}
                    </Tag>
                    <span className="text-[11px] text-ink-400">
                      {formatDateTime(c.createdAt)}
                    </span>
                  </div>
                  <p className="mt-1 line-clamp-2 text-sm leading-6 text-ink-700">
                    {c.definition}
                  </p>
                </div>
                {c.status === "pending" && (
                  <div className="flex flex-none items-center gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      leftIcon={<X className="h-3.5 w-3.5" />}
                      disabled={actioning}
                      onClick={() => void runReview(result, [c], "reject")}
                    >
                      拒绝
                    </Button>
                    <Button
                      size="sm"
                      variant="success"
                      leftIcon={<Check className="h-3.5 w-3.5" />}
                      disabled={actioning}
                      onClick={() => void runReview(result, [c], "accept")}
                    >
                      入库
                    </Button>
                  </div>
                )}
              </li>
            ))}
          </ul>
        </Panel>
      </div>
    );
  }

  // ---- 入口视图：最近批次 + 手动查询 ----
  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold text-ink-900">知识宝库</h1>
        <p className="mt-1 text-sm text-ink-500">
          展示最近生成的批次。后续版本会加入全文搜索、标签浏览与导出 Anki/Markdown。
        </p>
      </header>

      <Panel
        title="最近批次"
        description="生成和保存过的批次会自动出现在这里（仅存在本机）。"
        actions={
          <Link to="/generate">
            <Button size="sm" variant="secondary" leftIcon={<Sparkles className="h-3.5 w-3.5" />}>
              去生成
            </Button>
          </Link>
        }
      >
        {loading && (
          <div className="flex justify-center py-6">
            <Spinner label="加载批次…" />
          </div>
        )}
        {!loading && recent.length === 0 && (
          <EmptyState
            icon={<LibraryBig className="h-8 w-8" />}
            title="还没有生成过批次"
            description="从「生成卡片」页面开始你的第一批知识卡片吧。"
          />
        )}
        {!loading && recent.length > 0 && (
          <ul className="divide-y divide-ink-100">
            {recent.map((b) => (
              <li
                key={b.batchId}
                className="flex items-center gap-3 py-3 transition hover:bg-ink-50/50"
              >
                <button
                  type="button"
                  onClick={() => load(b.batchId)}
                  className="flex min-w-0 flex-1 items-start gap-3 text-left"
                >
                  <div className="mt-0.5 rounded-md bg-brand-50 p-1.5 text-brand-600">
                    <LibraryBig className="h-4 w-4" />
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

      <Panel
        title="按 Batch ID 查询"
        description="如果批次不在上方列表（例如换了机器或清理过缓存），可以手动输入 UUID。"
      >
        <div className="flex items-end gap-3">
          <div className="flex-1">
            <FieldLabel>Batch ID</FieldLabel>
            <Input
              type="text"
              placeholder="00000000-0000-0000-0000-000000000000"
              value={batchId}
              onChange={(e) => setBatchId(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void load(batchId);
              }}
            />
          </div>
          <Button
            leftIcon={<FileSearch className="h-4 w-4" />}
            loading={loading}
            onClick={() => void load(batchId)}
            disabled={!batchId.trim()}
          >
            查看批次
          </Button>
        </div>
      </Panel>
    </div>
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
