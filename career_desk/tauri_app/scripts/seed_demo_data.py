"""Insert an idempotent, linked CareerCraft demo dataset into a local profile."""

from __future__ import annotations

import argparse
import json
import sqlite3
from datetime import datetime
from pathlib import Path


def j(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=Path, default=Path.home() / ".careercraft" / "career.db")
    args = parser.parse_args()
    db = args.db.resolve()
    if not db.exists():
        raise SystemExit(f"database not found: {db}")

    backup_dir = db.parent / "backups"
    backup_dir.mkdir(parents=True, exist_ok=True)
    backup = backup_dir / f"career-pre-demo-seed-{datetime.now():%Y%m%d-%H%M%S}.db"
    source = sqlite3.connect(db, timeout=20)
    target = sqlite3.connect(backup)
    source.backup(target)
    target.close()
    source.execute("PRAGMA foreign_keys=ON")

    personas = [
        ("demo-persona-product", "模拟用户", "模拟｜数据产品经理", 1,
         "以用户洞察和数据实验推动产品增长的数据产品经理",
         "拥有 B 端产品、数据分析与跨团队协作经验，希望进一步发展实验设计和 AI 产品能力。",
         "professional", {"product": .35, "data": .35, "delivery": .2, "leadership": .1},
         ["数据产品经理", "AI 产品经理"], 6),
        ("demo-persona-lead", "模拟用户", "模拟｜产品负责人", 0,
         "能连接业务目标、产品策略与交付团队的产品负责人",
         "从一线需求分析成长为产品负责人，擅长路线图规划、项目推进与结果复盘。",
         "executive", {"strategy": .35, "leadership": .3, "product": .25, "data": .1},
         ["产品负责人", "高级产品经理"], 5),
    ]
    source.executemany(
        "INSERT INTO personas(id,user_id,name,is_default,identity_statement,career_narrative,tone_style,capability_weights,target_job_profiles,max_experiences) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,is_default=excluded.is_default,identity_statement=excluded.identity_statement,career_narrative=excluded.career_narrative,tone_style=excluded.tone_style,capability_weights=excluded.capability_weights,target_job_profiles=excluded.target_job_profiles,max_experiences=excluded.max_experiences,updated_at=CURRENT_TIMESTAMP",
        [(a,b,c,d,e,f,g,j(h),j(i),k) for a,b,c,d,e,f,g,h,i,k in personas],
    )

    experiences = [
        ("demo-exp-dashboard", "work", "模拟｜供应链数据产品经理", "示例科技", "2023-03-01", None,
         "负责供应链风险看板，从采购、库存和履约数据中识别缺货风险。联合数据与研发团队完成指标口径治理，上线后周报准备时间减少 60%，重点品类缺货率下降 12%。",
         ["建立统一供应链指标体系", "上线风险预警看板", "缺货率下降 12%"], ["数据分析", "SQL", "需求分析", "跨团队协作"], [{"name":"缺货率","value":"-12%"},{"name":"周报时间","value":"-60%"}], ["供应链", "SaaS"], "bachelor"),
        ("demo-exp-growth", "project", "模拟｜增长实验平台", "示例科技", "2024-01-01", "2024-08-31",
         "牵头搭建增长实验流程，沉淀实验假设、指标选择、分流与复盘模板，推动 8 个实验完成闭环，其中 3 个方案进入正式版本。",
         ["建立实验流程与模板", "推动 8 个实验闭环", "3 个方案正式上线"], ["A/B 测试", "数据驱动决策", "项目管理"], [{"name":"闭环实验","value":"8"}], ["互联网"], "bachelor"),
        ("demo-exp-research", "work", "模拟｜用户研究与需求体系", "示例软件", "2021-07-01", "2023-02-28",
         "通过客户访谈、工单分析和可用性测试重构需求分级机制，覆盖 30 家重点客户，使高优需求平均交付周期缩短 25%。",
         ["访谈 30 家重点客户", "重构需求分级机制", "交付周期缩短 25%"], ["用户研究", "需求分析", "PRD 撰写"], [{"name":"交付周期","value":"-25%"}], ["企业服务"], "bachelor"),
        ("demo-exp-ai", "project", "模拟｜AI 助手概念验证", "个人项目", "2025-01-01", "2025-04-30",
         "完成面向知识工作者的 AI 助手概念验证，设计检索、引用与人工确认流程，并通过 12 名种子用户测试迭代交互方案。",
         ["完成 AI 助手 MVP", "设计引用和人工确认机制", "完成 12 人可用性测试"], ["AI 产品设计", "MVP 验证", "用户研究"], [{"name":"种子用户","value":"12"}], ["人工智能"], "bachelor"),
        ("demo-exp-lead", "work", "模拟｜跨团队项目负责人", "示例科技", "2024-09-01", None,
         "协调产品、研发、数据和运营四个团队推进主数据治理项目，建立双周里程碑和风险升级机制，按期完成首期上线。",
         ["协调 4 个职能团队", "建立里程碑和风险机制", "首期按期上线"], ["项目管理", "跨团队协作", "向上管理"], [{"name":"协作团队","value":"4"}], ["供应链"], "bachelor"),
        ("demo-exp-cert", "certification", "模拟｜数据分析专项学习", "开放课程", "2022-01-01", "2022-06-30",
         "系统学习 SQL、统计分析和数据可视化，并完成零售经营分析结课项目。",
         ["完成零售经营分析项目"], ["SQL", "数据分析", "数据可视化"], [], ["零售"], "bachelor"),
    ]
    # Keep the data declaration readable while binding the exact table order.
    for eid, kind, title, org, start, end, raw, achievements, skills, metrics, industries, education in experiences:
        source.execute(
            "INSERT INTO experiences(id,user_id,type,title,organization,start_date,end_date,raw_description,structured_achievements,skills_demonstrated,metrics,status,version,industry_tags,education_level) VALUES(?,'模拟用户',?,?,?,?,?,?,?,?,?,'confirmed',1,?,?) ON CONFLICT(id) DO UPDATE SET title=excluded.title,organization=excluded.organization,start_date=excluded.start_date,end_date=excluded.end_date,raw_description=excluded.raw_description,structured_achievements=excluded.structured_achievements,skills_demonstrated=excluded.skills_demonstrated,metrics=excluded.metrics,industry_tags=excluded.industry_tags,education_level=excluded.education_level,updated_at=CURRENT_TIMESTAMP",
            (eid, kind, title, org, start, end, raw, j(achievements), j(skills), j(metrics), j(industries), education),
        )

    scores = {
        "demo-persona-product": [92, 88, 86, 80, 78, 72],
        "demo-persona-lead": [82, 76, 84, 65, 94, 60],
    }
    for persona_id, values in scores.items():
        for experience, score in zip(experiences, values):
            eid = experience[0]
            source.execute(
                "INSERT INTO role_experience_weights(id,persona_id,experience_id,relevance_score,highlighted_skills,user_overridden) VALUES(?,?,?,?,?,0) ON CONFLICT(persona_id,experience_id) DO UPDATE SET relevance_score=excluded.relevance_score,highlighted_skills=excluded.highlighted_skills,updated_at=CURRENT_TIMESTAMP",
                (f"demo-weight-{persona_id}-{eid}", persona_id, eid, score / 100, j(experience[8][:3])),
            )

    jobs = [
        ("demo-job-ai-product", "高级 AI 产品经理", "未来智能", "上海", ["AI 产品设计","用户研究","SQL","实验设计","跨团队协作"], ["人工智能"], "负责 AI 产品规划、用户验证、指标体系和跨团队交付。要求 5 年产品经验，熟悉 AI 产品设计、SQL、实验设计。"),
        ("demo-job-data-product", "数据产品经理", "云端数据", "深圳", ["需求分析","SQL","数据分析","A/B 测试"], ["SaaS"], "负责企业数据产品与分析平台建设，推动指标治理和实验闭环。"),
        ("demo-job-product-lead", "产品负责人", "示例零售", "杭州", ["产品路线图","项目管理","团队管理","数据驱动决策"], ["零售"], "负责产品战略、路线图和多团队协同，对业务结果负责。"),
    ]
    for jid, title, company, location, skills, industries, raw in jobs:
        source.execute(
            "INSERT INTO job_descs(id,raw_text,title,company,years_of_experience,location,job_type,education_requirement,responsibilities,parsed_skills,source,industry_tags,education_levels) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET raw_text=excluded.raw_text,title=excluded.title,company=excluded.company,location=excluded.location,parsed_skills=excluded.parsed_skills,industry_tags=excluded.industry_tags,updated_at=CURRENT_TIMESTAMP",
            (jid, raw, f"模拟｜{title}", company, "5", location, "full_time", "本科", j([raw]), j(skills), "demo", j(industries), j(["bachelor"])),
        )

    matches = [
        ("demo-match-ai", "demo-persona-product", "demo-job-ai-product", 76, ["AI 产品设计","用户研究","SQL","跨团队协作"], ["实验设计"], {"skills":40,"experience":22,"industry":8,"education":6}, "favorite"),
        ("demo-match-data", "demo-persona-product", "demo-job-data-product", 88, ["需求分析","SQL","数据分析","A/B 测试"], [], {"skills":50,"experience":23,"industry":9,"education":6}, "applied"),
        ("demo-match-lead", "demo-persona-lead", "demo-job-product-lead", 73, ["项目管理","数据驱动决策"], ["产品路线图","团队管理"], {"skills":25,"experience":24,"industry":14,"education":10}, "new"),
    ]
    for mid, pid, jid, score, matched, missing, breakdown, status in matches:
        source.execute(
            "INSERT INTO job_matches(id,persona_id,job_desc_id,match_score,matched_skills,missing_skills,score_breakdown,notes,ai_analysis,tracking_status,version) VALUES(?,?,?,?,?,?,?,?,?,?,1) ON CONFLICT(id) DO UPDATE SET match_score=excluded.match_score,matched_skills=excluded.matched_skills,missing_skills=excluded.missing_skills,score_breakdown=excluded.score_breakdown,tracking_status=excluded.tracking_status,updated_at=CURRENT_TIMESTAMP",
            (mid, pid, jid, score, j(matched), j(missing), j(breakdown), "模拟岗位匹配", "基于模拟经历证据生成，仅供功能测试。", status),
        )

    resume_data = {
        "header": {"full_name":"模拟用户","headline":"数据产品经理｜AI 产品方向","email":"demo@example.com","phone":None,"location":"上海","links":[]},
        "summary":"拥有企业服务与供应链数据产品经验，擅长从用户问题出发建立指标体系并推动跨团队交付。",
        "experience":[
            {"source_experience_id":"demo-exp-dashboard","title":"供应链数据产品经理","organization":"示例科技","period":"2023.03—至今","summary":None,"achievements":["建立供应链风险指标体系，推动重点品类缺货率下降 12%","上线风险看板，使周报准备时间减少 60%"],"skills":["SQL","数据分析","需求分析"]},
            {"source_experience_id":"demo-exp-growth","title":"增长实验平台","organization":"示例科技","period":"2024.01—2024.08","summary":None,"achievements":["建立增长实验流程并推动 8 个实验完成闭环"],"skills":["A/B 测试","数据驱动决策"]},
        ],
        "education":[], "skills":["需求分析","SQL","数据分析","用户研究","A/B 测试","跨团队协作"], "extra_sections":{"说明":["本简历为模拟测试数据"]},
    }
    resume_data_v2 = dict(resume_data)
    resume_data_v2["summary"] = "数据产品经理，聚焦 AI 产品验证、指标体系与可衡量的业务结果。"
    resumes = [
        ("demo-resume-product-v1","demo-persona-product","模拟｜数据产品简历 v1","modern",1,resume_data,None),
        ("demo-resume-product-v2","demo-persona-product","模拟｜AI 产品定向简历","technical",2,resume_data_v2,"demo-resume-product-v1"),
        ("demo-resume-lead-v1","demo-persona-lead","模拟｜产品负责人简历","executive",1,{**resume_data,"summary":"产品负责人，擅长路线图规划、多团队协作和交付风险管理。"},None),
    ]
    for rid, pid, label, template, revision, data, parent in resumes:
        source.execute(
            "INSERT INTO resume_versions(id,persona_id,label,template,revision,data_json,parent_id) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET label=excluded.label,template=excluded.template,data_json=excluded.data_json,parent_id=excluded.parent_id",
            (rid, pid, label, template, revision, j(data), parent),
        )

    context = {"skillId":"ab_testing","origin":"skill_graph","personaName":"模拟｜数据产品经理","generationMode":"rules_only","reason":"模拟学习路径，用于验证学习进度与成果转化","guidance":"先掌握实验设计，再完成一个可复盘的产品实验。"}
    items = [
        {"itemId":"demo-learning-1","sequence":1,"objective":"理解实验假设与核心指标","practiceTask":"为一个真实产品问题写出实验假设","completionCriteria":"假设包含目标人群、改动和预期指标"},
        {"itemId":"demo-learning-2","sequence":2,"objective":"掌握分流与样本量基础","practiceTask":"设计 A/B 分组及护栏指标","completionCriteria":"说明主指标、护栏指标和停止条件"},
        {"itemId":"demo-learning-3","sequence":3,"objective":"完成实验复盘","practiceTask":"输出一页实验结论与下一步建议","completionCriteria":"结论能区分事实、推断和行动建议"},
    ]
    source.execute("INSERT INTO learning_paths(id,persona_id,target_gap,items,source_type,status,context_json,version) VALUES('demo-learning-path','demo-persona-product','A/B 测试',?,'skill_graph','active',?,1) ON CONFLICT(id) DO UPDATE SET items=excluded.items,context_json=excluded.context_json,status='active',updated_at=CURRENT_TIMESTAMP", (j(items), j(context)))
    learning_rows = [
        ("demo-learning-1","A/B 测试入门","https://www.optimizely.com/resources/introduction-to-a-b-testing/",2,"completed","已完成基础概念笔记","Optimizely","article"),
        ("demo-learning-2","实验设计与指标选择","https://vwo.com/ab-testing/",3,"in_progress",None,"VWO","article"),
        ("demo-learning-3","实验复盘实践","https://www.nngroup.com/articles/ab-testing/",2,"pending",None,"NNGroup","article"),
    ]
    for iid, title, url, hours, status, note, provider, kind in learning_rows:
        source.execute("INSERT INTO learning_items(id,path_id,skill_id,title,resource_url,estimated_hours,status,completion_note,source,resource_kind,version) VALUES(?,'demo-learning-path','ab_testing',?,?,?,?,?,?,?,1) ON CONFLICT(id) DO UPDATE SET title=excluded.title,status=excluded.status,completion_note=excluded.completion_note,updated_at=CURRENT_TIMESTAMP", (iid,title,url,hours,status,note,provider,kind))

    source.commit()
    counts = {name: source.execute(f"SELECT COUNT(*) FROM {name} WHERE id LIKE 'demo-%'").fetchone()[0] for name in ("personas","experiences","job_descs","job_matches","resume_versions","learning_paths","learning_items")}
    source.close()
    print(j({"database":str(db),"backup":str(backup),"demoCounts":counts}))


if __name__ == "__main__":
    main()
