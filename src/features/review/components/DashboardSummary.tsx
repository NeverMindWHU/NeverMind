import { Brain, CalendarClock, Check, Flame } from "lucide-react";
import { StatTile } from "@/components/Card";
import { formatRelative } from "@/lib/format";
import type { ReviewDashboardData } from "@/types/review";

interface Props {
  dashboard: ReviewDashboardData | null;
  completedThisSession: number;
}

export function DashboardSummary({ dashboard, completedThisSession }: Props) {
  return (
    <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
      <StatTile
        icon={<Brain className="h-4 w-4" />}
        label="今日待复习"
        value={dashboard?.dueToday ?? "—"}
        hint="未完成的到期卡片数"
      />
      <StatTile
        icon={<Check className="h-4 w-4" />}
        label="今日已完成"
        value={dashboard?.completedToday ?? "—"}
        hint={
          completedThisSession > 0
            ? `本次会话 +${completedThisSession}`
            : "全部完成后可休息"
        }
      />
      <StatTile
        icon={<Flame className="h-4 w-4" />}
        label="连击天数"
        value={dashboard?.streakDays ?? "—"}
        hint="连续有复习记录的天数"
      />
      <StatTile
        icon={<CalendarClock className="h-4 w-4" />}
        label="下一张到期"
        value={formatRelative(dashboard?.nextDueAt)}
        hint={dashboard?.nextDueAt ?? "暂无安排"}
      />
    </div>
  );
}
