fn main() {
    vcpkg::find_package("gdal").unwrap();
    println!("cargo:rustc-link-lib=shell32");
    println!("cargo:rustc-link-lib=secur32");
    println!("cargo:rustc-link-lib=dylib=shlwapi");
    println!("cargo:rustc-link-lib=Iphlpapi");
    println!("cargo:rustc-link-lib=bcrypt");
    println!("cargo:rustc-link-lib=ole32");
    // println!("cargo:rustc-link-lib=ssl");
    // println!("cargo:rustc-link-lib=crypto");
}
