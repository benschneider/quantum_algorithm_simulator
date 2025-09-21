use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("circuit_templates.rs");

    let mut templates_map = String::from("let mut templates = std::collections::HashMap::new();\n");

    let circuits_dir = "../quantsim_core/circuits";
    for entry in fs::read_dir(circuits_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let name = path.file_stem().unwrap().to_str().unwrap();
            let content = fs::read_to_string(&path).unwrap();
            templates_map.push_str(&format!(
                "templates.insert(\"{}.json\", r#\"{}\"#);\n",
                name, content
            ));
        }
    }

    fs::write(
        &dest_path,
        format!(
            "fn load_templates() -> std::collections::HashMap<&'static str, &'static str> {{
                {}
                templates
            }}",
            templates_map
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", circuits_dir);
}
