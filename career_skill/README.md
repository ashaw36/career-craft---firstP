# Career Skill Workspace

文件优先的个人经历库 + 简历 + 岗位匹配。对应三个项目内 Skill：

| Skill | 版本 | 触发语示例 |
|-------|------|------------|
| `career-experience-sync` | v1 | 同步经历 / sync raw / 校验经历 |
| `career-resume-build` | v2 | 生成简历 / 用 ai-pm 生成简历 |
| `career-jd-match` | v3 | 匹配岗位 / 分析这份 JD / gap |

Skill 路径：`.cursor/skills/{skill-name}/`

## 目录

```
career_skill/
  raw/                 # v1 原始材料（Agent 不改写）
  experiences/         # v1 标准化经历 Markdown
  .sync-state.json     # v1 增量 hash
  REVIEW.md            # v1 待确认清单
  personas/            # v2 角色档案
  resumes/{persona}/   # v2 resume.md + selection.md
  jobs/raw/            # v3 JD 原文
  jobs/parsed/         # v3 结构化 JD
  jobs/matches/        # v3 match.md + reframes.md
```

## 推荐使用顺序

1. **v1** — 往 `raw/` 丢材料 →「同步经历」→ 在 `REVIEW.md` 确认后把经历 `status` 改为 `confirmed`
2. **v2** — 编辑 `personas/*.md` →「生成简历」→ 查看 `resumes/{id}/`
3. **v3** — 粘贴 JD 或放入 `jobs/raw/` →「用 {persona} 匹配」→ 查看 `jobs/matches/`

## 演示数据

已预置：

- `raw/sample-acme-notes.md` → 两条经历
- `personas/ai-pm.md` → `resumes/ai-pm/`
- `jobs/raw/sample-ai-pm.md` → `jobs/matches/ai-pm__sample-ai-pm/`

## 校验（v1）

```bash
python .cursor/skills/career-experience-sync/scripts/validate.py career_skill/experiences
```

## 与 CareerCraft 桌面端

不写入 `career_hermes` SQLite；可后续再做导入桥接。
