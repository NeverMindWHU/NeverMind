import { useState } from "react";
import clsx from "clsx";
import { Check, X, RotateCw, Tag as TagIcon } from "lucide-react";
import { Button } from "@/components/Button";
import { Tag } from "@/components/Card";
import type { GeneratedCard } from "@/types/card";

type Decision = "accept" | "reject" | "pending";

interface Props {
  index: number;
  card: GeneratedCard;
  decision: Decision;
  onAccept: () => void;
  onReject: () => void;
}

/**
 * Anki 风格卡片单元：
 * - 上半是正面（keyword + 定义），下半是解释与扩展
 * - 右上角是"接受 / 拒绝"开关
 */
export function GeneratedCardItem({ index, card, decision, onAccept, onReject }: Props) {
  const [expanded, setExpanded] = useState(true);

  const accentClass =
    decision === "accept"
      ? "border-emerald-300 shadow-[0_0_0_1px_rgba(16,185,129,0.25)]"
      : decision === "reject"
        ? "border-rose-200 opacity-60"
        : "border-ink-200";

  return (
    <article
      className={clsx(
        "rounded-2xl border bg-white p-5 shadow-card transition",
        accentClass
      )}
    >
      <header className="mb-3 flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="text-[11px] uppercase tracking-wider text-ink-400">
            Card #{index + 1}
          </div>
          <h3 className="mt-0.5 truncate text-lg font-semibold text-ink-900">
            {card.keyword}
          </h3>
        </div>
        <div className="flex flex-none items-center gap-2">
          <Button
            size="sm"
            variant={decision === "reject" ? "danger" : "ghost"}
            leftIcon={<X className="h-3.5 w-3.5" />}
            onClick={onReject}
          >
            拒绝
          </Button>
          <Button
            size="sm"
            variant={decision === "accept" ? "success" : "secondary"}
            leftIcon={<Check className="h-3.5 w-3.5" />}
            onClick={onAccept}
          >
            {decision === "accept" ? "已接受" : "接受"}
          </Button>
        </div>
      </header>

      <p className="text-sm leading-6 text-ink-800">{card.definition}</p>

      {expanded && (
        <div className="mt-4 space-y-3 border-t border-ink-100 pt-4 text-sm">
          {card.explanation && (
            <section>
              <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-ink-400">
                解释
              </div>
              <p className="whitespace-pre-wrap leading-6 text-ink-700">{card.explanation}</p>
            </section>
          )}
          {card.relatedTerms.length > 0 && (
            <section>
              <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-ink-400">
                相关词
              </div>
              <div className="flex flex-wrap gap-1.5">
                {card.relatedTerms.map((t) => (
                  <Tag key={t} tone="brand">
                    <TagIcon className="mr-1 h-3 w-3" />
                    {t}
                  </Tag>
                ))}
              </div>
            </section>
          )}
          {card.scenarios.length > 0 && (
            <section>
              <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-ink-400">
                应用场景
              </div>
              <ul className="list-disc space-y-0.5 pl-5 leading-6 text-ink-700">
                {card.scenarios.map((s, i) => (
                  <li key={i}>{s}</li>
                ))}
              </ul>
            </section>
          )}
          {card.sourceExcerpt && (
            <section>
              <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-ink-400">
                原文摘录
              </div>
              <blockquote className="rounded-md border-l-4 border-ink-200 bg-ink-50 px-3 py-2 text-xs italic leading-5 text-ink-600">
                {card.sourceExcerpt}
              </blockquote>
            </section>
          )}
        </div>
      )}

      <footer className="mt-3 flex items-center justify-end">
        <button
          type="button"
          onClick={() => setExpanded((x) => !x)}
          className="inline-flex items-center gap-1 text-xs text-ink-500 hover:text-ink-700"
        >
          <RotateCw className="h-3 w-3" />
          {expanded ? "收起详情" : "展开详情"}
        </button>
      </footer>
    </article>
  );
}
