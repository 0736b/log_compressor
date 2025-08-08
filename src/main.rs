use chrono::Local;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};

const BUFFER_SIZE: usize = 1048576;     // 1MB

fn is_file_in_use(file_path: &Path) -> bool {
    OpenOptions::new().write(true).open(file_path).is_err()
}

fn compress_log_file(
    log_path: &PathBuf,
    mp: &MultiProgress,
) -> Result<(), Box<dyn std::error::Error>> {
    let log_file_name = log_path.file_name().unwrap().to_str().unwrap();

    if is_file_in_use(log_path) {
        mp.println(format!(
            "File '{}' is currently in use, skipped",
            log_file_name
        ))?;
        return Ok(());
    }

    let file_size = std::fs::metadata(log_path)?.len();

    let pb = mp.add(ProgressBar::new(file_size));
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")?
        .progress_chars("#>-"));
    pb.set_message(log_file_name.to_string());

    let current_time = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let zip_file_name = format!("{}_{}.zip", current_time, log_file_name);
    let zip_path = log_path.with_file_name(zip_file_name);

    let zip_file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);

    zip.start_file(log_file_name, options)?;

    let mut source_file = File::open(log_path)?;

    let mut buffer = vec![0; BUFFER_SIZE];
    loop {
        let bytes_read = source_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        zip.write_all(&buffer[..bytes_read])?;
        pb.inc(bytes_read as u64);
    }

    zip.finish()?;

    pb.finish_with_message(format!("Compressed: {}", log_file_name));

    match std::fs::remove_file(log_path) {
        Ok(_) => {}
        Err(e) => mp.println(format!("Failed to remove '{}': {}", log_file_name, e))?,
    }

    Ok(())
}

fn main() {
    println!("===== Start =====");

    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current dir: {}", e);
            return;
        }
    };

    let m = MultiProgress::new();

    println!("Finding .log in current dir: {:?}", current_dir);

    let log_files: Vec<_> = WalkDir::new(&current_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.is_file()
                && path.extension().map_or(false, |ext| ext == "log")
                && !path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("log_compressor")
        })
        .collect();

    println!("Found {} log files to process.", log_files.len());

    log_files.into_par_iter().for_each(|path| {
        if let Err(e) = compress_log_file(&path, &m) {
            m.println(format!(
                "Failed to process file '{}': {}",
                path.display(),
                e
            ))
            .unwrap();
        }
    });

    m.clear().unwrap();

    println!("===== Finish =====");
}
