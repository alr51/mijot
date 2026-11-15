// Projet Gradle de l'application Android : il empaquette dans un APK la
// bibliothèque Rust (compilée à part par cargo-ndk, voir ../build.sh) et les
// classes Java hôtes de gpui-mobile.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Mijot"
include(":app")
