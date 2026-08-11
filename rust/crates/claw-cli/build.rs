use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Get git SHA (short hash)
    let git_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());

    println!("cargo:rustc-env=GIT_SHA={git_sha}");

    // TARGET is always set by Cargo during build
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=TARGET={target}");

    // Build date from SOURCE_DATE_EPOCH (reproducible builds) or current UTC date.
    // Intentionally ignoring time component to keep output deterministic within a day.
    let build_date = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|epoch| epoch.parse::<i64>().ok())
        .map(|_ts| {
            // Use SOURCE_DATE_EPOCH to derive date via chrono if available;
            // for simplicity we just use the env var as a signal and fall back
            // to build-time env. In practice CI sets this via workflow.
            std::env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string())
        })
        .or_else(|| std::env::var("BUILD_DATE").ok())
        .unwrap_or_else(|| {
            // Fall back to current date via `date` command
            Command::new("date")
                .args(["+%Y-%m-%d"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
        });
    println!("cargo:rustc-env=BUILD_DATE={build_date}");

    // Rerun if git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    // ========================================================================
    // App icon embedding
    // ========================================================================
    match ensure_app_icon() {
        Ok(ico_path) => {
            // Re-embed whenever the icon file or the output path changes.
            println!("cargo:rerun-if-changed=assets/icons/clawcode.ico");

            // Only embed on Windows targets; the embed_resource crate is a
            // no-op elsewhere but we still avoid the work.
            if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
                // embed-resource 2.x: `compile` expects a `.rc` (resource
                // script) path, not the raw `.ico`. Generate a minimal RC
                // that references the .ico by relative filename, then hand
                // that to the resource compiler.
                match write_icon_rc(&ico_path) {
                    Ok(rc_path) => {
                        // embed-resource 2.x: `compile` returns `()` and
                        // panics on internal failure (e.g. windres/RC.EXE
                        // unavailable). We let any panic surface in the
                        // build output rather than swallow it.
                        embed_resource::compile(&rc_path, embed_resource::NONE);
                    }
                    Err(e) => println!("cargo:warning=failed to write .rc: {e}"),
                }
            }
        }
        Err(reason) => {
            println!("cargo:warning=app icon not embedded: {reason}");
        }
    }
}

/// Writes a minimal Windows resource script (`app-icon.rc`) that points
/// at the multi-resolution `clawcode.ico`. The RC compiler (`RC.EXE`
/// or `windres`) takes the `.rc` and emits a linkable `.res` for the
/// linker to consume.
fn write_icon_rc(ico_path: &Path) -> Result<PathBuf, String> {
    let rc_path = ico_path.with_extension("rc");
    let ico_name = ico_path
        .file_name()
        .ok_or_else(|| "icon path has no filename".to_string())?;
    // The RC compiler resolves the icon path relative to the current
    // working directory at compile time. We chdir to the icon directory
    // implicitly by writing the RC there and using just the filename.
    let body = format!("1 ICON \"{}\"\n", ico_name.to_string_lossy());
    fs::write(&rc_path, body).map_err(|e| format!("write rc: {e}"))?;
    Ok(rc_path)
}

/// Ensures a multi-resolution `clawcode.ico` exists in the build output
/// directory. Tries to rasterize the SVG via the first available of
/// `magick`, `resvg`, or `rsvg-convert`. Falls back to the pre-committed
/// `assets/icons/clawcode.ico` if the rasterizer is missing or fails.
fn ensure_app_icon() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let svg = manifest_dir.join("assets").join("icon.svg");
    let fallback_ico = manifest_dir
        .join("assets")
        .join("icons")
        .join("clawcode.ico");

    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|e| e.to_string())?);
    let build_dir = out_dir.join("icon");
    fs::create_dir_all(&build_dir).map_err(|e| e.to_string())?;
    let ico = build_dir.join("clawcode.ico");

    if svg.exists() && try_rasterize_svg(&svg, &build_dir).is_ok() {
        return Ok(ico);
    }
    if fallback_ico.exists() {
        fs::copy(&fallback_ico, &ico).map_err(|e| format!("copy fallback: {e}"))?;
        return Ok(ico);
    }
    Err("no rasterizer and no fallback icon found".to_string())
}

/// Tries to rasterize `svg` to a multi-resolution `.ico` at
/// `out_dir/clawcode.ico`. Returns Ok(()) on success.
fn try_rasterize_svg(svg: &Path, out_dir: &Path) -> Result<(), String> {
    let rasterizers = ["magick", "resvg", "rsvg-convert"];
    let tool = *rasterizers
        .iter()
        .find(|name| which_on_path(name).is_some())
        .ok_or_else(|| {
            "no SVG rasterizer on PATH (tried magick, resvg, rsvg-convert)".to_string()
        })?;

    let status = match tool {
        "magick" => {
            let mut cmd = Command::new(tool);
            cmd.current_dir(out_dir)
                .arg(svg)
                .arg("-define")
                .arg("icon:auto-resize=16,32,48,64,128,256")
                .arg("clawcode.ico");
            cmd.status().map_err(|e| e.to_string())?
        }
        "resvg" => {
            // resvg only emits a single PNG; we deliberately do NOT try
            // to bundle it into a multi-frame .ico here. The function
            // returns Err, and ensure_app_icon falls back to the
            // pre-committed clawcode.ico. The PNG bytes are discarded.
            let png = out_dir.join("clawcode.png");
            let mut cmd = Command::new(tool);
            cmd.current_dir(out_dir)
                .arg(svg)
                .arg(&png)
                .arg("-w")
                .arg("256")
                .arg("-h")
                .arg("256");
            cmd.status().map_err(|e| e.to_string())?
        }
        "rsvg-convert" => {
            // Same as resvg above: emits a single PNG; the caller falls
            // back to the pre-committed .ico.
            let png = out_dir.join("clawcode.png");
            let mut cmd = Command::new(tool);
            cmd.current_dir(out_dir)
                .arg(svg)
                .arg("-w")
                .arg("256")
                .arg("-h")
                .arg("256")
                .arg("-o")
                .arg(&png);
            cmd.status().map_err(|e| e.to_string())?
        }
        _ => unreachable!(),
    };

    if !status.success() {
        return Err(format!("{tool} exited with {status}"));
    }

    let ico = out_dir.join("clawcode.ico");
    if ico.exists() {
        return Ok(());
    }
    Err(format!("{tool} did not produce clawcode.ico"))
}

fn which_on_path(cmd: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) {
        &["", ".exe", ".bat", ".cmd"]
    } else {
        &[""]
    };
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        for ext in exts {
            let candidate = dir.join(format!("{cmd}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
