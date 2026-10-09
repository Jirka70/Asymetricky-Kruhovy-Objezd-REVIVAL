fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=../openapi.yaml");
    println!("cargo:rerun-if-changed=../swagger.html");
}
