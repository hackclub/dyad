use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/static");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");
    println!("cargo:rerun-if-changed=ui/package.json");

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("ui")
        .status()
        .expect("failed to run `npm run build` in ui/");

    if !status.success() {
        panic!("`npm run build` failed");
    }
}
