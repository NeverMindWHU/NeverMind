import { useState } from "react";
import { AlertTriangle, Trash2 } from "lucide-react";
import { Button } from "@/components/Button";
import { Input, Panel } from "@/components/Card";

interface Props {
  clearing: boolean;
  onClear: () => Promise<unknown>;
}

const CONFIRM_PHRASE = "清空库";

/**
 * 设置页的"危险区块"：提供一键清库入口。
 *
 * 二次确认流程：
 * 1. 默认状态只显示说明和"开始清库"按钮；
 * 2. 点按钮后进入确认阶段，必须在输入框里键入指定短语才能真正提交；
 * 3. 清库完成会由 `useSettings.clearLibrary` 弹 toast 汇报数字。
 *
 * 设置项（主题、语言、模型配置）不会被影响。
 */
export function DangerZonePanel({ clearing, onClear }: Props) {
  const [confirming, setConfirming] = useState(false);
  const [phrase, setPhrase] = useState("");
  const phraseOk = phrase.trim() === CONFIRM_PHRASE;

  async function handleConfirm() {
    if (!phraseOk || clearing) return;
    try {
      await onClear();
      setConfirming(false);
      setPhrase("");
    } catch {
      // toast 已在 hook 里抛出；这里保持输入状态便于用户重试。
    }
  }

  function handleCancel() {
    if (clearing) return;
    setConfirming(false);
    setPhrase("");
  }

  return (
    <Panel
      title="危险区块"
      description="以下操作不可撤销，请确认已做好备份或已通过导出保存重要数据。"
    >
      <div className="rounded-lg border border-rose-200 bg-rose-50/60 p-4">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-rose-600" />
          <div className="flex-1 space-y-1">
            <div className="text-sm font-semibold text-rose-700">一键清空知识库</div>
            <div className="text-xs leading-relaxed text-rose-700/80">
              会删除 <strong>全部</strong> 卡片、生成批次、复习排程与复习日志。
              设置、模型配置和 API Key <strong>不会</strong> 受影响。
            </div>
          </div>
        </div>

        {!confirming ? (
          <div className="mt-4 flex justify-end">
            <Button
              variant="danger"
              size="sm"
              leftIcon={<Trash2 className="h-4 w-4" />}
              onClick={() => setConfirming(true)}
            >
              开始清库
            </Button>
          </div>
        ) : (
          <div className="mt-4 space-y-3 rounded-md border border-rose-300 bg-white p-3">
            <div className="text-xs text-rose-700">
              请在下方输入 <code className="rounded bg-rose-100 px-1 py-0.5 font-mono text-rose-800">{CONFIRM_PHRASE}</code> 以确认清空。该操作不可撤销。
            </div>
            <Input
              type="text"
              placeholder={CONFIRM_PHRASE}
              value={phrase}
              onChange={(e) => setPhrase(e.target.value)}
              autoFocus
            />
            <div className="flex justify-end gap-2">
              <Button
                variant="secondary"
                size="sm"
                onClick={handleCancel}
                disabled={clearing}
              >
                取消
              </Button>
              <Button
                variant="danger"
                size="sm"
                leftIcon={<Trash2 className="h-4 w-4" />}
                loading={clearing}
                disabled={!phraseOk || clearing}
                onClick={() => void handleConfirm()}
              >
                确认清空
              </Button>
            </div>
          </div>
        )}
      </div>
    </Panel>
  );
}
