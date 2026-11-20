//! Best-effort account telemetry, independent of the worker pool.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

fn weekly(result: &Value) -> Option<i64> {
    let limits = result["rateLimitsByLimitId"]
        .get("codex")
        .unwrap_or(&result["rateLimits"]);
    ["secondary", "primary"].iter().find_map(|k| {
        let window = &limits[*k];
        (window["windowDurationMins"].as_i64() == Some(7 * 24 * 60))
            .then(|| window["usedPercent"].as_i64())
            .flatten()
            .filter(|p| (0..=100).contains(p))
            .map(|p| 100 - p)
    })
}

fn read() -> Option<i64> {
    // Bound startup, reads and shutdown even if an older CLI never responds.
    let mut cmd = Command::new("timeout");
    cmd.args(["-k", "2", "15", "codex", "app-server"]);
    crate::bind_worker_lifetime(&mut cmd);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let result = (|| {
        let mut input = child.stdin.take()?;
        let lines = BufReader::new(child.stdout.take()?).lines();
        writeln!(input,"{}",json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"haqor-review","version":env!("CARGO_PKG_VERSION")}}})).ok()?;
        for line in lines {
            let event: Value = serde_json::from_str(&line.ok()?).ok()?;
            match event["id"].as_i64() {
                Some(1) if event.get("result").is_some() => {
                    writeln!(input, "{}", json!({"method":"initialized"})).ok()?;
                    writeln!(
                        input,
                        "{}",
                        json!({"id":2,"method":"account/rateLimits/read","params":{}})
                    )
                    .ok()?;
                }
                Some(2) => return weekly(&event["result"]),
                Some(1) => break,
                _ => (),
            }
        }
        None
    })();
    let _ = child.wait();
    result
}

pub fn monitor() -> mpsc::Sender<()> {
    let (stop, stopped) = mpsc::channel();
    thread::spawn(move || {
        loop {
            match read() {
                Some(left) => eprintln!("[Codex] Weekly usage remaining: {left}%"),
                None => eprintln!("[Codex] Weekly usage unavailable"),
            }
            if stopped.recv_timeout(Duration::from_secs(300))
                != Err(mpsc::RecvTimeoutError::Timeout)
            {
                break;
            }
        }
    });
    stop
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_only_a_weekly_window() {
        let result = json!({"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":3,"windowDurationMins":300},"secondary":{"usedPercent":41,"windowDurationMins":10080}}}});
        assert_eq!(weekly(&result), Some(59));
        assert_eq!(
            weekly(&json!({"rateLimits":{"secondary":{"usedPercent":1}}})),
            None
        );
    }
}
