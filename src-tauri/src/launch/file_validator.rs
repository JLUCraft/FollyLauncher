use std::path::Path;
use tokio::fs;
use zip::write::SimpleFileOptions;

pub async fn save_crash_report(
    instance_id: &str,
    game_dir: &Path,
    save_path: &Path,
) -> anyhow::Result<()> {
    let log_path = game_dir.join("logs").join("latest.log");
    let crash_dir = game_dir.join("crash-reports");

    let file = std::fs::File::create(save_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    if fs::try_exists(&log_path).await? {
        let content = fs::read_to_string(&log_path).await?;
        zip.start_file(format!("{}/latest.log", instance_id), options)?;
        use std::io::Write;
        zip.write_all(content.as_bytes())?;
    }

    if fs::try_exists(&crash_dir).await? {
        if let Ok(mut entries) = fs::read_dir(&crash_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "txt") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Some(name) = path.file_name() {
                            zip.start_file(
                                format!("{}/{}", instance_id, name.to_string_lossy()),
                                options,
                            )?;
                            use std::io::Write;
                            zip.write_all(content.as_bytes())?;
                        }
                    }
                }
            }
        }
    }

    zip.finish()?;
    Ok(())
}
