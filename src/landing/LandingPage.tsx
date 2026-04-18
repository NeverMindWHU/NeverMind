import bubbleBg from "../../images/光合日历气泡球背景.png";
import pcBg from "../../images/PC背景.png";

const base = import.meta.env.BASE_URL;

function spiritUrl(file: string): string {
  return `${base}spirits/${file}`;
}

const SPIRITS: { file: string; name: string; tag: string; blurb: string }[] = [
  { file: "spirit_air.png", name: "气灵", tag: "节奏与呼吸", blurb: "帮你感知一天的起伏，把碎片时间连成气流。" },
  { file: "spirit_water.png", name: "水灵", tag: "流动与澄清", blurb: "让信息像水一样顺滑汇聚，思绪不再淤塞。" },
  { file: "spirit_soil.png", name: "土灵", tag: "扎根与沉淀", blurb: "把重要事项埋进土壤，慢慢长出可执行的芽。" },
  { file: "spirit_light.png", name: "光灵", tag: "聚焦与能量", blurb: "照亮当下该做的事，把注意力留给真正重要的人与事。" },
  { file: "spirit_nutrition.png", name: "养灵", tag: "补给与恢复", blurb: "提醒你补充能量与空白，日程之外也要好好生长。" },
];

export default function LandingPage() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-emerald-50/90 via-white to-lime-50/80 text-slate-800 antialiased">
      <header className="sticky top-0 z-40 border-b border-emerald-100/80 bg-white/75 backdrop-blur-md">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-3 sm:px-6">
          <a href="#top" className="flex items-center gap-2 text-emerald-950">
            <span className="grid h-9 w-9 place-items-center rounded-xl bg-gradient-to-br from-emerald-400 to-lime-400 text-sm font-bold text-white shadow-sm shadow-emerald-900/15">
              光
            </span>
            <div className="leading-tight">
              <div className="text-sm font-semibold tracking-tight sm:text-base">光合日历</div>
              <div className="text-[11px] text-emerald-800/65 sm:text-xs">桌面观察 · 日程陪伴</div>
            </div>
          </a>
          <nav className="hidden items-center gap-6 text-sm font-medium text-emerald-900/80 sm:flex">
            <a className="hover:text-emerald-700" href="#highlights">
              亮点
            </a>
            <a className="hover:text-emerald-700" href="#spirits">
              五灵
            </a>
            <a className="hover:text-emerald-700" href="#bubble">
              气泡球
            </a>
            <a className="hover:text-emerald-700" href="#faq">
              常见问题
            </a>
          </nav>
          <a
            href="#cta"
            className="shrink-0 rounded-full bg-emerald-600 px-4 py-2 text-sm font-medium text-white shadow-sm shadow-emerald-900/20 transition hover:bg-emerald-500"
          >
            抢先体验
          </a>
        </div>
      </header>

      <main id="top">
        <section className="relative overflow-hidden">
          <div className="pointer-events-none absolute inset-0">
            <img
              src={pcBg}
              alt=""
              className="h-full min-h-[420px] w-full object-cover opacity-[0.42] sm:min-h-[520px]"
              style={{ objectPosition: "center 28%" }}
            />
            <div className="absolute inset-0 bg-gradient-to-b from-emerald-50/20 via-white/88 to-emerald-50/95" />
          </div>

          <div className="relative mx-auto max-w-6xl px-4 pb-16 pt-12 sm:px-6 sm:pb-24 sm:pt-20">
            <p className="mb-3 inline-flex items-center rounded-full border border-emerald-200/80 bg-white/70 px-3 py-1 text-xs font-medium text-emerald-900/80 shadow-sm backdrop-blur">
              灵感来自「把日记做成温柔产品」的体验范式 · 参考{" "}
              <a
                className="ml-1 underline decoration-emerald-300 underline-offset-2 hover:text-emerald-700"
                href="https://www.lifelight.me/"
                target="_blank"
                rel="noreferrer"
              >
                Lifelight
              </a>
            </p>
            <h1 className="max-w-3xl text-4xl font-semibold tracking-tight text-emerald-950 sm:text-5xl sm:leading-[1.1]">
              把日程种在桌面上，
              <span className="text-emerald-700">像植物进行光合作用那样自然生长</span>
            </h1>
            <p className="mt-5 max-w-2xl text-base leading-relaxed text-slate-600 sm:text-lg">
              光合桌面观察串联时间线、前台窗口与剪贴板上下文，用柔和的绿色系界面与桌面气泡球，把「记录—理解—行动」留在手边，而不是淹没在另一个
              App 里不见踪影。
            </p>
            <div id="cta" className="mt-8 flex flex-wrap items-center gap-3">
              <span className="rounded-full border border-emerald-200 bg-white/90 px-4 py-2 text-sm font-medium text-emerald-900 shadow-sm">
                桌面版开发中 · 关注更新
              </span>
              <a
                className="rounded-full border border-emerald-300/80 bg-emerald-600 px-5 py-2.5 text-sm font-semibold text-white shadow-md shadow-emerald-900/15 transition hover:bg-emerald-500"
                href={`${import.meta.env.BASE_URL}index.html`}
              >
                打开调试主界面
              </a>
            </div>
            <p className="mt-4 text-xs text-emerald-900/55">
              上图背景为产品现有「PC 背景」素材，通过 object-cover + object-position 做视口裁切，避免拉伸失真。
            </p>
          </div>
        </section>

        <section id="highlights" className="mx-auto max-w-6xl px-4 py-14 sm:px-6 sm:py-20">
          <h2 className="text-center text-2xl font-semibold text-emerald-950 sm:text-3xl">三种陪伴方式</h2>
          <p className="mx-auto mt-3 max-w-2xl text-center text-sm text-slate-600 sm:text-base">
            借鉴 Lifelight 首页「三句核心价值」的排版节奏，用更符合光合日历的文案呈现。
          </p>
          <div className="mt-10 grid gap-6 sm:grid-cols-3">
            {[
              {
                title: "主动帮你整理「今天发生了什么」",
                body: "时间线把前台与剪贴板串成故事线，让回顾像翻日记一样轻松，而不是冷冰冰的日志列表。",
              },
              {
                title: "理解语境，而不是只会下指令",
                body: "围绕你正在使用的窗口与复制内容做就近分析，回答更贴桌面现场，减少来回切换。",
              },
              {
                title: "记住对你重要的节奏",
                body: "把高频场景与关键节点留在手边：一键 AI 总结、录音标记、气泡球轻打扰提醒。",
              },
            ].map((card) => (
              <article
                key={card.title}
                className="rounded-2xl border border-emerald-100/90 bg-white/85 p-6 shadow-sm shadow-emerald-900/5 backdrop-blur"
              >
                <div className="mb-3 h-1.5 w-10 rounded-full bg-gradient-to-r from-emerald-400 to-lime-400" />
                <h3 className="text-lg font-semibold text-emerald-950">{card.title}</h3>
                <p className="mt-2 text-sm leading-relaxed text-slate-600">{card.body}</p>
              </article>
            ))}
          </div>
        </section>

        <section id="spirits" className="border-y border-emerald-100/80 bg-gradient-to-br from-white via-emerald-50/40 to-lime-50/50 py-14 sm:py-20">
          <div className="mx-auto max-w-6xl px-4 sm:px-6">
            <h2 className="text-center text-2xl font-semibold text-emerald-950 sm:text-3xl">五灵同行</h2>
            <p className="mx-auto mt-3 max-w-2xl text-center text-sm text-slate-600 sm:text-base">
              角色原画来自现有 onboarding 资源（气、水、土、光、养），在网页中以统一卡片栅格展示；头像区域用圆形容器 +
              <code className="mx-1 rounded bg-emerald-100/80 px-1.5 py-0.5 text-xs text-emerald-900">object-cover</code>
              做「软裁切」，适配不同长宽比。
            </p>

            <div className="mt-10 grid gap-5 sm:grid-cols-2 lg:grid-cols-5">
              {SPIRITS.map((s) => (
                <article
                  key={s.file}
                  className="group flex flex-col overflow-hidden rounded-2xl border border-emerald-100/90 bg-white/90 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md"
                >
                  <div className="relative aspect-square overflow-hidden bg-gradient-to-br from-emerald-100/60 to-lime-100/50">
                    <img
                      src={spiritUrl(s.file)}
                      alt={s.name}
                      className="h-full w-full object-cover transition duration-500 group-hover:scale-[1.04]"
                      style={{ objectPosition: "center 35%" }}
                      loading="lazy"
                    />
                    <div className="pointer-events-none absolute inset-x-0 bottom-0 h-1/3 bg-gradient-to-t from-emerald-950/35 to-transparent" />
                    <span className="absolute bottom-3 left-3 rounded-full bg-white/90 px-2.5 py-0.5 text-[11px] font-medium text-emerald-900 shadow-sm">
                      {s.tag}
                    </span>
                  </div>
                  <div className="flex flex-1 flex-col p-4">
                    <h3 className="text-base font-semibold text-emerald-950">{s.name}</h3>
                    <p className="mt-2 flex-1 text-xs leading-relaxed text-slate-600 sm:text-sm">{s.blurb}</p>
                  </div>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section id="bubble" className="mx-auto max-w-6xl px-4 py-14 sm:px-6 sm:py-20">
          <div className="grid items-center gap-10 lg:grid-cols-2">
            <div>
              <h2 className="text-2xl font-semibold text-emerald-950 sm:text-3xl">桌面气泡球</h2>
              <p className="mt-3 text-sm leading-relaxed text-slate-600 sm:text-base">
                复用现有「光合日历气泡球背景」圆形素材：外层
                <code className="mx-1 rounded bg-emerald-100/80 px-1.5 py-0.5 text-xs">rounded-full</code>
                + <code className="rounded bg-emerald-100/80 px-1.5 py-0.5 text-xs">overflow-hidden</code>
                ，内层图片使用 <code className="rounded bg-emerald-100/80 px-1.5 py-0.5 text-xs">object-contain</code> 与桌面版一致，避免形变；整块区域可想象为真实小窗预览。
              </p>
              <ul className="mt-6 space-y-2 text-sm text-emerald-900/85">
                <li className="flex gap-2">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500" />
                  右键菜单、拖动位移、双击回到主界面 —— 轻打扰、强可达。
                </li>
                <li className="flex gap-2">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-lime-500" />
                  与主面板同一视觉基因：绿色渐变 + 通透白底。
                </li>
              </ul>
            </div>

            <div className="flex justify-center lg:justify-end">
              <div className="relative">
                <div
                  className="relative h-56 w-56 overflow-hidden rounded-full border-4 border-white shadow-xl shadow-emerald-900/20 sm:h-64 sm:w-64"
                  style={{ boxShadow: "0 25px 50px -12px rgba(6, 78, 59, 0.35)" }}
                >
                  <img src={bubbleBg} alt="光合日历气泡球预览" className="h-full w-full object-contain bg-emerald-50/30" />
                </div>
                <div className="absolute -bottom-3 left-1/2 w-[118%] max-w-none -translate-x-1/2 rounded-2xl border border-emerald-100/80 bg-white/80 px-4 py-2 text-center text-xs text-emerald-900/75 shadow-sm backdrop-blur sm:text-sm">
                  圆形裁切由 CSS 完成，无需额外导出切图
                </div>
              </div>
            </div>
          </div>
        </section>

        <section id="faq" className="bg-white/70 py-14 sm:py-20">
          <div className="mx-auto max-w-3xl px-4 sm:px-6">
            <h2 className="text-center text-2xl font-semibold text-emerald-950">常见问题</h2>
            <dl className="mt-8 space-y-5">
              {[
                {
                  q: "光合桌面观察是什么？",
                  a: "面向 Windows 的 Tauri 原型：聚合今日时间线、前台窗口快照、剪贴板与录音状态，并可调用方舟模型做当日分析。本页为同仓库内的宣传落地页入口。",
                },
                {
                  q: "数据会离开我的电脑吗？",
                  a: "默认以本地调试为主；是否上传云端取决于后端与模型配置。正式上线前会在隐私政策中逐项说明。",
                },
                {
                  q: "官网素材可以改吗？",
                  a: "可以。Hero 与气泡球区块已直接复用 `images/PC背景.png` 与 `images/光合日历气泡球背景.png`，五灵来自 `public/spirits/`。调整 `object-position` 或容器比例即可改变裁切观感。",
                },
              ].map((item) => (
                <div key={item.q} className="rounded-2xl border border-emerald-100/90 bg-emerald-50/35 p-5">
                  <dt className="font-medium text-emerald-950">{item.q}</dt>
                  <dd className="mt-2 text-sm leading-relaxed text-slate-600">{item.a}</dd>
                </div>
              ))}
            </dl>
          </div>
        </section>
      </main>

      <footer className="border-t border-emerald-100/90 bg-gradient-to-r from-emerald-50/80 to-lime-50/80 py-10 text-center text-sm text-emerald-900/70">
        <p className="font-medium text-emerald-950">光合日历 · 光合桌面观察</p>
        <p className="mt-2 text-xs text-emerald-900/55">© {new Date().getFullYear()} ScheduleApp 本地原型页面</p>
      </footer>
    </div>
  );
}
