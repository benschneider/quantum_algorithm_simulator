fn main() {
    // Re-embed the template catalog when a circuit is added, edited, or removed.
    println!("cargo:rerun-if-changed=circuits");
}
