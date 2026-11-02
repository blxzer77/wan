---
name: wan-adr
description: Record a durable wan decision in a Chinese Nygard-style architecture decision record.
---

# wan ADR

Use this skill only for a long-term decision that code and ordinary documentation cannot explain. Do not create an ADR for a local edit, a mechanical change, or a temporary plan.

## File and number

- Write the file under `docs/dev/adr/`.
- Use the next unused four-digit number: `NNNN-short-english-title.md`.
- Read existing ADRs before choosing the number.
- Write the ADR body in Simplified Chinese.

## Nygard format

Use this structure:

```markdown
# ADR NNNN：标题

- 状态：已接受
- 日期：YYYY-MM-DD

## 背景

## 决策

## 后果

## 考虑过的其他方案
```

The Chinese sections correspond to Nygard's Status, Context, Decision, and Consequences. `## 考虑过的其他方案` is mandatory. Record each real alternative and why it was not selected; never invent alternatives to fill the section.

## Superseded decisions

Never delete a superseded ADR. Keep the file and mark its status only as `Superseded by ADR NNNN`, pointing to the replacing record.
