use rodio::{Decoder, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Play a sound file to completion. Blocks until done.
/// Returns Ok(()) on success, Err on any failure.
pub fn play_sound(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let source = Decoder::try_from(reader)?;

    let stream = rodio::OutputStreamBuilder::open_default_stream()?;
    let sink = Sink::connect_new(&stream.mixer());
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
