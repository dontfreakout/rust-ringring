use rodio::{Decoder, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Play a sound file in a detached background process so the caller can exit
/// immediately. The child process lives only as long as the audio lasts.
pub fn play_sound(path: &Path, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
    // Fork: parent returns immediately, child plays the sound and exits.
    match unsafe { libc::fork() } {
        -1 => Err("fork failed".into()),
        0 => {
            // Child — detach from parent's process group so we survive parent exit.
            unsafe { libc::setsid() };
            let _ = play_blocking(path, volume);
            std::process::exit(0);
        }
        _child_pid => Ok(()), // Parent — returns immediately.
    }
}

/// Play a sound file and block until playback completes.
pub fn play_sound_blocking(path: &Path, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
    play_blocking(path, volume)
}

fn play_blocking(path: &Path, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let source = Decoder::try_from(reader)?;

    let stream = rodio::OutputStreamBuilder::open_default_stream()?;
    let sink = Sink::connect_new(&stream.mixer());
    sink.set_volume(volume.clamp(0.0, 1.0));
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
