use std::path::PathBuf;
use std::{fs, io};

fn config_home() -> PathBuf {
    if let Some(custom) = std::env::var_os("CLAW_CONFIG_HOME") {
        return PathBuf::from(custom);
    }
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    };
    home.join(".claw")
}

fn main() -> io::Result<()> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let diffs_root = config_home().join("diffs");

    if !diffs_root.exists() {
        eprintln!("No diffs directory found at {}", diffs_root.display());
        return Ok(());
    }

    let mut total = 0u64;

    let entries: Vec<_> = fs::read_dir(&diffs_root)?
        .filter_map(|e| e.ok())
        .collect();

    for entry in &entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = path.file_name().unwrap().to_string_lossy();
        if !dir_name.starts_with('d') || dir_name.len() != 9 {
            continue;
        }
        let date_part = &dir_name[1..];

        let patch_files: Vec<_> = fs::read_dir(&path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "patch").unwrap_or(false))
            .collect();

        for patch in &patch_files {
            let old_name = patch.file_name().to_string_lossy().to_string();
            if !old_name.starts_with("diff_") {
                continue;
            }
            let suffix = old_name.strip_prefix("diff_").unwrap();
            let new_name = format!("{date_part}{suffix}");
            let new_path = diffs_root.join(&new_name);

            if dry_run {
                println!("[DRY-RUN] {} -> {}", patch.path().display(), new_path.display());
            } else {
                fs::rename(patch.path(), &new_path)?;
                println!("  Renamed: {} -> {}", old_name, new_name);
            }
            total += 1;
        }

        if !dry_run {
            let mut remaining = fs::read_dir(&path)?;
            if remaining.next().is_none() {
                fs::remove_dir(&path)?;
                println!("  Removed empty directory: {}", path.display());
            }
        }
    }

    if dry_run {
        println!("\n[Dry-run] {total} files would be migrated.");
    } else {
        println!("\nMigration complete: {total} files renamed.");
    }

    Ok(())
}
