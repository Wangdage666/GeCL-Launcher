use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" {
        println!(
            "gecl-agent.exe commands\ngecl-agent.exe invoke <plugin:namespace|command> '<JSON>'\ngecl-agent.exe invoke <command> @args.json\ngecl-agent.exe ui.navigate <route>\ngecl-agent.exe ui.snapshot\ngecl-agent.exe ui.click <CSS selector>\ngecl-agent.exe ui.fill <CSS selector> <value>\ngecl-agent.exe status\nSet THESEUS_CONFIG_DIR to target an isolated configuration; GECL_AGENT_TIMEOUT_SECONDS defaults to 600."
        );
        return Ok(());
    }
    let root = env::var_os("THESEUS_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("APPDATA").unwrap_or_default())
                .join("top.wangdage.gecl.modrinth")
        })
        .join("agent");
    let ready = fs::read_to_string(root.join("ready"))
        .map_err(|_| "请先启动 GeCL")?;
    let mut lines = ready.lines();
    let _pid = lines.next().ok_or("调试状态无效")?;
    let session = lines.next().ok_or("调试状态无效")?;
    let timestamp: u64 = lines.next().ok_or("调试状态无效")?.parse()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if now.abs_diff(timestamp) > 5 {
        return Err("GeCL 未就绪或已关闭".into());
    }
    let mut payload = "{}".to_string();
    let action = &args[0];
    match action.as_str() {
        "invoke" => {
            let command = args.get(1).ok_or("缺少接口名称")?;
            let input = args.get(2).map(String::as_str).unwrap_or("{}");
            let arguments = if input == "-" {
                let mut value = String::new();
                io::stdin().read_to_string(&mut value)?;
                value
            } else if let Some(path) = input.strip_prefix('@') {
                fs::read_to_string(path)?
            } else {
                input.to_owned()
            };
            let arguments = arguments.trim_start_matches('\u{feff}');
            payload = format!(
                "{{\"command\":{},\"args\":{arguments}}}",
                quote(command)
            );
        }
        "ui.navigate" | "ui.click" => {
            payload = format!(
                "{{\"value\":{}}}",
                quote(args.get(1).ok_or("缺少页面路径或选择器")?)
            )
        }
        "ui.fill" => {
            payload = format!(
                "{{\"selector\":{},\"value\":{}}}",
                quote(args.get(1).ok_or("缺少选择器")?),
                quote(args.get(2).ok_or("缺少输入内容")?)
            )
        }
        "commands" | "ui.snapshot" | "status" => (),
        _ => return Err("未知命令，请运行 --help".into()),
    }
    let serial = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let id = format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        std::process::id(),
        (serial >> 48) & 0xffff,
        (serial >> 36) & 0xfff,
        (serial >> 24) & 0xfff,
        serial & 0xffffffffffff
    );
    let request = format!(
        "{{\"session\":{},\"action\":{},\"payload\":{payload}}}",
        quote(session),
        quote(action)
    );
    if request.len() > 1024 * 1024 {
        return Err("命令超过 1 MB，请缩小参数".into());
    }
    let pending = root.join(format!("{id}.pending"));
    let path = root.join(format!("{id}.request.json"));
    {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&pending)?;
        file.write_all(request.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&pending, &path)?;
    let response = root.join(format!("{id}.response.json"));
    let deadline = Instant::now()
        + Duration::from_secs(
            env::var("GECL_AGENT_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(600)
                .clamp(1, 3600),
        );
    loop {
        if response.is_file() {
            let value = fs::read_to_string(&response)?;
            fs::remove_file(response)?;
            println!("{value}");
            if value.starts_with("{\"error\":") {
                std::process::exit(1);
            }
            return Ok(());
        }
        if Instant::now() > deadline {
            let queued = fs::remove_file(&path).is_ok();
            return Err(if queued {
                "命令等待超时，尚未执行"
            } else {
                "命令返回超时，操作可能仍在继续；查询任务状态后再操作"
            }
            .into());
        }
        thread::sleep(Duration::from_millis(30));
    }
}
fn main() {
    if let Err(error) = run() {
        println!("{{\"ok\":false,\"error\":{}}}", quote(&error.to_string()));
        std::process::exit(1);
    }
}
