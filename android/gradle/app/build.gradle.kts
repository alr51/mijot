// Application Android de Mijot : une NativeActivity (GpuiActivity) qui charge
// la bibliothèque Rust `libmijot_android.so` déposée dans src/main/jniLibs.

plugins {
    id("com.android.application")
}

android {
    namespace = "app.mijot"
    compileSdk = 36

    defaultConfig {
        applicationId = "app.mijot"
        minSdk = 26 // Vulkan 1.0 est garanti à partir de l'API 26.
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"

        // Nom de la bibliothèque chargée par l'activité, sans `lib` ni `.so`.
        manifestPlaceholders["nativeLibraryName"] = "mijot_android"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

// Bibliothèques dont dépendent les classes hôtes de gpui-mobile
// (src/main/java/dev/gpui/mobile), reprises telles quelles de son exemple.
dependencies {
    implementation("androidx.core:core:1.12.0")
    implementation("androidx.core:core-splashscreen:1.0.1")
    implementation("androidx.biometric:biometric:1.1.0")
    implementation("androidx.media:media:1.7.1")
}
