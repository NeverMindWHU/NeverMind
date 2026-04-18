import { useState } from "react";
import { Activity, CheckCircle2, Plus, Save, Zap } from "lucide-react";
import { Button } from "@/components/Button";
import { FieldLabel, Input, Panel, Select, Tag } from "@/components/Card";
import { EmptyState } from "@/components/EmptyState";
import type {
  ModelProfileItem,
  ModelProvider,
  SaveModelProfileInput,
  TestModelProfileInput,
} from "@/types/settings";

interface Props {
  profiles: ModelProfileItem[];
  onSave: (input: SaveModelProfileInput) => Promise<unknown>;
  onTest: (input: TestModelProfileInput) => Promise<unknown>;
}

const DEFAULT_TIMEOUT_MS = 15_000;

export function ModelProfilesPanel({ profiles, onSave, onTest }: Props) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<SaveModelProfileInput>({
    profileId: null,
    name: "",
    provider: "openai-compatible",
    endpoint: "",
    apiKey: "",
    model: "",
    timeoutMs: DEFAULT_TIMEOUT_MS,
  });
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);

  function startCreate() {
    setEditingId("__new__");
    setDraft({
      profileId: null,
      name: "",
      provider: "openai-compatible",
      endpoint: "",
      apiKey: "",
      model: "",
      timeoutMs: DEFAULT_TIMEOUT_MS,
    });
  }

  function startEdit(p: ModelProfileItem) {
    setEditingId(p.profileId);
    setDraft({
      profileId: p.profileId,
      name: p.name,
      provider: p.provider as ModelProvider,
      endpoint: p.endpoint,
      apiKey: "",
      model: "",
      timeoutMs: DEFAULT_TIMEOUT_MS,
    });
  }

  async function handleSave() {
    setSaving(true);
    try {
      await onSave({
        ...draft,
        model: draft.model?.trim() || null,
      });
      setEditingId(null);
    } finally {
      setSaving(false);
    }
  }

  async function handleTest() {
    setTesting(true);
    try {
      await onTest({
        profileId: draft.profileId ?? null,
        provider: draft.provider,
        endpoint: draft.endpoint,
        apiKey: draft.apiKey,
        model: draft.model?.trim() || null,
        timeoutMs: draft.timeoutMs,
      });
    } finally {
      setTesting(false);
    }
  }

  return (
    <Panel
      title="模型配置"
      description="管理多个 LLM 连接；当前卡片生成优先使用环境变量里的 Ark 豆包客户端。"
      actions={
        <Button
          size="sm"
          variant="secondary"
          leftIcon={<Plus className="h-3.5 w-3.5" />}
          onClick={startCreate}
        >
          新增配置
        </Button>
      }
    >
      {profiles.length === 0 && editingId === null ? (
        <EmptyState
          icon={<Zap className="h-8 w-8" />}
          title="还没有自定义模型"
          description="Ark 豆包通过 .env 的 ARK_API_KEY 自动接入，也可以在这里登记其他 OpenAI 兼容接口。"
          action={
            <Button variant="secondary" onClick={startCreate} leftIcon={<Plus className="h-4 w-4" />}>
              新增第一项
            </Button>
          }
        />
      ) : (
        <div className="space-y-2">
          {profiles.map((p) => (
            <div
              key={p.profileId}
              className="flex items-center justify-between gap-3 rounded-lg border border-ink-100 bg-white px-4 py-3"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="truncate font-medium text-ink-900">{p.name}</span>
                  {p.isDefault && <Tag tone="brand">默认</Tag>}
                  {p.isAvailable ? (
                    <Tag tone="success">
                      <CheckCircle2 className="mr-1 h-3 w-3" />
                      可用
                    </Tag>
                  ) : (
                    <Tag tone="warn">未验证</Tag>
                  )}
                </div>
                <div className="mt-0.5 truncate text-xs text-ink-500">
                  {p.provider} · {p.endpoint}
                </div>
              </div>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => startEdit(p)}
              >
                编辑
              </Button>
            </div>
          ))}
        </div>
      )}

      {editingId && (
        <div className="mt-5 rounded-xl border border-ink-200 bg-ink-50/30 p-4">
          <div className="mb-3 text-sm font-medium text-ink-800">
            {editingId === "__new__" ? "新增模型配置" : "编辑模型配置"}
          </div>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <div>
              <FieldLabel required>名称</FieldLabel>
              <Input
                type="text"
                placeholder="如：Qwen-Plus"
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              />
            </div>
            <div>
              <FieldLabel required>Provider</FieldLabel>
              <Select
                value={draft.provider}
                onChange={(e) =>
                  setDraft({ ...draft, provider: e.target.value as ModelProvider })
                }
                options={[
                  { value: "openai-compatible", label: "OpenAI 兼容" },
                  { value: "qwen", label: "Qwen / DashScope" },
                  { value: "custom", label: "自定义" },
                ]}
              />
            </div>
            <div className="md:col-span-2">
              <FieldLabel required>Endpoint</FieldLabel>
              <Input
                type="text"
                placeholder="https://api.openai.com/v1"
                value={draft.endpoint}
                onChange={(e) => setDraft({ ...draft, endpoint: e.target.value })}
              />
            </div>
            <div className="md:col-span-2">
              <FieldLabel required>API Key</FieldLabel>
              <Input
                type="password"
                placeholder="sk-..."
                value={draft.apiKey}
                onChange={(e) => setDraft({ ...draft, apiKey: e.target.value })}
              />
            </div>
            <div>
              <FieldLabel>Model</FieldLabel>
              <Input
                type="text"
                placeholder="可选：model 名"
                value={draft.model ?? ""}
                onChange={(e) => setDraft({ ...draft, model: e.target.value })}
              />
            </div>
            <div>
              <FieldLabel>Timeout (ms)</FieldLabel>
              <Input
                type="number"
                value={draft.timeoutMs}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    timeoutMs: Math.max(
                      1000,
                      Math.min(120000, Number(e.target.value) || DEFAULT_TIMEOUT_MS)
                    ),
                  })
                }
              />
            </div>
          </div>
          <div className="mt-4 flex items-center justify-end gap-2">
            <Button variant="ghost" onClick={() => setEditingId(null)}>
              取消
            </Button>
            <Button
              variant="secondary"
              leftIcon={<Activity className="h-4 w-4" />}
              loading={testing}
              onClick={handleTest}
            >
              测试连通性
            </Button>
            <Button
              leftIcon={<Save className="h-4 w-4" />}
              loading={saving}
              onClick={handleSave}
            >
              保存
            </Button>
          </div>
        </div>
      )}
    </Panel>
  );
}
