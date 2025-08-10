// build.rs 
use cmake::Config;

fn main() {
    // Adjust path if your Cargo crate is not adjacent to backend/SPFresh
    let dst = Config::new("backend/SPFresh")
        .profile("Release")
        .build();


    // ให้ cargo หา compiled libraryและเชื่อมต่อ
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=spfresh");

    // println!("cargo:rustc-link-lib=static=AnnService");

    // รัน build.rs อีกครั้งถ้า wrapper headers หรือ sources มีการเปลียน
    println!("cargo:rerun-if-changed=backend/SPFresh/src/spfresh_c_api.cpp");
    println!("cargo:rerun-if-changed=backend/SPFresh/include/spfresh_c_api.h");
    println!("cargo:rerun-if-changed=backend/SPFresh/include/spfresh/index.hpp");
}

