# CareerCraft 修改任务

请对 CareerCraft 项目做以下 3 个修改，所有文件路径均相对于 /mnt/d/workplace_for_hermes/career-agent/。

## 1. 经历库明细表简化（只保留项目名称）

**File:** `prototype/ui-prototype.html`
**Location:** `renderExperiences` 函数，约第 1414 行

当前列表项显示了技能标签，把项目名称挡住了。需要删掉技能标签部分，只保留项目名称。

当前代码：
```javascript
        div.innerHTML =
          '<div class="exp-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2" ry="2"/><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/></svg></div>' +
          '<div class="exp-info">' +
            '<div class="exp-title">' + escapeHtml(item.title || '') + '</div>' +
          '</div>' +
          '<div class="exp-tags">' + (item.skills || []).slice(0,3).map(function(s){ return '<span class="pill">' + escapeHtml(s) + '</span>'; }).join('') + '</div>';
```

修改为：
```javascript
        div.innerHTML =
          '<div class="exp-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2" ry="2"/><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/></svg></div>' +
          '<div class="exp-info">' +
            '<div class="exp-title">' + escapeHtml(item.title || '') + '</div>' +
          '</div>';
```

## 2. 修复岗位匹配简历修饰的 SQLAlchemy lazy load 错误

**File:** `src/services/jd_reframe_engine.py`
**Location:** `reframe_experiences_for_job` 函数，约第 86 行

错误信息：`Parent instance <JobMatchExperienceReframe at 0x...> is not bound to a Session; lazy load operation of attribute 'experience' cannot proceed`

原因：查询 `JobMatchExperienceReframe` 时没有 `selectinload(JobMatchExperienceReframe.experience)`，导致返回结果后外部访问 `.experience` 时 session 已关闭。

当前代码：
```python
                existing = await session.execute(
                    select(JobMatchExperienceReframe).where(
                        JobMatchExperienceReframe.job_match_id == match_id
                    )
                )
```

修改为：
```python
                existing = await session.execute(
                    select(JobMatchExperienceReframe)
                    .options(selectinload(JobMatchExperienceReframe.experience))
                    .where(
                        JobMatchExperienceReframe.job_match_id == match_id
                    )
                )
```

## 3. 技能 Gap 雷达图换成用户上传的岗位描述

**需要修改两个文件：**

### 3a. 后端添加岗位描述字段

**File:** `src/ui/webview/api_handler.py`
**Location:** `_match_to_dict` 函数，约第 366-387 行

在返回字典中添加 `job_description` 字段：

在第 385 行附近（`"job_title": ...` 之后或之前）添加：
```python
            "job_description": getattr(job_desc, "raw_description", "") or getattr(job_desc, "description", "") or "",
```

### 3b. 前端删掉雷达图、换成岗位描述展示

**File:** `prototype/ui-prototype.html`
**Location:** 岗位匹配详情面板，约第 2090 行

当前代码：
```javascript
            '<div class="skill-radar-section">' +
              '<div class="card-title" style="font-size: 14px;">技能 Gap 雷达图</div>' +
              '<div id="skill-radar-container"></div>' +
            '</div>' +
```

修改为：
```javascript
            '<div style="margin-bottom: 12px;">' +
              '<div style="margin-bottom: 8px;"><strong>岗位描述:</strong></div>' +
              '<div style="font-size: 13px; color: var(--text-secondary); line-height: 1.6; max-height: 200px; overflow-y: auto; padding: 10px; background: rgba(255,255,255,0.02); border-radius: var(--radius-md); border: 1px solid var(--border-subtle);">' + escapeHtml(m.job_description || '暂无岗位描述') + '</div>' +
            '</div>' +
```

---

## 验证

修改完成后，运行测试：
```bash
.venv/bin/pytest tests/ -q
```

确保 149 个测试通过。然后提交：
```bash
git add -A && git commit -m "fix: 经历库简化显示+修复lazy load+雷达图换岗位描述"
```
