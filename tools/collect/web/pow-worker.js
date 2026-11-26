/* SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
 * SPDX-License-Identifier: GPL-3.0-or-later */

"use strict";

self.onmessage = async (event) => {
  const data = event.data || {};
  const challenge = data.challenge;
  const difficulty = data.difficulty;

  if (typeof challenge !== "string" || !Number.isInteger(difficulty) || difficulty < 4) {
    self.postMessage({ error: "bad request" });
    return;
  }
  if (!self.crypto || !self.crypto.subtle) {
    self.postMessage({ error: "crypto unavailable" });
    return;
  }

  const prefix = "0".repeat(Math.ceil(difficulty / 4));
  const encoder = new TextEncoder();

  for (let nonce = 0; ; nonce += 1) {
    let digest;
    try {
      digest = await self.crypto.subtle.digest(
        "SHA-256",
        encoder.encode(`${challenge}:${nonce}`),
      );
    } catch {
      self.postMessage({ error: "crypto failed" });
      return;
    }
    const bytes = new Uint8Array(digest);
    let hex = "";
    for (const byte of bytes) hex += byte.toString(16).padStart(2, "0");
    if (hex.startsWith(prefix)) {
      self.postMessage({ nonce: String(nonce) });
      return;
    }
    if (nonce % 4096 === 0) self.postMessage({ progress: nonce });
  }
};
