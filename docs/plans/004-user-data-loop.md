# 004: Long-term user data loop

Status: proposed; needs infrastructure and legal work
Date: 2026-11-08

## Goal

Collect real online handwriting from consenting writers, evaluate models on
writer-separated data, and retrain on a predictable cadence. This is the
highest-value long-term accuracy lever.

## Missing prerequisites

- **Web capture backend**: a static-only site cannot accept submissions; a
  small append-only service or serverless endpoint is required.
- **Privacy and consent**: clear license (CC-BY-4.0 or CC0), retention policy,
  deletion contact, no names/emails, no long-term IP storage.
- **Anti-abuse**: rate limits, session IDs, duplicate detection, optional
  CAPTCHA, manual spot checks.
- **Storage and moderation**: append-only JSONL/object storage, review queue,
  contributor withdrawal process.
- **Recruitment**: a reason for writers to contribute and a plan to reach
  communities.
- **Retraining pipeline**: scheduled snapshot, manifest, training, evaluation,
  release notes, rollback.

## Work

### Phase 4a: invite-only capture

- Static HTML/JS capture page or the existing Tkinter tool.
- Local JSONL download; promote with `scripts/promote_capture.py`.
- Validate prompt/consent flow with 3–5 writers.

### Phase 4b: public capture

- Minimal backend with rate limits and anonymous writer IDs.
- Privacy policy and license notice on the page.
- Append-only storage; no personal data.
- Review queue; reject broken or duplicate samples.

### Phase 4c: evaluation and retraining loop

- Writer-separated evaluation snapshots.
- Per-language top-1/5/12 and latency reports.
- Retrain Zinnia or the neural model on a schedule.
- Publish model/data manifests and license notices; support deletion requests.

## Acceptance

- At least 3–5 writers per language in the first private round, then a larger
  consented set.
- Every committed sample has consent and a manifest entry.
- Evaluation writers never appear in training.
- A model update can be reproduced from pinned data and manifests.

## Effort

Months, mostly infrastructure, legal/privacy and community work. Do not start
the public backend until the invite-only flow and promotion script are proven.
