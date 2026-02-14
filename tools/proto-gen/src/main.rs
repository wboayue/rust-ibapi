use std::path::{Path, PathBuf};
use std::process::Command;

// Private repo — requires IB partner access and SSH key configured for GitHub.
// For HTTPS, change to "https://github.com/InteractiveBrokers/tws-api.git"
// and ensure credentials are available (e.g. via GH_TOKEN or credential helper).
const IB_REPO: &str = "git@github.com:InteractiveBrokers/tws-api.git";
const PROTO_PATH: &str = "source/proto";

fn is_proto(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "proto")
}

fn fetch_proto_files(dest: &Path) {
    if dest.exists() {
        std::fs::remove_dir_all(dest).expect("failed to clean proto dir");
    }
    std::fs::create_dir_all(dest).expect("failed to create proto dir");

    let tmp = dest.parent().expect("dest has no parent").join("tws-api-sparse");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).expect("failed to clean temp dir");
    }

    let tmp_str = tmp.to_str().expect("non-UTF-8 path");

    // Sparse clone — fetch only source/proto/
    run("git", &["clone", "--depth", "1", "--filter=blob:none", "--sparse", IB_REPO, tmp_str]);
    run("git", &["-C", tmp_str, "sparse-checkout", "set", PROTO_PATH]);

    // Flat scan — assumes all .proto files live directly under source/proto/
    let src = tmp.join(PROTO_PATH);
    let mut count = 0u32;
    for entry in std::fs::read_dir(&src).expect("failed to read cloned proto dir") {
        let path = entry.expect("bad entry").path();
        if is_proto(&path) {
            std::fs::copy(&path, dest.join(path.file_name().unwrap())).unwrap_or_else(|e| panic!("failed to copy {}: {e}", path.display()));
            count += 1;
        }
    }

    std::fs::remove_dir_all(&tmp).ok();
    println!("Fetched {count} proto files from IB repo");
}

fn run(cmd: &str, args: &[&str]) {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {cmd}: {e}"));
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("{cmd} failed with {}:\n{stderr}", output.status);
    }
}

fn workspace_root() -> PathBuf {
    let output = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .expect("failed to run cargo locate-project");
    assert!(output.status.success(), "cargo locate-project failed");
    let cargo_toml = String::from_utf8(output.stdout).expect("non-UTF-8 path");
    PathBuf::from(cargo_toml.trim()).parent().expect("Cargo.toml has no parent").to_path_buf()
}

fn main() {
    let root = workspace_root();

    let proto_dir = root.join("target/proto");
    let out_dir = root.join("src/proto");

    fetch_proto_files(&proto_dir);

    std::fs::create_dir_all(&out_dir).expect("failed to create src/proto/");

    let protos: Vec<PathBuf> = std::fs::read_dir(&proto_dir)
        .expect("failed to read proto dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            is_proto(&path).then_some(path)
        })
        .collect();

    println!("Compiling {} proto files...", protos.len());

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&protos, &[&proto_dir])
        .expect("failed to compile proto files");

    println!("Generated Rust code in {}", out_dir.display());
}
