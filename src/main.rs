use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};
use chrono::Local;
use flexi_logger::Logger;
use log::{info, warn, error};

fn is_file_in_use(file_path: &Path) -> bool {
    OpenOptions::new().write(true).open(file_path).is_err()
}

fn compress_log_file(log_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let log_file_name = log_path.file_name().unwrap().to_str().unwrap();

    if is_file_in_use(log_path) {
        warn!("File '{}' is currently using, skipped", log_file_name);
        return Ok(());
    }

    info!("Compressing file: {}", log_file_name);

    let current_time = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let zip_file_name = format!("{}_{}.zip", current_time, log_file_name);
    let zip_path = log_path.with_file_name(zip_file_name);

    let zip_file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);

    zip.start_file(log_file_name, options)?;

    let mut f = File::open(log_path)?;
    std::io::copy(&mut f, &mut zip)?;

    zip.finish()?;
    info!("Compressed '{}'", zip_path.file_name().unwrap().to_str().unwrap());

    match std::fs::remove_file(log_path) {
        Ok(_) => info!("Removed '{}'", log_file_name),
        Err(e) => error!("Failed to remove '{}': {}", log_file_name, e),
    }

    Ok(())
}

fn main() {

    Logger::try_with_str("info")
        .unwrap()
        .start()
        .expect("Failed to init Logger");

    info!("===== Start =====");

    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Failed to get current dir: {}", e);
            return;
        }
    };

    info!("Finding .log in current dir: {:?}", current_dir);

    for entry in WalkDir::new(&current_dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "log" && !path.file_name().unwrap().to_str().unwrap().contains("log_compressor") {
                    if let Err(e) = compress_log_file(&path.to_path_buf()) {
                        error!("Failed to process file '{}': {}", path.display(), e);
                    }
                }
            }
        }
    }

    info!("===== Finish =====");
}