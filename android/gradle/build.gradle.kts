// La bibliothèque Rust n'est pas compilée par Gradle : ../build.sh lance
// cargo-ndk, qui la dépose dans app/src/main/jniLibs, puis `assembleDebug`.

buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.1.0")
    }
}
