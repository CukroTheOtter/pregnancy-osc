//! belly-cli: send a belly size to VRChat over OSC to prove the loop works.
//!
//! Examples:
//!   belly-cli --stage 0.5          set the belly to half size
//!   belly-cli --sweep 60           grow from 0 to full over 60 seconds
//!   belly-cli --query              show the avatar's parameters (OSCQuery)
//!   belly-cli --listen 10          print what VRChat sends for 10 seconds

use clap::Parser;
use pregsys_core::mapping::Mapping;
use pregsys_osc::{Bridge, BridgeConfig, OscValue};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(name = "belly-cli", about = "Send a belly size to VRChat over OSC")]
struct Args {
    /// Belly size to send, 0 to 1.
    #[arg(long)]
    stage: Option<f32>,
    /// Grow the belly from 0 to 1 over this many seconds.
    #[arg(long)]
    sweep: Option<u64>,
    /// Hold the "app active" flag and heartbeat while running.
    #[arg(long)]
    app_active: bool,
    /// List the current avatar's parameters via OSCQuery and exit.
    #[arg(long)]
    query: bool,
    /// Print incoming OSC for this many seconds.
    #[arg(long)]
    listen: Option<u64>,
    /// Use a different mapping file (default: the bundled one).
    #[arg(long)]
    mapping: Option<PathBuf>,
    /// Send to this host when OSCQuery can't find VRChat.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    /// Send to this port when OSCQuery can't find VRChat.
    #[arg(long, default_value_t = 9000)]
    port: u16,
    /// Skip OSCQuery and use plain ports only.
    #[arg(long)]
    no_oscquery: bool,
}

fn main() {
    let args = Args::parse();
    let mapping = match &args.mapping {
        Some(p) => Mapping::load(p).unwrap_or_else(|e| fail(&e.to_string())),
        None => Mapping::default(),
    };
    let stage_addr = mapping.address("stage").unwrap_or_else(|| fail("mapping has no \"stage\" parameter name"));

    let mut cfg = BridgeConfig {
        use_oscquery: !args.no_oscquery,
        send_host: args.host.clone(),
        send_port: args.port,
        // Only claim VRChat's output port when we actually listen.
        recv_port: if args.listen.is_some() { 9001 } else { 0 },
        ..Default::default()
    };
    cfg.listen_addresses.extend(mapping.parameters.values().filter(|p| !p.name.is_empty()).map(|p| format!("/avatar/parameters/{}", p.name)));
    let bridge = Bridge::start(&cfg).unwrap_or_else(|e| fail(&e));

    // Give mDNS a moment to find VRChat (fixed ports still use it to read the avatar).
    let t = Instant::now();
    while !bridge.status().vrchat_found && t.elapsed() < Duration::from_secs(3) {
        sleep(Duration::from_millis(100));
    }
    bridge.refresh_target();
    let st = bridge.status();
    println!(
        "OSCQuery: {} | VRChat found: {} | sending to {} | listening on {}",
        if st.oscquery { "on" } else { "off" },
        if st.vrchat_found { "yes" } else { "no (using fallback)" },
        st.send_target,
        st.recv_port
    );
    if let Some(n) = &st.note {
        println!("{n}");
    }

    if args.query {
        match bridge.avatar_info() {
            Some(info) => {
                println!("Avatar: {}", info.avatar_id.as_deref().unwrap_or("(unknown)"));
                let params = info.params;
                println!("{} parameters on the current avatar:", params.len());
                let mut params = params;
                params.sort_by(|a, b| a.name.cmp(&b.name));
                for p in params {
                    let key = mapping.key_for_name(&p.name).map(|k| format!("  <- {k}")).unwrap_or_default();
                    println!("  {} = {:?}{key}", p.name, p.value);
                }
            }
            None => println!("Could not read avatar parameters. Is VRChat running with OSC enabled?"),
        }
        return;
    }

    let heartbeat = |beat: bool| {
        if args.app_active {
            if let Some(a) = mapping.address("appActive") {
                bridge.send(&a, OscValue::Bool(true));
            }
            if let Some(a) = mapping.address("appHeartbeat") {
                bridge.send(&a, OscValue::Bool(beat));
            }
        }
    };

    if let Some(s) = args.stage {
        let s = s.clamp(0.0, 1.0);
        bridge.send(&stage_addr, OscValue::Float(s));
        heartbeat(true);
        println!("Sent {stage_addr} = {s}");
    }

    if let Some(secs) = args.sweep {
        let total = Duration::from_secs(secs.max(1));
        let start = Instant::now();
        let mut beat = false;
        while start.elapsed() <= total {
            let v = (start.elapsed().as_secs_f32() / total.as_secs_f32()).min(1.0);
            bridge.send(&stage_addr, OscValue::Float(v));
            beat = !beat;
            heartbeat(beat);
            print!("\rStage {:>5.1}%", v * 100.0);
            use std::io::Write;
            let _ = std::io::stdout().flush();
            sleep(Duration::from_millis(250));
        }
        bridge.send(&stage_addr, OscValue::Float(1.0));
        println!("\rStage 100.0% - done");
    }

    if let Some(secs) = args.listen {
        println!("Listening for {secs} s...");
        let end = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < end {
            for m in bridge.drain().unwrap_or_default() {
                println!("  {} = {:?}", m.addr, m.value);
            }
            sleep(Duration::from_millis(100));
        }
    }

    if args.app_active {
        if let Some(a) = mapping.address("appActive") {
            bridge.send(&a, OscValue::Bool(false));
            println!("Released the app-active flag.");
        }
    }

    if args.stage.is_none() && args.sweep.is_none() && args.listen.is_none() {
        println!("Nothing to do. Try --stage 0.5, --sweep 60, --query or --listen 10 (see --help).");
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1)
}
