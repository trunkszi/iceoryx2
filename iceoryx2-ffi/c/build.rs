// Copyright (c) 2024 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

extern crate cbindgen;

use std::env;
use std::path::Path;

#[cfg(unix)]
unsafe extern "C" {
    fn dup2(oldfd: i32, newfd: i32) -> i32;
}

#[cfg(unix)]
fn detach_stdin() {
    use std::fs::File;
    use std::os::fd::AsRawFd;

    if let Ok(dev_null) = File::open("/dev/null") {
        unsafe {
            let _ = dup2(dev_null.as_raw_fd(), 0);
        }
    }
}

#[cfg(not(unix))]
fn detach_stdin() {}

#[cfg(unix)]
fn shell_single_quote(value: &str) -> String {
    let mut quoted = String::from("'");
    quoted.push_str(&value.replace('\'', "'\\''"));
    quoted.push('\'');
    quoted
}

#[cfg(unix)]
fn force_cargo_stdin_to_dev_null(out_dir: &Path) {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let Ok(real_cargo) = env::var("CARGO") else {
        return;
    };

    let cargo_wrapper = out_dir.join("cargo-stdin-null.sh");
    let script = format!(
        "#!/bin/sh\nexec {} \"$@\" </dev/null\n",
        shell_single_quote(&real_cargo)
    );

    if fs::write(&cargo_wrapper, script).is_ok() {
        let _ = fs::set_permissions(&cargo_wrapper, fs::Permissions::from_mode(0o755));
        unsafe {
            env::set_var("CARGO", &cargo_wrapper);
        }
    }
}

#[cfg(not(unix))]
fn force_cargo_stdin_to_dev_null(_out_dir: &Path) {}

fn main() {
    // cbindgen 0.29.x inherits stdin for its nested cargo metadata command.
    // Keep stray parent-process input from being parsed by Cargo's rustc probe.
    detach_stdin();

    // this is the out dir of the iceoryx2-ffi-c crate not the workspace out dir,
    // therefore we need to traverse to a known location and create the path for the header
    let out_dir = env::var("OUT_DIR").expect("Target output directory");
    let out_dir_path = Path::new(&out_dir);
    force_cargo_stdin_to_dev_null(out_dir_path);

    let mut header_path = out_dir_path
        .join("../../../")
        .canonicalize()
        .expect("Path to iceoryx2 base dir for header generation");
    header_path.push("iceoryx2-ffi-c-cbindgen/include/iox2/iceoryx2.h");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("Cargo manifest dir");

    cbindgen::generate(crate_dir)
        .expect("Unable to generate c bindings")
        .write_to_file(header_path);
}
