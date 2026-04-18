import { useEffect, useState } from "react";
import clsx from "clsx";
import { Check, RotateCw, SkipForward, X, BadgeCheck, Tag as TagIcon } from "lucide-react";
import { Button } from "@/components/Button";
import { Tag } from "@/components/Card";
import { formatRelative } from "@/lib/format";
import type { DueReviewCard, ReviewResult } from "@/types/review";

interface Props {
  card: DueReviewCard;
  submitting: boolean;
  onSubmit: (result: ReviewResult) => void;
}

/**
 * Anki 风格翻卡：
 * - 正面：只显示 keyword（大字）+ meta（复习步骤、到期时间）
 * - 点"显示答案"或空格 → 翻面
 * - 翻面后出现 4 个动作：忘记 / 记住 / 跳过 / 完成
 */
export function ReviewCard({ card, submitting, onSubmit }: Props) {
  const [flipped, setFlipped] = useState(false);

  useEffect(() => {
    setFlipped(false);
  }, [card.reviewId]);

  useEffect(() => {
    function handler(e: KeyboardEvent) {
      if (submitting) return;
      if (e.target && (e.target as HTMLElement).tagName === "INPUT") return;
      if (!flipped && (e.code === "Space" || e.code === "Enter")) {
        e.preventDefault();
        setFlipped(true);
        return;
      }
      if (flipped) {
        if (e.key === "1") onSubmit("forgotten");
        else if (e.key === "2") onSubmit("skipped");
        else if (e.key === "3") onSubmit("remembered");
        else if (e.key === "4") onSubmit("done");
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [flipped, onSubmit, submitting]);

  return (
    <div className="flex flex-col items-stretch gap-6">
      <div className="card-flip-scene mx-auto w-full max-w-2xl">
        <div
          className={clsx(
            "card-flip-inner relative min-h-[20rem] w-full",
            flipped && "is-flipped"
          )}
        >
          {/* 正面 */}
          <section
            className={clsx(
              "card-face absolute inset-0 flex flex-col items-center justify-center rounded-3xl border border-ink-200 bg-white p-8 shadow-card",
              "cursor-pointer select-none"
            )}
            onClick={() => !flipped && setFlipped(true)}
            role="button"
            aria-label="显示答案"
          >
            <div className="mb-4 flex items-center gap-2 text-xs text-ink-500">
              <BadgeCheck className="h-3.5 w-3.5" />
              第 {card.reviewStep} 次复习 · 到期 {formatRelative(card.dueAt)}
            </div>
            <h2 className="text-center text-2xl font-semibold leading-snug text-ink-900 md:text-3xl">
              {card.question || `${card.keyword}是什么？`}
            </h2>
            {(card.keywords?.length ?? 0) > 0 && (
              <div className="mt-4 flex flex-wrap justify-center gap-1.5">
                {card.keywords.map((t) => (
                  <Tag key={t} tone="brand">
                    <TagIcon className="mr-1 h-3 w-3" />
                    {t}
                  </Tag>
                ))}
              </div>
            )}
            <div className="mt-8 text-xs text-ink-400">
              点击卡片或按空格显示答案
            </div>
          </section>

          {/* 背面 */}
          <section
            className={clsx(
              "card-face card-face-back absolute inset-0 flex flex-col gap-4 overflow-auto rounded-3xl border border-ink-200 bg-white p-8 shadow-card"
            )}
          >
            <header>
              <div className="text-[11px] uppercase tracking-wider text-ink-400">
                Question
              </div>
              <h2 className="mt-0.5 text-xl font-semibold leading-snug text-ink-900">
                {card.question || `${card.keyword}是什么？`}
              </h2>
              {(card.keywords?.length ?? 0) > 0 && (
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {card.keywords.map((k) => (
                    <Tag key={k} tone="brand">
                      {k}
                    </Tag>
                  ))}
                </div>
              )}
            </header>

            <section>
              <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-ink-400">
                定义
              </div>
              <p className="text-sm leading-6 text-ink-800">{card.definition}</p>
            </section>

            {card.explanation && (
              <section>
                <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-ink-400">
                  解释
                </div>
                <p className="whitespace-pre-wrap text-sm leading-6 text-ink-700">
                  {card.explanation}
                </p>
              </section>
            )}
          </section>
        </div>
      </div>

      {!flipped ? (
        <div className="flex justify-center">
          <Button size="lg" onClick={() => setFlipped(true)}>
            显示答案（空格）
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
          <Button
            variant="danger"
            size="lg"
            loading={submitting}
            leftIcon={<RotateCw className="h-4 w-4" />}
            onClick={() => onSubmit("forgotten")}
          >
            忘记 · 1
          </Button>
          <Button
            variant="secondary"
            size="lg"
            disabled={submitting}
            leftIcon={<SkipForward className="h-4 w-4" />}
            onClick={() => onSubmit("skipped")}
          >
            跳过 · 2
          </Button>
          <Button
            variant="primary"
            size="lg"
            loading={submitting}
            leftIcon={<Check className="h-4 w-4" />}
            onClick={() => onSubmit("remembered")}
          >
            记住 · 3
          </Button>
          <Button
            variant="success"
            size="lg"
            disabled={submitting}
            leftIcon={<X className="h-4 w-4" />}
            onClick={() => onSubmit("done")}
          >
            彻底掌握 · 4
          </Button>
        </div>
      )}
    </div>
  );
}
