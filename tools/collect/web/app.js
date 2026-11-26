/* SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
 * SPDX-License-Identifier: GPL-3.0-or-later */

"use strict";

const LANGUAGES = ["zh_CN", "zh_TW", "ja"];
const HTML_LANG = { zh_CN: "zh-CN", zh_TW: "zh-TW", ja: "ja" };
const MODE = location.pathname === "/eval" ? "eval" : "train";
const API_BASE = MODE === "eval" ? "/eval-api" : "/api";
document.body.dataset.mode = MODE;
const SIZE = 420;
const BATCH_SIZE = 20;
const STORAGE_LANG = "ink_lang_v1";
const STORAGE_CONSENT = "ink_consent_v1";
const roundKey = (lang) =>
  MODE === "eval" ? `ink_eval_round_v1_${lang}` : `ink_round_v1_${lang}`;

const STRINGS = {
  zh_CN: {
    title: "墨笔 · 手写样本",
    description: "为墨笔（fcitx5-ink）贡献匿名手写笔画，用于训练和评估手写识别模型。",
    brand: "墨笔",
    brand_sub: "手写样本",
    language: "语言",
    hero_title: "写 {batch} 个字，帮墨笔更懂手写。",
    hero_lead: "墨笔（fcitx5-ink）是一个 fcitx5 手写输入工具。这里收集的匿名笔画会进入训练池，用于训练和评估手写识别模型，让它在更多写法上认得更准。不需要账号，不收集姓名或邮箱。",
    start: "开始书写",
    round_note: "每一轮 {batch} 个字，来自训练池；写完就可以停，想多写可以再来一轮。",
    write_title: "书写",
    write_this: "请书写这个字",
    target_aria: "目标字：{char}",
    canvas_aria: "书写区域",
    undo: "撤销",
    clear: "清空",
    skip: "跳过",
    submit: "提交",
    restart: "再来一轮",
    consent_text: "我同意将这份手写样本以 CC BY 4.0 协议发布，并用于训练和评估手写识别模型。我确认这是我本人的书写，且不包含个人信息。",
    data_policy: "数据说明",
    source_code: "源代码",
    stats_line: "训练池：已收集 {samples} 份样本 · {writers} 位书写者 · 覆盖 {chars}/{total} 个字",
    progress: "已写 {written} 份 · 本轮还剩 {left} 个字",
    status_loading: "正在加载…",
    status_ready: "请在白色区域内书写。",
    status_saving: "正在提交…",
    status_preparing: "正在准备安全校验…",
    status_saved: "已保存，谢谢！接着写下一个字。",
    status_duplicate: "这份样本已经记录过了，换下一个字。",
    status_error: "提交失败，请稍后重试。",
    status_rate: "提交太频繁了，请稍后再试。",
    status_offline: "网络连接失败，请检查网络后重试。",
    status_consent: "请先勾选同意。",
    status_finished: "这一轮写完了，谢谢！",
    status_load_error: "加载失败，请刷新页面。",
    status_pow_failed: "安全校验失败，请刷新页面重试。",
    empty_stats: "训练池还没有样本，成为第一个书写者。",
    about_title: "为什么要收集",
    about_p1: "手写识别需要大量真实写法。训练池按常用标准字表覆盖：简体 3,500、繁体 4,808、日文 2,136 个常用字和假名；字序按使用频率排列。",
    about_p2: "训练池和评估集分开：评估集是每语言 300 个留出字，由维护者用不同书写者收集；同一书写者不会同时进入两边。",
    about_p3: "样本以 CC BY 4.0 发布，任何人都可以复用；项目代码以 GPL-3.0-or-later 发布。",
    privacy_title: "隐私与许可",
    privacy_p1: "不收集姓名、邮箱或原始 IP。浏览器里只保存一个随机书写者编号，用来把同一人的样本分组；IP 只以加盐哈希形式用于限流。",
    privacy_p2: "样本包含目标字、笔画坐标、书写语言和匿名编号。请不要书写含有个人信息的内容。想删除自己的样本，请通过 GitHub 联系。",
    privacy_p3: "样本以 CC BY 4.0 发布；站点代码以 GPL-3.0-or-later 发布。",
    footer_note: "墨笔（fcitx5-ink）· 样本以 CC BY 4.0 发布",
    eval_hero: "维护者评估模式：写留出评估集。",
    eval_new_writer: "换一位书写者",
    eval_stats_line: "评估集：本池已写 {samples} 份样本 · {writers} 位书写者 · 覆盖 {chars}/300 个字",
    eval_progress: "已写 {written} 份 · 本池还剩 {left} 个字",
    eval_empty: "评估集还没有样本。",
    eval_finished: "这一份评估集写完了，谢谢！",
    eval_round_note: "本池每语言 300 个字；写完可以停，进度自动保存。",
  },
  zh_TW: {
    title: "墨筆 · 手寫樣本",
    description: "為墨筆（fcitx5-ink）貢獻匿名手寫筆畫，用於訓練與評估手寫辨識模型。",
    brand: "墨筆",
    brand_sub: "手寫樣本",
    language: "語言",
    hero_title: "寫 {batch} 個字，讓墨筆更懂手寫。",
    hero_lead: "墨筆（fcitx5-ink）是一個 fcitx5 手寫輸入工具。這裡收集的匿名筆畫會進入訓練池，用於訓練與評估手寫辨識模型，讓它在更多寫法上認得更準。不需要帳號，不收集姓名或電子郵件。",
    start: "開始書寫",
    round_note: "每一輪 {batch} 個字，來自訓練池；寫完就可以停，想多寫可以再來一輪。",
    write_title: "書寫",
    write_this: "請書寫這個字",
    target_aria: "目標字：{char}",
    canvas_aria: "書寫區域",
    undo: "復原",
    clear: "清除",
    skip: "跳過",
    submit: "送出",
    restart: "再來一輪",
    consent_text: "我同意將這份手寫樣本以 CC BY 4.0 授權發布，並用於訓練與評估手寫辨識模型。我確認這是我本人的書寫，且不包含個人資料。",
    data_policy: "資料說明",
    source_code: "原始碼",
    stats_line: "訓練池：已收集 {samples} 份樣本 · {writers} 位書寫者 · 涵蓋 {chars}/{total} 個字",
    progress: "已寫 {written} 份 · 本輪還剩 {left} 個字",
    status_loading: "正在載入…",
    status_ready: "請在白色區域內書寫。",
    status_saving: "正在送出…",
    status_preparing: "正在準備安全驗證…",
    status_saved: "已儲存，謝謝！接著寫下一個字。",
    status_duplicate: "這份樣本已經記錄過了，換下一個字。",
    status_error: "送出失敗，請稍後再試。",
    status_rate: "送出太頻繁了，請稍後再試。",
    status_offline: "網路連線失敗，請檢查網路後再試。",
    status_consent: "請先勾選同意。",
    status_finished: "這一輪寫完了，謝謝！",
    status_load_error: "載入失敗，請重新整理頁面。",
    status_pow_failed: "安全驗證失敗，請重新整理頁面再試。",
    empty_stats: "訓練池還沒有樣本，成為第一位書寫者。",
    about_title: "為什麼要收集",
    about_p1: "手寫辨識需要大量真實寫法。訓練池依常用標準字表涵蓋：簡體 3,500、繁體 4,808、日文 2,136 個常用字與假名；字序依使用頻率排列。",
    about_p2: "訓練池與評估集分開：評估集是每語言 300 個留出字，由維護者用不同書寫者收集；同一書寫者不會同時進入兩邊。",
    about_p3: "樣本以 CC BY 4.0 發布，任何人都可以再利用；專案程式碼以 GPL-3.0-or-later 發布。",
    privacy_title: "隱私與授權",
    privacy_p1: "不收集姓名、電子郵件或原始 IP。瀏覽器裡只儲存一個隨機書寫者編號，用來把同一人的樣本分組；IP 只以加鹽雜湊形式用於限流。",
    privacy_p2: "樣本包含目標字、筆畫座標、書寫語言和匿名編號。請不要書寫含有個人資料的內容。想刪除自己的樣本，請透過 GitHub 聯絡。",
    privacy_p3: "樣本以 CC BY 4.0 發布；網站程式碼以 GPL-3.0-or-later 發布。",
    footer_note: "墨筆（fcitx5-ink）· 樣本以 CC BY 4.0 發布",
    eval_hero: "維護者評估模式：寫出留出評估集。",
    eval_new_writer: "換一位書寫者",
    eval_stats_line: "評估集：本池已寫 {samples} 份樣本 · {writers} 位書寫者 · 涵蓋 {chars}/300 個字",
    eval_progress: "已寫 {written} 份 · 本池還剩 {left} 個字",
    eval_empty: "評估集還沒有樣本。",
    eval_finished: "這一份評估集寫完了，謝謝！",
    eval_round_note: "本池每語言 300 個字；寫完可以停，進度會自動儲存。",
  },
  ja: {
    title: "墨筆 · 手書きサンプル",
    description: "墨筆（fcitx5-ink）に匿名の手書き筆跡を提供して、手書き認識モデルの訓練と評価に役立ててください。",
    brand: "墨筆",
    brand_sub: "手書きサンプル",
    language: "言語",
    hero_title: "{batch} 文字書いて、墨筆の手書き認識を良くしよう。",
    hero_lead: "墨筆（fcitx5-ink）は fcitx5 の手書き入力ツールです。ここで集めた匿名の筆跡は訓練プールに入り、手書き認識モデルの訓練と評価に使われます。アカウントは不要で、氏名やメールアドレスは収集しません。",
    start: "書き始める",
    round_note: "1 ラウンド {batch} 文字です。訓練プールから出題されます。書き終えたら終了して構いません。続けたい場合はもう一度どうぞ。",
    write_title: "書く",
    write_this: "この文字を書いてください",
    target_aria: "お題の文字：{char}",
    canvas_aria: "手書き入力欄",
    undo: "元に戻す",
    clear: "消去",
    skip: "スキップ",
    submit: "送信",
    restart: "もう一度",
    consent_text: "この手書きサンプルを CC BY 4.0 で公開し、手書き認識モデルの訓練と評価に使用することに同意します。自分自身の筆跡であり、個人情報を含まないことを確認します。",
    data_policy: "データについて",
    source_code: "ソースコード",
    stats_line: "訓練プール：収集済み {samples} 件 · {writers} 人の書き手 · {chars}/{total} 文字をカバー",
    progress: "書き済み {written} 件 · このラウンドは残り {left} 文字",
    status_loading: "読み込み中…",
    status_ready: "白い枠の中に書いてください。",
    status_saving: "送信中…",
    status_preparing: "確認処理を準備しています…",
    status_saved: "保存しました。ありがとうございます。次の文字へ。",
    status_duplicate: "このサンプルは記録済みです。次の文字へ。",
    status_error: "送信に失敗しました。しばらくしてから再試行してください。",
    status_rate: "送信が多すぎます。しばらくしてから再試行してください。",
    status_offline: "ネットワークに接続できません。接続を確認して再試行してください。",
    status_consent: "先に同意にチェックしてください。",
    status_finished: "このラウンドは完了しました。ありがとうございます。",
    status_load_error: "読み込みに失敗しました。ページを再読み込みしてください。",
    status_pow_failed: "確認処理に失敗しました。ページを再読み込みしてください。",
    empty_stats: "訓練プールにはまだサンプルがありません。最初の書き手になりませんか。",
    about_title: "なぜ集めるのか",
    about_p1: "手書き認識には多様な実筆跡が必要です。訓練プールは常用標準字表をカバーします：簡体字 3,500、繁体字 4,808、日本語 2,136 の常用漢字と仮名。字の順序は使用頻度順です。",
    about_p2: "訓練プールと評価セットは分けています。評価セットは言語ごとに 300 文字のホールドアウトで、別の書き手から管理者が集めます。同じ書き手が両方に入ることはありません。",
    about_p3: "サンプルは CC BY 4.0 で公開され、誰でも再利用できます。プロジェクトのコードは GPL-3.0-or-later です。",
    privacy_title: "プライバシーとライセンス",
    privacy_p1: "氏名、メールアドレス、生の IP は収集しません。ブラウザにはランダムな書き手 ID だけを保存し、同じ人のサンプルをまとめるために使います。IP はソルト付きハッシュとしてレート制限にのみ使います。",
    privacy_p2: "サンプルにはお題の文字、筆跡の座標、言語、匿名 ID が含まれます。個人情報を含む内容は書かないでください。自分のサンプルの削除を希望する場合は GitHub で連絡してください。",
    privacy_p3: "サンプルは CC BY 4.0、サイトのコードは GPL-3.0-or-later で公開されます。",
    footer_note: "墨筆（fcitx5-ink）· サンプルは CC BY 4.0",
    eval_hero: "管理者用の評価モード：ホールドアウト評価セットを書きます。",
    eval_new_writer: "別の書き手に切り替える",
    eval_stats_line: "評価セット：このプールは {samples} 件 · {writers} 人の書き手 · {chars}/300 文字をカバー",
    eval_progress: "書き済み {written} 件 · このプールは残り {left} 文字",
    eval_empty: "評価セットにはまだサンプルがありません。",
    eval_finished: "この評価セットは完了しました。ありがとうございます。",
    eval_round_note: "このプールは言語ごとに 300 文字です。書き終えたら停止して構いません。進捗は自動保存されます。",
  },
};

const el = {
  langSelect: document.getElementById("lang-select"),
  statsLine: document.getElementById("stats-line"),
  targetChar: document.getElementById("target-char"),
  progress: document.getElementById("progress"),
  status: document.getElementById("status"),
  canvas: document.getElementById("canvas"),
  undo: document.getElementById("undo"),
  clear: document.getElementById("clear"),
  skip: document.getElementById("skip"),
  submit: document.getElementById("submit"),
  restart: document.getElementById("restart"),
  consent: document.getElementById("consent"),
  writeBody: document.getElementById("write-body"),
  evalNewWriter: document.getElementById("eval-new-writer"),
};

const ctx = el.canvas.getContext("2d");

const model = {
  lang: "zh_CN",
  targets: {},
  order: [],
  pos: 0,
  written: 0,
  finished: false,
  busy: false,
  hasSession: false,
  stats: null,
  consent: false,
  loadToken: 0,
};

let strokes = [];
let current = null;
let drawing = false;
let lastPoint = null;

function storageGet(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function storageSet(key, value) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    /* private mode: keep the session in memory */
  }
}

function fmt(value) {
  return new Intl.NumberFormat(HTML_LANG[model.lang] || "zh-CN").format(value);
}

function t(key, vars) {
  const primary = STRINGS[model.lang] || STRINGS.zh_CN;
  let text = primary[key];
  if (typeof text !== "string") text = STRINGS.zh_CN[key];
  if (typeof text !== "string") return key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.split(`{${name}}`).join(String(value));
    }
  }
  return text;
}

function applyLanguage() {
  const strings = STRINGS[model.lang] || STRINGS.zh_CN;
  document.documentElement.lang = HTML_LANG[model.lang] || "zh-CN";
  document.title = strings.title;
  const meta = document.querySelector('meta[name="description"]');
  if (meta) meta.content = strings.description;
  for (const node of document.querySelectorAll("[data-i18n]")) {
    const key = node.getAttribute("data-i18n");
    const text = strings[key];
    if (typeof text === "string") node.textContent = text;
  }
  for (const node of document.querySelectorAll("[data-i18n-aria]")) {
    const key = node.getAttribute("data-i18n-aria");
    const text = strings[key];
    if (typeof text === "string") node.setAttribute("aria-label", text);
  }
  const heroTitle = document.querySelector('[data-i18n="hero_title"]');
  if (heroTitle) {
    heroTitle.textContent = t(MODE === "eval" ? "eval_hero" : "hero_title", {
      batch: BATCH_SIZE,
    });
  }
  const roundNote = document.querySelector('[data-i18n="round_note"]');
  if (roundNote) {
    roundNote.textContent = t(
      MODE === "eval" ? "eval_round_note" : "round_note",
      { batch: BATCH_SIZE },
    );
  }
  el.langSelect.value = model.lang;
}

function setStatus(key, isError = false) {
  el.status.textContent = key ? t(key) : "";
  el.status.classList.toggle("error", Boolean(isError));
}

function renderTarget() {
  if (model.finished) {
    el.targetChar.textContent = "✓";
    el.targetChar.setAttribute(
      "aria-label",
      t(MODE === "eval" ? "eval_finished" : "status_finished"),
    );
    return;
  }
  const char = model.order[model.pos] || "";
  el.targetChar.textContent = char || "…";
  if (char) el.targetChar.setAttribute("aria-label", t("target_aria", { char }));
}

function renderProgress() {
  const left = Math.max(0, model.order.length - model.pos);
  el.progress.textContent = t(MODE === "eval" ? "eval_progress" : "progress", {
    written: fmt(model.written),
    left: fmt(left),
  });
}

function renderStats() {
  if (!model.stats) {
    el.statsLine.textContent = t("status_loading");
    return;
  }
  const total = model.targets[model.lang]?.length || 0;
  const statsKey = MODE === "eval" ? "eval_stats_line" : "stats_line";
  const emptyKey = MODE === "eval" ? "eval_empty" : "empty_stats";
  const stats = model.stats.languages?.[model.lang] || {
    samples: 0,
    writers: 0,
    characters: 0,
  };
  if (!stats.samples) {
    el.statsLine.textContent = t(emptyKey);
    return;
  }
  el.statsLine.textContent = t(statsKey, {
    samples: fmt(stats.samples),
    writers: fmt(stats.writers),
    chars: fmt(stats.characters),
    total: fmt(MODE === "eval" ? 300 : total),
  });
}

function canSubmit() {
  return (
    model.consent &&
    strokes.length > 0 &&
    !model.busy &&
    !model.finished &&
    !drawing
  );
}

function updateControls() {
  const idle = !model.busy && !model.finished;
  el.undo.disabled = !idle || strokes.length === 0;
  el.clear.disabled = !idle || strokes.length === 0;
  el.skip.disabled = !idle;
  el.submit.disabled = !canSubmit();
  el.restart.hidden = !model.finished;
  el.langSelect.disabled = model.busy;
}

function drawPath(stroke) {
  if (!stroke || stroke.length < 2) return;
  ctx.beginPath();
  ctx.moveTo(stroke[0][0], stroke[0][1]);
  for (let index = 1; index < stroke.length; index += 1) {
    ctx.lineTo(stroke[index][0], stroke[index][1]);
  }
  ctx.stroke();
}

function redraw() {
  ctx.fillStyle = "#fff";
  ctx.fillRect(0, 0, SIZE, SIZE);
  ctx.strokeStyle = "#141414";
  ctx.lineWidth = 2.6;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  for (const stroke of strokes) drawPath(stroke);
  if (current) drawPath(current);
}

function pointFromEvent(event) {
  const rect = el.canvas.getBoundingClientRect();
  const x = ((event.clientX - rect.left) * SIZE) / rect.width;
  const y = ((event.clientY - rect.top) * SIZE) / rect.height;
  const clamp = (value) => Math.round(Math.min(SIZE, Math.max(0, value)) * 10) / 10;
  return [clamp(x), clamp(y)];
}

function startStroke(event) {
  if (model.busy || model.finished) return;
  if (event.pointerType === "mouse" && event.button !== 0) return;
  if (!event.isPrimary || drawing) return;
  event.preventDefault();
  drawing = true;
  current = [pointFromEvent(event)];
  lastPoint = current[0];
  try {
    el.canvas.setPointerCapture(event.pointerId);
  } catch {
    /* pointer capture is best effort */
  }
  redraw();
  updateControls();
}

function moveStroke(event) {
  if (!drawing || !current) return;
  event.preventDefault();
  const point = pointFromEvent(event);
  if (lastPoint) {
    const dx = point[0] - lastPoint[0];
    const dy = point[1] - lastPoint[1];
    if (Math.hypot(dx, dy) < 0.8) return;
  }
  current.push(point);
  lastPoint = point;
  redraw();
}

function endStroke(event) {
  if (!drawing) return;
  event.preventDefault();
  drawing = false;
  try {
    el.canvas.releasePointerCapture(event.pointerId);
  } catch {
    /* the pointer may already be released */
  }
  if (event.type !== "pointercancel" && current && current.length >= 2) {
    strokes.push(current);
  }
  current = null;
  lastPoint = null;
  redraw();
  updateControls();
}

function undo() {
  if (model.busy) return;
  strokes.pop();
  current = null;
  drawing = false;
  lastPoint = null;
  redraw();
  updateControls();
}

function clearCanvas() {
  if (model.busy) return;
  strokes = [];
  current = null;
  drawing = false;
  lastPoint = null;
  redraw();
  updateControls();
}

function mulberry32(seed) {
  let value = seed >>> 0;
  return function next() {
    value = (value + 0x6d2b79f5) | 0;
    let t = Math.imul(value ^ (value >>> 15), 1 | value);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function shuffled(chars, seed) {
  const items = chars.slice();
  const random = mulberry32(seed);
  for (let index = items.length - 1; index > 0; index -= 1) {
    const other = Math.floor(random() * (index + 1));
    const swap = items[index];
    items[index] = items[other];
    items[other] = swap;
  }
  return items;
}

function newSeed() {
  return Math.floor(Math.random() * 0xffffffff) >>> 0;
}

function sampleBatch(chars, size) {
  return shuffled(chars, newSeed()).slice(0, Math.min(size, chars.length));
}

function saveRound() {
  storageSet(
    roundKey(model.lang),
    JSON.stringify({
      batch: model.order,
      pos: model.pos,
      written: model.written,
    }),
  );
}

function readRound(lang, chars, expected) {
  const raw = storageGet(roundKey(lang));
  if (!raw) return null;
  try {
    const value = JSON.parse(raw);
    if (
      !Array.isArray(value.batch) ||
      value.batch.length !== expected ||
      !value.batch.every((char) => typeof char === "string" && chars.includes(char)) ||
      !Number.isInteger(value.pos) ||
      !Number.isInteger(value.written) ||
      value.pos < 0 ||
      value.pos > value.batch.length ||
      value.written < 0 ||
      value.written > value.pos
    ) {
      return null;
    }
    return value;
  } catch {
    return null;
  }
}

function setupQueue(chars) {
  const expected = MODE === "eval" ? chars.length : Math.min(BATCH_SIZE, chars.length);
  const saved = readRound(model.lang, chars, expected);
  if (saved) {
    model.order = saved.batch;
    model.pos = saved.pos;
    model.written = saved.written;
  } else {
    model.order = MODE === "eval" ? chars.slice() : sampleBatch(chars, BATCH_SIZE);
    model.pos = 0;
    model.written = 0;
  }
  model.finished = model.pos >= model.order.length;
  saveRound();
}

async function loadTargets(lang) {
  if (model.targets[lang]) return model.targets[lang];
  const response = await fetch(`${API_BASE}/targets?lang=${encodeURIComponent(lang)}`, {
    cache: "no-store",
  });
  if (!response.ok) throw new Error("targets");
  const data = await response.json();
  if (!Array.isArray(data.characters) || data.characters.length === 0) {
    throw new Error("targets");
  }
  model.targets[lang] = data.characters;
  return data.characters;
}

async function switchLanguage(lang) {
  if (!LANGUAGES.includes(lang)) lang = "zh_CN";
  model.lang = lang;
  storageSet(STORAGE_LANG, lang);
  applyLanguage();
  strokes = [];
  current = null;
  drawing = false;
  lastPoint = null;
  el.status.textContent = "";
  redraw();
  const token = (model.loadToken += 1);
  setStatus("status_loading");
  try {
    const chars = await loadTargets(lang);
    if (token !== model.loadToken) return;
    setupQueue(chars);
    renderTarget();
    renderProgress();
    renderStats();
    updateControls();
    setStatus(
      model.finished
        ? MODE === "eval"
          ? "eval_finished"
          : "status_finished"
        : "status_ready",
    );
  } catch {
    if (token === model.loadToken) setStatus("status_load_error", true);
  }
}

async function loadStats() {
  try {
    const response = await fetch(`${API_BASE}/stats`, { cache: "no-store" });
    if (!response.ok) return;
    model.stats = await response.json();
    renderStats();
  } catch {
    /* the page still works without statistics */
  }
}

async function checkSession() {
  try {
    const response = await fetch(`${API_BASE}/session`, { cache: "no-store" });
    if (!response.ok) return;
    const data = await response.json();
    model.hasSession = data.ok === true;
  } catch {
    /* solved on the first submit instead */
  }
}

function solvePow(challenge, difficulty) {
  return new Promise((resolve, reject) => {
    const worker = new Worker("/pow-worker.js");
    const timer = setTimeout(() => {
      worker.terminate();
      reject(new Error("pow"));
    }, 120000);
    worker.onmessage = (event) => {
      const data = event.data || {};
      if (typeof data.nonce === "string") {
        clearTimeout(timer);
        worker.terminate();
        resolve(data.nonce);
        return;
      }
      if (data.error) {
        clearTimeout(timer);
        worker.terminate();
        reject(new Error("pow"));
      }
    };
    worker.onerror = () => {
      clearTimeout(timer);
      worker.terminate();
      reject(new Error("pow"));
    };
    worker.postMessage({ challenge, difficulty });
  });
}

async function ensureSession() {
  if (model.hasSession) return;
  for (let attempt = 0; attempt < 2; attempt += 1) {
    setStatus("status_preparing");
    const response = await fetch(`${API_BASE}/challenge`, { cache: "no-store" });
    if (!response.ok) throw new Error("challenge");
    const data = await response.json();
    if (typeof data.challenge !== "string" || !Number.isInteger(data.difficulty)) {
      throw new Error("challenge");
    }
    const nonce = await solvePow(data.challenge, data.difficulty);
    const session = await fetch(`${API_BASE}/session`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ challenge: data.challenge, nonce }),
    });
    if (session.ok) {
      model.hasSession = true;
      return;
    }
    if (session.status === 429) throw new Error("rate");
    if (session.status !== 400) throw new Error("session");
  }
  throw new Error("session");
}

function submitSample(payload) {
  return fetch(`${API_BASE}/samples`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}

async function submit() {
  if (!canSubmit()) {
    if (!model.consent) setStatus("status_consent", true);
    return;
  }
  const payload = {
    language: model.lang,
    character: model.order[model.pos],
    width: SIZE,
    height: SIZE,
    strokes: strokes.map((stroke) => stroke.map((point) => [point[0], point[1]])),
    consent: true,
  };
  model.busy = true;
  updateControls();
  setStatus("status_saving");
  try {
    await ensureSession();
    let response = await submitSample(payload);
    if (response.status === 401) {
      model.hasSession = false;
      await ensureSession();
      response = await submitSample(payload);
    }
    let data = null;
    try {
      data = await response.json();
    } catch {
      /* an empty or non-JSON error body is still an error */
    }
    if (!response.ok) {
      if (response.status === 429) setStatus("status_rate", true);
      else if (data && data.error === "consent") setStatus("status_consent", true);
      else setStatus("status_error", true);
      return;
    }
    const duplicate = Boolean(data && data.duplicate);
    strokes = [];
    current = null;
    drawing = false;
    lastPoint = null;
    model.pos = Math.min(model.pos + 1, model.order.length);
    model.written += 1;
    model.finished = model.pos >= model.order.length;
    saveRound();
    redraw();
    renderTarget();
    renderProgress();
    if (model.finished) {
      setStatus(MODE === "eval" ? "eval_finished" : "status_finished");
    } else setStatus(duplicate ? "status_duplicate" : "status_saved");
    loadStats();
  } catch (error) {
    if (error instanceof TypeError) setStatus("status_offline", true);
    else if (error && (error.message === "pow" || error.message === "challenge")) {
      setStatus("status_pow_failed", true);
    } else if (error && error.message === "rate") setStatus("status_rate", true);
    else setStatus("status_error", true);
  } finally {
    model.busy = false;
    updateControls();
  }
}

function skip() {
  if (model.busy || model.finished) return;
  strokes = [];
  current = null;
  drawing = false;
  lastPoint = null;
  model.pos = Math.min(model.pos + 1, model.order.length);
  model.finished = model.pos >= model.order.length;
  saveRound();
  redraw();
  renderTarget();
  renderProgress();
  updateControls();
  setStatus(
    model.finished
      ? MODE === "eval"
        ? "eval_finished"
        : "status_finished"
      : "status_ready",
  );
}

function restart() {
  if (!model.targets[model.lang]) return;
  model.pos = 0;
  model.written = 0;
  model.finished = false;
  model.order =
    MODE === "eval"
      ? model.targets[model.lang].slice()
      : sampleBatch(model.targets[model.lang], BATCH_SIZE);
  strokes = [];
  current = null;
  drawing = false;
  lastPoint = null;
  saveRound();
  redraw();
  renderTarget();
  renderProgress();
  updateControls();
  setStatus("status_ready");
}

function updateConsent() {
  model.consent = el.consent.checked;
  storageSet(STORAGE_CONSENT, model.consent ? "1" : null);
  updateControls();
}

function wireEvents() {
  el.langSelect.addEventListener("change", () => switchLanguage(el.langSelect.value));
  el.consent.addEventListener("change", updateConsent);
  el.undo.addEventListener("click", undo);
  el.clear.addEventListener("click", clearCanvas);
  el.skip.addEventListener("click", skip);
  el.submit.addEventListener("click", submit);
  el.restart.addEventListener("click", restart);
  el.canvas.addEventListener("pointerdown", startStroke);
  el.canvas.addEventListener("pointermove", moveStroke);
  el.canvas.addEventListener("pointerup", endStroke);
  el.canvas.addEventListener("pointercancel", endStroke);
  el.canvas.addEventListener("contextmenu", (event) => event.preventDefault());
  if (el.evalNewWriter) el.evalNewWriter.addEventListener("click", evalNewWriter);
  document.addEventListener("keydown", (event) => {
    const tag = (event.target && event.target.tagName
      ? event.target.tagName
      : ""
    ).toLowerCase();
    if (["input", "select", "textarea", "button", "a"].includes(tag)) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
      event.preventDefault();
      undo();
    } else if (event.key === "Escape") {
      event.preventDefault();
      clearCanvas();
    }
  });
}

async function evalNewWriter() {
  try {
    await fetch(`${API_BASE}/eval/reset`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "{}",
    });
  } catch {
    /* reload anyway; a stale writer cookie is harmless here */
  }
  location.reload();
}

async function init() {
  const savedLang = storageGet(STORAGE_LANG);
  model.lang = LANGUAGES.includes(savedLang) ? savedLang : "zh_CN";
  model.consent = storageGet(STORAGE_CONSENT) === "1";
  el.consent.checked = model.consent;
  applyLanguage();
  redraw();
  wireEvents();
  checkSession();
  loadStats();
  await switchLanguage(model.lang);
}

init();
