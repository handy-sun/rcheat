use std::process::Command;

/// Record the build time as an environment variable `RCHEAT_BUILD_TIME`.
fn set_build_time() {
    let args = &["+%Y-%m-%d %H:%M:%S %:z"];
    let Ok(output) = Command::new("date").args(args).output() else {
        return;
    };
    let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if date.is_empty() {
        return;
    }
    println!("cargo:rustc-env=RCHEAT_BUILD_TIME={}", date);
}

/// Make the current git hash available to the build as the environment variable `RCHEAT_BUILD_GIT_HASH`.
fn set_git_revision_hash() {
    let args = &["rev-parse", "--short=7", "HEAD"];
    let Ok(output) = Command::new("git").args(args).output() else {
        return;
    };
    let rev = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if rev.is_empty() {
        return;
    }
    println!("cargo:rustc-env=RCHEAT_BUILD_GIT_HASH={}", rev);
}

/// Make the latest software version according to git tag to the build as the
/// environment variable `RCHEAT_GIT_TAG_VERSION`.
fn set_git_tag_version() {
    let args = &["describe", "--tags", "--abbrev=0"];
    let Ok(output) = Command::new("git").args(args).output() else {
        return;
    };
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tag.is_empty() {
        return;
    }
    println!("cargo:rustc-env=RCHEAT_GIT_TAG_VERSION={}", tag);
}

/// If there are current no pending changes(exclude untracked files), set this env.
fn set_git_is_clean_commit() {
    let args = &["status", "--porcelain"];
    let Ok(output) = Command::new("git").args(args).output() else {
        return;
    };
    if output.stdout.is_empty() {
        println!("cargo:rustc-env=RCHEAT_GIT_IS_CLEAN_COMMIT=1");
    }
}

fn main() {
    set_build_time();
    set_git_revision_hash();
    set_git_tag_version();
    set_git_is_clean_commit();
}
