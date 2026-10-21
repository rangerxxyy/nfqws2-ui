import { useEffect } from 'react';
import { bootPanel } from './panel';

export default function App() {
  useEffect(() => { bootPanel(); }, []);
  return (
    <>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
    <header className={"bg-[#090e1a]/95 backdrop-blur-md border-b border-slate-800/90 shrink-0 z-40 px-4 lg:px-6 py-2.5"}>
      {"\n    "}
      <div className={"max-w-[1920px] mx-auto grid grid-cols-1 lg:grid-cols-12 items-center gap-3"}>
        {"\n      \n      "}
        {"\n      "}
        <div className={"lg:col-span-5 flex flex-wrap items-center gap-2.5 justify-between lg:justify-start"}>
          {"\n        \n        "}
          {"\n        "}
          <div className={"flex items-center gap-2.5 bg-[#060a12] border border-slate-800/90 px-3.5 py-2 rounded-xl shadow-inner"}>
            {"\n          "}
            <div className={"relative flex items-center justify-center w-3 h-3"}>
              {"\n            "}
              <span id={"statusRadarRing"} className={"absolute inline-flex h-full w-full rounded-full bg-slate-400"}></span>
              {"\n            "}
              <span id={"statusMainDot"} className={"relative inline-flex rounded-full h-2.5 w-2.5 bg-slate-400"}></span>
              {"\n          "}
            </div>
            {"\n          "}
            <span id={"statusTextLabel"} className={"text-xs sm:text-sm font-semibold text-slate-400 tracking-tight"}>
              {"Требуется вход"}
            </span>
            <span id={"statusDetail"} className={"sr-only"}></span>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div className={"flex items-center gap-2.5"}>
            {"\n          \n          "}
            {"\n          "}
            <div className={"bg-[#060a12] border border-slate-800/90 px-3 py-1.5 rounded-xl flex items-center gap-2.5"}>
              {"\n            "}
              <div className={"p-1.5 rounded-lg bg-sky-500/10 text-sky-400 border border-sky-500/20"} title={"Оперативная память роутера"}>
                {"\n              "}
                <svg className={"w-4 h-4"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  {"\n                "}
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"1.9"} d={"M3 7h18v7H3V7zm3 7v3m4-3v3m4-3v3m4-3v3M7 10h2m4 0h2"}></path>
                  {"\n              "}
                </svg>
                {"\n            "}
              </div>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex items-center gap-1.5 text-xs font-mono"}>
                  {"\n                "}
                  <span className={"text-slate-400 font-semibold"}>
                    {"RAM"}
                  </span>
                  {"\n                "}
                  <span id={"ramMetricText"} className={"text-slate-100 font-bold"}>
                    {"—"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"w-24 sm:w-28 bg-slate-800/90 h-1.5 rounded-full overflow-hidden mt-1"}>
                  {"\n                "}
                  <div id={"ramProgressBar"} className={"bg-sky-400 h-full rounded-full transition-all duration-500"} style={{ width: "51.7%" }}></div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"bg-[#060a12] border border-slate-800/90 px-3 py-1.5 rounded-xl flex items-center gap-2.5"}>
              {"\n            "}
              <div className={"p-1.5 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"} title={"Загрузка процессора"}>
                {"\n              "}
                <svg className={"w-4 h-4"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  {"\n                "}
                  <rect x={"6"} y={"6"} width={"12"} height={"12"} rx={"2"} strokeWidth={"1.9"}></rect>
                  {"\n                "}
                  <rect x={"9"} y={"9"} width={"6"} height={"6"} rx={"1"} strokeWidth={"1.7"}></rect>
                  {"\n                "}
                  <path strokeLinecap={"round"} strokeWidth={"1.9"} d={"M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3"}></path>
                  {"\n              "}
                </svg>
                {"\n            "}
              </div>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex items-center gap-1.5 text-xs font-mono"}>
                  {"\n                "}
                  <span className={"text-slate-400 font-semibold"}>
                    {"CPU"}
                  </span>
                  {"\n                "}
                  <span id={"cpuMetricText"} className={"text-emerald-400 font-bold"}>
                    {"—"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"w-14 sm:w-16 bg-slate-800/90 h-1.5 rounded-full overflow-hidden mt-1"}>
                  {"\n                "}
                  <div id={"cpuProgressBar"} className={"bg-emerald-400 h-full rounded-full transition-all duration-500"} style={{ width: "8%" }}></div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n      "}
        {"\n      "}
        <div className={"lg:col-span-2 flex items-center justify-center order-first lg:order-none"}>
          {"\n        "}
          <span className={"text-lg sm:text-xl font-bold tracking-tight text-white select-none"}>
            {"\n          nfqws2"}
            <span className={"text-blue-400 drop-shadow-[0_0_8px_rgba(96,165,250,0.35)]"}>
              {"-UI"}
            </span>
            {"\n        "}
          </span>
          {"\n      "}
        </div>
        {"\n\n      "}
        {"\n      "}
        <div className={"lg:col-span-5 flex flex-wrap items-center justify-end gap-2"}>
          {"\n        \n        "}
          {"\n        "}
          <button data-action-click={"openUpdateModal"} data-args={JSON.stringify(["core"])} title={"Обновить бинарное ядро nfqws2"} className={"version-pill px-3.5 py-2 rounded-xl flex items-center gap-2 text-xs sm:text-sm cursor-pointer"}>
            {"\n          "}
            <svg className={"w-4 h-4 text-slate-200"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <rect x={"6"} y={"6"} width={"12"} height={"12"} rx={"2"} strokeWidth={"1.9"}></rect>
              {"\n            "}
              <rect x={"9.5"} y={"9.5"} width={"5"} height={"5"} rx={"0.8"} strokeWidth={"1.7"}></rect>
              {"\n            "}
              <path strokeLinecap={"round"} strokeWidth={"1.9"} d={"M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <span className={"font-bold text-white tracking-tight"}>
              {"nfqws2"}
            </span>
            {"\n          "}
            <span id={"headerCoreVersion"} className={"font-mono text-slate-400 font-medium"}>
              {"—"}
            </span>
            {"\n        "}
          </button>
          {"\n\n        "}
          {"\n        "}
          <button data-action-click={"openUpdateModal"} data-args={JSON.stringify(["ui"])} title={"Обновить веб-панель nfqws2-UI"} className={"version-pill px-3.5 py-2 rounded-xl flex items-center gap-2 text-xs sm:text-sm cursor-pointer"}>
            {"\n          "}
            <svg className={"w-4 h-4 text-slate-200"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"1.9"} d={"M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"}></path>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"1.9"} d={"M3.27 6.96L12 12.01l8.73-5.05M12 22.08V12"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <span id={"headerUiVersion"} className={"font-bold text-white font-mono"}>
              {"—"}
            </span>
            {"\n        "}
          </button>
          {"\n\n        "}
          <div className={"h-5 w-[1px] bg-slate-800 mx-0.5 hidden sm:block"}></div>
          {"\n\n        "}
          {"\n        "}
          <button data-action-click={"restartService"} className={"px-3.5 py-2 rounded-xl bg-slate-800/90 hover:bg-slate-700 text-slate-100 border border-slate-700 text-xs sm:text-sm font-medium flex items-center gap-2 transition-all active:scale-95"}>
            {"\n          "}
            <svg id={"restartIcon"} className={"w-4 h-4 text-blue-400 transition-transform duration-500"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <span>
              {"Перезапустить"}
            </span>
            {"\n        "}
          </button>
          {"\n\n        "}
          {"\n        "}
          <button id={"togglePowerBtn"} data-action-click={"toggleDaemonPower"} className={"px-3.5 py-2 rounded-xl bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border border-rose-500/30 text-xs sm:text-sm font-semibold flex items-center gap-2 transition-all active:scale-95"}>
            {"\n          "}
            <svg className={"w-4 h-4"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <span id={"togglePowerText"}>
              {"Остановить"}
            </span>
            {"\n        "}
          </button>
          {"\n\n      "}
        </div>
        {"\n\n    "}
      </div>
      {"\n  "}
    </header>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
    <div className={"flex-1 flex overflow-hidden max-w-[1920px] w-full mx-auto"}>
      {"\n\n    "}
      {"\n    "}
      <aside className={"w-64 lg:w-72 bg-[#090e1a] border-r border-slate-800/90 flex flex-col justify-between shrink-0 overflow-y-auto select-none"}>
        {"\n      \n      "}
        <div className={"p-3.5 space-y-5"}>
          {"\n        \n        "}
          {"\n        "}
          <div>
            {"\n          "}
            <div className={"px-3 mb-2 text-[11px] font-mono font-bold uppercase tracking-wider text-slate-500"}>
              {"\n            Управление и Правила\n          "}
            </div>
            {"\n          "}
            <nav className={"space-y-1"}>
              {"\n            \n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["conf"])} id={"navBtn-conf"} className={"side-nav-btn active w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-semibold border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Конфигурация"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[11px] font-mono px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300"}>
                  {"conf"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["lists"])} id={"navBtn-lists"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-sky-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Списки доменов и IP"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[11px] font-mono px-1.5 py-0.5 rounded bg-slate-800 text-slate-300"}>
                  {"5"}
                </span>
                {"\n            "}
              </button>
              {"\n\n          "}
            </nav>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div>
            {"\n          "}
            <div className={"px-3 mb-2 text-[11px] font-mono font-bold uppercase tracking-wider text-slate-500"}>
              {"\n            Диагностика и Автоматика\n          "}
            </div>
            {"\n          "}
            <nav className={"space-y-1"}>
              {"\n            \n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["check"])} id={"navBtn-check"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-emerald-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Проверка сайта"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-400"}>
                  {"Диагноз"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["autotune"])} id={"navBtn-autotune"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-amber-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M13 10V3L4 14h7v7l9-11h-7z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Авто-подбор стратегии"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300"}>
                  {"Q:301"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["monitor"])} id={"navBtn-monitor"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-cyan-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Мониторинг и Telegram"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span id={"sideMonitorBadge"} className={"w-2 h-2 rounded-full bg-slate-400"}></span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["community"])} id={"navBtn-community"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-purple-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Сообщество и ASN"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300"}>
                  {"RIPE"}
                </span>
                {"\n            "}
              </button>
              {"\n\n          "}
            </nav>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div>
            {"\n          "}
            <div className={"px-3 mb-2 text-[11px] font-mono font-bold uppercase tracking-wider text-slate-500"}>
              {"\n            Система и Архивы\n          "}
            </div>
            {"\n          "}
            <nav className={"space-y-1"}>
              {"\n            \n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["snapshots"])} id={"navBtn-snapshots"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-indigo-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Снимки и Копии (NAS)"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-800 text-slate-300"}>
                  {"4"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["logs"])} id={"navBtn-logs"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-teal-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Журнал и Счётчики"}
                  </span>
                  {"\n              "}
                </div>
                {"\n            "}
              </button>
              {"\n"}
            </nav>
            {"\n        "}
          </div>
          {"\n\n      "}
        </div>
        {"\n\n      "}
        {"\n      "}
        <div className={"p-3.5 border-t border-slate-800/80 bg-[#070b14] space-y-2.5"}>
          {"\n        "}
          <div className={"flex items-center justify-between text-xs"}>
            {"\n          "}
            <span className={"text-slate-400"}>
              {"Провайдер (ASN):"}
            </span>
            {"\n          "}
            <span className={"font-mono font-semibold text-slate-200"}>
              {"Не обнаружен"}
            </span>
            {"\n        "}
          </div>
          {"\n        "}
          <div className={"flex items-center justify-between text-xs"}>
            {"\n          "}
            <span className={"text-slate-400"}>
              {"Конфликты служб:"}
            </span>
            {"\n          "}
            <span className={"font-mono font-semibold text-slate-400"}>
              {"Не проверено"}
            </span>
            {"\n        "}
          </div>
          {"\n        "}
          <div className={"flex items-center justify-between text-[11px] font-mono text-slate-500 pt-1 border-t border-slate-800/70"}>
            {"\n          "}
            <span>
              {"Устройство не обнаружено"}
            </span>
            {"\n          "}
            <span>
              {"uhttpd :90"}
            </span>
            {"\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n    "}
      </aside>
      {"\n\n    "}
      {"\n    "}
      {"\n    "}
      {"\n    "}
      <main className={"flex-1 overflow-y-auto p-4 lg:p-6 bg-slate-950"}>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-conf"} className={"view-animate"}>
          {"\n        "}
          <div className={"grid grid-cols-1 xl:grid-cols-12 gap-5 items-start"}>
            {"\n          \n          "}
            {"\n          "}
            <div className={"xl:col-span-8 ui-card rounded-2xl overflow-hidden shadow-2xl border border-slate-800 flex flex-col"}>
              {"\n            \n            "}
              {"\n            "}
              <div className={"bg-[#090e1a] px-4 py-3 border-b border-slate-800 flex flex-wrap items-center justify-between gap-2.5"}>
                {"\n              "}
                <div className={"flex items-center gap-2.5"}>
                  {"\n                "}
                  <span className={"w-2.5 h-2.5 rounded-full bg-blue-400 shadow-[0_0_8px_#60a5fa]"}></span>
                  {"\n                "}
                  <span id={"currentConfigPath"} className={"font-mono text-sm sm:text-base font-bold text-white tracking-tight"}>
                    {"Путь будет обнаружен backend"}
                  </span>
                  {"\n                "}
                  <span id={"confLinesBadge"} className={"text-xs font-mono px-2 py-0.5 rounded-md bg-slate-800 text-slate-300 border border-slate-700"}>
                    {"192 строки"}
                  </span>
                  {"\n                "}
                  <span id={"linterStatusBadge"} className={"text-xs font-mono px-2.5 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/25"}>
                    {"\n                  Проверка не выполнялась\n                "}
                  </span>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"flex flex-wrap items-center gap-2"}>
                  {"\n                "}
                  <button data-action-click={"toggleDiffModal"} className={"px-3 py-1.5 rounded-lg bg-slate-800/90 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-medium transition-all"}>
                    {"\n                  Сравнить изменения\n                "}
                  </button>
                  {"\n                "}
                  <button data-action-click={"copyConfigText"} className={"px-3 py-1.5 rounded-lg bg-slate-800/90 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-medium transition-all"}>
                    {"\n                  Копировать\n                "}
                  </button>
                  {"\n                "}
                  <button data-action-click={"applyConfigAndRestart"} className={"px-4 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-semibold text-xs transition-all shadow-md shadow-blue-600/25 active:scale-95"}>
                    {"\n                  Применить конфигурацию\n                "}
                  </button>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div className={"bg-[#070b14] px-4 py-2 border-b border-slate-800/80 flex flex-wrap items-center justify-between gap-2 text-xs font-mono"}>
                {"\n              "}
                <div className={"flex flex-wrap items-center gap-1.5"}>
                  {"\n                "}
                  <span className={"text-slate-500 mr-1"}>
                    {"Секции:"}
                  </span>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["ISP_INTERFACE"])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-emerald-300 border border-slate-800"}>
                    {"ISP / BASE"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["NFQWS_ARGS="])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-blue-300 border border-slate-800"}>
                    {"TCP (1–6)"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["NFQWS_ARGS_QUIC"])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-amber-300 border border-slate-800"}>
                    {"QUIC"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["NFQWS_ARGS_UDP"])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-indigo-300 border border-slate-800"}>
                    {"UDP DISCORD"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["MODE_LIST="])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-cyan-300 border border-slate-800"}>
                    {"MODE / IPSET"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["NFQWS_ARGS_CUSTOM"])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-purple-300 border border-slate-800"}>
                    {"iPhone (1–4)"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["# NETWORK"])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-slate-300 border border-slate-800"}>
                    {"NETWORK"}
                  </button>
                  {"\n                "}
                  <button data-action-click={"scrollToConfMarker"} data-args={JSON.stringify(["# DOCUMENTAION"])} className={"px-2 py-0.5 rounded bg-slate-900 hover:bg-slate-800 text-slate-400 border border-slate-800"}>
                    {"DOCS"}
                  </button>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <label className={"flex items-center gap-1.5 text-xs text-slate-400 cursor-pointer select-none"}>
                  {"\n                "}
                  <input type={"checkbox"} data-action-change={"toggleWordWrapConf"} data-args={JSON.stringify([{"source": "checked"}])} defaultChecked={true} className={"rounded bg-slate-900 border-slate-700 text-blue-500 focus:ring-0"} />
                  {"\n                "}
                  <span>
                    {"Перенос строк"}
                  </span>
                  {"\n              "}
                </label>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div className={"flex h-[740px] bg-[#060a12] font-mono text-[14px] leading-[1.68] overflow-hidden"}>
                {"\n              "}
                <div id={"confLineNumbers"} className={"line-numbers-col bg-[#05080f] border-r border-slate-800/80 py-4 px-3 w-13 overflow-hidden select-none text-xs leading-[1.68]"}>
                  {"\n                1\n              "}
                </div>
                {"\n              "}
                <textarea id={"nfqwsConfTextarea"} data-action-input={"handleConfInput"} data-action-scroll={"syncConfScroll"} data-args={JSON.stringify([{"source": "self"}])} spellCheck={"false"} className={"flex-1 bg-transparent text-slate-100 py-4 px-5 outline-none resize-none leading-[1.68] font-mono overflow-y-auto selection:bg-blue-500/30 focus:ring-0 border-none"}></textarea>
                {"\n            "}
              </div>
              {"\n\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"xl:col-span-4 space-y-4"}>
              {"\n            \n            "}
              {"\n            "}
              <div className={"ui-card rounded-2xl p-4 shadow-lg"}>
                {"\n              "}
                <div className={"flex items-center justify-between pb-3 mb-3 border-b border-slate-800"}>
                  {"\n                "}
                  <div className={"flex items-center gap-2"}>
                    {"\n                  "}
                    <span className={"w-2 h-4 rounded-full bg-blue-500"}></span>
                    {"\n                  "}
                    <h3 className={"text-sm font-bold text-white"}>
                      {"Быстрые параметры основы"}
                    </h3>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <button data-action-click={"resetDefaultConf"} className={"text-xs text-slate-400 hover:text-blue-400 transition-colors"}>
                    {"\n                  Сбросить к эталону\n                "}
                  </button>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"grid grid-cols-2 gap-3"}>
                  {"\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"ISP_INTERFACE"}
                      </span>
                      {"\n                    "}
                      <span className={"text-emerald-400"}>
                        {"WAN"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={"eth3"} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["ISP_INTERFACE", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"eth3"}>
                        {"eth3 (Провайдер)"}
                      </option>
                      {"\n                    "}
                      <option value={"ppp0"}>
                        {"ppp0 (PPPoE)"}
                      </option>
                      {"\n                    "}
                      <option value={"eth2.2"}>
                        {"eth2.2 (VLAN)"}
                      </option>
                      {"\n                    "}
                      <option value={"nwg0"}>
                        {"nwg0 (WireGuard)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"NFQWS_EXTRA_ARGS"}
                      </span>
                      {"\n                    "}
                      <span className={"text-blue-400"}>
                        {"РЕЖИМ"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={"$MODE_LIST"} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["NFQWS_EXTRA_ARGS", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-blue-300 focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"$MODE_LIST"}>
                        {"$MODE_LIST (user.list)"}
                      </option>
                      {"\n                    "}
                      <option value={"$MODE_AUTO"}>
                        {"$MODE_AUTO (auto.list)"}
                      </option>
                      {"\n                    "}
                      <option value={"$MODE_ALL"}>
                        {"$MODE_ALL (Весь трафик)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"NFQUEUE_NUM"}
                      </span>
                      {"\n                    "}
                      <span className={"text-amber-400"}>
                        {"QUEUE"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={"300"} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["NFQUEUE_NUM", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"300"}>
                        {"300 (Штатная)"}
                      </option>
                      {"\n                    "}
                      <option value={"200"}>
                        {"200"}
                      </option>
                      {"\n                    "}
                      <option value={"100"}>
                        {"100"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"POLICY_NAME"}
                      </span>
                      {"\n                    "}
                      <span className={"text-indigo-400"}>
                        {"NDM"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={"nfqws"} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["POLICY_NAME", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"nfqws"}>
                        {"nfqws"}
                      </option>
                      {"\n                    "}
                      <option value={"zapret"}>
                        {"zapret"}
                      </option>
                      {"\n                    "}
                      <option value={"default"}>
                        {"default"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"IPV6_ENABLED"}
                      </span>
                      {"\n                    "}
                      <span>
                        {"IPv6"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={"0"} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["IPV6_ENABLED", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"0"}>
                        {"0 (Выключен)"}
                      </option>
                      {"\n                    "}
                      <option value={"1"}>
                        {"1 (Включен)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"LOG_LEVEL"}
                      </span>
                      {"\n                    "}
                      <span>
                        {"ЛОГ"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={"0"} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["LOG_LEVEL", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"0"}>
                        {"0 (Выкл)"}
                      </option>
                      {"\n                    "}
                      <option value={"1"}>
                        {"1 (Отладка)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div className={"ui-card rounded-2xl p-4 shadow-lg space-y-3"}>
                {"\n              "}
                <div className={"flex items-center justify-between pb-2.5 border-b border-slate-800"}>
                  {"\n                "}
                  <div className={"flex items-center gap-2"}>
                    {"\n                  "}
                    <span className={"w-2 h-4 rounded-full bg-emerald-400"}></span>
                    {"\n                  "}
                    <h3 className={"text-sm font-bold text-white"}>
                      {"Активные модули и профили"}
                    </h3>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <span className={"text-[11px] font-mono text-emerald-400"}>
                    {"5 файлов OK"}
                  </span>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"space-y-2 text-xs font-mono"}>
                  {"\n                "}
                  <div className={"p-2.5 rounded-xl bg-[#070b14] border border-slate-800/90 flex items-center justify-between"}>
                    {"\n                  "}
                    <span className={"text-slate-300"}>
                      {"TCP Circular (Стратегии 1–6)"}
                    </span>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-bold"}>
                      {"Активен"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <div className={"p-2.5 rounded-xl bg-[#070b14] border border-slate-800/90 flex items-center justify-between"}>
                    {"\n                  "}
                    <span className={"text-slate-300"}>
                      {"UDP Discord / WireGuard (1–3)"}
                    </span>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-blue-500/15 text-blue-400 font-bold"}>
                      {"Активен"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <div className={"p-2.5 rounded-xl bg-[#070b14] border border-slate-800/90 flex items-center justify-between"}>
                    {"\n                  "}
                    <span className={"text-slate-300"}>
                      {"Custom Профиль For iPhone (1–4)"}
                    </span>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-purple-500/15 text-purple-300 font-bold"}>
                      {"Активен"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n              "}
                </div>
                {"\n\n              "}
                {"\n              "}
                <div className={"pt-1"}>
                  {"\n                "}
                  <div className={"text-[11px] text-slate-400 mb-2"}>
                    {"Файлы в "}
                    <code className={"text-slate-300"}>
                      {"/opt/etc/nfqws2/"}
                    </code>
                    {" (клик — вставить путь):"}
                  </div>
                  {"\n                "}
                  <div className={"flex flex-wrap gap-1.5 font-mono text-[11px]"}>
                    {"\n                  "}
                    <button data-action-click={"copySnippetToClipboard"} data-args={JSON.stringify(["--lua-init=@/opt/etc/nfqws2/lua/zapret-lib.lua"])} className={"px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-emerald-300 border border-slate-800"}>
                      {"zapret-lib.lua"}
                    </button>
                    {"\n                  "}
                    <button data-action-click={"copySnippetToClipboard"} data-args={JSON.stringify(["--lua-init=@/opt/etc/nfqws2/lua/zapret-antidpi.lua"])} className={"px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-emerald-300 border border-slate-800"}>
                      {"zapret-antidpi.lua"}
                    </button>
                    {"\n                  "}
                    <button data-action-click={"copySnippetToClipboard"} data-args={JSON.stringify(["--lua-init=@/opt/etc/nfqws2/lua/zapret-auto.lua"])} className={"px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-emerald-300 border border-slate-800"}>
                      {"zapret-auto.lua"}
                    </button>
                    {"\n                  "}
                    <button data-action-click={"copySnippetToClipboard"} data-args={JSON.stringify(["--blob=tls_clienthello:@/opt/etc/nfqws2/blobs/tls_clienthello.bin"])} className={"px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-sky-300 border border-slate-800"}>
                      {"tls_clienthello.bin"}
                    </button>
                    {"\n                  "}
                    <button data-action-click={"copySnippetToClipboard"} data-args={JSON.stringify(["--blob=quic_initial:@/opt/etc/nfqws2/blobs/quic_initial.bin"])} className={"px-2 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-sky-300 border border-slate-800"}>
                      {"quic_initial.bin"}
                    </button>
                    {"\n                "}
                  </div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div className={"ui-card rounded-2xl p-4 shadow-lg flex flex-col h-[315px]"}>
                {"\n              "}
                <div className={"flex items-center justify-between pb-2.5 mb-2.5 border-b border-slate-800"}>
                  {"\n                "}
                  <div className={"flex items-center gap-2"}>
                    {"\n                  "}
                    <span className={"w-2 h-4 rounded-full bg-amber-400"}></span>
                    {"\n                  "}
                    <h3 className={"text-sm font-bold text-white"}>
                      {"Справочник флагов nfqws2"}
                    </h3>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <span className={"text-[11px] text-slate-400"}>
                    {"Клик = копировать"}
                  </span>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <input type={"text"} data-action-input={"filterDocs"} data-args={JSON.stringify([{"source": "value"}])} placeholder={"Поиск флага (circular, seqovl, hostfakesplit)..."} className={"w-full bg-[#070b14] border border-slate-800 rounded-xl px-3 py-1.5 text-xs font-mono text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 mb-2.5"} />
                {"\n\n              "}
                <div id={"docItemsContainer"} className={"flex-1 overflow-y-auto space-y-2 pr-1"}>
                  {"\n                "}
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n\n          "}
            </div>
            {"\n\n        "}
          </div>
          {"\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-lists"} className={"hidden view-animate space-y-5"}>
          {"\n        \n        "}
          <div className={"ui-card rounded-2xl overflow-hidden shadow-xl"}>
            {"\n          "}
            {"\n          "}
            <div className={"flex items-center overflow-x-auto bg-[#080d18] border-b border-slate-800 px-2"}>
              {"\n            \n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["user.list"])} id={"listTab-user.list"} className={"list-file-tab active flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-bold text-white border-b-2 border-sky-400 bg-slate-800/40 transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-200"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span>
                  {"user.list"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["exclude.list"])} id={"listTab-exclude.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span>
                  {"exclude.list"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["auto.list"])} id={"listTab-auto.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span>
                  {"auto.list"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["ipset.list"])} id={"listTab-ipset.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span>
                  {"ipset.list"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["ipset_exclude.list"])} id={"listTab-ipset_exclude.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span>
                  {"ipset_exclude.list"}
                </span>
                {"\n            "}
              </button>
              {"\n\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"p-4 bg-[#0b111e] border-b border-slate-800 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4"}>
              {"\n            "}
              <div className={"flex flex-wrap items-center gap-3"}>
                {"\n              "}
                <span id={"currentListFilePath"} className={"font-mono text-xs px-3 py-1 rounded-lg bg-slate-950 text-sky-400 border border-slate-800"}>
                  {"Путь будет обнаружен backend"}
                </span>
                {"\n              "}
                <span id={"currentListDescription"} className={"text-xs text-slate-400"}>
                  {"Основной пользовательский список доменов для обхода DPI"}
                </span>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"flex flex-wrap items-center gap-2"}>
                {"\n              "}
                <button data-action-click={"injectPresetDomains"} data-args={JSON.stringify(["youtube"])} className={"px-3 py-1.5 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 border border-rose-500/25 text-xs font-medium transition-all"}>
                  {"+ YouTube CDN"}
                </button>
                {"\n              "}
                <button data-action-click={"injectPresetDomains"} data-args={JSON.stringify(["discord"])} className={"px-3 py-1.5 rounded-lg bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 border border-indigo-500/25 text-xs font-medium transition-all"}>
                  {"+ Discord"}
                </button>
                {"\n              "}
                <button data-action-click={"injectPresetDomains"} data-args={JSON.stringify(["meta"])} className={"px-3 py-1.5 rounded-lg bg-sky-500/10 hover:bg-sky-500/20 text-sky-300 border border-sky-500/25 text-xs font-medium transition-all"}>
                  {"+ Instagram / X"}
                </button>
                {"\n              "}
                <button data-action-click={"deduplicateCurrentList"} className={"px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-medium transition-all"}>
                  {"Удалить дубли и А-Я"}
                </button>
                {"\n              "}
                <button data-action-click={"saveActiveListFile"} className={"px-4 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-semibold text-xs transition-all"}>
                  {"Применить список"}
                </button>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"flex h-[610px] bg-[#060a12] font-mono text-[14.5px] leading-7 overflow-hidden"}>
              {"\n            "}
              <div id={"listLineNumbers"} className={"line-numbers-col bg-[#05080f] border-r border-slate-800/80 py-4 px-3.5 w-14 overflow-hidden select-none text-xs leading-7"}>
                {"\n              1\n            "}
              </div>
              {"\n            "}
              <textarea id={"listEditorTextarea"} data-action-input={"syncListLineNumbers"} data-action-scroll={"syncListScroll"} data-args={JSON.stringify([{"source": "self"}])} spellCheck={"false"} className={"flex-1 bg-transparent text-slate-100 py-4 px-6 outline-none resize-none leading-7 font-mono overflow-y-auto selection:bg-blue-500/30 border-none"}></textarea>
              {"\n          "}
            </div>
            {"\n\n          "}
            <div className={"px-5 py-2.5 bg-[#080d18] border-t border-slate-800 flex items-center justify-between text-xs font-mono text-slate-400"}>
              {"\n            "}
              <span id={"listFooterCount"}>
                {"Всего записей: 20"}
              </span>
              {"\n            "}
              <span>
                {"Проверка валидности доменов и CIDR-масок активна • Служба не перезапускается при сохранении"}
              </span>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-check"} className={"hidden view-animate space-y-5"}>
          {"\n        \n        "}
          <div className={"ui-card rounded-2xl p-6 shadow-xl"}>
            {"\n          "}
            <div className={"flex flex-col lg:flex-row lg:items-center justify-between gap-4 pb-5 border-b border-slate-800"}>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex items-center gap-2.5"}>
                  {"\n                "}
                  <span className={"px-2.5 py-0.5 rounded-md bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-xs font-mono font-bold"}>
                    {"Диагностика сайта"}
                  </span>
                  {"\n                "}
                  <h2 className={"text-lg font-bold text-white"}>
                    {"«Сайт не открывается?» — полная проверка маршрута и профилей"}
                  </h2>
                  {"\n              "}
                </div>
                {"\n              "}
                <p className={"text-xs sm:text-sm text-slate-400 mt-1"}>
                  {"\n                Определяет, какой профиль обрабатывает домен по HTTPS, QUIC и HTTP, не уходит ли он в Podkop/VPN, и не ломает ли стратегия незаблокированный сайт\n              "}
                </p>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"mt-5 flex flex-col sm:flex-row gap-3"}>
              {"\n            "}
              <input id={"diagDomainInput"} type={"text"} defaultValue={"youtube.com"} placeholder={"Введите домен (например: youtube.com, gosuslugi.ru, discord.com)..."} className={"flex-1 bg-[#060a12] border border-slate-700 rounded-xl px-4 py-2.5 text-sm font-mono text-white focus:outline-none focus:border-blue-500"} />
              {"\n            "}
              <button data-action-click={"runSiteDiagnosis"} className={"px-6 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-bold text-sm transition-all shadow-lg shadow-blue-600/20"}>
                {"\n              Проверить домен\n            "}
              </button>
              {"\n          "}
            </div>
            {"\n\n          "}
            <div className={"flex flex-wrap items-center gap-2 mt-3 text-xs font-mono"}>
              {"\n            "}
              <span className={"text-slate-500"}>
                {"Быстрые сценарии:"}
              </span>
              {"\n            "}
              <button data-action-click={"setDiagPreset"} data-args={JSON.stringify(["youtube.com"])} className={"px-2.5 py-1 rounded-lg bg-slate-900 hover:bg-slate-800 text-slate-300 border border-slate-800"}>
                {"youtube.com (Через обход)"}
              </button>
              {"\n            "}
              <button data-action-click={"setDiagPreset"} data-args={JSON.stringify(["bank-client.ru"])} className={"px-2.5 py-1 rounded-lg bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 border border-amber-500/30"}>
                {"bank-client.ru (Ломается профилем #1)"}
              </button>
              {"\n            "}
              <button data-action-click={"setDiagPreset"} data-args={JSON.stringify(["openai.com"])} className={"px-2.5 py-1 rounded-lg bg-purple-500/10 hover:bg-purple-500/20 text-purple-300 border border-purple-500/30"}>
                {"openai.com (Уходит в Podkop/VPN)"}
              </button>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div id={"diagResultContainer"} className={"grid grid-cols-1 lg:grid-cols-3 gap-5"}>
            {"\n          "}
            {"\n        "}
          </div>
          {"\n\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-autotune"} className={"hidden view-animate space-y-5"}>
          {"\n        \n        "}
          <div className={"ui-card rounded-2xl p-6 shadow-xl"}>
            {"\n          "}
            <div className={"flex flex-col lg:flex-row lg:items-center justify-between gap-4 pb-5 border-b border-slate-800"}>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex items-center gap-2.5"}>
                  {"\n                "}
                  <span className={"px-2.5 py-0.5 rounded-md bg-amber-500/15 text-amber-300 border border-amber-500/30 text-xs font-mono font-bold"}>
                    {"NFQUEUE #301 • Порты 40000–40999"}
                  </span>
                  {"\n                "}
                  <h2 className={"text-lg font-bold text-white"}>
                    {"Авто-подбор стратегии без остановки боевого nfqws2"}
                  </h2>
                  {"\n              "}
                </div>
                {"\n              "}
                <p className={"text-xs sm:text-sm text-slate-400 mt-1"}>
                  {"\n                Запускает тестовую копию `nfqws2` на отдельной очереди `301` (`iptables nfqws_test_pre/post`), проверяет TCP (HTTPS/HTTP) и QUIC (UDP 443)\n              "}
                </p>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"flex flex-wrap items-center gap-3"}>
                {"\n              "}
                <label className={"flex items-center gap-2 text-xs font-medium text-slate-300 bg-[#060a12] px-3.5 py-2 rounded-xl border border-slate-800 cursor-pointer"}>
                  {"\n                "}
                  <input id={"autoApplyPickToggle"} type={"checkbox"} defaultChecked={true} className={"rounded bg-slate-900 border-slate-700 text-blue-500 focus:ring-0"} />
                  {"\n                "}
                  <span>
                    {"Применять самому (с авто-откатом)"}
                  </span>
                  {"\n              "}
                </label>
                {"\n              "}
                <button id={"startScanBtn"} data-action-click={"startStrategyAutoScan"} className={"px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-bold text-sm flex items-center gap-2 shadow-lg shadow-blue-600/20 transition-all active:scale-95"}>
                  {"\n                "}
                  <svg className={"w-4 h-4"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2.2"} d={"M13 10V3L4 14h7v7l9-11h-7z"}></path>
                  </svg>
                  {"\n                "}
                  <span id={"scanBtnText"}>
                    {"Запустить авто-подбор"}
                  </span>
                  {"\n              "}
                </button>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"grid grid-cols-1 md:grid-cols-4 gap-4 mt-5"}>
              {"\n            "}
              <div className={"bg-[#060a12] border border-slate-800/90 rounded-xl p-3.5"}>
                {"\n              "}
                <label className={"block text-xs font-mono text-slate-400"}>
                  {"Целевой ресурс"}
                </label>
                {"\n              "}
                <select defaultValue={"rr1---sn-gvnuxaxjvh.googlevideo.com"} data-action-change={"syncCustomHostInput"} data-args={JSON.stringify([{"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs sm:text-sm font-mono text-white focus:outline-none focus:border-blue-500"}>
                  {"\n                "}
                  <option value={"rr1---sn-gvnuxaxjvh.googlevideo.com"}>
                    {"YouTube Video CDN (googlevideo.com)"}
                  </option>
                  {"\n                "}
                  <option value={"www.youtube.com"}>
                    {"YouTube Web (www.youtube.com)"}
                  </option>
                  {"\n                "}
                  <option value={"gateway.discord.gg"}>
                    {"Discord Gateway (gateway.discord.gg)"}
                  </option>
                  {"\n                "}
                  <option value={"instagram.com"}>
                    {"Instagram (instagram.com)"}
                  </option>
                  {"\n                "}
                  <option value={"rutracker.org"}>
                    {"RuTracker (rutracker.org)"}
                  </option>
                  {"\n              "}
                </select>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"bg-[#060a12] border border-slate-800/90 rounded-xl p-3.5"}>
                {"\n              "}
                <label className={"block text-xs font-mono text-slate-400"}>
                  {"Проверяемый SNI хост"}
                </label>
                {"\n              "}
                <input id={"scanCustomHost"} type={"text"} defaultValue={"rr1---sn-gvnuxaxjvh.googlevideo.com"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs sm:text-sm font-mono text-blue-300 focus:outline-none focus:border-blue-500"} />
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"bg-[#060a12] border border-slate-800/90 rounded-xl p-3.5"}>
                {"\n              "}
                <label className={"block text-xs font-mono text-slate-400"}>
                  {"Режим тестирования"}
                </label>
                {"\n              "}
                <select defaultValue={"tcp"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs sm:text-sm font-mono text-white focus:outline-none focus:border-blue-500"}>
                  {"\n                "}
                  <option value={"tcp"}>
                    {"Быстрый (6 стратегий TCP) — ~35 сек"}
                  </option>
                  {"\n                "}
                  <option value={"full"}>
                    {"Полный (22 стратегии TCP + QUIC) — ~2 мин"}
                  </option>
                  {"\n                "}
                  <option value={"iphone"}>
                    {"Профиль iPhone (HostFakeSplit)"}
                  </option>
                  {"\n              "}
                </select>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"bg-[#060a12] border border-slate-800/90 rounded-xl p-3.5 flex flex-col justify-between"}>
                {"\n              "}
                <div className={"flex items-center justify-between text-xs font-mono"}>
                  {"\n                "}
                  <span className={"text-slate-400"}>
                    {"Статус очереди 301:"}
                  </span>
                  {"\n                "}
                  <span id={"scanProgressLabel"} className={"text-blue-400 font-bold"}>
                    {"Готов к тесту"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"w-full bg-slate-800 h-2.5 rounded-full overflow-hidden mt-2"}>
                  {"\n                "}
                  <div id={"scanProgressBar"} className={"bg-blue-500 h-full rounded-full transition-all duration-300"} style={{ width: "0%" }}></div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div className={"grid grid-cols-1 xl:grid-cols-12 gap-5"}>
            {"\n          "}
            <div className={"xl:col-span-8 space-y-3"} id={"autotuneStrategiesList"}></div>
            {"\n\n          "}
            <div className={"xl:col-span-4 ui-card rounded-2xl p-5 flex flex-col h-[540px]"}>
              {"\n            "}
              <div className={"pb-3 border-b border-slate-800 flex items-center justify-between"}>
                {"\n              "}
                <span className={"text-sm font-bold text-white font-mono"}>
                  {"Журнал NFQUEUE #301"}
                </span>
                {"\n              "}
                <span className={"text-xs font-mono text-amber-400"}>
                  {"picks.json"}
                </span>
                {"\n            "}
              </div>
              {"\n            "}
              <div id={"autotuneConsoleBox"} className={"flex-1 overflow-y-auto font-mono text-xs space-y-2 mt-3 pr-1 text-slate-300 leading-relaxed"}>
                {"\n              "}
                <div className={"text-slate-500"}>
                  {"[Система] Тестовая очередь 301 изолирована. Основной трафик роутера продолжает идти через NFQUEUE 300 без обрывов."}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-monitor"} className={"hidden view-animate space-y-5"}>
          {"\n        "}
          <div className={"grid grid-cols-1 lg:grid-cols-12 gap-5"}>
            {"\n          \n          "}
            {"\n          "}
            <div className={"lg:col-span-7 ui-card rounded-2xl p-6 space-y-5"}>
              {"\n            "}
              <div className={"flex items-center justify-between pb-4 border-b border-slate-800"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <h2 className={"text-base sm:text-lg font-bold text-white"}>
                    {"Автоматический мониторинг доступности сайтов"}
                  </h2>
                  {"\n                "}
                  <p className={"text-xs text-slate-400 mt-0.5"}>
                    {"Если сайты перестают открываться N проверок подряд — автоматически запускается подбор новой стратегии"}
                  </p>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-xs font-mono font-bold"}>
                  {"Активен (каждые 15 мин)"}
                </span>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"grid grid-cols-1 sm:grid-cols-3 gap-3.5"}>
                {"\n              "}
                <div className={"bg-[#060a12] border border-slate-800 rounded-xl p-3.5"}>
                  {"\n                "}
                  <label className={"text-xs text-slate-400 font-mono"}>
                    {"Интервал проверки"}
                  </label>
                  {"\n                "}
                  <select defaultValue={"15"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs font-mono text-white"}>
                    {"\n                  "}
                    <option value={"5"}>
                      {"Каждые 5 минут"}
                    </option>
                    {"\n                  "}
                    <option value={"15"}>
                      {"Каждые 15 минут"}
                    </option>
                    {"\n                  "}
                    <option value={"30"}>
                      {"Каждые 30 минут"}
                    </option>
                    {"\n                "}
                  </select>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"bg-[#060a12] border border-slate-800 rounded-xl p-3.5"}>
                  {"\n                "}
                  <label className={"text-xs text-slate-400 font-mono"}>
                    {"Порог сбоев подряд"}
                  </label>
                  {"\n                "}
                  <select defaultValue={"3"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs font-mono text-white"}>
                    {"\n                  "}
                    <option value={"2"}>
                      {"2 сбоя подряд"}
                    </option>
                    {"\n                  "}
                    <option value={"3"}>
                      {"3 сбоя подряд"}
                    </option>
                    {"\n                  "}
                    <option value={"4"}>
                      {"4 сбоя подряд"}
                    </option>
                    {"\n                "}
                  </select>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"bg-[#060a12] border border-slate-800 rounded-xl p-3.5"}>
                  {"\n                "}
                  <label className={"text-xs text-slate-400 font-mono"}>
                    {"Реакция при падении"}
                  </label>
                  {"\n                "}
                  <select defaultValue={"autopick"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-2.5 py-1.5 text-xs font-mono text-emerald-300"}>
                    {"\n                  "}
                    <option value={"autopick"}>
                      {"Авто-подбор + Уведомление"}
                    </option>
                    {"\n                  "}
                    <option value={"restart"}>
                      {"Перезапуск nfqws2"}
                    </option>
                    {"\n                  "}
                    <option value={"notify"}>
                      {"Только Telegram"}
                    </option>
                    {"\n                "}
                  </select>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div className={"space-y-2.5"}>
                {"\n              "}
                <div className={"text-xs font-mono text-slate-400"}>
                  {"Контрольные сайты (`monitor.json`):"}
                </div>
                {"\n              "}
                <div className={"grid grid-cols-1 sm:grid-cols-2 gap-2.5 font-mono text-xs"}>
                  {"\n                "}
                  <div className={"p-3 rounded-xl bg-[#060a12] border border-slate-800 flex items-center justify-between"}>
                    {"\n                  "}
                    <div>
                      {"\n                    "}
                      <div className={"font-bold text-white"}>
                        {"youtube.com"}
                      </div>
                      {"\n                    "}
                      <div className={"text-[11px] text-slate-400"}>
                        {"TLS 1.3 + QUIC • Стратегия #2"}
                      </div>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-bold"}>
                      {"21 ms • OK"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <div className={"p-3 rounded-xl bg-[#060a12] border border-slate-800 flex items-center justify-between"}>
                    {"\n                  "}
                    <div>
                      {"\n                    "}
                      <div className={"font-bold text-white"}>
                        {"rr1---sn-gvnuxaxjvh.googlevideo.com"}
                      </div>
                      {"\n                    "}
                      <div className={"text-[11px] text-slate-400"}>
                        {"Video CDN • Стратегия #1"}
                      </div>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-bold"}>
                      {"18 ms • OK"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <div className={"p-3 rounded-xl bg-[#060a12] border border-slate-800 flex items-center justify-between"}>
                    {"\n                  "}
                    <div>
                      {"\n                    "}
                      <div className={"font-bold text-white"}>
                        {"gateway.discord.gg"}
                      </div>
                      {"\n                    "}
                      <div className={"text-[11px] text-slate-400"}>
                        {"WSS + UDP STUN • Стратегия #4"}
                      </div>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-bold"}>
                      {"42 ms • OK"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <div className={"p-3 rounded-xl bg-[#060a12] border border-slate-800 flex items-center justify-between"}>
                    {"\n                  "}
                    <div>
                      {"\n                    "}
                      <div className={"font-bold text-white"}>
                        {"instagram.com"}
                      </div>
                      {"\n                    "}
                      <div className={"text-[11px] text-slate-400"}>
                        {"TLS 1.3 • Стратегия #2"}
                      </div>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <span className={"px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-bold"}>
                      {"37 ms • OK"}
                    </span>
                    {"\n                "}
                  </div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"lg:col-span-5 ui-card rounded-2xl p-6 space-y-4"}>
              {"\n            "}
              <div className={"flex items-center justify-between pb-3 border-b border-slate-800"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <h3 className={"text-base font-bold text-white"}>
                    {"Уведомления в Telegram"}
                  </h3>
                  {"\n                "}
                  <p className={"text-xs text-slate-400"}>
                    {"Поддержка прокси и VPN-интерфейсов при блокировке Telegram"}
                  </p>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-xs font-mono text-sky-400"}>
                  {"notify.json"}
                </span>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"space-y-3 text-xs"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <label className={"block text-slate-400 font-mono mb-1"}>
                    {"Bot Token (@BotFather)"}
                  </label>
                  {"\n                "}
                  <input type={"password"} defaultValue={""} className={"w-full bg-[#060a12] border border-slate-800 rounded-xl px-3.5 py-2 font-mono text-white"} />
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"grid grid-cols-3 gap-2 items-end"}>
                  {"\n                "}
                  <div className={"col-span-2"}>
                    {"\n                  "}
                    <label className={"block text-slate-400 font-mono mb-1"}>
                      {"Chat ID получателя"}
                    </label>
                    {"\n                  "}
                    <input id={"tgChatIdInput"} type={"text"} defaultValue={"482910492"} className={"w-full bg-[#060a12] border border-slate-800 rounded-xl px-3.5 py-2 font-mono text-white"} />
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <button data-action-click={"findTelegramChatId"} className={"py-2 px-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-sky-300 border border-slate-700 font-semibold transition-all"}>
                    {"\n                  Найти Chat ID\n                "}
                  </button>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div>
                  {"\n                "}
                  <label className={"block text-slate-400 font-mono mb-1"}>
                    {"Маршрут до api.telegram.org (Прокси / Туннель)"}
                  </label>
                  {"\n                "}
                  <select defaultValue={"nwg0"} className={"w-full bg-[#060a12] border border-slate-800 rounded-xl px-3 py-2 font-mono text-white"}>
                    {"\n                  "}
                    <option value={"direct"}>
                      {"Напрямую (через обход nfqws2)"}
                    </option>
                    {"\n                  "}
                    <option value={"nwg0"}>
                      {"Через интерфейс WireGuard / AWG (nwg0)"}
                    </option>
                    {"\n                  "}
                    <option value={"socks5"}>
                      {"Через локальный SOCKS5 (127.0.0.1:1080)"}
                    </option>
                    {"\n                "}
                  </select>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"pt-2 flex items-center gap-2.5"}>
                  {"\n                "}
                  <button data-action-click={"sendTestTelegram"} className={"flex-1 py-2.5 rounded-xl bg-sky-600 hover:bg-sky-500 text-white font-bold transition-all"}>
                    {"\n                  Отправить тестовое сообщение\n                "}
                  </button>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n        "}
          </div>
          {"\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-community"} className={"hidden view-animate space-y-5"}>
          {"\n        "}
          <div className={"grid grid-cols-1 lg:grid-cols-12 gap-5"}>
            {"\n          \n          "}
            {"\n          "}
            <div className={"lg:col-span-7 ui-card rounded-2xl p-6 space-y-4"}>
              {"\n            "}
              <div className={"flex flex-wrap items-center justify-between gap-2 pb-4 border-b border-slate-800"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <h2 className={"text-base sm:text-lg font-bold text-white"}>
                    {"Проверенные стратегии сообщества по провайдерам"}
                  </h2>
                  {"\n                "}
                  <p className={"text-xs text-slate-400"}>
                    {"Автоматическое определение провайдера через RIPEstat (`stat.ripe.net`)"}
                  </p>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"px-3 py-1 rounded-lg bg-purple-500/15 text-purple-300 border border-purple-500/30 text-xs font-mono font-bold"}>
                  {"\n                Ваш ISP: Ростелеком (AS12389)\n              "}
                </span>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"space-y-3"}>
                {"\n              "}
                <div className={"p-4 rounded-xl bg-[#060a12] border border-slate-800 hover:border-blue-500/40 transition-all"}>
                  {"\n                "}
                  <div className={"flex items-center justify-between"}>
                    {"\n                  "}
                    <span className={"font-bold text-sm text-white"}>
                      {"Ростелеком / Теле2 — Circular v2 (AutoTTL + SNI Overlap)"}
                    </span>
                    {"\n                  "}
                    <button data-action-click={"applyCommunityPreset"} data-args={JSON.stringify(["Ростелеком Circular v2"])} className={"px-3 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold"}>
                      {"Применить"}
                    </button>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <p className={"text-xs text-slate-400 mt-1"}>
                    {"Рейтинг успешности: 96% • Работает для YouTube 4K, Discord Voice и Telegram"}
                  </p>
                  {"\n                "}
                  <code className={"block mt-2 text-[11px] font-mono text-blue-300 bg-slate-950 p-2 rounded border border-slate-800"}>
                    {"--lua-desync=multisplit:pos=1,midsld:seqovl=sniext+3:seqovl_pattern=tls_clienthello:tcp_ts_up:disorder_after"}
                  </code>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"p-4 rounded-xl bg-[#060a12] border border-slate-800 hover:border-blue-500/40 transition-all"}>
                  {"\n                "}
                  <div className={"flex items-center justify-between"}>
                    {"\n                  "}
                    <span className={"font-bold text-sm text-white"}>
                      {"МТС / МГТС — HostFakeSplit + BadSUM + MD5"}
                    </span>
                    {"\n                  "}
                    <button data-action-click={"applyCommunityPreset"} data-args={JSON.stringify(["МТС / МГТС HostFakeSplit"])} className={"px-3 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold"}>
                      {"Применить"}
                    </button>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <p className={"text-xs text-slate-400 mt-1"}>
                    {"Рейтинг успешности: 94% • Пробивает жесткую фильтрацию SNI на стационарных сетях"}
                  </p>
                  {"\n                "}
                  <code className={"block mt-2 text-[11px] font-mono text-blue-300 bg-slate-950 p-2 rounded border border-slate-800"}>
                    {"--lua-desync=hostfakesplit:midhost=host-2:altorder=1:seqovl=sniext+3:badsum:tcp_md5:disorder_after"}
                  </code>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"p-4 rounded-xl bg-[#060a12] border border-slate-800 hover:border-blue-500/40 transition-all"}>
                  {"\n                "}
                  <div className={"flex items-center justify-between"}>
                    {"\n                  "}
                    <span className={"font-bold text-sm text-white"}>
                      {"Билайн / Дом.ру — Double Fake + VK SNI Mod"}
                    </span>
                    {"\n                  "}
                    <button data-action-click={"applyCommunityPreset"} data-args={JSON.stringify(["Билайн / Дом.ру VK SNI"])} className={"px-3 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold"}>
                      {"Применить"}
                    </button>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <p className={"text-xs text-slate-400 mt-1"}>
                    {"Рейтинг успешности: 92% • Устойчивый обход ТСПУ для региональных узлов"}
                  </p>
                  {"\n                "}
                  <code className={"block mt-2 text-[11px] font-mono text-blue-300 bg-slate-950 p-2 rounded border border-slate-800"}>
                    {"--lua-desync=fake:blob=tls_clienthello:tls_mod=rnd,dupsid,sni=vk.com:ip_autottl=-2,2-10:badsum:repeats=2"}
                  </code>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"lg:col-span-5 ui-card rounded-2xl p-6 space-y-4"}>
              {"\n            "}
              <div className={"pb-3 border-b border-slate-800"}>
                {"\n              "}
                <h3 className={"text-base font-bold text-white"}>
                  {"Генератор подсетей по ASN в `ipset.list`"}
                </h3>
                {"\n              "}
                <p className={"text-xs text-slate-400 mt-0.5"}>
                  {"Загружает все анонсируемые IPv4-префиксы компании из базы RIPEstat"}
                </p>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"space-y-3 text-xs"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <label className={"block text-slate-400 font-mono mb-1"}>
                    {"Выберите сервис или введите номер ASN"}
                  </label>
                  {"\n                "}
                  <select id={"asnSelector"} className={"w-full bg-[#060a12] border border-slate-800 rounded-xl px-3.5 py-2.5 font-mono text-white"}>
                    {"\n                  "}
                    <option value={"AS15169"}>
                      {"AS15169 — Google / YouTube (42 префикса)"}
                    </option>
                    {"\n                  "}
                    <option value={"AS49544"}>
                      {"AS49544 — Discord Voice / Media (18 префиксов)"}
                    </option>
                    {"\n                  "}
                    <option value={"AS13335"}>
                      {"AS13335 — Cloudflare CDN (36 префиксов)"}
                    </option>
                    {"\n                  "}
                    <option value={"AS32934"}>
                      {"AS32934 — Meta / Instagram (24 префикса)"}
                    </option>
                    {"\n                "}
                  </select>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <button data-action-click={"fetchAsnPrefixes"} className={"w-full py-2.5 rounded-xl bg-purple-600 hover:bg-purple-500 text-white font-bold transition-all"}>
                  {"\n                Загрузить IPv4-подсети и добавить в ipset.list\n              "}
                </button>
                {"\n\n              "}
                <div id={"asnResultBox"} className={"p-3.5 rounded-xl bg-[#060a12] border border-slate-800 font-mono text-[11px] text-slate-400 space-y-1"}>
                  {"\n                "}
                  <div>
                    {"Готов к запросу `https://stat.ripe.net/data/announced-prefixes/data.json`"}
                  </div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n        "}
          </div>
          {"\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-snapshots"} className={"hidden view-animate space-y-5"}>
          {"\n        "}
          <div className={"grid grid-cols-1 lg:grid-cols-12 gap-5"}>
            {"\n          \n          "}
            <div className={"lg:col-span-7 ui-card rounded-2xl p-6 space-y-4"}>
              {"\n            "}
              <div className={"flex items-center justify-between pb-3 border-b border-slate-800"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <h2 className={"text-base sm:text-lg font-bold text-white"}>
                    {"Резервные копии конфигурации"}
                  </h2>
                  {"\n                "}
                  <p className={"text-xs text-slate-400"}>
                    {"Снимок текущего nfqws2.conf; автоматические снимки и списки пока не поддерживаются"}
                  </p>
                  {"\n              "}
                </div>
                {"\n              "}
                <button data-action-click={"createNewSnapshot"} className={"px-3.5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-xs font-bold transition-all"}>
                  {"\n                + Создать снимок сейчас\n              "}
                </button>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div id={"snapshotsListBox"} className={"space-y-2.5 font-mono text-xs"}>
                {"\n              "}
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            <div className={"lg:col-span-5 ui-card rounded-2xl p-6 space-y-4"}>
              {"\n            "}
              <div className={"pb-3 border-b border-slate-800"}>
                {"\n              "}
                <h3 className={"text-base font-bold text-white"}>
                  {"Удалённое копирование на NAS по SSH"}
                </h3>
                {"\n              "}
                <p className={"text-xs text-slate-400"}>
                  {"Скрипт `~/.nfqws-ui-store.sh` для выгрузки архива на домашний сервер"}
                </p>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"space-y-3 text-xs font-mono"}>
                {"\n              "}
                <div>
                  {"\n                "}
                  <label className={"block text-slate-400 mb-1"}>
                    {"SSH-хост и папка назначения"}
                  </label>
                  {"\n                "}
                  <input type={"text"} defaultValue={"root@192.168.1.10:/mnt/nas/backups/keenetic-nfqws2"} className={"w-full bg-[#060a12] border border-slate-800 rounded-xl px-3.5 py-2 text-white"} />
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"flex gap-2.5 pt-1"}>
                  {"\n                "}
                  <button data-action-click={"pushBackupToNas"} className={"flex-1 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-sans font-bold transition-all"}>
                    {"\n                  Отправить копию на NAS\n                "}
                  </button>
                  {"\n                "}
                  <button data-action-click={"exportFullBackup"} className={"px-4 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 font-sans font-semibold transition-all"}>
                    {"\n                  Скачать .json\n                "}
                  </button>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n        "}
          </div>
          {"\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-logs"} className={"hidden view-animate space-y-5"}>
          {"\n        \n        "}
          {"\n        "}
          <div className={"grid grid-cols-1 md:grid-cols-3 gap-4"}>
            {"\n          "}
            <div className={"ui-card rounded-2xl p-4 flex items-center justify-between"}>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"text-xs font-mono text-slate-400"}>
                  {"Профиль TCP (80, 443, 2053...)"}
                </div>
                {"\n              "}
                <div className={"text-lg font-bold font-mono text-white mt-1"}>
                  {"148,920 пакетов"}
                </div>
                {"\n            "}
              </div>
              {"\n            "}
              <span className={"px-2.5 py-1 rounded-lg bg-emerald-500/15 text-emerald-400 text-xs font-mono font-bold"}>
                {"64.2 MB"}
              </span>
              {"\n          "}
            </div>
            {"\n          "}
            <div className={"ui-card rounded-2xl p-4 flex items-center justify-between"}>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"text-xs font-mono text-slate-400"}>
                  {"Профиль UDP (Discord / STUN / WG)"}
                </div>
                {"\n              "}
                <div className={"text-lg font-bold font-mono text-white mt-1"}>
                  {"52,410 пакетов"}
                </div>
                {"\n            "}
              </div>
              {"\n            "}
              <span className={"px-2.5 py-1 rounded-lg bg-blue-500/15 text-blue-400 text-xs font-mono font-bold"}>
                {"19.8 MB"}
              </span>
              {"\n          "}
            </div>
            {"\n          "}
            <div className={"ui-card rounded-2xl p-4 flex items-center justify-between"}>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"text-xs font-mono text-slate-400"}>
                  {"Профиль iPhone Custom (443)"}
                </div>
                {"\n              "}
                <div className={"text-lg font-bold font-mono text-white mt-1"}>
                  {"31,105 пакетов"}
                </div>
                {"\n            "}
              </div>
              {"\n            "}
              <span className={"px-2.5 py-1 rounded-lg bg-purple-500/15 text-purple-300 text-xs font-mono font-bold"}>
                {"11.4 MB"}
              </span>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div className={"ui-card rounded-2xl overflow-hidden"}>
            {"\n          "}
            <div className={"p-4 bg-[#090e1a] border-b border-slate-800 flex flex-wrap items-center justify-between gap-3"}>
              {"\n            "}
              <div className={"flex items-center gap-3"}>
                {"\n              "}
                <span className={"w-2.5 h-2.5 rounded-full bg-emerald-400 status-dot-active"}></span>
                {"\n              "}
                <span className={"font-mono text-sm font-bold text-white"}>
                  {"/opt/var/log/nfqws2.log"}
                </span>
                {"\n              "}
                <span className={"text-xs font-mono text-slate-400"}>
                  {"Живой поток событий `zapret-auto.lua`"}
                </span>
                {"\n            "}
              </div>
              {"\n            "}
              <div className={"flex items-center gap-2"}>
                {"\n              "}
                <button data-action-click={"emitTestLogEvent"} className={"px-3 py-1.5 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 text-blue-300 border border-blue-500/30 text-xs font-mono"}>
                  {"\n                + Симулировать пакет\n              "}
                </button>
                {"\n              "}
                <button data-action-click={"clearLogsBox"} className={"px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-mono"}>
                  {"\n                Очистить\n              "}
                </button>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            <div id={"liveLogContainer"} className={"h-[490px] bg-[#060a12] p-5 font-mono text-xs sm:text-[13.5px] overflow-y-auto space-y-2 leading-relaxed"}></div>
            {"\n        "}
          </div>
          {"\n\n      "}
        </section>
        {"\n"}
      </main>
      {"\n\n  "}
    </div>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
    <div id={"updateModalBackdrop"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-50 hidden items-center justify-center p-4"}>
      {"\n    "}
      <div className={"bg-[#0b101b] border border-slate-800 rounded-2xl max-w-2xl w-full overflow-hidden shadow-2xl view-animate"}>
        {"\n      \n      "}
        <div className={"px-6 pt-5 pb-4 flex items-center justify-between"}>
          {"\n        "}
          <div className={"flex items-center gap-3"}>
            {"\n          "}
            <svg className={"w-6 h-6 text-blue-500"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2.2"} d={"M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5m0 0l5-5m-5 5V3"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <h3 id={"updateModalTitle"} className={"text-lg sm:text-xl font-bold text-white tracking-tight"}>
              {"Обновление nfqws2-UI"}
            </h3>
            {"\n        "}
          </div>
          {"\n\n        "}
          <button data-action-click={"closeUpdateModal"} className={"text-slate-400 hover:text-white p-1 rounded-lg transition-colors"}>
            {"\n          "}
            <svg className={"w-5 h-5"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M6 18L18 6M6 6l12 12"}></path>
              {"\n          "}
            </svg>
            {"\n        "}
          </button>
          {"\n      "}
        </div>
        {"\n\n      "}
        <div className={"px-6 pb-3 flex flex-wrap items-center justify-between gap-2"}>
          {"\n        "}
          <div className={"flex items-center gap-2"}>
            {"\n          "}
            <span className={"text-sm text-slate-400 mr-1"}>
              {"Выберите версию"}
            </span>
            {"\n          "}
            <button id={"modalSwitchUiBtn"} data-action-click={"switchUpdateTarget"} data-args={JSON.stringify(["ui"])} className={"px-2.5 py-1 rounded-lg text-xs font-semibold bg-blue-500/20 text-blue-400 border border-blue-500/40 transition-all"}>
              {"\n            nfqws2-UI\n          "}
            </button>
            {"\n          "}
            <button id={"modalSwitchCoreBtn"} data-action-click={"switchUpdateTarget"} data-args={JSON.stringify(["core"])} className={"px-2.5 py-1 rounded-lg text-xs font-semibold bg-slate-900 text-slate-400 border border-slate-800 hover:text-slate-200 transition-all"}>
              {"\n            nfqws2 (Ядро)\n          "}
            </button>
            {"\n        "}
          </div>
          {"\n\n        "}
          <div className={"flex items-center gap-2"}>
            {"\n          "}
            <span id={"updateRepoSlug"} className={"px-3 py-1 rounded-full bg-[#111827] border border-slate-700/80 text-xs font-mono font-bold text-slate-200"}>
              {"\n            zemidala/nfqws2-ui\n          "}
            </span>
            {"\n          "}
            <span className={"px-2.5 py-1 rounded-full bg-[#064e3b] text-[#34d399] text-xs font-bold"}>
              {"\n            GitHub\n          "}
            </span>
            {"\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n      "}
        <div id={"updateVersionsList"} className={"px-6 py-2 max-h-[410px] overflow-y-auto space-y-2.5"}></div>
        {"\n\n      "}
        <div id={"updateInstallProgressBox"} className={"hidden px-6 pt-3"}>
          {"\n        "}
          <div className={"flex items-center justify-between text-xs font-mono text-blue-300 mb-1.5"}>
            {"\n          "}
            <span id={"updateInstallStepText"}>
              {"Скачивание пакета с GitHub Releases..."}
            </span>
            {"\n          "}
            <span id={"updateInstallPercent"}>
              {"45%"}
            </span>
            {"\n        "}
          </div>
          {"\n        "}
          <div className={"w-full bg-slate-900 h-2 rounded-full overflow-hidden"}>
            {"\n          "}
            <div id={"updateInstallBar"} className={"bg-blue-500 h-full transition-all duration-300"} style={{ width: "0%" }}></div>
            {"\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n      "}
        <div className={"p-6 pt-4"}>
          {"\n        "}
          <button id={"confirmUpdateInstallBtn"} data-action-click={"installSelectedRelease"} className={"w-full py-3 rounded-xl bg-[#1d4ed8] hover:bg-[#2563eb] text-white font-bold text-sm transition-all shadow-lg shadow-blue-600/25 active:scale-[0.99]"}>
            {"\n          Установить\n        "}
          </button>
          {"\n      "}
        </div>
        {"\n\n    "}
      </div>
      {"\n  "}
    </div>
{"\n\n  "}
{"\n  "}
    <div id={"diffModalBackdrop"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-50 hidden items-center justify-center p-4"}>
      {"\n    "}
      <div className={"bg-[#0b101b] border border-slate-800 rounded-2xl max-w-3xl w-full overflow-hidden shadow-2xl view-animate"}>
        {"\n      "}
        <div className={"px-6 py-4 border-b border-slate-800 flex items-center justify-between"}>
          {"\n        "}
          <h3 className={"text-base font-bold text-white"}>
            {"Сравнение с сохранённым `nfqws.conf`"}
          </h3>
          {"\n        "}
          <button data-action-click={"toggleDiffModal"} className={"text-slate-400 hover:text-white"}>
            {"✕"}
          </button>
          {"\n      "}
        </div>
        {"\n      "}
        <div id={"diffModalBody"} className={"p-6 font-mono text-xs max-h-[460px] overflow-y-auto space-y-1.5 bg-[#060a12]"}></div>
        {"\n    "}
      </div>
      {"\n  "}
    </div>
{"\n\n  "}
{"\n  "}
    <div id={"toastWrapper"} className={"fixed bottom-5 right-5 z-50 space-y-2.5 pointer-events-none"}></div>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
{"\n"}
    </>
  );
}
