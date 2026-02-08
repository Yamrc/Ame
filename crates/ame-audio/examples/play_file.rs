use std::env;
use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

use ame_audio::{AudioEngine, FileSource, Source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("Usage: play_file <path>");

    let mut engine = AudioEngine::new()?;
    let source = FileSource::new(&path)?;
    let total_duration = source.total_duration();

    println!("Playing: {}", path);
    if let Some(dur) = total_duration {
        println!("Duration: {}", format_duration(dur));
    }
    engine.play_file(source)?;

    println!("\nControls:");
    println!("  [Space]   - Pause/Resume");
    println!("  [←/a/h]   - Seek backward 5s");
    println!("  [→/d/l]   - Seek forward 5s");
    println!("  [q]       - Quit");
    println!();

    let mut paused = false;
    let mut last_line = String::new();

    loop {
        let pos = engine.current_position();
        let line = format_progress(pos, total_duration, paused);

        // 只在内容变化时更新，减少闪烁
        if line != last_line {
            print!("\r\x1B[K{}", line);
            io::stdout().flush()?;
            last_line = line;
        }

        // 非阻塞键盘输入
        if let Ok(input) = try_read_key() {
            match input {
                ' ' => {
                    if paused {
                        engine.resume();
                        paused = false;
                    } else {
                        engine.pause();
                        paused = true;
                    }
                    last_line.clear(); // 强制刷新
                }
                'a' | 'A' | 'h' | 'H' => {
                    let current = engine.current_position();
                    let target = current.saturating_sub(Duration::from_secs(5));
                    engine.seek_to(target)?;
                    last_line.clear();
                }
                'd' | 'D' | 'l' | 'L' => {
                    let current = engine.current_position();
                    let target = current + Duration::from_secs(5);
                    engine.seek_to(target)?;
                    last_line.clear();
                }
                'q' | 'Q' => {
                    println!("\n[Quit]");
                    break;
                }
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    engine.stop();
    Ok(())
}

fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

fn format_progress(pos: Duration, total: Option<Duration>, paused: bool) -> String {
    let pos_str = format_duration(pos);

    let (bar, _pct) = if let Some(total) = total {
        let progress = pos.as_millis() as f64 / total.as_millis() as f64;
        let pct = (progress * 100.0).min(100.0) as u8;
        let filled = (progress * 20.0).min(20.0) as usize;
        let bar = format!(
            "[{}{}] {:3}%",
            "=".repeat(filled),
            " ".repeat(20 - filled),
            pct
        );
        (bar, Some(pct))
    } else {
        ("".to_string(), None)
    };

    let icon = if paused { "⏸" } else { "▶" };

    if let Some(total) = total {
        let total_str = format_duration(total);
        format!("{} {} / {} {}", icon, pos_str, total_str, bar)
    } else {
        format!("{} {}", icon, pos_str)
    }
}

fn try_read_key() -> io::Result<char> {
    let mut buffer = [0u8; 1];
    match io::stdin().read_exact(&mut buffer) {
        Ok(_) => Ok(buffer[0] as char),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            Err(io::Error::new(io::ErrorKind::WouldBlock, "no input"))
        }
        Err(e) => Err(e),
    }
}
