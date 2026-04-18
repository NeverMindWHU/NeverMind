use clap::{Parser, Subcommand};
use harness_core::{
    current_foreground, default_db_path, DesktopEvent, RecordingState, TimelineStore,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "harness", version, about = "Schedule desktop harness — observe & timeline (debug)")]
struct Cli {
    /// SQLite database path (default: %LOCALAPPDATA%/ScheduleHarness/harness.db)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Observe {
        #[command(subcommand)]
        cmd: ObserveCmd,
    },
    Timeline {
        #[command(subcommand)]
        cmd: TimelineCmd,
    },
    Analyze {
        #[command(subcommand)]
        cmd: AnalyzeCmd,
    },
}

#[derive(Subcommand)]
enum ObserveCmd {
    /// Print current foreground app/title once
    Status,
    /// Poll foreground + clipboard and append events when changed (Ctrl+C to stop)
    Run {
        #[arg(long, default_value_t = 2u64)]
        interval_secs: u64,
    },
}

#[derive(Subcommand)]
enum TimelineCmd {
    /// List events recorded for the local calendar day
    Today {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum AnalyzeCmd {
    /// Rolling stats for the last N minutes (app_switch + clipboard + dominant apps)
    Window {
        #[arg(long, default_value_t = 15)]
        minutes: i64,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db = cli.db.unwrap_or_else(default_db_path);

    match cli.command {
        Commands::Observe { cmd } => match cmd {
            ObserveCmd::Status => {
                let s = current_foreground()?;
                println!("app:  {}", s.app);
                println!("title: {}", s.title.as_deref().unwrap_or(""));
            }
            ObserveCmd::Run { interval_secs } => {
                let store = TimelineStore::open(&db)?;
                let running = Arc::new(AtomicBool::new(true));
                let r = running.clone();
                let _ = ctrlc::set_handler(move || {
                    r.store(false, Ordering::SeqCst);
                });
                let mut state = RecordingState::new();
                println!(
                    "Recording to {} (foreground + clipboard, Ctrl+C to stop)",
                    db.display()
                );
                while running.load(Ordering::SeqCst) {
                    state.poll_once(&store)?;
                    std::thread::sleep(Duration::from_secs(interval_secs));
                }
            }
        },
        Commands::Timeline { cmd } => match cmd {
            TimelineCmd::Today { json } => {
                let store = TimelineStore::open(&db)?;
                let events = store.events_today_local()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&events)?);
                } else if events.is_empty() {
                    println!("(no events today)");
                } else {
                    println!("{}\t{}\t{}", "time(UTC)", "kind", "detail");
                    for ev in events {
                        match ev {
                            DesktopEvent::AppSwitch {
                                app,
                                title,
                                image_rel,
                                at,
                                ..
                            } => {
                                let mut line = format!(
                                    "{}\tapp_switch\t{} | {}",
                                    at.format("%H:%M:%S"),
                                    app,
                                    title.as_deref().unwrap_or("")
                                );
                                if let Some(rel) = image_rel {
                                    line.push_str(&format!(" | shot: {}", rel));
                                }
                                println!("{}", line);
                            }
                            DesktopEvent::Clipboard {
                                text_preview,
                                char_len,
                                truncated,
                                at,
                            } => {
                                let tail = if truncated { " …" } else { "" };
                                println!(
                                    "{}\tclipboard\t{} chars | {}{tail}",
                                    at.format("%H:%M:%S"),
                                    char_len,
                                    text_preview.replace('\n', " ")
                                );
                            }
                            DesktopEvent::LearningSnapshot {
                                image_rel,
                                app,
                                title,
                                width_px,
                                height_px,
                                at,
                            } => {
                                println!(
                                    "{}\tlearning_snapshot\t{} | {}×{} | {} | {}",
                                    at.format("%H:%M:%S"),
                                    app,
                                    width_px,
                                    height_px,
                                    image_rel,
                                    title.as_deref().unwrap_or("")
                                );
                            }
                            DesktopEvent::BrowserMoment {
                                trigger,
                                browser_app,
                                page_title,
                                learning_content,
                                summary,
                                at,
                                ..
                            } => {
                                println!(
                                    "{}\tbrowser_moment\t{} | {} | {} | {} | {}",
                                    at.format("%H:%M:%S"),
                                    trigger,
                                    browser_app,
                                    learning_content,
                                    page_title,
                                    summary
                                );
                            }
                        }
                    }
                }
            }
        },
        Commands::Analyze { cmd } => match cmd {
            AnalyzeCmd::Window { minutes, json } => {
                if !(1..=24 * 60).contains(&minutes) {
                    return Err("minutes must be in 1..1440".into());
                }
                let store = TimelineStore::open(&db)?;
                let stats = store.analyze_window_minutes(minutes)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                } else {
                    println!("window: last {minutes} minutes (UTC)");
                    println!(
                        "interval: {} .. {}",
                        stats.interval_start_utc, stats.interval_end_utc
                    );
                    println!("app_switch events: {}", stats.app_switch_count);
                    println!("clipboard events: {}", stats.clipboard_event_count);
                    println!("dominant apps (app_switch):");
                    for a in &stats.dominant_apps {
                        println!("  {} × {}", a.app, a.count);
                    }
                }
            }
        },
    }
    Ok(())
}
