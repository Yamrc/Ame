use std::env;
use std::thread;
use std::time::Duration;

use ame_audio::{AudioEngine, FileSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .expect("Usage: play_file <path>");

    let mut engine = AudioEngine::new()?;
    let source = FileSource::new(&path)?;

    println!("Playing: {}", path);
    engine.play(Box::new(source))?;

    // Wait for playback (simplified - real app would use proper sync)
    thread::sleep(Duration::from_secs(30));

    Ok(())
}
