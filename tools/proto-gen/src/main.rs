use std::path::{Path, PathBuf};
use std::process::Command;

const IB_REPO: &str = "git@github.com:InteractiveBrokers/tws-api.git";
const PROTO_PATH: &str = "source/proto";

fn fetch_proto_files(dest: &Path) {
    if dest.exists() {
        std::fs::remove_dir_all(dest).expect("failed to clean proto dir");
    }
    std::fs::create_dir_all(dest).expect("failed to create proto dir");

    let tmp = dest.parent().unwrap().join("tws-api-sparse");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).expect("failed to clean temp dir");
    }

    // Sparse clone — fetch only source/proto/
    run("git", &["clone", "--depth", "1", "--filter=blob:none", "--sparse", IB_REPO, tmp.to_str().unwrap()]);
    run("git", &["-C", tmp.to_str().unwrap(), "sparse-checkout", "set", PROTO_PATH]);

    let src = tmp.join(PROTO_PATH);
    let mut count = 0u32;
    for entry in std::fs::read_dir(&src).expect("failed to read cloned proto dir") {
        let path = entry.expect("bad entry").path();
        if path.extension().is_some_and(|ext| ext == "proto") {
            std::fs::copy(&path, dest.join(path.file_name().unwrap()))
                .unwrap_or_else(|e| panic!("failed to copy {}: {e}", path.display()));
            count += 1;
        }
    }

    std::fs::remove_dir_all(&tmp).ok();
    println!("Fetched {count} proto files from IB repo");
}

fn run(cmd: &str, args: &[&str]) {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cmd}: {e}"));
    if !status.success() {
        panic!("{cmd} failed with {status}");
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve project root");

    let proto_dir = root.join("target/proto");
    let out_dir = root.join("src/proto");

    fetch_proto_files(&proto_dir);

    std::fs::create_dir_all(&out_dir).expect("failed to create src/proto/");

    let protos: Vec<PathBuf> = std::fs::read_dir(&proto_dir)
        .expect("failed to read proto dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension().is_some_and(|ext| ext == "proto") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    println!("Compiling {} proto files...", protos.len());

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&protos, &[&proto_dir])
        .expect("failed to compile proto files");

    println!("Generated Rust code in {}", out_dir.display());
}
