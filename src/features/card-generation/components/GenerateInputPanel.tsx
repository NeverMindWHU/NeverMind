import { useRef, useState } from "react";
import { Image as ImageIcon, Trash2, Upload } from "lucide-react";
import clsx from "clsx";
import { Button } from "@/components/Button";
import { FieldLabel, Input, Panel, Textarea, Select } from "@/components/Card";
import { useToast } from "@/lib/toast";
import { formatBytes } from "@/lib/format";
import type { GenerateCardsInput, SourceType } from "@/types/card";

const MAX_IMAGE_COUNT = 8;
const MAX_IMAGE_BYTES = 6 * 1024 * 1024;

interface Props {
  submitting: boolean;
  onSubmit: (input: GenerateCardsInput) => void | Promise<unknown>;
}

interface ImageItem {
  id: string;
  dataUrl: string;
  name: string;
  size: number;
}

/**
 * 卡片生成输入面板：
 * - 文本区域（可选）
 * - 关键词 + 上下文标题（可选）
 * - 图片（可选，多张，前端读为 base64 data URL 后交给 Ark 多模态）
 * - 来源类型选择（手动/划选/导入/图片）
 */
export function GenerateInputPanel({ submitting, onSubmit }: Props) {
  const toast = useToast();
  const [sourceText, setSourceText] = useState("");
  const [selectedKeyword, setSelectedKeyword] = useState("");
  const [contextTitle, setContextTitle] = useState("");
  const [sourceType, setSourceType] = useState<SourceType>("manual");
  const [images, setImages] = useState<ImageItem[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);

  function handlePickImages(files: FileList | null) {
    if (!files || files.length === 0) return;
    const incoming = Array.from(files);
    if (images.length + incoming.length > MAX_IMAGE_COUNT) {
      toast.error("图片数量超限", `单次最多 ${MAX_IMAGE_COUNT} 张`);
      return;
    }
    for (const f of incoming) {
      if (f.size > MAX_IMAGE_BYTES) {
        toast.error("图片过大", `${f.name} 超过 ${formatBytes(MAX_IMAGE_BYTES)}`);
        continue;
      }
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        setImages((prev) => [
          ...prev,
          {
            id: `${f.name}-${f.size}-${prev.length}-${Math.random().toString(16).slice(2, 6)}`,
            dataUrl,
            name: f.name,
            size: f.size,
          },
        ]);
      };
      reader.onerror = () =>
        toast.error("读取失败", `${f.name} 无法读取为 base64`);
      reader.readAsDataURL(f);
    }
  }

  function removeImage(id: string) {
    setImages((prev) => prev.filter((x) => x.id !== id));
  }

  const canSubmit =
    !submitting && (sourceText.trim().length > 0 || images.length > 0);

  function handleSubmit() {
    const input: GenerateCardsInput = {
      sourceText: sourceText.trim(),
      selectedKeyword: selectedKeyword.trim() || null,
      contextTitle: contextTitle.trim() || null,
      sourceType:
        images.length > 0 && sourceText.trim().length === 0
          ? "image"
          : sourceType,
      imageUrls: images.map((i) => i.dataUrl),
    };
    void onSubmit(input);
  }

  return (
    <Panel
      title="新的卡片来源"
      description="支持文本、图片或图文混合；调用豆包 seed-2.0 多模态理解后提炼卡片。填写关注关键词时围绕它提炼 1–3 张；留空则由模型自动挑选 3 个关键词，各生成 1 张，共 3 张。"
    >
      <div className="space-y-5">
        <div>
          <FieldLabel>原始文本</FieldLabel>
          <Textarea
            rows={6}
            placeholder="粘贴一段想学的内容，或直接描述主题。若只想基于图片提炼卡片，此处可留空。"
            value={sourceText}
            onChange={(e) => setSourceText(e.target.value)}
          />
        </div>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>关注关键词（可选）</FieldLabel>
            <Input
              type="text"
              placeholder="如：闭包、P/NP、Transformer"
              value={selectedKeyword}
              onChange={(e) => setSelectedKeyword(e.target.value)}
            />
            <div className="mt-1 text-[11px] leading-relaxed text-ink-500">
              留空则由模型自动挑 3 个关键词，各生成 1 张卡片。
            </div>
          </div>
          <div>
            <FieldLabel>上下文标题（可选）</FieldLabel>
            <Input
              type="text"
              placeholder="来源章节或文章题目"
              value={contextTitle}
              onChange={(e) => setContextTitle(e.target.value)}
            />
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>来源类型</FieldLabel>
            <Select
              value={sourceType}
              onChange={(e) => setSourceType(e.target.value as SourceType)}
              options={[
                { value: "manual", label: "手动输入" },
                { value: "selection", label: "划选文本" },
                { value: "import", label: "导入文件" },
                { value: "image", label: "图片" },
              ]}
            />
          </div>
          <div className="flex items-end">
            <div className="text-xs text-ink-500">
              文本或图片至少填一项；最多 {MAX_IMAGE_COUNT} 张图、
              每张 ≤ {formatBytes(MAX_IMAGE_BYTES)}。
            </div>
          </div>
        </div>

        <div>
          <FieldLabel>图片（可选，多模态）</FieldLabel>
          <div className="flex flex-wrap items-start gap-3">
            {images.map((img) => (
              <div
                key={img.id}
                className="group relative h-24 w-24 overflow-hidden rounded-lg border border-ink-200 bg-ink-50"
              >
                <img src={img.dataUrl} alt={img.name} className="h-full w-full object-cover" />
                <button
                  type="button"
                  onClick={() => removeImage(img.id)}
                  className={clsx(
                    "absolute right-1 top-1 rounded-md bg-black/55 p-1 text-white opacity-0 transition",
                    "group-hover:opacity-100"
                  )}
                  aria-label="移除图片"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
                <div className="pointer-events-none absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-black/70 to-transparent px-1 pb-0.5 pt-2 text-[10px] text-white">
                  {img.name}
                </div>
              </div>
            ))}
            {images.length < MAX_IMAGE_COUNT && (
              <button
                type="button"
                onClick={() => fileInputRef.current?.click()}
                className="flex h-24 w-24 flex-col items-center justify-center gap-1 rounded-lg border border-dashed border-ink-300 bg-ink-50 text-ink-500 hover:border-brand-400 hover:bg-brand-50 hover:text-brand-600"
              >
                <ImageIcon className="h-5 w-5" />
                <span className="text-[11px]">添加图片</span>
              </button>
            )}
            <input
              ref={fileInputRef}
              type="file"
              accept="image/*"
              multiple
              className="hidden"
              onChange={(e) => {
                handlePickImages(e.target.files);
                e.target.value = "";
              }}
            />
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 border-t border-ink-100 pt-4">
          <Button
            variant="primary"
            size="lg"
            loading={submitting}
            disabled={!canSubmit}
            onClick={handleSubmit}
            leftIcon={<Upload className="h-4 w-4" />}
          >
            {submitting ? "正在调用豆包…" : "生成卡片"}
          </Button>
        </div>
      </div>
    </Panel>
  );
}
