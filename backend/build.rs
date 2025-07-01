fn main() {
    // This tells cargo to re-run this build script if the migrations change
    println!("cargo:rerun-if-changed=migrations");
}