use careercraft_lib::{infra::db::Database, interface::commands};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
struct Case {
    name: &'static str,
    elapsed_ms: u128,
    threshold_ms: u128,
    passed: bool,
    detail: String,
}
fn timed(
    name: &'static str,
    threshold_ms: u128,
    work: impl FnOnce() -> Result<String, String>,
) -> Case {
    let start = Instant::now();
    let result = work();
    let elapsed_ms = start.elapsed().as_millis();
    Case {
        name,
        elapsed_ms,
        threshold_ms,
        passed: result.is_ok() && elapsed_ms <= threshold_ms,
        detail: result.unwrap_or_else(|e| e),
    }
}
fn ok(value: impl Serialize) -> Result<serde_json::Value, String> {
    let v = serde_json::to_value(value).map_err(|e| e.to_string())?;
    if v["success"] == true {
        Ok(v["data"].clone())
    } else {
        Err(v["error"].to_string())
    }
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn wal_helper(db: &Path, ready: &Path) -> Result<(), String> {
    let c = rusqlite::Connection::open(db).map_err(|e| e.to_string())?;
    c.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    c.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('wal-committed','u','project','Committed','durable','draft',1)",[]).map_err(|e|e.to_string())?;
    c.execute_batch("BEGIN IMMEDIATE; INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('wal-pending-a','u','project','Pending A','must rollback','draft',1); INSERT INTO experiences(id,user_id,type,title,raw_description,status,version) VALUES('wal-pending-b','u','project','Pending B','must rollback','draft',1);").map_err(|e|e.to_string())?;
    fs::write(ready, b"transaction-open").map_err(|e| e.to_string())?;
    loop {
        thread::sleep(Duration::from_secs(60))
    }
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let result = if args.get(1).is_some_and(|v| v == "--wal-helper") {
        match (args.get(2), args.get(3)) {
            (Some(db), Some(ready)) => wal_helper(Path::new(db), Path::new(ready)),
            _ => Err("wal helper requires db and ready paths".into()),
        }
    } else {
        run()
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1)
    }
}
fn run() -> Result<(), String> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/w7-gate"));
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let db_path = root.join(format!(
        "gate-{}-{}.db",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::env::set_var("CAREERCRAFT_DB_PATH", &db_path);
    careercraft_lib::infra::db::configure_runtime_path(&db_path).map_err(|e| e.to_string())?;
    let mut cases = Vec::new();
    cases.push(timed("backend_cold_database_start", 2000, || {
        drop(Database::open_and_migrate(&db_path).map_err(|e| e.to_string())?);
        Ok("schema migrated and integrity checked".into())
    }));
    {
        let c = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        let tx = c.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p1','u','Primary',1,'{}','[]',20)",[]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO personas(id,user_id,name,is_default,capability_weights,target_job_profiles,max_experiences) VALUES('p2','u','Secondary',0,'{}','[]',20)",[]).map_err(|e|e.to_string())?;
        for n in 0..1000 {
            tx.execute("INSERT INTO experiences(id,user_id,type,title,raw_description,structured_achievements,skills_demonstrated,status,version) VALUES(?1,'u','work',?2,'Built offline','[]','[]','confirmed',1)",params![format!("e{n:04}"),format!("Experience {n}")]).map_err(|e|e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }
    cases.push(timed("query_1000_experiences", 500, || {
        let data = ok(commands::get_experiences(Some(json!({"userId":"u"}))))?;
        if data.as_array().map_or(0, |v| v.len()) != 1000 {
            return Err("expected 1000 rows".into());
        }
        Ok("1000 typed rows".into())
    }));
    cases.push(timed("persona_switch_local", 500, || {
        for id in ["p1", "p2", "p1", "p2"] {
            let data = ok(commands::get_persona_by_id(Some(json!({"personaId":id}))))?;
            if data["id"] != id {
                return Err("persona mismatch".into());
            }
        }
        Ok("four persisted persona reads".into())
    }));
    cases.push(timed("jd_parse_fixed_local", 10_000, || {
        let data = ok(commands::parse_jd(
            "Senior Rust engineer, SQL, 5 years, bachelor degree".into(),
        ))?;
        if data["id"].is_null() {
            return Err("missing parsed id".into());
        }
        Ok("deterministic local parser; no network".into())
    }));
    cases.push(timed("resume_preview_fixed_local", 30_000, || {
        let before = rusqlite::Connection::open(&db_path)
            .map_err(|e| e.to_string())?
            .query_row("SELECT COUNT(*) FROM resume_versions", [], |r| {
                r.get::<_, u32>(0)
            })
            .map_err(|e| e.to_string())?;
        let data = ok(commands::preview_resume(Some(
            json!({"personaId":"p1","template":"modern"}),
        )))?;
        let after = rusqlite::Connection::open(&db_path)
            .map_err(|e| e.to_string())?
            .query_row("SELECT COUNT(*) FROM resume_versions", [], |r| {
                r.get::<_, u32>(0)
            })
            .map_err(|e| e.to_string())?;
        if data["markdown"].as_str().is_none_or(str::is_empty) || before != after {
            return Err("preview missing or wrote a version".into());
        }
        Ok("offline side-effect-free preview".into())
    }));
    cases.push(timed("offline_crud",500,||{ok(commands::save_experience(Some(json!({"newId":"offline","userId":"u","type":"project","title":"Offline","rawDescription":"No network"}))))?;ok(commands::delete_experience(Some(json!({"experienceId":"offline","version":1}))))?;Ok("create/delete without provider".into())}));
    cases.push(timed("wal_forced_crash_atomic_recovery",2000,||{let ready=root.join("wal-helper.ready");let _=fs::remove_file(&ready);let exe=std::env::current_exe().map_err(|e|e.to_string())?;let mut child=Command::new(exe).arg("--wal-helper").arg(&db_path).arg(&ready).spawn().map_err(|e|e.to_string())?;let deadline=Instant::now()+Duration::from_secs(2);while !ready.exists()&&Instant::now()<deadline{if child.try_wait().map_err(|e|e.to_string())?.is_some(){return Err("WAL helper exited before opening transaction".into())}thread::sleep(Duration::from_millis(10));}if !ready.exists(){let _=child.kill();return Err("WAL helper did not reach transaction-open stage".into())}child.kill().map_err(|e|e.to_string())?;let _=child.wait().map_err(|e|e.to_string())?;let db=Database::open_and_migrate(&db_path).map_err(|e|e.to_string())?;careercraft_lib::infra::db::integrity_check(db.connection()).map_err(|e|e.to_string())?;let counts:(u32,u32)=db.connection().query_row("SELECT SUM(id='wal-committed'),SUM(id IN ('wal-pending-a','wal-pending-b')) FROM experiences",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;let check:String=db.connection().query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;let _=fs::remove_file(&ready);if counts!=(1,0)||check!="ok"{return Err(format!("committed={}, pending={}, integrity={check}",counts.0,counts.1))}Ok("helper was force-killed with two-row transaction open; committed=1, pending=0, integrity=ok".into())}));
    let passed = cases.iter().all(|c| c.passed);
    let report = json!({"suite":"w7-performance-recovery","passed":passed,"profile":"debug-repeatable-thresholds","cases":cases});
    fs::write(
        root.join("w7-gate.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let failures = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["passed"] == false)
        .count();
    let mut junit = format!(
        "<testsuite name=\"w7-performance-recovery\" tests=\"{}\" failures=\"{}\">",
        report["cases"].as_array().unwrap().len(),
        failures
    );
    for case in report["cases"].as_array().unwrap() {
        junit.push_str(&format!(
            "<testcase name=\"{}\" time=\"{}\">",
            xml(case["name"].as_str().unwrap()),
            case["elapsed_ms"].as_u64().unwrap_or(0) as f64 / 1000.0
        ));
        if case["passed"] == false {
            junit.push_str(&format!(
                "<failure message=\"threshold exceeded or assertion failed\">{}</failure>",
                xml(&case.to_string())
            ))
        }
        junit.push_str("</testcase>")
    }
    junit.push_str("</testsuite>");
    fs::write(root.join("w7-gate.junit.xml"), junit).map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    let _ = fs::remove_file(&db_path);
    if passed {
        Ok(())
    } else {
        Err("W7 gate failed; see machine-readable reports".into())
    }
}
