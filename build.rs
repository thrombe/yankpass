

fn main() {
    cxx_build::bridge("src/main.rs")
        .file("src/test.cc")
        // .cpp(true)
        .include("sdk/firebase_cpp_sdk/include")
        // .include("sdk/firebase_cpp_sdk/libs/linux/x86_64/cxx11")
        // .flag("-Lsdk/firebase_cpp_sdk/libs/linux/x86_64/cxx11 -lfirebase_firestore -lfirebase_auth -lfirebase_app -lpthread -lsecret-1 -lgio-2.0 -lgobject-2.0 -lglib-2.0")
        // .flag("-Lsdk/firebase_cpp_sdk/libs/linux/x86_64/cxx11")
        // .flag("-lfirebase_firestore")
        // .flag("-lfirebase_auth")
        // .flag("-lfirebase_app")
        // .flag("-lpthread")
        // .flag("-lsecret-1")
        // .flag("-lgio-2.0")
        // .flag("-lgobject-2.0")
        // .flag("-lglib-2.0")
        // .flag("-Lsdk/firebase_cpp_sdk/libs/linux/x86_64/legacy")
        // .flag_if_supported("-std=c++11")
        .compile("demo");

    println!("cargo:rustc-flags=-Lsdk/firebase_cpp_sdk/libs/linux/x86_64/cxx11 -lfirebase_firestore -lfirebase_auth -lfirebase_app -lpthread -lsecret-1 -lgio-2.0 -lgobject-2.0 -lglib-2.0");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/test.cc");
    println!("cargo:rerun-if-changed=include/test.h");

}



