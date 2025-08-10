// build.rs 
use std::path::PathBuf;

fn main() {
    // Adjust path if  Cargo crate is not adjacent to backend/SPFresh
     let dst = cmake::Config::new("backend/SPFresh")
        .profile("Release")
        .build();
     // CMake places built libs under {dst}/lib by default
    let lib_dir = PathBuf::from(&dst).join("lib");
    // ให้ cargo หา compiled libraryและเชื่อมต่อ
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println-lib=static=spfresh!("cargo:rustc-link");

    // println!("cargo:rustc-link-lib=static=AnnService");

    // รัน build.rs อีกครั้งถ้า wrapper headers หรือ sources มีการเปลียน
    println!("cargo:rerun-if-changed=backend/SPFresh/src/spfresh_c_api.cpp");
    println!("cargo:rerun-if-changed=backend/SPFresh/include/spfresh_c_api.h");
    println!("cargo:rerun-if-changed=backend/SPFresh/include/spfresh/index.hpp");
}

