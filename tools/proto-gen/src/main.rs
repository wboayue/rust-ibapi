use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve project root");

    let proto_dir = root.join("proto");
    let out_dir = root.join("src/proto");

    std::fs::create_dir_all(&out_dir).expect("failed to create src/proto/");

    let protos: Vec<PathBuf> = std::fs::read_dir(&proto_dir)
        .expect("failed to read proto/")
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
